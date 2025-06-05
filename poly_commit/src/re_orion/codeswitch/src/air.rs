use std::{marker::PhantomData, mem::transmute_copy};

use arith::Field;
use itertools::izip;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir, FilteredAirBuilder};
use p3_field::{Field as P3Field, PrimeField32, PrimeCharacteristicRing};
use p3_matrix::{dense::RowMajorMatrix, Matrix};
use p3_uni_stark::{prove as p3prove, verify as p3verify, StarkConfig};

use encoder::*;

use crate::{P3Config, P3FieldConfig, Plonky3Config};

use super::utils::*;

const TARGET_DISTANCE: f64 = 0.07;
const DISTANCE_THRESHOLD: usize = ((1.0 / TARGET_DISTANCE) - 1.0) as usize;

/// Assumes the field size is at least 16 bits.
#[derive(Debug)]
// TODO: constrain EvalF in ExtensionField?
pub struct CodeSwitchAir<EvalF, ResF: Field> 
where
    EvalF: Field<UnitField = ResF::UnitField>,
    ResF::UnitField: P3FieldConfig,
{
    pub encoder: Encoder<<ResF::UnitField as P3FieldConfig>::P3Field>,
    eval_degree: usize,
    res_pack_size: usize,
    
    pub msg_len: usize,
    pub code_len: usize,
    pub column_size: usize,

    // pub idxs: &'a Vec<usize>,
    pub idxs: Vec<usize>,
    _marker: PhantomData<EvalF>,
}

impl<EvalF, ResF: Field> CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField>,
    ResF::UnitField: P3FieldConfig,
{
    #[inline(always)]
    pub fn new(
        encoder: &Encoder<ResF::UnitField>,
        msg_len: usize,
        code_len: usize,
        column_size: usize,
        idxs: Vec<usize>,
    ) -> Self 
    where
    {
        assert!(EvalF::get_pack_size() == 1 && EvalF::get_degree() == ResF::get_degree());
        Self {
            encoder: Encoder::<<ResF::UnitField as P3FieldConfig>::P3Field>::new_from(encoder),
            eval_degree: EvalF::get_degree(),
            res_pack_size: ResF::get_pack_size(),
            msg_len,
            code_len,
            column_size,
            idxs,
            _marker: PhantomData,
        }
    }

    pub fn prove(
        &self,
        y_gamma: &[ResF],
        y1: &[ResF],
        r1: &[EvalF],
        y: ResF,
        c_gamma: &[ResF],
    ) -> Vec<u8> {
        let witness_size = self.eval_degree * self.res_pack_size;
        let width = self.msg_len * 2 * witness_size;
        let mut trace = <ResF::UnitField as P3FieldConfig>::P3Field::zero_vec(width * 4);
        unsafe { 
            let mut pos = 0;
            std::ptr::copy_nonoverlapping(y_gamma.as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, trace.as_mut_ptr(), witness_size * y_gamma.len()); 
            pos += y_gamma.len() * witness_size;
            std::ptr::copy_nonoverlapping(y1.as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, trace.as_mut_ptr().add(pos), witness_size * y1.len()); 
            pos += y1.len() * witness_size;
            std::ptr::copy_nonoverlapping(c_gamma.as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, trace.as_mut_ptr().add(pos), c_gamma.len() * witness_size);
            pos += c_gamma.len() * witness_size;
        }

        let challenge_size = self.eval_degree;
        // TODO: borrow
        let mut pis = <ResF::UnitField as P3FieldConfig>::P3Field::zero_vec(r1.len() * challenge_size + (c_gamma.len() + 1) * witness_size);
        unsafe {
            std::ptr::copy_nonoverlapping(r1.as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, pis.as_mut_ptr().add(self.encoder.code_len * witness_size), r1.len() * challenge_size);
            std::ptr::copy_nonoverlapping(vec![y].as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, pis.as_mut_ptr().add(self.encoder.code_len * witness_size + r1.len() * challenge_size), witness_size);
        }

        let config = <P3Config as Plonky3Config<ResF::UnitField>>::init();

        let proof = p3prove(&config, self, &mut <P3Config as Plonky3Config<ResF::UnitField>>::get_challenger(), RowMajorMatrix::new(trace, width), &pis);
        serde_cbor::to_vec(&proof).unwrap()
    }

    pub fn verify(
        &self,
        proof: &[u8],
        r1: &[EvalF],
        y: ResF,
        c_gamma: &[ResF],
    ) -> bool 
    {
        let witness_size = self.eval_degree * self.res_pack_size;
        let challenge_size = self.eval_degree;
        // TODO: borrow
        let mut pis = <ResF::UnitField as P3FieldConfig>::P3Field::zero_vec(r1.len() * challenge_size + (c_gamma.len() + 1) * witness_size);
        unsafe {
            std::ptr::copy_nonoverlapping(r1.as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, pis.as_mut_ptr().add(self.encoder.code_len * witness_size), r1.len() * challenge_size);
            std::ptr::copy_nonoverlapping(vec![y].as_ptr() as *const <ResF::UnitField as P3FieldConfig>::P3Field, pis.as_mut_ptr().add(self.encoder.code_len * witness_size + r1.len() * challenge_size), witness_size);
        }

        let config = <P3Config as Plonky3Config<ResF::UnitField>>::init();

        let rst = p3verify(&config, self, &mut <P3Config as Plonky3Config<ResF::UnitField>>::get_challenger(), &serde_cbor::from_slice(proof).unwrap(), &pis);
        if let Err(e) = rst {
            println!("{:?}", e);
            false
        }
        else {
            true
        }
    }
}

