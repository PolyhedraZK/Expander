use arith::{powers_series, Field};

#[derive(Debug, Clone, Copy)]
pub struct MatRef<'a, F: Field> {
    pub(crate) coeffs: &'a [F],
    pub(crate) rows: usize,
    pub(crate) cols: usize,
}

#[derive(Debug, Clone)]
pub struct Matrix<F: Field> {
    pub(crate) coeffs: Vec<F>,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
}

impl<'a, F: Field> MatRef<'a, F> {
    pub fn mat_mul(self, other: MatRef<'a, F>) -> Vec<F> {
        assert_eq!(
            self.cols, other.rows,
            "Matrix dimensions must match for multiplication"
        );

        let mut result = vec![F::zero(); self.rows * other.cols];

        for i in 0..self.rows {
            for j in 0..other.cols {
                let mut sum = F::zero();
                for k in 0..self.cols {
                    sum += self.coeffs[i * self.cols + k] * other.coeffs[k * other.cols + j];
                }
                result[i * other.cols + j] = sum;
            }
        }

        result
    }

    /// Sample a random element r from the transcript, build a vector [1, r, r^2, ...]
    /// then use it as a coefficient to random linearly combine the rows of the matrix
    pub fn from_mle_via_rlc(&self, rlc: &F) -> Vec<F> {
        let powers_of_r = powers_series::<F>(rlc, self.rows);
        let mut res = vec![F::zero(); self.cols];

        for i in 0..self.rows {
            for j in 0..self.cols {
                res[j] += self.coeffs[i * self.cols + j] * powers_of_r[i];
            }
        }

        res
    }

    pub fn transpose(&self) -> Matrix<F> {
        let mut transposed_coeffs = vec![F::zero(); self.rows * self.cols];

        for i in 0..self.rows {
            for j in 0..self.cols {
                // Element at (i, j) in original matrix goes to (j, i) in transposed matrix
                transposed_coeffs[j * self.rows + i] = self.coeffs[i * self.cols + j];
            }
        }

        Matrix {
            coeffs: transposed_coeffs,
            rows: self.cols, // rows become columns
            cols: self.rows, // columns become rows
        }
    }
}

impl<F: Field> Matrix<F> {
    #[inline(always)]
    pub fn row_vectors(&self) -> Vec<Vec<F>> {
        (0..self.rows)
            .map(|i| self.coeffs[i * self.cols..(i + 1) * self.cols].to_vec())
            .collect()
    }

    #[inline(always)]
    pub fn row_vectors_ref(&self) -> Vec<&[F]> {
        (0..self.rows)
            .map(|i| &self.coeffs[i * self.cols..(i + 1) * self.cols])
            .collect()
    }
}

impl<'a, F: Field> MatRef<'a, F> {
    #[inline(always)]
    pub fn row_vectors(&self) -> Vec<Vec<F>> {
        (0..self.rows)
            .map(|i| self.coeffs[i * self.cols..(i + 1) * self.cols].to_vec())
            .collect()
    }

    #[inline(always)]
    pub fn row_vectors_ref(&self) -> Vec<&[F]> {
        (0..self.rows)
            .map(|i| &self.coeffs[i * self.cols..(i + 1) * self.cols])
            .collect()
    }
}
