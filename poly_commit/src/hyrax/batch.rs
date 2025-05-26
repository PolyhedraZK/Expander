//! credit: https://github.com/EspressoSystems/hyperplonk/blob/main/subroutines/src/pcs/multilinear_kzg/batching.rs#L43

use std::collections::BTreeMap;

use arith::{ExtensionField, Field};
use ark_std::log2;
use gkr_engine::Transcript;
use halo2curves::{ff::PrimeField, CurveAffine};
use polynomials::{EqPolynomial, MultiLinearPoly, MultilinearExtension};
use serdes::ExpSerde;

use super::{HyraxOpening, PedersenParams};

// batch open a set of mle_polys at the same point
// returns a set of eval points and a signle opening
// NOTE: random linear combination is used to merge polynomials
pub(crate) fn hyrax_batch_open<C>(
    params: &PedersenParams<C>,
    mle_poly_list: &[impl MultilinearExtension<C::Scalar>],
    eval_points: &[&[C::Scalar]],
    transcript: &mut impl Transcript,
) -> (Vec<C::Scalar>, HyraxOpening<C>)
where
    C: CurveAffine + ExpSerde,
    C::Scalar: ExtensionField + PrimeField,
    C::ScalarExt: ExtensionField + PrimeField,
{
    // sanity checks
    assert_eq!(mle_poly_list.len(), eval_points.len());

    let num_var = mle_poly_list[0].num_vars();
    let k = mle_poly_list.len();
    let ell = log2(k) as usize;

    for poly in mle_poly_list {
        assert_eq!(poly.num_vars(), num_var);
    }

    // compute the evaluation of each polynomial at the given point
    // todo: check if the evals are already computed by the caller and if so
    // pass the data in
    let mut buf = vec![C::Scalar::zero(); mle_poly_list[0].hypercube_size()];
    let evals = mle_poly_list
        .iter()
        .zip(eval_points.iter())
        .map(|(poly, point)| poly.evaluate_with_buffer(point, &mut buf));

    // set the transcript state
    for &point in eval_points {
        for element in point {
            transcript.append_serializable_data(element);
        }
    }
    for e in evals {
        transcript.append_serializable_data(&e);
    }

    // challenge point t
    let t = transcript.generate_field_elements::<C::Scalar>(ell);

    // eq(t, i) for i in [0..k]
    let eq_t_i_list = EqPolynomial::<C::Scalar>::build_eq_x_r(t.as_ref());

    // combine the polynomials that have same opening point first to reduce the
    // cost of sum check later.
    let point_indices = eval_points
        .iter()
        .fold(BTreeMap::<_, _>::new(), |mut indices, point| {
            let idx = indices.len();
            indices.entry(point).or_insert(idx);
            indices
        });
    let deduped_points =
        BTreeMap::from_iter(point_indices.iter().map(|(point, idx)| (*idx, *point)))
            .into_values()
            .collect::<Vec<_>>();

    todo!()
}
