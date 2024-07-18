//! Espresso derivation utilities for rollup integration.
// Ideally we could directly import types and structs from `espresso-sequencer`` repo. However, one of its dependency,
// `HotShot` requires a minimum rust version of [1.76.0](https://github.com/EspressoSystems/HotShot/blob/4713af7704f88f38a8a69b708acd4677f76d8ff1/Cargo.toml#L5).
// This is incompatible with the current sp1 rust toolchain 1.75.0.

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
    pub consistency_check: bool,
}
