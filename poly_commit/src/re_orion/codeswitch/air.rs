use std::{marker::PhantomData, mem::transmute_copy};

use arith::Field as ExpField;
use itertools::izip;
use p3_air::{Air, AirBuilder, AirBuilderWithPublicValues, BaseAir, FilteredAirBuilder};
use p3_field::Field as P3Field;
use p3_matrix::Matrix;

use crate::re_orion::{
    parameters::*,
    BiGraph,
};

use super::utils::*;

/// Assumes the field size is at least 16 bits.
#[derive(Debug)]
pub struct CodeSwitchAir<EvalF: ExpField, ResF: ExpField> {
    // TODO: borrow
    pub graph_c: Vec<BiGraph<ResF::UnitField>>,
    pub graph_d: Vec<BiGraph<ResF::UnitField>>,
    
    pub msg_len: usize,
    pub code_len: usize,
    pub column_size: usize,

    // pub idxs: &'a Vec<usize>,
    pub idxs: Vec<usize>,
    _marker: PhantomData<(EvalF, ResF)>,
}

impl<EvalF: ExpField, ResF: ExpField> CodeSwitchAir<EvalF, ResF> {
    #[inline(always)]
    pub fn new(
        graph_c: Vec<BiGraph<ResF::UnitField>>,
        graph_d: Vec<BiGraph<ResF::UnitField>>,
        msg_len: usize,
        code_len: usize,
        column_size: usize,
        idxs: Vec<usize>,
    ) -> Self {
        assert!(EvalF::get_degree() == ResF::get_degree());
        Self {
            graph_c,
            graph_d,
            msg_len,
            code_len,
            column_size,
            idxs,
            _marker: PhantomData,
        }
    }
}

impl<PF, EvalF: ExpField, ResF: ExpField> BaseAir<PF> for CodeSwitchAir<EvalF, ResF> {
    fn width(&self) -> usize {
        self.msg_len * 2 * ResF::get_degree() * ResF::get_pack_size()
    }
}

impl<AB: AirBuilderWithPublicValues, EvalF: ExpField, ResF: ExpField> Air<AB> for CodeSwitchAir<EvalF, ResF> {
    #[inline]
    fn eval(&self, builder: &mut AB) {
println!("eval ? ");
        let msg_len = self.msg_len;
        let code_len = self.code_len;
        let witness_size = ResF::get_degree() * ResF::get_pack_size();
        let challenge_size = EvalF::get_degree() * EvalF::get_pack_size();
        let public_values = builder.public_values();
println!("eval {} {} {}", public_values.len(), msg_len, self.column_size);
        // let gamma = public_values[..pos].to_vec();
        let c_gamma_range = code_len * witness_size;
        let r1_range = msg_len * challenge_size;
        let r1: Vec<AB::Expr> = public_values[c_gamma_range..c_gamma_range + r1_range].iter().map(|&x| x.into()).collect();
        let mut y: Vec<AB::Expr> = public_values[c_gamma_range + r1_range..c_gamma_range + r1_range + witness_size].iter().map(|&x| x.into()).collect();
// println!("in eval r1 {:?}", &r1);

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
        for (r1_chunk, y1_chunk) in izip!(r1.chunks_exact(EvalF::get_degree()), y1.chunks_exact(witness_size)) {
            let y1expr: Vec<AB::Expr> = y1_chunk.iter().map(|&x| x.into()).collect();
            let mut res = r1_chunk.to_vec();
            for (y_unit, y1_unit) in izip!(y.chunks_exact_mut(ResF::get_degree()),y1expr.chunks_exact(ResF::get_degree())) {
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
        // check.assert_eq(y, 
        //     izip!(&r1, y1).map(|(&r, &y_)| r.into() * y_).sum::<AB::Expr>()
        // );

        // let mut check = builder.when_transition();
        // for i in 0..self.msg_len * 2 * witness_size {
        //     check.assert_eq(inputs[i], outputs[i]);
        // }

        // let mut check = builder.when_last_row();

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

impl<EvalF: ExpField, ResF: ExpField> CodeSwitchAir<EvalF, ResF> {
    fn encode<'a, AB: AirBuilderWithPublicValues>(&self, check: &mut FilteredAirBuilder<'a, AB>, src: &[AB::Var], dst: &[AB::Var], n: usize) {
println!("plonky encode {}", n);
        // let mut check = builder.when_first_row();
        let element_size = ResF::get_degree() * ResF::get_pack_size();
println!("element size {}", element_size);
        for (&s, &d) in izip!(src.iter(), dst.iter()).take(n * element_size) {
            check.assert_eq(s, d);
        }
        // for i in 0..n {
        //     check.assert_eq(src[i], dst[i]);
        // }
        if n <= DISTANCE_THRESHOLD {
            return
        }

println!("encode dst len {}", dst.len());
        let mut pos = n;
        let mut dep = 0;
        loop {
            let l = self.graph_c[dep].l_size;
            let r = self.graph_c[dep].r_size;
            let src_ = &dst[(pos - l) * element_size..pos * element_size];
            let dst_ = &dst[pos * element_size..(pos + r) * element_size];
            for (i, chunk) in dst_.chunks_exact(element_size).enumerate() {
                for (j, &v) in chunk.iter().enumerate() {
                    check.assert_eq(v, self.graph_c[dep].edge[l + i].iter().map(|(u, w)| src_[*u * element_size + j] * unsafe { transmute_copy::<_, AB::F>(w) }).sum::<AB::Expr>());
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
            let l = self.graph_d[dep].l_size;
            let r = self.graph_d[dep].r_size;
            let src_ = &dst[(pos - l) * element_size..pos * element_size];
            let dst_ = &dst[pos * element_size..(pos + r) * element_size];
            for (i, chunk) in dst_.chunks_exact(element_size).enumerate() {
                for (j, &v) in chunk.iter().enumerate() {
                    check.assert_eq(v, self.graph_d[dep].edge[l + i].iter().map(|(u, w)| src_[*u * element_size + j] * unsafe { transmute_copy::<_, AB::F>(w) }).sum::<AB::Expr>());
                }
            }
            pos += r;
        }
println!("coded {}", pos);
    }
}