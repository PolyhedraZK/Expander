use arith::{ExtensionField, Field, Fr};
use ark_std::test_rng;
use criterion::black_box;
use gkr_engine::StructuredReferenceString;
use gkr_engine::{root_println, MPIConfig, MPIEngine, Transcript};
use gkr_hashers::{Keccak256hasher, SHA256hasher};
use goldilocks::{Goldilocks, GoldilocksExt2};
use halo2curves::bn256::{Bn256, G1Affine};
use poly_commit::{
    BatchOpeningPCS, HyperUniKZGPCS, HyraxPCS, OrionSIMDFieldPCS, PolynomialCommitmentScheme,
    WhirPCS,
};
use polynomials::MultiLinearPoly;
use rand::RngCore;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;
use utils::timer::Timer;

const NUM_POLY_BATCH_OPEN: usize = 100;

fn main() {
    let universe = MPIConfig::init().unwrap();
    let world = universe.world();
    let mpi_config = MPIConfig::prover_new(Some(&universe), Some(&world));
    println!("==========================");
    for num_vars in 10..21 {
        root_println!(mpi_config, "num vars: {}", num_vars);
        bench_whir(&mpi_config, num_vars);
        bench_orion(&mpi_config, num_vars);
        bench_kzg(&mpi_config, num_vars);
        bench_hyrax(&mpi_config, num_vars);
        println!("==========================");
    }
}

fn bench_whir(mpi_config: &MPIConfig, num_vars: usize) {
    // full scalar
    let mut rng = test_rng();

    let params = WhirPCS::random_params(num_vars, &mut rng);
    let (srs, _) = WhirPCS::gen_srs_for_testing(&params, &mut rng);

    let poly = MultiLinearPoly::<Goldilocks>::random(num_vars, &mut rng);
    let eval_point: Vec<_> = (0..num_vars)
        .map(|_| GoldilocksExt2::random_unsafe(&mut rng))
        .collect();
    pcs_bench::<WhirPCS, GoldilocksExt2>(
        mpi_config,
        &params,
        &srs,
        &poly,
        &eval_point,
        "Whir goldilocks   ",
    );
}

fn bench_orion(mpi_config: &MPIConfig, num_vars: usize) {
    let mut rng = test_rng();
    {
        // Bn scalar
        let (srs, _) =
            OrionSIMDFieldPCS::<Fr, Fr, Fr, Fr>::gen_srs_for_testing(&num_vars, &mut rng);

        let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);
        let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

        pcs_bench::<OrionSIMDFieldPCS<Fr, Fr, Fr, Fr>, Fr>(
            mpi_config,
            &num_vars,
            &srs,
            &poly,
            &eval_point,
            "orion Fr ",
        );
    }
}

fn bench_hyrax(mpi_config: &MPIConfig, num_vars: usize) {
    // full scalar
    let mut rng = test_rng();
    let (srs, _) = HyraxPCS::<G1Affine>::gen_srs_for_testing(&num_vars, &mut rng);

    let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);
    let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

    pcs_bench::<HyraxPCS<G1Affine>, Fr>(
        mpi_config,
        &num_vars,
        &srs,
        &poly,
        &eval_point,
        "hyrax full scalar ",
    );

    // small scalar
    let input = (0..1 << num_vars)
        .map(|_| Fr::from(rng.next_u32()))
        .collect::<Vec<_>>();
    let poly = MultiLinearPoly::<Fr>::new(input);
    pcs_bench::<HyraxPCS<G1Affine>, Fr>(
        mpi_config,
        &num_vars,
        &srs,
        &poly,
        &eval_point,
        "hyrax small scalar",
    );

    // batch open
    bench_batch_open::<HyraxPCS<G1Affine>, Fr>(mpi_config, num_vars, NUM_POLY_BATCH_OPEN);
}

fn bench_kzg(mpi_config: &MPIConfig, num_vars: usize) {
    // full scalar
    let mut rng = test_rng();
    let (srs, _) = HyperUniKZGPCS::<Bn256>::gen_srs_for_testing(&num_vars, &mut rng);

    let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);
    let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

    pcs_bench::<HyperUniKZGPCS<Bn256>, Fr>(
        mpi_config,
        &num_vars,
        &srs,
        &poly,
        &eval_point,
        "kzg full scalar   ",
    );

    // small scalar
    let input = (0..1 << num_vars)
        .map(|_| Fr::from(rng.next_u32()))
        .collect::<Vec<_>>();
    let poly = MultiLinearPoly::<Fr>::new(input);
    pcs_bench::<HyperUniKZGPCS<Bn256>, Fr>(
        mpi_config,
        &num_vars,
        &srs,
        &poly,
        &eval_point,
        "kzg small scalar  ",
    );

    // batch open
    bench_batch_open::<HyperUniKZGPCS<Bn256>, Fr>(mpi_config, num_vars, NUM_POLY_BATCH_OPEN);
}

