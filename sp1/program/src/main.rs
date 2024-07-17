//! This program proves that the executed transactions are correctly derived from an espresso block.
// Inputs:
//  - Namespace ID (public)
//  - Namespace table (public)
//  - VID commitment (public)
//  - Rollup transactions commitment (public)
//  - All transactions
// This program proves that
//  - The namespace table contains an entry of this namespace ID which specifies its byte range in the payload.
//  - Transactions given by two offsets in the (VID) committed payload are the ones committed by the rollup.

#![no_main]
sp1_zkvm::entrypoint!(main);

use committable::Committable;
use espresso_derivation_utils::block::header::{
    BlockHeader, BlockMerkleCommitment, BlockMerkleTree, BlockMerkleTreeProof,
};
use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme};

pub fn main() {
    // Block Merkle tree commitment in the light client state
    let block_mt_comm = sp1_zkvm::io::read::<BlockMerkleCommitment>();
    // public input
    sp1_zkvm::io::commit(&block_mt_comm);

    // The block header
    let header = sp1_zkvm::io::read::<BlockHeader>();
    // Make block height public
    sp1_zkvm::io::commit(&header.height);

    // A membership proof for a given block header in the block Merkle tree
    let mt_proof = sp1_zkvm::io::read::<BlockMerkleTreeProof>();

    // Assert that the membership proof is valid
    assert_eq!(block_mt_comm.height() + 1, mt_proof.proof.len());
    assert!(
        BlockMerkleTree::verify(block_mt_comm.digest(), mt_proof.pos, &mt_proof)
            .unwrap()
            .is_ok()
    );
    // Assert that the header is the one committed in the block Merkle tree
    assert_eq!(&header.commit(), mt_proof.elem().unwrap());

    // let header = mt_proof.elem().unwrap();
    let ns_id = sp1_zkvm::io::read::<u32>();
    // public input
    sp1_zkvm::io::commit(&ns_id);

    let (ns_range_start, ns_range_end) = header
        .ns_table
        .scan_for_id(ns_id)
        .expect("Namespace ID not found.");

    std::println!("Byte range: ({}, {})", ns_range_start, ns_range_end);

    // Temporarily commit the range
    sp1_zkvm::io::commit(&ns_range_start);
    sp1_zkvm::io::commit(&ns_range_end);
}
