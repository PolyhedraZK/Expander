use halo2curves::bn256::Fr;

use crate::MiMCSponge;

pub type MiMCFrTranscriptSponge = MiMCSponge<Fr, Fr>;
