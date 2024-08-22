//! This program proves that the executed transactions are correctly derived from espresso blocks.

#![no_main]
sp1_zkvm::entrypoint!(main);

use committable::Committable;
use espresso_derivation_utils::{
    block::{
        header::{BlockMerkleCommitment, BlockMerkleTree},
        payload::{rollup_commit, vid_scheme, Payload, Vid, VidParam},
    },
    BlockDerivationProof, EspressoDerivationProof, PublicInputs,
};
use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme};
use jf_vid::{
    payload_prover::{PayloadProver, Statement},
    VidScheme,
};

pub fn main() {
    let payload = sp1_zkvm::io::read::<Payload>();
    let espresso_derivation_proof = sp1_zkvm::io::read::<EspressoDerivationProof>();
    std::println!("All inputs are loaded");

    let public_inputs = PublicInputs {
        verification_result: verify_espresso_derivation(&payload.0, &espresso_derivation_proof),
        rollup_txs_commit: rollup_commit(&payload),
        espresso_derivation_commit: espresso_derivation_proof.into(),
    };

    sp1_zkvm::io::commit(&public_inputs);
}

fn verify_espresso_derivation(payload: &[u8], proof: &EspressoDerivationProof) -> bool {
    let mut end = 0;
    proof.block_proofs.iter().all(|(range, block_proof)| {
        let result = range.start == end
            && verify_block_derivation_proof(
                &payload[range.start..range.end],
                &proof.vid_param,
                proof.ns_id,
                &proof.bmt_commitment,
                block_proof,
            );
        end = range.end;
        result
    }) && end == payload.len()
}

fn verify_block_derivation_proof(
    payload_slice: &[u8],
    vid_param: &VidParam,
    ns_id: u32,
    bmt_commitment: &BlockMerkleCommitment,
    proof: &BlockDerivationProof,
) -> bool {
    // Assert that the membership proof is valid
    if bmt_commitment.height() + 1 != proof.bmt_proof.proof.len()
        || !BlockMerkleTree::verify(
            bmt_commitment.digest(),
            proof.bmt_proof.pos,
            &proof.bmt_proof,
        )
        .is_ok_and(|result| result.is_ok())
    {
        std::println!("Incorrect membership proof for block Merkle tree.");
        return false;
    }
    // Assert that the header is the one committed in the block Merkle tree
    if !proof
        .bmt_proof
        .elem()
        .is_some_and(|elem| elem == &proof.block_header.commit())
    {
        std::println!("Membership proof is not consistent with the given block header.");
        return false;
    }

    match proof.block_header.ns_table.scan_for_id(ns_id) {
        None => {
            std::println!("Namespace ID not found in the block.");
            false
        }
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
                || <Vid as VidScheme>::is_consistent(
                    &proof.block_header.payload_commitment,
                    &proof.vid_common,
                )
                .is_err()
            {
                std::println!("Failed namespace proof.");
                false
            } else {
                true
            }
        }
    }
}