impl<PF, EvalF, ResF: Field> BaseAir<PF> for CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField>,
    ResF::UnitField: P3FieldConfig,
{
    fn width(&self) -> usize {
        self.msg_len * 2 * self.eval_degree * self.res_pack_size
    }
}

impl<AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3FieldConfig>::P3Field>, EvalF, ResF: Field> Air<AB> for CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField>,
    ResF::UnitField: P3FieldConfig,
{
    #[inline]
    fn eval(&self, builder: &mut AB) {
        let msg_len = self.msg_len;
        let code_len = self.code_len;
        let witness_size = self.eval_degree * self.res_pack_size;
        let challenge_size = self.eval_degree;
        let public_values = builder.public_values();
        // let gamma = public_values[..pos].to_vec();
        let c_gamma_range = code_len * witness_size;
        let r1_range = msg_len * challenge_size;
        let r1: Vec<AB::Expr> = public_values[c_gamma_range..c_gamma_range + r1_range].iter().map(|&x| x.into()).collect();
        let mut y: Vec<AB::Expr> = public_values[c_gamma_range + r1_range..c_gamma_range + r1_range + witness_size].iter().map(|&x| x.into()).collect();

        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

// println!("inputs {} outputs {}", inputs.len(), outputs.len());
        let y_gamma = &inputs[..msg_len * witness_size];
        let y1 = &inputs[msg_len * witness_size..msg_len * 2 * witness_size];
        // let code: Vec<&[AB::Var]> = Vec::with_capacity(self.idxs.len());
        // code.append(inputs[msg_len * 2..].chunks(self.column_size).collect());
        // let code: Vec<&[AB::Var]> = inputs[msg_len * 2 * witness_size..].chunks(self.column_size).collect();

        let c_gamma = &outputs[..code_len * witness_size];
        // let c1 = &outputs[code_len..code_len * 2];

        let mut check = builder.when_first_row();
        self.encode(&mut check, y_gamma, c_gamma, self.msg_len);
        for (r1_chunk, y1_chunk) in izip!(r1.chunks_exact(self.eval_degree), y1.chunks_exact(witness_size)) {
            let y1expr: Vec<AB::Expr> = y1_chunk.iter().map(|&x| x.into()).collect();
            let mut res = r1_chunk.to_vec();
            for (y_unit, y1_unit) in izip!(y.chunks_exact_mut(self.eval_degree),y1expr.chunks_exact(self.eval_degree)) {
                unit_mul(r1_chunk, y1_unit, &mut res);
                for (u, v) in izip!(y_unit.iter_mut(), res.iter()) {
                    *u -= v.clone();
                }
            }
            // let mut res: Vec<AB::Expr> = y1expr.clone();
            // unit_mul(r1_chunk, &y1expr, &mut res);
// println!("r {:?} y {:?} rst {:?}", r1_chunk, y1expr, res);
            // let ryi = unit_mul(&r1expr, &y1expr, &mut res);
            // for (u, v) in izip!(y.iter_mut(), res.iter()) {
            //     *u -= v.clone();
            // }
        }
        for v in y.iter() {
            check.assert_zero(v.clone());
        }

        /*
        for &idx in &self.idxs {
            check.assert_eq(c_gamma[idx], 
                izip!(&gamma, code[idx]).map(|(&gm, &c)| gm.into() * c).sum::<AB::Expr>()
            );
            check.assert_eq(c1[idx], 
                izip!(&r0, code[idx]).map(|(&r, &c)| r.into() * c).sum::<AB::Expr>()
            );
        } */
    }
}

impl<EvalF, ResF: Field> CodeSwitchAir<EvalF, ResF> 
where
    EvalF: Field<UnitField = ResF::UnitField>,
    ResF::UnitField: P3FieldConfig,
{
    fn encode<'a, AB: AirBuilderWithPublicValues<F = <ResF::UnitField as P3FieldConfig>::P3Field>>(&self, check: &mut FilteredAirBuilder<'a, AB>, src: &[AB::Var], dst: &[AB::Var], n: usize) {
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
println!("coded {}", pos);
    }
}