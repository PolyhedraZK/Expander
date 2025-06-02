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
    pub _marker: PhantomData<(EvalF, ResF)>,
}

impl<PF: P3Field, EvalF: ExpField, ResF: ExpField> BaseAir<PF> for CodeSwitchAir<EvalF, ResF> {
    fn width(&self) -> usize {
        self.msg_len * 2
    }
}

impl<AB: AirBuilderWithPublicValues, EvalF: ExpField, ResF: ExpField> Air<AB> for CodeSwitchAir<EvalF, ResF> {
    #[inline]
    fn eval(&self, builder: &mut AB) {
println!("eval ? ");
        let msg_len = self.msg_len;
        let code_len = self.code_len;
        let public_values = builder.public_values();
println!("{} {} {}", public_values.len(), msg_len, self.column_size);
        // let gamma = public_values[..pos].to_vec();
        let r1 = public_values[..self.msg_len].to_vec();
        let y = public_values[self.msg_len];

        let main = builder.main();
        let inputs = main.row_slice(0);
        let outputs = main.row_slice(1);

        let y_gamma = &inputs[..msg_len];
        let y1 = &inputs[msg_len..msg_len * 2];
        // let code: Vec<&[AB::Var]> = Vec::with_capacity(self.idxs.len());
        // code.append(inputs[msg_len * 2..].chunks(self.column_size).collect());
        let code: Vec<&[AB::Var]> = inputs[msg_len * 2..].chunks(self.column_size).collect();

        let c_gamma = &outputs[..code_len];
        // let c1 = &outputs[code_len..code_len * 2];

        let mut check = builder.when_first_row();
        self.encode(&mut check, y_gamma, c_gamma, self.msg_len);
        check.assert_eq(y, 
            izip!(&r1, y1).map(|(&r, &y_)| r.into() * y_).sum::<AB::Expr>()
        );

        let mut check = builder.when_transition();
        for i in 0..self.msg_len * 2 {
            check.assert_eq(inputs[i], outputs[i]);
        }

        let mut check = builder.when_last_row();

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
    fn encode<'a, AB: AirBuilderWithPublicValues>(&self, check: &mut FilteredAirBuilder<'a, AB>, src: &[AB::Var], dst: &[AB::Var], n: usize) -> usize {
println!("plonky encode");
        // let mut check = builder.when_first_row();
        for i in 0..n {
            check.assert_eq(src[i], dst[i]);
        }
        if n <= DISTANCE_THRESHOLD {
            return n
        }

        let mut pos = n;
        let mut dep = 0;
        loop {
            let l = self.graph_c[dep].l_size;
            let r = self.graph_c[dep].r_size;
            let src_ = &dst[pos - l..pos];
            let dst_ = &dst[pos..pos + r];
            for (i, &v) in dst_.iter().enumerate() {
                // check.assert_eq(v, self.graph_c[dep].edge[l + i].iter().fold(AB::Expr::zero(), |acc, (u, w)| acc + src_[*u] * unsafe { transmute_copy(w) }));
                check.assert_eq(v, self.graph_c[dep].edge[l + i].iter().map(|(u, w)| src_[*u] * unsafe { transmute_copy::<_, AB::F>(w) }).sum::<AB::Expr>());
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
            let src_ = &dst[pos - l..pos];
            let dst_ = &dst[pos..pos + r];
            for (i, &v) in dst_.iter().enumerate() {
                check.assert_eq(v, self.graph_d[dep].edge[l + i].iter().map(|(u, w)| src_[*u] * unsafe { transmute_copy::<_, AB::F>(w) }).sum::<AB::Expr>());
            }
            pos += r;
        }

        pos
    }
}