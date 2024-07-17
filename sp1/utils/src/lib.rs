//! Espresso derivation utilities for rollup integration.

use block::{header::BlockMerkleCommitment, RollupCommitment};
use serde::{Deserialize, Serialize};

pub mod block;
pub mod ns_table;

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Debug)]
/// Public inputs
pub struct PublicInputs {
    pub block_merkle_tree_comm: BlockMerkleCommitment,
    pub block_height: u64,
    pub ns_id: u32,
    pub rollup_txs_comm: RollupCommitment,
}
