use std::io::Read;

use arith::{ExtensionField, Field};
use ark_std::iterable::Iterable;
use circuit::{Circuit, CircuitLayer, CoefType, GateAdd};
use gkr_engine::{
    ExpanderDualVarChallenge, ExpanderSingleVarChallenge, FieldEngine, MPIConfig, MPIEngine,
    Transcript,
};
use itertools::izip;
use sumcheck::{
    sumcheck_prove_gkr_layer, sumcheck_verify_gkr_layer, ProverScratchPad, VerifierScratchPad,
};

use crate::orion::linear_code::OrionCode;

/// Generate Orion code switching GKR circuit (before queries has been decided).
///
/// The code-switching circuit is decided partially during setup, and the other part is
/// determined during opening, where randomness for queries are decided.
///
/// This method generates the code-switching circuit evaluating Orion evaluation response,
/// encoding evaluation and proximity responses, and eventually the output of the circuit
/// is the MLE poly evaluation, and codewords.
///
/// Once the query randomness is decided, the prover/verifier can append the selection
/// layer to the output of the circuit, by relaying codeword alphabets to the final outputs
/// of the circuit, then start the proving/verifying procedure.
#[allow(unused)]
pub(crate) fn code_switching_gkr_circuit<F, C>(
    encoder: &OrionCode,
    challenge_point: &[C::ChallengeField],
    proximity_rep: usize,
) -> Circuit<C>
where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    assert_eq!(1 << challenge_point.len(), encoder.msg_len());
    assert!((1..=2).contains(&proximity_rep));

    let num_computation_layers = {
        let num_challenge_layers = challenge_point.len();
        let num_encoding_layers = encoder.g0s.len() + encoder.g1s.len();

        std::cmp::max(num_challenge_layers, num_encoding_layers)
    };

    let mut circuit: Circuit<C> = Circuit::default();

    let layer_iter = (0..num_computation_layers).map(|i| {
        let output_var_num = challenge_point.len() + 3;

        let input_var_num = if i == 0 {
            challenge_point.len() + 2
        } else {
            output_var_num
        };

        let mut layer: CircuitLayer<C> = CircuitLayer {
            input_var_num,
            output_var_num,
            ..Default::default()
        };

        code_switching_gkr_layer(&mut layer, encoder, challenge_point, proximity_rep, i);

        layer
    });

    circuit.layers.extend(layer_iter);

    // NOTE(HS) We know it is a bit early to say it is skipping second phase,
    // but even with query randomness, it is still pure addition circuit,
    // so we just identify structure here.
    circuit.identify_structure_info();
    circuit
}

// TODO(HS) prepare query complexity

/// On given an vanilla Orion proof evaluation response and proximity responses,
/// output the input MLE polynomial coefficients for the code switching GKR circuit.
#[allow(unused)]
pub(crate) fn prepare_code_switching_inputs<F: Field>(
    eval_resp: &[F],
    prox_resps: &[Vec<F>],
) -> Vec<F> {
    assert!(eval_resp.len().is_power_of_two());
    let eval_width = eval_resp.len();

    assert!((1..=2).contains(&prox_resps.len()));
    prox_resps
        .iter()
        .for_each(|p| assert_eq!(p.len(), eval_width));

    let mut buffer = eval_resp.to_vec();
    buffer.resize(eval_width * 4, F::ZERO);

    izip!((2..4), prox_resps)
        .for_each(|(i, p)| buffer[i * eval_width..(i + 1) * eval_width].copy_from_slice(p));

    buffer
}

pub(crate) const CODE_SWITCHING_WORLD_SIZE: usize = 1;

#[allow(unused)]
pub(crate) fn prepare_code_switching_gkr_prover_mem<F, C>(
    circuit: &Circuit<C>,
) -> ProverScratchPad<C>
where
    F: Field + ExtensionField,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    let max_i_vars = circuit
        .layers
        .iter()
        .map(|l| l.input_var_num)
        .max()
        .unwrap();

    let max_o_vars = circuit
        .layers
        .iter()
        .map(|layer| layer.output_var_num)
        .max()
        .unwrap();

    ProverScratchPad::<C>::new(max_i_vars, max_o_vars, CODE_SWITCHING_WORLD_SIZE)
}

