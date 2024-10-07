//! This program proves that the executed transactions are correctly derived
//! from espresso blocks.

#![no_main]
sp1_zkvm::entrypoint!(main);

use committable::Committable;
use espresso_derivation_utils::{
    block::{
        header::{BlockMerkleCommitment, BlockMerkleTree},
        payload::{compute_vid_param_hash, rollup_commit, vid_scheme, Vid, VidParam},
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
    std::println!("cycle-tracker-start: input");
    std::println!("cycle-tracker-start: payload");
    // (private): `rollup_txs` is the list of all transactions in bytes form.
    let rollup_txs = sp1_zkvm::io::read_vec();
    std::println!("cycle-tracker-end: payload");
    // (private): (its hash is public) VID public parameter for checking the
    // namespace proofs
    std::println!("cycle-tracker-start: vid_param");
    let vid_param_bytes = sp1_zkvm::io::read_vec();
    std::println!("cycle-tracker-end: vid_param");
    // (public): namespace ID of this rollup
    let ns_id = sp1_zkvm::io::read::<u32>();
    // (public): `bmt_commitment`: the Espresso block Merkle tree commitment that
    // accumulates all block commitments up to the current `BlockHeight`.
    let bmt_commitment = sp1_zkvm::io::read::<BlockMerkleCommitment>();
    // (private): a pair of `(range, proof)` where the
    //    `proof` asserts that a `range` of `payload` is derived from some block
    //    committed in the block Merkle tree above.
    std::println!("cycle-tracker-start: block derivation proof input");
    let block_derivation_proofs_bytes = sp1_zkvm::io::read_vec();
    std::println!("cycle-tracker-end: block derivation proof input");

    std::println!("cycle-tracker-start: deserialize");
    std::println!("cycle-tracker-start: VidParam");
    let vid_param: VidParam = bincode::deserialize(&vid_param_bytes).unwrap();
    std::println!("{}", vid_param.0.powers_of_h.len());
    std::println!("cycle-tracker-end: VidParam");
    std::println!("cycle-tracker-start: BlockDerivationProof");
    let block_derivation_proofs: Vec<(Range<usize>, BlockDerivationProof)> =
        bincode::deserialize(&block_derivation_proofs_bytes).unwrap();
    std::println!("cycle-tracker-end: BlockDerivationProof");
    std::println!("cycle-tracker-end: deserialize");

    std::println!("cycle-tracker-end: input");

    std::println!("cycle-tracker-start: rollup_commitment");
    // Compute the commitment of all the transactions
    let rollup_txs_commit = rollup_commit(&rollup_txs);
    std::println!("cycle-tracker-end: rollup_commitment");

    // Verify the Espresso derivation proof
    // 1. Check that the ranges cover the whole payload with no overlapping
    // 2. Check each block derivation proof
    let mut end = 0;
    std::println!("cycle-tracker-start: derivation");
    block_derivation_proofs
        .iter()
        .for_each(|(range, block_proof)| {
            assert_eq!(range.start, end);
            verify_block_derivation_proof(
                &rollup_txs[range.start..range.end],
                &vid_param,
                ns_id,
                &bmt_commitment,
                block_proof,
            );
            end = range.end;
        });
    assert_eq!(end, rollup_txs.len());
    std::println!("cycle-tracker-end: derivation");

    std::println!("cycle-tracker-start: vid_param_hash");
    // Wrap all the public inputs
    let public_inputs = PublicInputs {
        rollup_txs_commit,
        vid_param_hash: compute_vid_param_hash(&vid_param),
        ns_id,
        bmt_commitment,
    };
    std::println!("cycle-tracker-end: vid_param_hash");

    // Mark them as public inputs
    sp1_zkvm::io::commit(&public_inputs);
}

#[sp1_derive::cycle_tracker]
/// Verifies the block derivation proof against the public inputs
fn verify_block_derivation_proof(
    payload_slice: &[u8],
    vid_param: &VidParam,
    ns_id: u32,
    bmt_commitment: &BlockMerkleCommitment,
    proof: &BlockDerivationProof,
) {
    std::println!("cycle-tracker-start: bmt membership proof");
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
    std::println!("cycle-tracker-end: bmt membership proof");
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
            // Namespace proof w.r.t the VidCommitment
            let num_storage_nodes =
                <Vid as VidScheme>::get_num_storage_nodes(proof.vid_common.as_ref());
            std::println!("cycle-tracker-start: construct vid");
            let vid = vid_scheme(num_storage_nodes, vid_param);
            std::println!("cycle-tracker-end: construct vid");

            std::println!("cycle-tracker-start: payload_verify");
            if !vid
                .payload_verify(
                    Statement {
                        payload_subslice: payload_slice,
                        range: (ns_range_start as usize..ns_range_end as usize),
                        commit: proof.block_header.payload_commitment.as_ref(),
                        common: proof.vid_common.as_ref(),
                    },
                    proof.ns_proof.as_ref(),
                )
                .is_ok_and(|result| result.is_ok())
            {
                panic!("Failed namespace proof.");
            }
            std::println!("cycle-tracker-end: payload_verify");
        },
    }
}
