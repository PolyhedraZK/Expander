use arith::{Field, Fr};
use ark_std::test_rng;
use criterion::black_box;
use gkr_engine::StructuredReferenceString;
use gkr_engine::{root_println, MPIConfig, MPIEngine, Transcript};
use gkr_hashers::Keccak256hasher;
use halo2curves::bn256::{Bn256, G1Affine};
use poly_commit::{HyperKZGPCS, HyraxPCS, PolynomialCommitmentScheme};
use polynomials::MultiLinearPoly;
use rand::RngCore;
use serdes::ExpSerde;
use transcript::BytesHashTranscript;
use utils::timer::Timer;

fn main() {
    let universe = MPIConfig::init().unwrap();
    let world = universe.world();
    let mpi_config = MPIConfig::prover_new(&universe, &world);
    println!("==========================");
    for num_vars in 10..19 {
        root_println!(mpi_config, "num vars: {}", num_vars);
        bench_hyrax(&mpi_config, num_vars);
        bench_kzg(&mpi_config, num_vars);
        println!("==========================");
    }
}

fn bench_hyrax(mpi_config: &MPIConfig, num_vars: usize) {
    // full scalar
    let mut rng = test_rng();

    let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);
    bench_hyrax_helper(mpi_config, num_vars, &poly, "full scalar ");

    // small scalar
    let input = (0..1 << num_vars)
        .map(|_| Fr::from(rng.next_u32()))
        .collect::<Vec<_>>();
    let poly = MultiLinearPoly::<Fr>::new(input);
    bench_hyrax_helper(mpi_config, num_vars, &poly, "small scalar");
}

fn bench_hyrax_helper(
    mpi_config: &MPIConfig,
    num_vars: usize,
    poly: &MultiLinearPoly<Fr>,
    label: &str,
) {
    let mut rng = test_rng();
    let timer = Timer::new(
        format!("{} hyrax commit    ", label).as_ref(),
        mpi_config.is_root(),
    );

    let (srs, _) = HyraxPCS::<G1Affine>::gen_srs_for_testing(&num_vars, &mut rng);

    let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut scratch_pad = ();

    let com = black_box(HyraxPCS::<G1Affine>::commit(
        &num_vars,
        &srs,
        &poly,
        &mut scratch_pad,
    ));
    timer.stop();

    let timer = Timer::new(
        format!("{} hyrax open      ", label).as_ref(),
        mpi_config.is_root(),
    );
    let (eval, open) = black_box(HyraxPCS::<G1Affine>::open(
        &num_vars,
        &srs,
        &poly,
        &eval_point,
        &scratch_pad,
        &mut transcript,
    ));
    timer.stop();

    let timer = Timer::new(
        format!("{} hyrax verify    ", label).as_ref(),
        mpi_config.is_root(),
    );
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    assert!(black_box(HyraxPCS::<G1Affine>::verify(
        &num_vars,
        &srs,
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

    root_println!(mpi_config, "hyrax com size       {}", com_size);
    root_println!(mpi_config, "hyrax open size      {}", open_size);

    root_println!(mpi_config, "  --- ");
}

fn bench_kzg(mpi_config: &MPIConfig, num_vars: usize) {
    // full scalar
    let mut rng = test_rng();

    let poly = MultiLinearPoly::<Fr>::random(num_vars, &mut rng);
    bench_kzg_helper(mpi_config, num_vars, &poly, "full scalar ");

    // small scalar
    let input = (0..1 << num_vars)
        .map(|_| Fr::from(rng.next_u32()))
        .collect::<Vec<_>>();
    let poly = MultiLinearPoly::<Fr>::new(input);
    bench_kzg_helper(mpi_config, num_vars, &poly, "small scalar");
}

fn bench_kzg_helper(
    mpi_config: &MPIConfig,
    num_vars: usize,
    poly: &MultiLinearPoly<Fr>,
    label: &str,
) {
    let mut rng = test_rng();

    let (srs, _) = HyperKZGPCS::<Bn256>::gen_srs_for_testing(&num_vars, &mut rng);
    let (pk, vk) = srs.clone().into_keys();
    let eval_point: Vec<_> = (0..num_vars).map(|_| Fr::random_unsafe(&mut rng)).collect();

    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    let mut scratch_pad = HyperKZGPCS::<Bn256>::init_scratch_pad(&num_vars);

    let timer = Timer::new(
        format!("{} kzg commit      ", label).as_ref(),
        mpi_config.is_root(),
    );
    let com = black_box(HyperKZGPCS::<Bn256>::commit(
        &num_vars,
        &pk,
        &poly,
        &mut scratch_pad,
    ));
    timer.stop();

    let timer = Timer::new(
        format!("{} kzg open        ", label).as_ref(),
        mpi_config.is_root(),
    );
    let (eval, open) = black_box(HyperKZGPCS::<Bn256>::open(
        &num_vars,
        &pk,
        &poly,
        &eval_point,
        &scratch_pad,
        &mut transcript,
    ));
    timer.stop();

    let timer = Timer::new(
        format!("{} kzg verify      ", label).as_ref(),
        mpi_config.is_root(),
    );
    let mut transcript = BytesHashTranscript::<Keccak256hasher>::new();
    assert!(black_box(HyperKZGPCS::<Bn256>::verify(
        &num_vars,
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

    root_println!(mpi_config, "kzg com size         {}", com_size);
    root_println!(mpi_config, "kzg open size        {}", open_size);

    root_println!(mpi_config, "  --- ");
}
