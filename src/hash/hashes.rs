use sha2::{
    digest::{generic_array::GenericArray, Output},
    Digest, Sha256, Sha256VarCore,
};
#[derive(Debug, Clone, Default)]
pub struct SHA256hasher {
    pub h: Sha256,
    pub output_size: usize,
}

impl SHA256hasher {
    pub fn new() -> SHA256hasher {
        let mut ret = SHA256hasher {
            h: Sha256::new(),
            output_size: Sha256::output_size(),
        };
        ret.h.reset();
        ret
    }

    // pub fn hash(&self, output: &mut [u8], input: &[u8], input_len: usize) {
    //     let hashed = Sha256::digest(&input[..input_len]);
    //     output.copy_from_slice(&hashed[..]);
    // }

    pub fn hash(&mut self, output: &mut [u8], input: &[u8]) {
        self.h.update(&input);
        self.h
            .finalize_into_reset(Output::<Sha256>::from_mut_slice(output));
    }

    // pub fn hash_inplace(&mut self, buffer: &mut [u8], input_len: usize) {
    //     let hashed = Sha256::digest(&buffer[..input_len]);
    //     buffer.copy_from_slice(&hashed[..]);
    // }

    pub fn hash_inplace(&mut self, buffer: &mut [u8]) {
        self.h.update(&buffer);
        self.h
            .finalize_into_reset(Output::<Sha256>::from_mut_slice(buffer));
    }
}
