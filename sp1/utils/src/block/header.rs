//! Define the header struct of an espresso block.

use super::payload::VidCommitment;
use crate::ns_table::NsTable;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Valid, Validate,
};
use committable::{Commitment, Committable, RawCommitmentBuilder};
use either::Either;
use jf_merkle_tree::{
    prelude::{SHA3MerkleTree, Sha3Digest, Sha3Node, UniversalMerkleTree},
    MerkleTreeScheme, ToTraversalPath,
};
use primitive_types::{H160, H256, U256};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::io::Read;
use tagged_base64::tagged;

/// Types for block Merkle tree
pub type BlockMerkleTree = SHA3MerkleTree<Commitment<BlockHeader>>;
pub type BlockMerkleTreeProof = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;
pub type BlockMerkleCommitment = <BlockMerkleTree as MerkleTreeScheme>::Commitment;

/// Types for Fee Merkle tree.
/// Although it's not used. Imported for convenience of serialization.
pub type FeeMerkleTree = UniversalMerkleTree<FeeAmount, Sha3Digest, FeeAccount, 256, Sha3Node>;
pub type FeeMerkleCommitment = <FeeMerkleTree as MerkleTreeScheme>::Commitment;

/// Builder commitment.
/// Although it's not used. Imported for convenience of serialization.
#[tagged("BUILDER_COMMITMENT")]
#[derive(Clone, Debug, Hash, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct BuilderCommitment(pub [u8; 32]);

#[derive(Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub chain_config: ResolvableChainConfig,
    pub height: u64,
    pub timestamp: u64,

    pub l1_head: u64,

    pub l1_finalized: Option<L1BlockInfo>,

    pub payload_commitment: VidCommitment,
    /// Builder commitment is a Sha256 hash output, 32 bytes.
    pub builder_commitment: BuilderCommitment,
    /// A namespace table
    pub ns_table: NsTable,
    /// Root Commitment of Block Merkle Tree
    pub block_merkle_tree_root: BlockMerkleCommitment,
    /// Serialized root Commitment of `FeeMerkleTree`
    pub fee_merkle_tree_root: FeeMerkleCommitment,
    /// Fee information of this block
    pub fee_info: FeeInfo,
    // Builder signature is not formally part of the header and not committed.
}

impl Committable for BlockHeader {
    fn commit(&self) -> Commitment<Self> {
        let mut bmt_bytes = vec![];
        self.block_merkle_tree_root
            .serialize_with_mode(&mut bmt_bytes, ark_serialize::Compress::Yes)
            .unwrap();
        let mut fmt_bytes = vec![];
        self.fee_merkle_tree_root
            .serialize_with_mode(&mut fmt_bytes, ark_serialize::Compress::Yes)
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
            .fixed_size_bytes(&self.builder_commitment.0)
            .field("ns_table", self.ns_table.commit())
            .var_size_field("block_merkle_tree_root", &bmt_bytes)
            .var_size_field("fee_merkle_tree_root", &fmt_bytes)
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
/// Although it's not used. Imported for convenience of serialization.
pub struct FeeInfo {
    pub account: FeeAccount,
    pub amount: FeeAmount,
}
impl FeeInfo {
    pub fn account(&self) -> FeeAccount {
        self.account
    }

    pub fn amount(&self) -> FeeAmount {
        self.amount
    }
}

impl Committable for FeeInfo {
    fn commit(&self) -> Commitment<Self> {
        let mut amt_bytes = [0u8; 32];
        self.amount.0.to_little_endian(&mut amt_bytes);
        RawCommitmentBuilder::new(&Self::tag())
            .fixed_size_field("account", &self.account.0.to_fixed_bytes())
            .fixed_size_field("amount", &amt_bytes)
            .finalize()
    }
    fn tag() -> String {
        "FEE_INFO".into()
    }
}

/// Global variables for an Espresso blockchain.
#[serde_as]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ChainConfig {
    /// Espresso chain ID
    pub chain_id: U256,

    /// Maximum size in bytes of a block
    #[serde_as(as = "DisplayFromStr")]
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

#[derive(
    Debug, Copy, Serialize, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default,
)]
pub struct FeeAccount(pub H160);

impl ToTraversalPath<256> for FeeAccount {
    fn to_traversal_path(&self, height: usize) -> Vec<usize> {
        self.0
            .to_fixed_bytes()
            .into_iter()
            .take(height)
            .map(|i| i as usize)
            .collect()
    }
}

impl CanonicalSerialize for FeeAccount {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        Ok(writer.write_all(&self.0.to_fixed_bytes())?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<H160>()
    }
}
impl CanonicalDeserialize for FeeAccount {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<H160>()];
        reader.read_exact(&mut bytes)?;
        let value = H160::from_slice(&bytes);
        Ok(Self(value))
    }
}

impl Valid for FeeAmount {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl Valid for FeeAccount {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

#[derive(
    Debug, Copy, Serialize, Deserialize, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Default,
)]
pub struct FeeAmount(pub U256);

impl CanonicalSerialize for FeeAmount {
    fn serialize_with_mode<W: std::io::prelude::Write>(
        &self,
        mut writer: W,
        _compress: Compress,
    ) -> Result<(), SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<U256>()];
        self.0.to_little_endian(&mut bytes);
        Ok(writer.write_all(&bytes)?)
    }

    fn serialized_size(&self, _compress: Compress) -> usize {
        core::mem::size_of::<U256>()
    }
}
impl CanonicalDeserialize for FeeAmount {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        _compress: Compress,
        _validate: Validate,
    ) -> Result<Self, SerializationError> {
        let mut bytes = [0u8; core::mem::size_of::<U256>()];
        reader.read_exact(&mut bytes)?;
        let value = U256::from_little_endian(&bytes);
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::BlockHeader;

    #[test]
    fn test_header_serialization() {
        // This string is tweaked from an actual data from Espresso's staging testnet.
        let raw_header_string = r#"{
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
            "payload_commitment": "HASH~3XOkaXVZS5e_7xjbbqN22voRnSe_p7Di-U4OPmdCD0JF",
            "builder_commitment": "BUILDER_COMMITMENT~tEvs0rxqOiMCvfe2R0omNNaphSlUiEDrb2q0IZpRcgA_",
            "ns_table": {
                "bytes": "AAAAAA=="
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
         }"#;

        let header: BlockHeader = serde_json::from_str(raw_header_string).unwrap();
        std::println!("{:?}", header);
    }
}