#[allow(unused)]
pub(crate) fn code_switching_gkr_prove<F, C>(
    circuit: &Circuit<C>,
    sp: &mut ProverScratchPad<C>,
    fs_transcript: &mut impl Transcript<F>,
    mpi_config: &MPIConfig,
) -> (F, ExpanderSingleVarChallenge<C>)
where
    F: Field + ExtensionField,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    assert_eq!(mpi_config.world_size(), CODE_SWITCHING_WORLD_SIZE);

    let layer_num = circuit.layers.len();

    let mut challenge: ExpanderDualVarChallenge<C> =
        ExpanderDualVarChallenge::sample_from_transcript(
            fs_transcript,
            circuit.layers.last().unwrap().output_var_num,
            CODE_SWITCHING_WORLD_SIZE,
        );

    let output_vals = &circuit.layers.last().unwrap().output_vals;
    let claimed_v = C::collectively_eval_circuit_vals_at_expander_challenge(
        output_vals,
        &challenge.challenge_x(),
        &mut sp.hg_evals,
        &mut sp.eq_evals_first_half,
        mpi_config,
    );

    for i in (0..layer_num).rev() {
        sumcheck_prove_gkr_layer(
            &circuit.layers[i],
            &mut challenge,
            None,
            fs_transcript,
            sp,
            mpi_config,
            i == layer_num - 1,
        );

        assert!(challenge.rz_1.is_none());
    }

    // NOTE(HS) since the circuit is purely addition gate, then no rz_1,
    // or in a sense no need to be a dual var challenge.
    assert!(challenge.rz_1.is_none());

    let final_challenge: ExpanderSingleVarChallenge<C> = challenge.into();

    (claimed_v, final_challenge)
}

#[allow(unused)]
pub(crate) fn code_switching_gkr_verify<F, C>(
    circuit: &Circuit<C>,
    claimed_v: &F,
    fs_transcript: &mut impl Transcript<F>,
    mut proof_reader: impl Read,
) -> (bool, ExpanderSingleVarChallenge<C>, F)
where
    F: Field + ExtensionField,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    let mut sp = VerifierScratchPad::<C>::new(circuit, CODE_SWITCHING_WORLD_SIZE);

    let layer_num = circuit.layers.len();

    let mut challenge: ExpanderDualVarChallenge<C> =
        ExpanderDualVarChallenge::sample_from_transcript(
            fs_transcript,
            circuit.layers.last().unwrap().output_var_num,
            CODE_SWITCHING_WORLD_SIZE,
        );

    let mut verified = true;
    let mut claimed_v0 = *claimed_v;
    for i in (0..layer_num).rev() {
        let cur_verified = sumcheck_verify_gkr_layer(
            CODE_SWITCHING_WORLD_SIZE,
            &circuit.layers[i],
            &[],
            &mut challenge,
            &mut claimed_v0,
            &mut None,
            None,
            &mut proof_reader,
            fs_transcript,
            &mut sp,
            i == layer_num - 1,
        );

        verified &= cur_verified;

        assert!(challenge.rz_1.is_none());
    }

    (verified, challenge.into(), claimed_v0)
}

/// A wire that links output gate from lower layer to input gate from higher layer,
/// the wire is weighted, namely the coefficient can be other than ONE.
fn add_wire<F, C>(i_id: usize, o_id: usize, coef: C::ChallengeField) -> GateAdd<C>
where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F>,
{
    GateAdd {
        i_ids: [i_id],
        o_id,
        coef,
        coef_type: CoefType::Constant,
        gate_type: 0,
    }
}

/// A relay wire that links output gate from lower layer to input gate from higher layer.
fn relay<F, C>(i_id: usize, o_id: usize) -> GateAdd<C>
where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F>,
{
    GateAdd {
        i_ids: [i_id],
        o_id,
        coef: C::ChallengeField::ONE,
        coef_type: CoefType::Constant,
        gate_type: 0,
    }
}

/// The tuple describes the position of a partial encoding circuit inside a global circuit.
///
/// The i_srt stands for input starting index from lower layer, while o_srt stands for the
/// output starting index on the higher index.
struct ExpanderEncodingPosition {
    i_srt: usize,
    o_srt: usize,
}

