//! Using the SP1 SDK to generate a proof of correct derivation from an espresso block.
//!
//! You can run this script using the following command:
//! ```shell
//! RUST_LOG=info cargo run --package espresso-derivation-prover --bin prove --release
//! ```

use std::path::PathBuf;

use clap::Parser;
use espresso_derivation_utils::ns_table::NamespaceId;
use serde::{Deserialize, Serialize};
use sp1_sdk::{HashableKey, ProverClient, SP1PlonkBn254Proof, SP1Stdin, SP1VerifyingKey};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
///
/// This file is generated by running `cargo prove build` inside the `program` directory.
pub const FIBONACCI_ELF: &[u8] = include_bytes!("../../../program/elf/riscv32im-succinct-zkvm-elf");

/// The arguments for the prove command.
// TODO: fill in other details
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct ProveArgs {
    #[clap(long, default_value = "0")]
    ns_id: NamespaceId,

    #[clap(long, default_value = "false")]
    evm: bool,
}

fn main() {
    // Setup the logger.
    sp1_sdk::utils::setup_logger();

    // Parse the command line arguments.
    let args = ProveArgs::parse();

    // Setup the prover client.
    let client = ProverClient::new();

    // Setup the program.
    let (pk, vk) = client.setup(FIBONACCI_ELF);

    // Setup the inputs.;
    let mut stdin = SP1Stdin::new();
    stdin.write(&args.ns_id);

    println!("Namespace ID: {:#x}", args.ns_id);

    if args.evm {
        // Generate the proof.
        let proof = client
            .prove_plonk(&pk, stdin)
            .expect("failed to generate proof");
        create_plonk_fixture(&proof, &vk);
    } else {
        // Generate the proof.
        let proof = client.prove(&pk, stdin).expect("failed to generate proof");
        let ns_id = u32::from_le_bytes(proof.public_values.as_slice().try_into().unwrap());
        println!("Successfully generated proof!");
        println!("Namespace ID: {:#x}", ns_id);

        // Verify the proof.
        client.verify(&proof, &vk).expect("failed to verify proof");
    }
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProofFixture {
    vkey: String,
    public_values: String,
    proof: String,
}

/// Create a fixture for the given proof.
fn create_plonk_fixture(proof: &SP1PlonkBn254Proof, vk: &SP1VerifyingKey) {
    // Create the testing fixture so we can test things end-to-end.
    let fixture = ProofFixture {
        vkey: vk.bytes32().to_string(),
        public_values: proof.public_values.bytes().to_string(),
        proof: proof.bytes().to_string(),
    };

    // The verification key is used to verify that the proof corresponds to the execution of the
    // program on the given input.
    //
    // Note that the verification key stays the same regardless of the input.
    println!("Verification Key: {}", fixture.vkey);

    // The public values are the values whicha are publicly committed to by the zkVM.
    //
    // If you need to expose the inputs or outputs of your program, you should commit them in
    // the public values.
    println!("Public Values: {}", fixture.public_values);

    // The proof proves to the verifier that the program was executed with some inputs that led to
    // the give public values.
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
