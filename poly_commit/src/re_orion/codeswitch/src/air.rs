use std::marker::PhantomData;

use arith::Field;
use itertools::izip;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir, FilteredAirBuilder};
use p3_field::PrimeCharacteristicRing;
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{prove as p3prove, verify as p3verify};

use tracing_forest::util::LevelFilter;
use tracing_forest::ForestLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};


use encoder::*;

use crate::{utils::Timer, P3Config, P3Multiply};

pub const CHALLENGE_SIZE: usize = 1500;

#[derive(Debug)]
pub struct CodeSwitchAir<EvalF, ResF: Field> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF> + P3Multiply<EvalF>,
    ResF::UnitField: P3Config,
{
    pub encoder: Encoder<<ResF::UnitField as P3Config>::Val>,
    eval_degree: usize,
    res_pack_size: usize,
    
    pub msg_len: usize,
    pub code_len: usize,
    pub column_size: usize,
    pub r1_log: usize,
    head_mask: usize,
    tail_source: usize,

    pub idxs: Vec<usize>,
    _marker: PhantomData<(ResF, EvalF)>,
}

impl<EvalF, ResF: Field> CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF> + P3Multiply<EvalF>,
    ResF::UnitField: P3Config,
{
    #[inline(always)]
    pub fn new(
        encoder: &Encoder<ResF::UnitField>,
        msg_len: usize,
        column_size: usize,
        idxs: Vec<usize>,
        r1_log: usize,
    ) -> Self 
    where
    {
        assert!(EvalF::get_pack_size() == 1 && EvalF::get_degree() == ResF::get_degree());


let env_filter = EnvFilter::builder()
    .with_default_directive(LevelFilter::DEBUG.into())
    .from_env_lossy();
Registry::default()
    .with(env_filter)
    .with(ForestLayer::default())
    .init();

        Self {
            encoder: Encoder::<<ResF::UnitField as P3Config>::Val>::new_from(encoder),
            eval_degree: EvalF::get_degree(),
            res_pack_size: ResF::get_pack_size(),
            msg_len,
            code_len: encoder.code_len,
            column_size,
            r1_log,
            head_mask: (1 << (r1_log >> 1)) - 1,
            tail_source: r1_log >> 1,
            idxs,
            _marker: PhantomData,
        }
    }

    pub fn prove(
        &self,
        y_gamma: &[ResF],
        y1: &[ResF],
        c_gamma: &[ResF],
        c_gamma_idx: &[ResF],
        r1: &[EvalF],
        y: &ResF,
    ) -> Vec<u8> {
        let witness_size = self.eval_degree * self.res_pack_size;
        let width = self.code_len * witness_size;
        let mut trace = <ResF::UnitField as P3Config>::Val::zero_vec(width * 4);
        // TODO: unify arrange mem
        unsafe { 
            std::ptr::copy_nonoverlapping(y_gamma.as_ptr() as *const <ResF::UnitField as P3Config>::Val, trace.as_mut_ptr(), witness_size * y_gamma.len()); 
            std::ptr::copy_nonoverlapping(c_gamma.as_ptr() as *const <ResF::UnitField as P3Config>::Val, trace.as_mut_ptr().add(width), c_gamma.len() * witness_size);
            std::ptr::copy_nonoverlapping(y1.as_ptr() as *const <ResF::UnitField as P3Config>::Val, trace.as_mut_ptr().add(width * 3), witness_size * y1.len()); 
            // pos += c_gamma.len() * witness_size;
        }
println!("trace len {}", trace.len());

        let challenge_size = self.eval_degree;
        // TODO: borrow
        // TODO: scratch pis
        let mut pis = <ResF::UnitField as P3Config>::Val::zero_vec(r1.len() * challenge_size + (c_gamma_idx.len() + 1) * witness_size);
        unsafe {
            std::ptr::copy_nonoverlapping(c_gamma_idx.as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr(), c_gamma_idx.len() * witness_size);
            std::ptr::copy_nonoverlapping(r1.as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr().add(c_gamma_idx.len() * witness_size), r1.len() * challenge_size);
            std::ptr::copy_nonoverlapping(vec![*y].as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr().add(c_gamma_idx.len() * witness_size + r1.len() * challenge_size), witness_size);
        }
println!("{} {} ", r1.len(), self.r1_log);
println!("pis len {}", pis.len());

        ResF::UnitField::p3prove(self, width, trace, &pis)
    }

    pub fn verify(
        &self,
        proofu8: &[u8],
        c_gamma_idx: &[ResF],
        r1: &[EvalF],
        y: &ResF,
    ) -> bool 
    {
        let witness_size = self.eval_degree * self.res_pack_size;
        let challenge_size = self.eval_degree;
        // TODO: borrow
        let mut pis = <ResF::UnitField as P3Config>::Val::zero_vec(r1.len() * challenge_size + (c_gamma_idx.len() + 1) * witness_size);
        unsafe {
            std::ptr::copy_nonoverlapping(c_gamma_idx.as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr(), c_gamma_idx.len() * witness_size);
            std::ptr::copy_nonoverlapping(r1.as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr().add(c_gamma_idx.len() * witness_size), r1.len() * challenge_size);
            std::ptr::copy_nonoverlapping(vec![*y].as_ptr() as *const <ResF::UnitField as P3Config>::Val, pis.as_mut_ptr().add(c_gamma_idx.len() * witness_size + r1.len() * challenge_size), witness_size);
        }
println!("pis len in verify {}", pis.len());

        ResF::UnitField::p3verify(self, proofu8, &pis)
    }
}

