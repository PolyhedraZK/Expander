use std::{default, marker::PhantomData};

use arith::Field;

use super::FiatShamirHash;

#[derive(Debug, Clone, Default)]
pub struct MIMCHasher<F: Field> {
    _phantom: PhantomData<F>,
}

impl<F: Field> FiatShamirHash for MIMCHasher<F> {
    const DIGEST_SIZE: usize = F::SIZE;

    #[inline]
    fn new() -> MIMCHasher<F> {
        Self::default()
    }
    
    fn hash(output: &mut [u8], input: &[u8]) {
        todo!()
    }
    
    fn hash_inplace(buffer: &mut [u8]) {
        todo!()
    }

    
}

