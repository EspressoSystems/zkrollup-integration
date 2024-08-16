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
        - all replicas unequivocally refer to the payload by a `VidCommitment`, a cryptographic commitment to the entire payload sent to every replica alongside their designated chunk
    - `BlockHeader`: the summarized new consensus state and chain metadata (for consensus nodes)
        - `BlockCommitment`: a short cryptographic commitment to the `BlockHeader`
        - `BlockMerkleCommitment`: the root of `BlockMerkleTree` that accumulates all block commitments, enabling efficient historical block header lookup via `BlockMerkleTreeProof`
        - `VidCommitment` is a part of of `BlockHeader`
- a _light client_ is an agent that can verify the latest finalized _consensus state_ without running a full node
    - an off-chain light client usually receive the block header and the quorum certificate (QC)
    - an on-chain light client stores a pruned `LightClientState`, a strict subset of fields in `BlockHeader`, verified through _light client proof_ (the simplest form is using the QC from consensus, but we use a more EVM and SNARK-friendly light client protocol).
- each rollup occupies a _namespace_ (distinguished by a unique namespace ID) in a block
    - `NsTable` is the compact encoding of a _namespace table_ mapping namespace id to their offset and range in `BlockPayload`
    - `NsProof` is a _namespace proof_, attesting that some subset of bytes is the complete range of data designated to a particular namespace in a `BlockPayload` identified by its `VidCommitment` given a `NsTable`

## Circuit Spec

Assume we are proving a batch of `n` Espresso blocks.

**Public Inputs**
- `blk_cm_root_old: BlockMerkleCommitment`: root of the old block commitment tree
- `blk_cm_root_new: BlockMerkleCommitment`: root of the new block commitment tree
- `ns_txs_hash: ?`: commitment to all the txs for a namespace (type depends on the commitment scheme, e.g. Sha256 results in `[u8; 32]`) 
    - this commitment/hash is computed by the rollup VM prover and the concrete algorithm is usually different for different rollups
    - other public inputs to the "VM proof" (e.g. `prevStateRoot`, `newStateRoot`, `Withdraw/ExitRoot`) are skipped here since our circuit won't use them
- `vid_pp_hash: [u8; 32]`: Sha256 of `VidPublicParam` for the VID scheme

**Private Witness**

- `ns_id: u32`: namespace ID
- `all_tx: Vec<Vec<Transaction>>`: all txs between `blk_cm_root_old` and `blk_cm_root_new` grouped by each block
    - `ns_txs: Vec<Vec<Transaction>>` is a subset range of the `all_tx` that corresponds to a specific namespace
- `blk_headers: Vec<BlockHeader>`: a list of n `BlockHeader`
- `blk_header_proofs: Vec<BlockMerkleTreeProof>`: a list of n MT proof proving membership of foregoing block headers in `blk_cm_root_new`
- `ns_proofs: Vec<NsProof>`: a list of n `NsProof` for every `VidCommitment` and every group of txs `txs_in_blk_i`
- `vid_pp: VidPublicParam`
- `vid_common: VidCommon`: some auxiliary data broadcast to and used by each replica when verifying their chunks

**Relations**
1. Namespace-specific tx filtering: `ns_txs` is the correct subset range of payload for `ns_id`
    - `for i in 0..n`: 
        - compute subset range: `range := blk_headers[i].ns_table.lookup(ns_id)`
        - locate sub-slice of txs: `ns_txs[i]:= all_tx[i][range]`
        - check consistency between `blk_headers[i].vid_cm` and `vid_common`
        - verify the namespace proof: `payload_verify(vid_pp, vid_common, ns_txs[i], ns_proofs[i])`
        - ensure legit block header: 
            - compute block commitment: `blk_cm := COM.commit(blk_headers[i])`
            - verify inclusion via membership proof: `MT.verify(blk_cm, blk_cm_root_new, blk_header_proofs[i])`
2. Transaction batch commitment equivalence: same batch of txs used in VID and in VM proof
    - compute the `ns_txs_hash := H(ns_txs[0] | ns_txs[1] | ... | ns_txs[n-1])` using whichever commitment/hashing scheme rollup VM prover chooses


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
