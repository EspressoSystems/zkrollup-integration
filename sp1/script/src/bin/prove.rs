//! Using the SP1 SDK to generate a proof of correct derivation from an espresso
//! block.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --package espresso-derivation-prover --bin prove --release
//! ```

use clap::Parser;
use committable::Committable;
use espresso_derivation_utils::{
    block::{
        header::{BlockHeader, BlockMerkleTree},
        payload::{vid_scheme, NsProof, Payload, Vid, VidCommitment, VidCommon, VidParam},
    },
    ns_table::NsTable,
    BlockDerivationProof, PublicInputs,
};
use jf_merkle_tree::{AppendableMerkleTreeScheme, MerkleTreeScheme};
use jf_pcs::prelude::UnivariateUniversalParams;
use jf_vid::{payload_prover::PayloadProver, VidScheme};
use rand::{Rng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey};
use std::path::PathBuf;

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
///
/// This file is generated by running `cargo prove build` inside the `program`
/// directory.
pub const ELF: &[u8] = include_bytes!("../../../program/elf/riscv32im-succinct-zkvm-elf");
/// low degree for demo only
pub const SRS_DEGREE: usize = 8usize;
/// payload bytes for each block shouldn't exceed max size
/// during encoding, every 30 bytes is converted to a 254-bit field element
pub const MAX_PAYLOAD_BYTES_PER_BLOCK: usize = SRS_DEGREE * 30;
/// number of storage node for VID
pub const NUM_STORAGE_NODES: u32 = 10;
/// produce derivation proof for a batch of espresso blocks
pub const NUM_BLOCKS: u64 = 5;

/// The arguments for the prove command.
// TODO: fill in other details
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct ProveArgs {
    #[clap(short, long, default_value = "false")]
    bench: bool,

    #[clap(long, default_value = "false")]
    evm: bool,
}

fn mock_block<R: RngCore>(
    idx: u64,
    ns_id: u32,
    ns_payload: &[u8],
    vid: &mut Vid,
    rng: &mut R,
) -> (BlockHeader, VidCommon, NsProof) {
    // This is a tweak from an actual block header in Espresso's staging testnet
    let mut header: BlockHeader = serde_json::from_str(
        r#"{
            "chain_config": {
                "chain_config": {
                    "Left": {
                        "chain_id": "888888888",
                        "max_block_size": "30000000",
                        "base_fee": "0",
                        "fee_contract": null,
                        "fee_recipient": "0x0000000000000000000000000000000000000000"
                    }
                }
            },
            "height": 69781,
            "timestamp": 1720789795,
            "l1_head": 5113,
            "l1_finalized": {
                "number": 5088,
                "timestamp": "0x669129ec",
                "hash": "0xfc4249b13292d2617cc0dec8b0a9a666491d5fecdfe536c929207847364b2b60"
            },
            "payload_commitment": "HASH~KpvHX4MuDuZKk10QJctEoUj-fump6NIAO8fJ048RwNJo",
            "builder_commitment": "BUILDER_COMMITMENT~tEvs0rxqOiMCvfe2R0omNNaphSlUiEDrb2q0IZpRcgA_",
            "ns_table": {
                "bytes": "AQAAAB0AAAALAAAA"
            },
            "block_merkle_tree_root": "MERKLE_COMM~02gWBSt2tcz9XfOOO6xEVicluWIIP95BW8I11f2graggAAAAAAAAAJUQAQAAAAAAUQ",
            "fee_merkle_tree_root": "MERKLE_COMM~yB4_Aqa35_PoskgTpcCR1oVLh6BUdLHIs7erHKWi-usUAAAAAAAAAAEAAAAAAAAAJg",
            "fee_info": {
                "account": "0x23618e81e3f5cdf7f54c3d65f7fbc0abf5b21e8f",
                "amount": "0"
            },
            "builder_signature": {
                "r": "0x6291b473fdac85b9ce7b40b530ea4173ac6e71fd29acffc3cbc97ae637d4404d",
                "s": "0x3178fe07d5071df7a7ce4106e6e1e3727aa6edc458db03d1774948bdec32eac6",
                "v": 28
            }
         }"#,
    ).unwrap();

    // Mock a height
    header.height += idx;

    // Mock payload
    let payload_size = rng.gen_range(2 * ns_payload.len()..8 * ns_payload.len());
    let mut payload = vec![0u8; payload_size];
    rng.fill_bytes(&mut payload);
    let offset = rng.gen_range(1..payload_size - ns_payload.len() - 1);
    payload[offset..offset + ns_payload.len()].copy_from_slice(ns_payload);

    // Mock VID information
    let vid_disperse = vid.disperse(&payload).unwrap();
    let vid_common = VidCommon(vid_disperse.common);
    let vid_commitment = VidCommitment(vid_disperse.commit);
    // Update the payload commitment
    header.payload_commitment = vid_commitment;
    // Update the namespace table
    header.ns_table = NsTable::mock_ns_table(&[
        (rng.next_u32(), offset as u32),
        (ns_id, offset as u32 + ns_payload.len() as u32),
        (rng.next_u32(), payload_size as u32),
    ]);

    // Namespace proof
    let ns_range = offset..offset + ns_payload.len();
    let ns_proof = NsProof(vid.payload_proof(&payload, ns_range).unwrap());

    (header, vid_common, ns_proof)
}

