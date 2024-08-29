//! Espresso derivation utilities for rollup integration.
// Ideally we could directly import types and structs from `espresso-sequencer``
// repo. However, one of its dependency, `HotShot` requires a minimum rust version of [1.76.0](https://github.com/EspressoSystems/HotShot/blob/4713af7704f88f38a8a69b708acd4677f76d8ff1/Cargo.toml#L5).
// This is incompatible with the current sp1 rust toolchain 1.75.0.

use block::{
    header::{BlockHeader, BlockMerkleCommitment, BlockMerkleTreeProof},
    payload::{NsProof, VidCommon},
    RollupCommitment,
};
use primitive_types::H256;
use serde::{Deserialize, Serialize};

pub mod block;
pub mod ns_table;

#[derive(Serialize, Deserialize, Debug)]
/// Public inputs
pub struct PublicInputs {
    pub rollup_txs_commit: RollupCommitment,
    /// Hash of the used VID public parameter
    pub vid_param_hash: H256,
    /// Namespace ID of the rollup
    pub ns_id: u32,
    /// Block Merkle tree commitment. Block MT contains information about all
    /// historical blocks up to some block height.
    pub bmt_commitment: BlockMerkleCommitment,
}

#[derive(Serialize, Deserialize, Debug)]
/// Proves that a slice of payload bytes is derived from an espresso block.
pub struct BlockDerivationProof {
    /// A block MT proof for the block header
    pub bmt_proof: BlockMerkleTreeProof,
    /// Block header
    pub block_header: BlockHeader,
    /// Common data associated with the VID disperser, used for namespace proof
    /// verification
    pub vid_common: VidCommon,
    /// Namespace proof of the given payload
    pub ns_proof: NsProof,
}
