use arith::Field;
use gkr_engine::Transcript;
use polynomials::MultiLinearPoly;

use crate::SumOfProductsPoly;

use super::MatRef;

#[derive(Debug, Clone)]
pub struct MatMulWitnesses<'a, F: Field> {
    pub(crate) a: MatRef<'a, F>,
    pub(crate) b: MatRef<'a, F>,
    pub(crate) c: MatRef<'a, F>,
}

impl<'a, F: Field> MatMulWitnesses<'a, F> {
    #[inline(always)]
    pub fn new(a: MatRef<'a, F>, b: MatRef<'a, F>, c: MatRef<'a, F>) -> Self {
        Self { a, b, c }
    }

    #[inline]
    pub fn form_sumcheck_polynomial(
        &self,
        transcript: &mut impl Transcript,
    ) -> (SumOfProductsPoly<F>, MultiLinearPoly<F>) {
        let r = transcript.generate_field_element::<F>();
        println!("Forming sumcheck polynomial with r = {r:?}");

        let a_rlc_ed = self.a.from_mle_via_rlc(&r);
        let b_transposed = self.b.transpose();
        let mut c_rlc_ed = self.c.from_mle_via_rlc(&r);

        for (b, c_i) in b_transposed.row_vectors_ref().iter().zip(c_rlc_ed.iter()) {
            assert_eq!(
                b.len(),
                a_rlc_ed.len(),
                "Row length mismatch in matrix multiplication witness"
            );
            assert_eq!(inner_product(&a_rlc_ed, b), *c_i)
        }
        c_rlc_ed[0] = F::ZERO;

        let a_mle = MultiLinearPoly { coeffs: a_rlc_ed };
        let b_rows = b_transposed.row_vectors_ref();
        let b_row_mles = b_rows
            .iter()
            .map(|row| MultiLinearPoly {
                coeffs: row.to_vec(),
            })
            .collect::<Vec<_>>();
        let c_mle = MultiLinearPoly {
            coeffs: c_rlc_ed.clone(),
        };
        let neg_one_mle = MultiLinearPoly {
            coeffs: vec![F::ONE; a_mle.coeffs.len()],
        };

        let mut polys = SumOfProductsPoly::new();
        for b_mle in b_row_mles {
            polys.add_pair(a_mle.clone(), b_mle);
        }

        polys.add_pair(c_mle, neg_one_mle);

        (polys, MultiLinearPoly { coeffs: c_rlc_ed })
    }
}

fn inner_product<F: Field>(a: &[F], b: &[F]) -> F {
    assert_eq!(a.len(), b.len(), "Inner product length mismatch");
    a.iter().zip(b.iter()).map(|(x, y)| *x * *y).sum()
}
