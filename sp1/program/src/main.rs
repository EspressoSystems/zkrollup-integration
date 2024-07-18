//! This program proves that the executed transactions are correctly derived from an espresso block.
// Inputs:
//  - (Public)  A block Merkle tree commitment, from a light client state
//  - (Private) A block header containing the block height (public), namespace table, payload VID commitment, etc.
//  - (Private) A membership proof for the header above
//  - (Public)  Namespace ID
//  - (Private) Payload data of this block, with all transactions
//  - (Public)  Rollup transactions commitment
// This program proves that
//  - There's a block header who is a member of the committed block Merkle tree in the light client state.
//  - Given namespace ID appears in the namespace table of this block header, and it specifies a range in the payload data.
//  - The corresponding transactions of rollup commitment are exactly the ones in the given range of the payload data, where the payload data are committed in the block header.

#![no_main]
sp1_zkvm::entrypoint!(main);

use committable::Committable;
use espresso_derivation_utils::{
    block::{
        header::{BlockHeader, BlockMerkleCommitment, BlockMerkleTree, BlockMerkleTreeProof},
        RollupCommitment,
    },
    PublicInputs,
};
use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme};

#[allow(unused_assignments)]
pub fn main() {
    // Indicates that whether all inputs are consistent
    let mut consistency_check = true;

    // Block Merkle tree commitment in the light client state
    let block_merkle_tree_comm = sp1_zkvm::io::read::<BlockMerkleCommitment>();

    // The block header
    let header = sp1_zkvm::io::read::<BlockHeader>();

    // A membership proof for a given block header in the block Merkle tree
    let mt_proof = sp1_zkvm::io::read::<BlockMerkleTreeProof>();

    // Assert that the membership proof is valid
    if block_merkle_tree_comm.height() + 1 != mt_proof.proof.len()
        || !BlockMerkleTree::verify(block_merkle_tree_comm.digest(), mt_proof.pos, &mt_proof)
            .is_ok_and(|result| result.is_ok())
    {
        std::println!("Incorrect membership proof for block Merkle tree");
        consistency_check = false;
    }
    // Assert that the header is the one committed in the block Merkle tree
    if !mt_proof.elem().is_some_and(|elem| elem == &header.commit()) {
        std::println!("Membership proof is not consistent with the given block header.");
        consistency_check = false;
    }

    let ns_id = sp1_zkvm::io::read::<u32>();

    match header.ns_table.scan_for_id(ns_id) {
        None => {
            std::println!("Namespace ID not found in the block");
            consistency_check = false;
        }
        Some((ns_range_start, ns_range_end)) => {
            std::println!("Byte range: ({}, {})", ns_range_start, ns_range_end);
            todo!()
        }
    }

    // Rollup transaction commitment
    let rollup_txs_comm = sp1_zkvm::io::read::<RollupCommitment>();
    // TODO: commitment equivalence

    // Expose the public inputs
    let public_inputs = PublicInputs {
        block_merkle_tree_comm,
        block_height: header.height,
        ns_id,
        rollup_txs_comm,
        consistency_check,
    };
    sp1_zkvm::io::commit(&public_inputs);
}
