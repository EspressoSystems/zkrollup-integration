//! Define the header struct of an espresso block.

use super::payload::VidCommitment;
use crate::ns_table::NsTable;
use ark_serialize::CanonicalSerialize;
use committable::{Commitment, Committable, RawCommitmentBuilder};
use either::Either;
use jf_merkle_tree::{prelude::LightWeightSHA3MerkleTree, MerkleTreeScheme};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};

pub type BlockMerkleTree = LightWeightSHA3MerkleTree<Commitment<BlockHeader>>;
pub type BlockMerkleTreeProof = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;
pub type BlockMerkleCommitment = <BlockMerkleTree as MerkleTreeScheme>::Commitment;

#[derive(Serialize, Deserialize)]
pub struct BlockHeader {
    pub chain_config: ResolvableChainConfig,
    pub height: u64,
    pub timestamp: u64,

    pub l1_head: u64,

    pub l1_finalized: Option<L1BlockInfo>,

    pub payload_commitment: VidCommitment,
    /// Builder commitment is a Sha256 hash output, 32 bytes.
    pub builder_commitment: [u8; 32],
    /// A namespace table
    pub ns_table: NsTable,
    /// Root Commitment of Block Merkle Tree
    pub block_merkle_tree_root: BlockMerkleCommitment,
    /// Serialized root Commitment of `FeeMerkleTree`
    pub fee_merkle_tree_root: Vec<u8>,
    /// Fee infomation of this block
    pub fee_info: FeeInfo,
    // Builder signature is not formally part of the header and not committed.
}

impl Committable for BlockHeader {
    fn commit(&self) -> Commitment<Self> {
        let mut bmt_bytes = vec![];
        self.block_merkle_tree_root
            .serialize_with_mode(&mut bmt_bytes, ark_serialize::Compress::Yes)
            .unwrap();

        RawCommitmentBuilder::new(&Self::tag())
            .field("chain_config", self.chain_config.commit())
            .u64_field("height", self.height)
            .u64_field("timestamp", self.timestamp)
            .u64_field("l1_head", self.l1_head)
            .optional("l1_finalized", &self.l1_finalized)
            .constant_str("payload_commitment")
            .fixed_size_bytes(self.payload_commitment.as_ref().as_ref())
            .constant_str("builder_commitment")
            .fixed_size_bytes(&self.builder_commitment)
            .field("ns_table", self.ns_table.commit())
            .var_size_field("block_merkle_tree_root", &bmt_bytes)
            .var_size_field("fee_merkle_tree_root", &self.fee_merkle_tree_root)
            .field("fee_info", self.fee_info.commit())
            .finalize()
    }

    fn tag() -> String {
        // We use the tag "BLOCK" since blocks are identified by the hash of their header. This will
        // thus be more intuitive to users than "HEADER".
        "BLOCK".into()
    }
}

#[derive(Hash, Copy, Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
/// `FeeInfo` holds data related to builder fees.
pub struct FeeInfo {
    /// Directly using H160 because sp1 program cannot directly depends on ethers crate.
    account: H160,
    amount: U256,
}
impl FeeInfo {
    pub fn account(&self) -> H160 {
        self.account
    }

    pub fn amount(&self) -> U256 {
        self.amount
    }
}

impl Committable for FeeInfo {
    fn commit(&self) -> Commitment<Self> {
        let mut amt_bytes = [0u8; 32];
        self.amount.to_little_endian(&mut amt_bytes);
        RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account.to_fixed_bytes())
            .fixed_size_field("amount", &amt_bytes)
            .finalize()
    }
    fn tag() -> String {
        "FEE_INFO".into()
    }
}

/// Global variables for an Espresso blockchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ChainConfig {
    /// Espresso chain ID
    pub chain_id: U256,

    /// Maximum size in bytes of a block
    pub max_block_size: u64,

    /// Minimum fee in WEI per byte of payload
    pub base_fee: U256,

    /// Fee contract H160 on L1.
    ///
    /// This is optional so that fees can easily be toggled on/off, with no need to deploy a
    /// contract when they are off. In a future release, after fees are switched on and thoroughly
    /// tested, this may be made mandatory.
    pub fee_contract: Option<H160>,

    /// Account that receives sequencing fees.
    ///
    /// This account in the Espresso fee ledger will always receive every fee paid in Espresso,
    /// regardless of whether or not their is a `fee_contract` deployed. Once deployed, the fee
    /// contract can decide what to do with tokens locked in this account in Espresso.
    pub fee_recipient: H160,
}

impl Committable for ChainConfig {
    fn tag() -> String {
        "CHAIN_CONFIG".to_string()
    }

    fn commit(&self) -> Commitment<Self> {
        let mut chain_id_bytes = [0u8; 32];
        self.chain_id.to_little_endian(&mut chain_id_bytes);

        let mut base_fee_bytes = [0u8; 32];
        self.base_fee.to_little_endian(&mut base_fee_bytes);

        let comm = committable::RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("chain_id", &chain_id_bytes)
            .u64_field("max_block_size", self.max_block_size)
            .fixed_size_field("base_fee", &base_fee_bytes)
            .fixed_size_field("fee_recipient", &self.fee_recipient.to_fixed_bytes());
        let comm = if let Some(addr) = self.fee_contract {
            comm.u64_field("fee_contract", 1).fixed_size_bytes(&addr.0)
        } else {
            comm.u64_field("fee_contract", 0)
        };
        comm.finalize()
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Deserialize, Serialize, Eq, Hash)]
pub struct ResolvableChainConfig {
    chain_config: Either<ChainConfig, Commitment<ChainConfig>>,
}

impl Default for ResolvableChainConfig {
    fn default() -> Self {
        Self {
            chain_config: Either::Left(Default::default()),
        }
    }
}

impl ResolvableChainConfig {
    pub fn commit(&self) -> Commitment<ChainConfig> {
        match self.chain_config {
            Either::Left(config) => config.commit(),
            Either::Right(commitment) => commitment,
        }
    }
    pub fn resolve(self) -> Option<ChainConfig> {
        match self.chain_config {
            Either::Left(config) => Some(config),
            Either::Right(_) => None,
        }
    }
}

impl From<Commitment<ChainConfig>> for ResolvableChainConfig {
    fn from(value: Commitment<ChainConfig>) -> Self {
        Self {
            chain_config: Either::Right(value),
        }
    }
}

impl From<ChainConfig> for ResolvableChainConfig {
    fn from(value: ChainConfig) -> Self {
        Self {
            chain_config: Either::Left(value),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct L1BlockInfo {
    pub number: u64,
    pub timestamp: U256,
    pub hash: H256,
}

impl Committable for L1BlockInfo {
    fn commit(&self) -> Commitment<Self> {
        let mut timestamp = [0u8; 32];
        self.timestamp.to_little_endian(&mut timestamp);

        RawCommitmentBuilder::new(&Self::tag())
            .u64_field("number", self.number)
            // `RawCommitmentBuilder` doesn't have a `u256_field` method, so we simulate it:
            .constant_str("timestamp")
            .fixed_size_bytes(&timestamp)
            .constant_str("hash")
            .fixed_size_bytes(&self.hash.0)
            .finalize()
    }

    fn tag() -> String {
        "L1BLOCK".into()
    }
}
