// use babybear::BabyBearx16;
use p3_baby_bear::PackedBabyBearAVX512 as BabyBearx16;
use tree::Path;

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldIOPPQuery {
    // NOTE: the folding r's are in sumcheck verification, deriving from Fiat-Shamir.
    iopp_round_query: Vec<BasefoldIOPPQuerySingleRound>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldIOPPQuerySingleRound {
    left: Path,
    right: Path,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BasefoldVirtualIOPPQuery {
    virtual_queries: Vec<BasefoldIOPPQuerySingleRound>,
    iopp_query: BasefoldIOPPQuery,
}

impl BasefoldIOPPQuerySingleRound {
    pub fn check_expected_codeword(
        &self,
        entry_index: usize,
        oracle_len: usize,
        entry: &BabyBearx16,
    ) -> bool {
        let side = &match entry_index & (oracle_len >> 1) {
            0 => self.left.leaf(),
            _ => self.right.leaf(),
        };

        side.data == *entry
    }
}

impl BasefoldVirtualIOPPQuery {
    #[inline]
    fn deteriorate_to_basefold_iopp_query(&self) -> BasefoldIOPPQuery {
        // NOTE: the deterioration happens only when there is only one virtual query,
        // namely, using batch for one single polynomial.
        assert_eq!(self.virtual_queries.len(), 1);

        let mut iopp_round_query = self.virtual_queries.clone();
        iopp_round_query.extend_from_slice(&self.iopp_query.iopp_round_query);
        BasefoldIOPPQuery { iopp_round_query }
    }
}
