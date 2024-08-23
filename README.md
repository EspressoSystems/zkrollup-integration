# zkrollup-integration

Tools for zkRollups to integrate with Espresso.

When a zk-rollup joins a shared finality gadget, it will receive streams of finalized blocks containing transactions from all participating rollups.
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
   - the concrete commitment scheme depends on the VM prover design, we use `Sha256(payload)` in the demo
- `ns_id: u32`: namespace ID of this rollup
- `bmt_commitment: BlockMerkleCommitment`: root of the newest Espresso block commitment tree, accumulated all historical Espresso block commitments
- `vid_pp_hash: [u8; 32]`: Sha256 of `VidPublicParam` for the VID scheme
- `blocks_info: Vec<(Range, u64)>`: specifies the origin Espresso block where each slice of the rollup transactions is from

**Private Witness**

- `payload: Vec<u8>`: the byte representation of all transactions specific to rollup with `ns_id` filtered from a batch of Espresso blocks
- `vid_param: VidParam`: public parameter for Espresso's VID scheme
- `block_proofs: Vec<Range, BlockDerivationProof>`: a proof that each slice of the rollup's transactions is derived from an Espresso blocks. Each `BlockDerivationProof` contains the following:
    - `block_header: BlockHeader`: block header of the original Espresso block containing the block height, the namespace table `ns_table`, and a commitment `payload_commitment` to the entire Espresso block payload (which contains transactions from all rollups)
    - `bmt_proof: BlockMerkleTreeProof`: a proof that the given block is in the block Merkle tree committed by `bmt_commitment`
    - `vid_common: VidCommon`: auxiliary information needed to verify the namespace proof
    - `ns_proof: NsProof`: a namespace proof such that the given transactions slice is from the Espresso block payload committed in the `block_header` and specified by the namespace table entry `ns_id`

**Relations**
1. Ensure the commitment equivalence: `rollup_txs_commit == Sha256(payload)`
2. Correct espresso derivation
    - First the ranges in `blocks_info` and `block_proofs` should be non-overlapping and cover the whole payload
    - For each `BlockDerivationProof`, we check
        - the `block_header` is in the block Merkle tree, by checking the proof `bmt_proof` against the block Merkle tree commitment `bmt_commitment`
        - Namespace ID `ns_id` of this rollup is containd in the namespace table `block_header.ns_table`, and given the specified range in the Espresso block and a namespace proof `NsProof`, checks whether the slice of rollup's transactions `payload` matches the specified slice in the Espresso block payload committed by `block_header.payload_commitment`

Read [our doc](https://github.com/EspressoSystems/espresso-sequencer/blob/main/doc/zk-integration.md) for a more detailed description;
read our blog on [Derivation Pipeline](https://hackmd.io/@EspressoSystems/the-derivation-pipeline) for rollup integration.

## Getting Started

To enter the development shell: `nix develop`

### SP1 stack

To build the ELF executable for your program and generate the proof, you will have to run outside the nix dev-shell.
For contract developments, you can enter nix shell to use necessary tools.

```
# this will first rebuild the program to elf, then generate plonky3 proof and verify it
just sp1-prove

# this will generate a proof for solidity, and creates fixture for contract verifier
just sp1-prove --evm
```
