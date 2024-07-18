use sha2::{Digest, Sha256};
#[derive(Debug, Clone, Default)]
pub struct SHA256hasher;

impl SHA256hasher {
    pub fn hash(&self, output: &mut [u8], input: &[u8], input_len: usize) {
        let hashed = Sha256::digest(&input[..input_len]);
        output.copy_from_slice(&hashed[..]);
    }

    pub fn hash_inplace(&self, buffer: &mut [u8], input_len: usize) {
        let hashed = Sha256::digest(&buffer[..input_len]);
        buffer.copy_from_slice(&hashed[..]);
    }
}
