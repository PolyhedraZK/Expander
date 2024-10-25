use arith::ExtensionField;
use tree::{Leaf, Node};

use crate::{iop::BasefoldIOPPQuery, BasefoldIOPPQuerySingleRound};

#[derive(Debug, Clone)]
pub struct BasefoldProof<ExtF: ExtensionField> {
    pub(crate) sumcheck_transcript: SumcheckInstanceProof<ExtF::BaseField>,
    pub(crate) iopp_oracles: Vec<Node>,
    pub(crate) iopp_last_oracle_message: Vec<Leaf<ExtF::BaseField>>,
    pub(crate) first_iopp_query: Vec<BasefoldIOPPQuerySingleRound<ExtF::BaseField>>,
    // pub(crate) iopp_queries: Vec<BasefoldIOPPQuery<ExtF>>,
}
