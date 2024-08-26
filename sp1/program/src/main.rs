//! This program proves that the executed transactions are correctly derived
//! from espresso blocks.

#![no_main]
sp1_zkvm::entrypoint!(main);

use committable::Committable;
use espresso_derivation_utils::{
    block::{
        header::{BlockMerkleCommitment, BlockMerkleTree},
        payload::{compute_vid_param_hash, rollup_commit, vid_scheme, Payload, Vid, VidParam},
    },
    BlockDerivationProof, PublicInputs,
};
use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme};
use jf_vid::{
    payload_prover::{PayloadProver, Statement},
    VidScheme,
};
use std::ops::Range;

pub fn main() {
    // `payload` is the list of all transactions in bytes form.
    let payload = sp1_zkvm::io::read::<Payload>();
    // (its hash is public) VID public parameter for checking the namespace proofs
    let vid_param = sp1_zkvm::io::read::<VidParam>();
    // (public) namespace ID of this rollup
    let ns_id = sp1_zkvm::io::read::<u32>();
    // (public) `bmt_commitment`: the Espresso block Merkle tree commitment that
    // accumulates all block commitments up to the current `BlockHeight`.
    let bmt_commitment = sp1_zkvm::io::read::<BlockMerkleCommitment>();
    // (private): a pair of `(range, proof)` where the
    //    `proof` asserts that a `range` of `payload` is derived from some block
    //    committed in the block Merkle tree above.
    let block_derivation_proofs = sp1_zkvm::io::read::<Vec<(Range<usize>, BlockDerivationProof)>>();
    std::println!("All inputs are loaded");

    // Compute the commitment of all the transactions
    let rollup_txs_commit = rollup_commit(&payload);

    // Verify the Espresso derivation proof
    // 1. Check that the ranges cover the whole payload with no overlapping
    // 2. Check each block derivation proof
    let mut end = 0;
    block_derivation_proofs
        .iter()
        .for_each(|(range, block_proof)| {
            assert_eq!(range.start, end);
            verify_block_derivation_proof(
                &payload.0[range.start..range.end],
                &vid_param,
                ns_id,
                &bmt_commitment,
                block_proof,
            );
            end = range.end;
        });
    assert_eq!(end, payload.0.len());
    // Wrap all the public inputs along with the verification result
    let public_inputs = PublicInputs {
        rollup_txs_commit,
        vid_param_hash: compute_vid_param_hash(&vid_param),
        ns_id,
        bmt_commitment,
    };

    sp1_zkvm::io::commit(&public_inputs);
}

/// Verifies the block derivation proof against the public inputs
fn verify_block_derivation_proof(
    payload_slice: &[u8],
    vid_param: &VidParam,
    ns_id: u32,
    bmt_commitment: &BlockMerkleCommitment,
    proof: &BlockDerivationProof,
) {
    // Assert that the membership proof is valid
    if bmt_commitment.height() + 1 != proof.bmt_proof.proof.len()
        || !BlockMerkleTree::verify(
            bmt_commitment.digest(),
            proof.bmt_proof.pos,
            &proof.bmt_proof,
        )
        .is_ok_and(|result| result.is_ok())
    {
        panic!("Incorrect membership proof for block Merkle tree.");
    }
    // Assert that the header is the one committed in the block Merkle tree
    if !proof
        .bmt_proof
        .elem()
        .is_some_and(|elem| elem == &proof.block_header.commit())
    {
        panic!("Membership proof is not consistent with the given block header.");
    }

    match proof.block_header.ns_table.scan_for_id(ns_id) {
        None => {
            panic!("Namespace ID not found in the block.");
        },
        Some((ns_range_start, ns_range_end)) => {
            std::println!("Byte range: ({}, {})", ns_range_start, ns_range_end);

            // Namespace proof w.r.t the VidCommitment
            let num_storage_nodes = <Vid as VidScheme>::get_num_storage_nodes(&proof.vid_common);
            let vid = vid_scheme(num_storage_nodes, vid_param);
            if !vid
                .payload_verify(
                    Statement {
                        payload_subslice: payload_slice,
                        range: (ns_range_start as usize..ns_range_end as usize),
                        commit: &proof.block_header.payload_commitment,
                        common: &proof.vid_common,
                    },
                    &proof.ns_proof,
                )
                .is_ok_and(|result| result.is_ok())
            {
                panic!("Failed namespace proof.");
            }
        },
    }
}
