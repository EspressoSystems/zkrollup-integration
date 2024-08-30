# zkrollup-integration

Tools for zkRollups to integrate with Espresso.

When a zk-rollup integrates with a shared finality gadget, it will receive streams of finalized blocks containing transactions from all participating rollups.
As part of (batched) state update on L1, provers for a zk-rollup periodically submit a validity proof attesting to the correctness of a new rollup state.
A rollup state corresponds to a specific finalized block.
Solely proving valid state transition on the rollup VM against a list of transactions from the block (a.k.a "VM proof") is insufficient.
With a decentralized finality gadget, the validity proof needs to encapsulate a _proof of consensus_ on the new finalized block commitment (a.k.a "(consensus) light client proof").
With a shared finality gadget, the proof needs to further encapsulate correct "_filtering_ of rollup-specific transactions" from the overall block (a.k.a. "derivation proof").
This repo **provides circuits for the derivation proof and the consistency among the three (sub-)proofs**.[^1]

[^1]: Circuits for VM proof are usually offered by zk-rollup project themselves (e.g. Polygon's [CDK](https://docs.polygon.technology/cdk/overview/), Scroll's [zkEVM](https://github.com/scroll-tech/zkevm-circuits)); circuit (written using [`jellyfish`'s constraint system](https://github.com/EspressoSystems/jellyfish/blob/main/relation/src/constraint_system.rs)) for Espresso's light client proof can be found [here](https://github.com/EspressoSystems/espresso-sequencer/blob/main/hotshot-state-prover/src/circuit.rs).

> [!NOTE]
> Espresso uses a [non-executing consensus](https://eprint.iacr.org/2024/1189), thus the _consensus state_ doesn't embed post-execution rollup states: the consensus nodes only agree on the committed block payload (thus the total ordering among txs) and its availability.
> The validity of updated _rollup/VM states_, after executing those newly committed txs, is left to the rollup prover (aka batcher).

Terminology wise: 
- a _block_ is abstractly referring to a list of new txs/state transitions; concretely manifests as:
    - `BlockPayload`: the actual payload that contains the raw tx data
        - instead of full block replication at each node, we use Verifiable Information Dispersal (VID) schemes where each replica stores a chunk. 
            - our concrete instantiation of VID relies on KZG polynomial commitment, thus requires _structured reference string_ (SRS) from a trusted setup
        - all replicas unequivocally refer to the payload by a `PayloadCommitment`, a cryptographic commitment to the entire payload sent to every replica alongside their designated chunk
    - `BlockHeader`: the summarized new consensus state and chain metadata (for consensus nodes)
        - `BlockCommitment`: a short cryptographic commitment to the `BlockHeader`
        - `BlockMerkleCommitment`: the root of `BlockMerkleTree` that accumulates all block commitments up to the current `BlockHeight`, enabling efficient historical block header lookup via `BlockMerkleTreeProof`
        - `PayloadCommitment` is a part of `BlockHeader`
        - `NsTable` described below is a part of `BlockHeader`
- each rollup occupies a _namespace_ (distinguished by a unique namespace ID) in a block
    - `NsTable` is the compact encoding of a _namespace table_ mapping namespace id to their range in `BlockPayload`
    - `NsProof` is a _namespace proof_, attesting that some subset of bytes is the complete range of data designated to a particular namespace in a `BlockPayload` identified by its `PayloadCommitment` given a `NsTable`
- a _light client_ is an agent that can verify the latest finalized _consensus state_ without running a full node
    - an off-chain light client usually receive the block header and the quorum certificate (QC)
    - an on-chain light client stores a pruned `LightClientState`, a strict subset of fields in `BlockHeader`, verified through _light client proof_ (the simplest form is using the QC from consensus, but we use a more EVM and SNARK-friendly light client protocol).

## Circuit Spec

Generally speaking, we are proving that a list of rollup's transactions are correctly derived from finalized Espresso blocks.

**Public Inputs**
- `rollup_txs_commit: [u8; 32]`: commitment to the transactions designated to the rollup `ns_id`, also one of the public inputs from the VM execution proof
   - the concrete commitment scheme depends on the VM prover design, we use `Sha256(rollup_txs)` in the demo
- `ns_id: u32`: namespace ID of this rollup
- `bmt_commitment: BlockMerkleCommitment`: root of the newest Espresso block commitment tree, accumulated all historical Espresso block commitments
- `vid_pp_hash: [u8; 32]`: Sha256 of `VidPublicParam` for the VID scheme

**Private Inputs**

- `rollup_txs: Vec<u8>`: the byte representation of all transactions specific to rollup with `ns_id` filtered from a batch of Espresso blocks
- `vid_param: VidParam`: public parameter for Espresso's VID scheme
- `block_derivation_proofs: Vec<(Range, BlockDerivationProof)>`: a list of `(range, proof)` pairs, one for each block, where `proof` proves that `rollup_txs[range]` is the complete subset of namespace-specific transactions filtered from the Espresso block. 
Each `BlockDerivationProof` contains the following:
    - `block_header: BlockHeader`: block header of the original Espresso block containing the block height, the namespace table `ns_table`, and a commitment `payload_commitment` to the entire Espresso block payload (which contains transactions from all rollups)
    - `bmt_proof: BlockMerkleTreeProof`: a proof that the given block is in the block Merkle tree committed by `bmt_commitment`
    - `vid_common: VidCommon`: auxiliary information for the namespace proof `ns_proof` verification during which its consistency against `payload_commitment` is checked
    - `ns_proof: NsProof`: a namespace proof that proves some subslice of bytes (i.e. `rollup_txs[range]`) is the complete subset for the namespace `ns_id` from the overall Espresso block payload committed in `block_header`

**Relations**
1. Recompute the payload commitment using the "VM execution prover" way: `rollup_txs_commit == Sha256(rollup_txs)`
  - note: by marking this as a public input, the verifier can cross-check it with the public inputs from the "vm proof", thus ensuring the same batch of transactions is used in `rollup_txs` here and in the generation of the "vm proof"
2. Correct derivations for the namespace/rollup from committed Espresso blocks
    - First the ranges in `block_derivation_proofs` should be non-overlapping and cover the whole payload, i.e. `range[i].end == range[i+1].start && range[i].start == 0 && range[-1].end == rollup_txs.len()`.
    - For each `BlockDerivationProof`, we check
        - the `block_header` is in the block Merkle tree, by checking the proof `bmt_proof` against the block Merkle tree commitment `bmt_commitment`
        - Namespace ID `ns_id` of this rollup is contained in the namespace table `block_header.ns_table`, and given the specified range in the Espresso block and a namespace proof `NsProof`, checks whether the slice of rollup's transactions `rollup_txs` matches the specified slice in the Espresso block payload committed by `block_header.payload_commitment`

Read [our doc](https://github.com/EspressoSystems/espresso-sequencer/blob/main/doc/zk-integration.md) for a more detailed description;
read our blog on [Derivation Pipeline](https://hackmd.io/@EspressoSystems/the-derivation-pipeline) for rollup integration.

## Getting Started

To enter the development shell: `nix develop`


### Requirements

- [Rust](https://rustup.rs/)
- [SP1](https://succinctlabs.github.io/sp1/getting-started/install.html)

### SP1 stack

To build the ELF executable for your program and generate the proof, you will have to run outside the nix dev-shell.
For contract developments, you can enter nix shell to use necessary tools.

```
# this will first rebuild the program to elf, then generate plonky3 proof and verify it
just sp1-prove

# this will generate a proof for solidity, and creates fixture for contract verifier
just sp1-prove --evm
```

#### Playground

To quickly and locally benchmark some logic, try edit the [test program](./sp1/test-program/src/main.rs), and get a cycle report:

```
$ just sp1-play
```

<details>
<summary>Example Output</summary>

```
Rebuilding SP1 test program ...
[sp1]      Finished release [optimized] target(s) in 0.34s
... done
Bench SP1 test program ...
    Finished `release` profile [optimized] target(s) in 0.77s
     Running `target/release/prove`
n: 20
2024-08-30T04:11:37.226431Z  INFO execute: clk = 0 pc = 0x2016d0    
2024-08-30T04:11:37.226987Z  INFO execute: ┌╴dummy loop    
2024-08-30T04:11:37.227029Z  INFO execute: │ ┌╴fib compute    
2024-08-30T04:11:37.227052Z  INFO execute: │ │ ┌╴fibonacci    
2024-08-30T04:11:37.227244Z  INFO execute: │ │ └╴484 cycles    
2024-08-30T04:11:37.227315Z  INFO execute: │ └╴1,267 cycles    
2024-08-30T04:11:37.227333Z  INFO execute: │ ┌╴fib compute    
2024-08-30T04:11:37.227350Z  INFO execute: │ │ ┌╴fibonacci    
2024-08-30T04:11:37.227370Z  INFO execute: │ │ └╴484 cycles    
2024-08-30T04:11:37.227387Z  INFO execute: │ └╴1,267 cycles    
2024-08-30T04:11:37.227432Z  INFO execute: └╴3,672 cycles    
2024-08-30T04:11:37.228931Z  INFO execute: close time.busy=4.44ms time.idle=120µs
Program executed successfully.
n: 20
a: 6765
b: 10946
cycle-tracker-start: fibonacci
cycle-tracker-end: fibonacci
Values are correct!
Number of cycles: 6094
... done

=======================

Tracing SP1 test program ...
  [00:00:00] [########################################] 6094/6094 (0s)                                                                                                           
Total instructions in trace: 6094


 Instruction counts considering call graph
+--------------------------------------------------------------+-------------------+
| Function Name                                                | Instruction Count |
| __start                                                      | 6086              |
| main                                                         | 5193              |
| std::io::stdio::_print                                       | 3896              |
| &std::io::stdio::Stdout::write_fmt                           | 3233              |
| core::fmt::write                                             | 2218              |
| sp1_test_program::fibonacci                                  | 1766              |
| std::io::Write::write_fmt::Adapter::write_str                | 1728              |
| std::io::buffered::linewritershim::LineWriterShim::write_all | 1308              |
| sha2::sha256::compress256                                    | 824               |
| sp1_zkvm::syscalls::halt::syscall_halt                       | 785               |
| syscall_write                                                | 703               |
| std::sync::remutex::ReentrantMutex::lock                     | 555               |
| sp1_lib::io::commit_slice                                    | 519               |
| memset                                                       | 507               |
| core::slice::memchr::memrchr                                 | 338               |
| memcpy                                                       | 338               |
| std::sys::common::thread_local::os_local::Key::get           | 315               |
| sys_write                                                    | 210               |
| std::io::stdio::print_to_buffer_if_capture_used              | 210               |
| sp1_zkvm::heap::SimpleAlloc::alloc                           | 122               |
| __rust_alloc                                                 | 110               |
| std::sync::once_lock::OnceLock::initialize                   | 88                |
| sp1_lib::io::read_vec                                        | 71                |
| std::sys::zkvm::once::Once::call                             | 68                |
| syscall_sha256_extend                                        | 8                 |
| syscall_sha256_compress                                      | 6                 |
| syscall_hint_len                                             | 3                 |
| syscall_hint_read                                            | 2                 |
+--------------------------------------------------------------+-------------------+


 Instruction counts ignoring call graph
+--------------------------------------------------------------+-------------------+
| Function Name                                                | Instruction Count |
| sha2::sha256::compress256                                    | 552               |
| std::io::buffered::linewritershim::LineWriterShim::write_all | 550               |
| memset                                                       | 514               |
| core::fmt::write                                             | 490               |
| &std::io::stdio::Stdout::write_fmt                           | 450               |
| std::io::Write::write_fmt::Adapter::write_str                | 420               |
| std::io::stdio::_print                                       | 354               |
| memcpy                                                       | 350               |
| core::slice::memchr::memrchr                                 | 348               |
| sp1_test_program::fibonacci                                  | 288               |
| std::sys::common::thread_local::os_local::Key::get           | 269               |
| syscall_write                                                | 256               |
| std::sync::remutex::ReentrantMutex::lock                     | 240               |
| std::io::stdio::print_to_buffer_if_capture_used              | 220               |
| sp1_zkvm::syscalls::halt::syscall_halt                       | 219               |
| main                                                         | 219               |
| sp1_zkvm::heap::SimpleAlloc::alloc                           | 128               |
| __start                                                      | 45                |
| std::sys::zkvm::once::Once::call                             | 41                |
| sp1_lib::io::read_vec                                        | 35                |
| __rust_alloc                                                 | 28                |
| std::sync::once_lock::OnceLock::initialize                   | 20                |
| sys_write                                                    | 20                |
| syscall_sha256_extend                                        | 10                |
| syscall_sha256_compress                                      | 8                 |
| anonymous                                                    | 7                 |
| sp1_lib::io::commit_slice                                    | 6                 |
| syscall_hint_len                                             | 4                 |
| syscall_hint_read                                            | 3                 |
+--------------------------------------------------------------+-------------------+
... done
```
</details>