fn mock_inputs(stdin: &mut SP1Stdin) {
    let mut rng = rand::rngs::StdRng::from_seed([0u8; 32]);

    let ns_id = rng.next_u32();
    let mut block_merkle_tree = BlockMerkleTree::new(32);
    let mut rollup_payload = Payload(vec![]);
    let mut block_proofs = vec![];

    let vid_param = load_srs();
    let mut vid = vid_scheme(NUM_STORAGE_NODES, &vid_param);

    for i in 0..NUM_BLOCKS {
        // pick a payload length for each block
        let ns_payload_len = rng.gen_range(1..MAX_PAYLOAD_BYTES_PER_BLOCK);
        // fill with random payload bytes of `ns_payload_len`
        let mut block_ns_payload = vec![0u8; ns_payload_len];
        rng.fill_bytes(&mut block_ns_payload);

        // produce a mock block containing this namespace payload
        let (header, vid_common, ns_proof) =
            mock_block(i, ns_id, &block_ns_payload, &mut vid, &mut rng);

        // push the block commitment to the BMT
        block_merkle_tree.push(header.commit()).unwrap();

        // retrieve a merkle proof of this block commitment
        let (_, bmt_proof) = block_merkle_tree.lookup(i).expect_ok().unwrap();

        // prepare the block derivation proof
        block_proofs.push((
            rollup_payload.0.len()..rollup_payload.0.len() + ns_payload_len,
            BlockDerivationProof {
                bmt_proof,
                block_header: header,
                vid_common,
                ns_proof,
            },
        ));
        // append to overall rollup-specific payload
        // (as if filtered from a batch of blocks)
        rollup_payload.0.append(&mut block_ns_payload);
    }

    // update all the BMT inclusion proof since new block commitments where
    // accumulated, and old merkle proofs are holding outdated root
    for i in 0..NUM_BLOCKS {
        let (_, bmt_proof) = block_merkle_tree.lookup(i).expect_ok().unwrap();
        block_proofs.get_mut(i as usize).unwrap().1.bmt_proof = bmt_proof;
    }

    // push to inputs
    stdin.write(&rollup_payload);
    stdin.write(&vid_param);
    stdin.write(&ns_id);
    stdin.write(&block_merkle_tree.commitment());
    stdin.write(&block_proofs);
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = ProveArgs::parse();

    // Setup the prover client.
    let client = ProverClient::new();

    // Setup the program.
    let (pk, vk) = client.setup(ELF);

    // Setup the inputs.;
    let mut stdin = SP1Stdin::new();
    mock_inputs(&mut stdin);

    if args.bench {
        // Execute the program
        let (public_values, report) = client
            .execute(ELF, stdin)
            .run()
            .expect("failed to generate proof");
        let public_values: PublicInputs = bincode::deserialize(public_values.as_slice()).unwrap();
        println!("Public values: {:?}", public_values);
        println!("{}", report);
    } else if args.evm {
        // Generate the proof.
        let proof = client
            .prove(&pk, stdin)
            .plonk()
            .run()
            .expect("failed to generate proof");
        create_plonk_fixture(&proof, &vk);
    } else {
        // Generate the proof.
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");
        let public_values: PublicInputs =
            bincode::deserialize(proof.public_values.as_slice()).unwrap();
        println!("Public values: {:?}", public_values);

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
    }
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs
/// inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProofFixture {
    vkey: String,
    public_values: String,
    proof: String,
}

/// Create a fixture for the given proof.
fn create_plonk_fixture(proof: &SP1ProofWithPublicValues, vk: &SP1VerifyingKey) {
    // Create the testing fixture so we can test things end-to-end.
    let fixture = ProofFixture {
        vkey: vk.bytes32().to_string(),
        public_values: format!("0x{}", hex::encode(proof.public_values.as_slice())),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    // The verification key is used to verify that the proof corresponds to the
    // execution of the program on the given input.
    //
    // Note that the verification key stays the same regardless of the input.
    println!("Verification Key: {}", fixture.vkey);

    // The public values are the values whicha are publicly committed to by the
    // zkVM.
    //
    // If you need to expose the inputs or outputs of your program, you should
    // commit them in the public values.
    println!("Public Values: {}", fixture.public_values);

    // The proof proves to the verifier that the program was executed with some
    // inputs that led to the give public values.
    println!("Proof Bytes: {}", fixture.proof);

    // Save the fixture to a file.
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join("fixture.json"),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");
}

fn load_srs() -> VidParam {
    let srs = ark_srs::kzg10::aztec20::setup(SRS_DEGREE).expect("Aztec SRS failed to load");
    VidParam(UnivariateUniversalParams {
        powers_of_g: srs.powers_of_g,
        h: srs.h,
        beta_h: srs.beta_h,
        powers_of_h: vec![srs.h, srs.beta_h],
    })
}