fn pcs_bench<PCS, F>(
    mpi_config: &MPIConfig,
    params: &PCS::Params,
    srs: &PCS::SRS,
    poly: &PCS::Poly,
    eval_point: &PCS::EvalPoint,
    label: &str,
) where
    PCS: PolynomialCommitmentScheme<F>,
    F: Field + ExtensionField,
{
    let timer = Timer::new(
        format!("{} commit    ", label).as_ref(),
        mpi_config.is_root(),
    );

    let (pk, vk) = srs.clone().into_keys();

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut scratch_pad = PCS::init_scratch_pad(&params);

    let com = black_box(PCS::commit(&params, &pk, &poly, &mut scratch_pad));
    timer.stop();

    let timer = Timer::new(
        format!("{} open      ", label).as_ref(),
        mpi_config.is_root(),
    );
    let (eval, open) = black_box(PCS::open(
        &params,
        &com,
        &pk,
        &poly,
        &eval_point,
        &mut scratch_pad,
        &mut transcript,
    ));
    timer.stop();

    let timer = Timer::new(
        format!("{} verify    ", label).as_ref(),
        mpi_config.is_root(),
    );
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    assert!(black_box(PCS::verify(
        &params,
        &vk,
        &com,
        &eval_point,
        eval,
        &open,
        &mut transcript,
    )));
    timer.stop();

    let mut buf = vec![];
    com.serialize_into(&mut buf).unwrap();
    let com_size = buf.len();

    let mut buf = vec![];
    open.serialize_into(&mut buf).unwrap();
    let open_size = buf.len();

    root_println!(
        mpi_config,
        "{}",
        format!("{} commit size    {}", label, com_size),
    );
    root_println!(
        mpi_config,
        "{}",
        format!("{} open size      {}", label, open_size),
    );

    root_println!(mpi_config, "  --- ");
}

fn bench_batch_open<PCS, F>(mpi_config: &MPIConfig, num_vars: usize, num_poly: usize)
where
    PCS: BatchOpeningPCS<F, Params = usize, EvalPoint = Vec<F>, Poly = MultiLinearPoly<F>>,
    F: Field + ExtensionField,
{
    let mut rng = test_rng();

    let (srs, _) = PCS::gen_srs_for_testing(&num_vars, &mut rng);
    let (proving_key, verification_key) = srs.into_keys();
    let mut scratch_pad = PCS::init_scratch_pad(&num_vars);

    let polys = (0..num_poly)
        .map(|_| MultiLinearPoly::<F>::random(num_vars, &mut rng))
        .collect::<Vec<_>>();

    let points = (0..num_poly)
        .map(|_| {
            (0..num_vars)
                .map(|_| F::random_unsafe(&mut rng))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let commitments = polys
        .iter()
        .map(|poly| PCS::commit(&num_vars, &proving_key, poly, &mut scratch_pad))
        .collect::<Vec<_>>();
    let mut buf = vec![];
    commitments.serialize_into(&mut buf).unwrap();
    let com_size = buf.len();

    let mut transcript = BytesHashTranscript::<SHA256hasher>::new();
    let timer = Timer::new(
        format!("{} batch open {} polys   ", PCS::NAME, num_poly).as_ref(),
        mpi_config.is_root(),
    );
    let (values, batch_opening) = PCS::multiple_points_batch_open(
        &num_vars,
        &proving_key,
        &polys,
        &points,
        &mut scratch_pad,
        &mut transcript,
    );

    timer.stop();

    let mut buf = vec![];
    values.serialize_into(&mut buf).unwrap();
    batch_opening.serialize_into(&mut buf).unwrap();
    let open_size = buf.len();

    let mut transcript = BytesHashTranscript::<SHA256hasher>::new();
    let timer = Timer::new(
        format!("{} batch verify {} polys ", PCS::NAME, num_poly).as_ref(),
        mpi_config.is_root(),
    );
    assert!(PCS::multiple_points_batch_verify(
        &num_vars,
        &verification_key,
        &commitments,
        &points,
        &values,
        &batch_opening,
        &mut transcript
    ));
    timer.stop();

    root_println!(
        mpi_config,
        "{}",
        format!(
            "{} batch {} poly commit size  {}",
            PCS::NAME,
            num_poly,
            com_size
        ),
    );
    root_println!(
        mpi_config,
        "{}",
        format!(
            "{} batch {} poly open size    {}",
            PCS::NAME,
            num_poly,
            open_size
        ),
    );

    root_println!(mpi_config, "  --- ");
}