impl<PF, EvalF, ResF: Field> BaseAir<PF> for CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF> + P3Multiply<EvalF>,
    ResF::UnitField: P3Config,
{
    fn width(&self) -> usize {
        self.code_len * self.eval_degree * self.res_pack_size
    }
}

impl<AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3Config>::Val>, EvalF, ResF: Field> Air<AB> for CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF> + P3Multiply<EvalF>,
    ResF::UnitField: P3Config,
{
    #[inline]
    fn eval(&self, builder: &mut AB) {
        let msg_len = self.msg_len;
        let code_len = self.code_len;
        let witness_size = self.eval_degree * self.res_pack_size;
        let challenge_size = self.eval_degree;
        let public_values = builder.public_values();
        let c_gamma_range = self.idxs.len() * witness_size;
        let c_gamma_idx: Vec<AB::Expr> = public_values[..c_gamma_range].iter().map(|&x| x.into()).collect();
        let r1_range = self.r1_log * challenge_size;
        // let (r1_head, r1_tail) = self.eval_r1::<AB>(&public_values[c_gamma_range..c_gamma_range + r1_range]);
        // let r1_range = msg_len * challenge_size;
        // let r1: Vec<AB::Expr> = public_values[c_gamma_range..c_gamma_range + r1_range].iter().map(|&x| x.into()).collect();
        let mut y: Vec<AB::Expr> = public_values[c_gamma_range + r1_range..c_gamma_range + r1_range + witness_size].iter().map(|&x| x.into()).collect();

        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

        let y_gamma = &inputs[..msg_len * witness_size];
        let c_gamma = &outputs[..code_len * witness_size];

        let mut check = builder.when_first_row();

        self.encode(&mut check, y_gamma, c_gamma, self.msg_len);
        for (i, idx) in self.idxs.iter().enumerate() {
            for (u, v) in izip!(c_gamma[idx * witness_size..(idx + 1) * witness_size].iter(), c_gamma_idx[i * witness_size..(i + 1) * witness_size].iter()) {
                check.assert_eq(*u, v.clone());
            }
        }

        /*
        let mut check = builder.when_last_row();
        let y1 = &inputs[..msg_len * witness_size];
        let mut res: Vec<AB::Expr> = vec![AB::F::ZERO.into(); witness_size];
        let mut r1: Vec<AB::Expr> = vec![AB::F::ZERO.into(); challenge_size];
        for (i, y1_chunk) in y1.chunks_exact(witness_size).enumerate() {
            let y1expr: Vec<AB::Expr> = y1_chunk.iter().map(|&x| x.into()).collect();
            let hi = i & self.head_mask;
            let ti = i >> self.tail_source;
            <EvalF as P3Multiply::<EvalF>>::p3mul(&r1_head[hi..hi + challenge_size], &r1_tail[ti.. ti + challenge_size], &mut r1);
            <EvalF as P3Multiply::<ResF>>::p3mul(&r1, &y1expr, &mut res);
            for (u, v) in izip!(y.iter_mut(), res.iter()) {
                *u -= v.clone();
            }
        } */
        /*
        for (r1_chunk, y1_chunk) in izip!(r1.chunks_exact(self.eval_degree), y1.chunks_exact(witness_size)) {
            let y1expr: Vec<AB::Expr> = y1_chunk.iter().map(|&x| x.into()).collect();
            let mut res = y1expr.to_vec();
            <EvalF as P3Multiply::<ResF>>::p3mul(r1_chunk, &y1expr, &mut res);
            for (u, v) in izip!(y.iter_mut(), res.iter()) {
                *u -= v.clone();
            }
        } */
        // for v in y.iter() {
        //     check.assert_zero(v.clone());
        // }
    }
}

