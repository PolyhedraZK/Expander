use mersenne31::{M31Ext3, M31};

use crate::{PoseidonM31x16Ext3, PoseidonSponge};

pub type PoseidonM31TranscriptSponge = PoseidonSponge<M31, M31Ext3, PoseidonM31x16Ext3>;