/// This method generates a layer for code switching GKR circuit.
/// The lower index, the layer is nearer to the inputs.
/// MLE polynomial evaluation sequence starts from lowest variable in sumcheck challenge,
/// which is the left-most element in Expander little-endian boolean hypercube.
fn code_switching_gkr_layer<F, C>(
    layer: &mut CircuitLayer<C>,
    encoder: &OrionCode,
    challenge_point: &[C::ChallengeField],
    proximity_rep: usize,
    index: usize,
) where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    // NOTE(HS) MLE evals
    code_switching_gkr_layer_evaluating(layer, challenge_point, index);

    // NOTE(HS) expander code encoding
    let eval_width = encoder.msg_len();
    let scratch_len = eval_width * 2;

    // NOTE(HS) the if condition on input (index == 0) or not is that,
    // the input layer looks like follows:
    //
    // |- eval -|- 0000 -|- prox -|- prox -|
    // in total the input length is 4x eval length
    //
    // while the internal circuit layer looks like follows:
    //
    // |- eval -|- 0000 -|-- eval encode --|-- prox encode --|-- prox encode --|
    // in total the input length is 8x eval length
    //
    // Thus the first layer of circuit looks like
    // |- eval -|- 0000 -|-- eval encode --|-- prox encode --|-- prox encode --|
    // |        |       /                 /                 /                 /
    // |        |    *-*        *--------*     *-----------*           *-----*
    // |        |   /          /              /                       /
    // |        |  /          /              /                       /
    // |        | /          /              /                       /
    // |        |/          /              /                       /
    // | *------* *--------*        *-----*  *--------------------*
    // |/        /        /        /        /
    // |- eval -|- 0000 -|- prox -|- prox -|
    //
    // While the internal layers should look like
    // |- eval -|- 0000 -|-- eval encode --|-- prox encode --|-- prox encode --|
    // |        |        |                 |                 |                 |
    // |        |        |                 |                 |                 |
    // |        |        |                 |                 |                 |
    // |        |        |                 |                 |                 |
    // |        |        |                 |                 |                 |
    // |        |        |                 |                 |                 |
    // |- eval -|- 0000 -|-- eval encode --|-- prox encode --|-- prox encode --|
    //
    // Side note: scratch len is the encode length, or 2x prox/eval length.

    // NOTE(HS) evaluation response encoding circuit
    {
        let enc_position = ExpanderEncodingPosition {
            i_srt: if index == 0 { 0 } else { scratch_len },
            o_srt: scratch_len,
        };

        code_switching_gkr_layer_encoding(layer, encoder, index, enc_position);
    }

    // NOTE(HS) proximity response encoding circuit
    (2..proximity_rep + 2).for_each(|i| {
        let enc_position = ExpanderEncodingPosition {
            i_srt: if index == 0 { eval_width } else { scratch_len } * i,
            o_srt: scratch_len * i,
        };

        code_switching_gkr_layer_encoding(layer, encoder, index, enc_position);
    });
}

fn code_switching_gkr_layer_evaluating<F, C>(
    layer: &mut CircuitLayer<C>,
    challenge_point: &[C::ChallengeField],
    index: usize,
) where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    // NOTE(HS) MLE evals
    let eval_width = 1 << challenge_point.len();
    let evals_input_width = eval_width / (1 << index);
    let evals_output_width = evals_input_width / 2;

    // NOTE(HS) early exit - if output is 0, then relay prev layer evaluation to output
    if evals_output_width == 0 {
        layer.add.push(relay(0, 0));
        return;
    }

    let v = challenge_point[index];

    (0..evals_output_width).for_each(|out_i| {
        layer.add.extend_from_slice(&[
            add_wire(out_i * 2, out_i, C::ChallengeField::ONE - v),
            add_wire(out_i * 2 + 1, out_i, v),
        ]);
    });
}

fn code_switching_gkr_layer_encoding<F, C>(
    layer: &mut CircuitLayer<C>,
    encoder: &OrionCode,
    layer_index: usize,
    pos: ExpanderEncodingPosition,
) where
    F: Field,
    C: FieldEngine<CircuitField = F, ChallengeField = F, SimdCircuitField = F>,
{
    // NOTE(HS) expander code encoding
    let num_encoding_layers = encoder.g0s.len() + encoder.g1s.len();

    // NOTE(HS) early exit if no encoding happens, just relay encoding output
    if layer_index >= num_encoding_layers {
        let relay_iter = (0..encoder.code_len()).map(|i| relay(pos.i_srt + i, pos.o_srt + i));
        layer.add.extend(relay_iter);
        return;
    }

    let graph_ref = &encoder[layer_index];

    // NOTE(HS) clone prev level of inputs
    let relay_iter = (0..graph_ref.output_starts).map(|i| relay(i + pos.i_srt, i + pos.o_srt));
    layer.add.extend(relay_iter);

    // NOTE(HS) position the expander graph fan in
    let i_srt = pos.i_srt + graph_ref.input_starts;
    let o_srt = pos.o_srt + graph_ref.output_starts;

    let neighbors_ref = &graph_ref.graph.neighborings;
    neighbors_ref.iter().enumerate().for_each(|(out_i, in_s)| {
        let enc_iter = in_s.iter().map(|in_i| relay(in_i + i_srt, out_i + o_srt));
        layer.add.extend(enc_iter)
    });
}