impl<EvalF, ResF: Field> CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField> + P3Multiply<ResF> + P3Multiply<EvalF>,
    ResF::UnitField: P3Config,
{
    fn encode<'a, AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3Config>::Val>>(&self, check: &mut FilteredAirBuilder<'a, AB>, src: &[AB::Var], dst: &[AB::Var], n: usize) {
        let element_size = self.eval_degree * self.res_pack_size;
        for (&s, &d) in izip!(src.iter(), dst.iter()).take(n * element_size) {
            check.assert_eq(s, d);
        }
        if n <= DISTANCE_THRESHOLD {
            return
        }

        let mut pos = n;
        let mut dep = 0;
        loop {
            let l = self.encoder.c[dep].l_size;
            let r = self.encoder.c[dep].r_size;
            let src_ = &dst[(pos - l) * element_size..pos * element_size];
            let dst_ = &dst[pos * element_size..(pos + r) * element_size];
            for (i, chunk) in dst_.chunks_exact(element_size).enumerate() {
                for (j, &v) in chunk.iter().enumerate() {
                    check.assert_eq(v, self.encoder.c[dep].edge[l + i].iter().map(|(u, w)| src_[*u * element_size + j] * *w).sum::<AB::Expr>());
                }
                // check.assert_eq(v, self.graph_c[dep].edge[l + i].iter().fold(AB::Expr::zero(), |acc, (u, w)| acc + src_[*u] * unsafe { transmute_copy(w) }));
            }
            dep += 1;
            pos += r;
            if r <= DISTANCE_THRESHOLD {
                break
            }
        }

        for dep in (0..dep).rev() {
            let l = self.encoder.d[dep].l_size;
            let r = self.encoder.d[dep].r_size;
            let src_ = &dst[(pos - l) * element_size..pos * element_size];
            let dst_ = &dst[pos * element_size..(pos + r) * element_size];
            for (i, chunk) in dst_.chunks_exact(element_size).enumerate() {
                for (j, &v) in chunk.iter().enumerate() {
                    check.assert_eq(v, self.encoder.d[dep].edge[l + i].iter().map(|(u, w)| src_[*u * element_size + j] * *w).sum::<AB::Expr>());
                }
            }
            pos += r;
        }
    }

    fn eval_points<AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3Config>::Val>>(&self, points: &[AB::PublicVar]) -> Vec<AB::Expr> {
        let size = self.eval_degree;
        let mut res: Vec<AB::Expr> = vec![AB::F::ONE.into(); (1 << (points.len() / size)) * size];
        for (i, r) in points.chunks_exact(size).enumerate() {
            let n = 1 << i;
            let rexpr: Vec<AB::Expr> = r.iter().map(|&x| x.into()).collect();
            let (left, right) = res.split_at_mut(n * size);
            for (lst, nxt) in izip!(left.chunks_exact_mut(size), right.chunks_exact_mut(size)) {
                <EvalF as P3Multiply::<EvalF>>::p3mul(lst, &rexpr, nxt);
                for (u, v) in izip!(lst.iter_mut(), nxt.iter_mut()) {
                    *u -= v.clone();
                }
            }
        }
        res
    }

    fn eval_r1<AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3Config>::Val>>(&self, points: &[AB::PublicVar]) -> (Vec<AB::Expr>, Vec<AB::Expr>) {
        let head = self.eval_points::<AB>(&points[..self.tail_source * self.eval_degree]);
        let tail = self.eval_points::<AB>(&points[self.tail_source * self.eval_degree..]);
        (head, tail)
    }
}