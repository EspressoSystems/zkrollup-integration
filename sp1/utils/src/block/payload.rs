//! Define the payload for an espresso block.
use ark_bn254::Bn254;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_pcs::{
    prelude::UnivariateUniversalParams, univariate_kzg::UnivariateKzgPCS,
    PolynomialCommitmentScheme,
};
use jf_vid::{
    advz::{payload_prover::LargeRangeProof, Advz},
    VidScheme,
};
use primitive_types::H256;
use serde::{de::Error as _, ser::Error as _, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tagged_base64::tagged;

use super::RollupCommitment;

#[derive(Debug, Serialize, Deserialize)]
pub struct Payload(pub Vec<u8>);

/// Private type alias for the EC pairing type parameter for [`Advz`].
type E = Bn254;
/// Private type alias for the hash type parameter for [`Advz`].
type H = Sha256;

/// VidScheme
pub type Vid = Advz<E, H>;

/// VID commitment type
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
#[tagged("HASH")]
pub struct VidCommitment(pub <Vid as VidScheme>::Commit);

impl AsRef<<Vid as VidScheme>::Commit> for VidCommitment {
    fn as_ref(&self) -> &<Vid as VidScheme>::Commit {
        &self.0
    }
}

/// Type of common data for VID scheme
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct VidCommon(pub <Vid as VidScheme>::Common);

impl AsRef<<Vid as VidScheme>::Common> for VidCommon {
    fn as_ref(&self) -> &<Vid as VidScheme>::Common {
        &self.0
    }
}

impl Serialize for VidCommon {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Vec::new();
        self.0
            .serialize_uncompressed(&mut bytes)
            .map_err(|e| S::Error::custom(format!("{e:?}")))?;
        Serialize::serialize(&bytes, serializer)
    }
}

impl<'de> Deserialize<'de> for VidCommon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        <<Vid as VidScheme>::Common as CanonicalDeserialize>::deserialize_uncompressed_unchecked(
            &*bytes,
        )
        .map_err(|e| D::Error::custom(format!("{e:?}")))
        .map(VidCommon)
    }
}

/// Public parameters to setup the VID scheme
/// Manual (de)serialization to avoid the expensive validity check.
#[derive(Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct VidParam(pub UnivariateUniversalParams<E>);

impl Serialize for VidParam {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Vec::new();
        self.0
            .serialize_uncompressed(&mut bytes)
            .map_err(|e| S::Error::custom(format!("{e:?}")))?;
        Serialize::serialize(&bytes, serializer)
    }
}

impl<'de> Deserialize<'de> for VidParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        <UnivariateUniversalParams<E> as CanonicalDeserialize>::deserialize_uncompressed_unchecked(
            &*bytes,
        )
        .map_err(|e| D::Error::custom(format!("{e:?}")))
        .map(VidParam)
    }
}

type F = <UnivariateKzgPCS<E> as PolynomialCommitmentScheme>::Evaluation;
/// Namespace Proof type
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize)]
pub struct NsProof(pub LargeRangeProof<F>);

impl From<LargeRangeProof<F>> for NsProof {
    fn from(proof: LargeRangeProof<F>) -> Self {
        Self(proof)
    }
}

impl AsRef<LargeRangeProof<F>> for NsProof {
    fn as_ref(&self) -> &LargeRangeProof<F> {
        &self.0
    }
}

impl Serialize for NsProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut bytes = Vec::new();
        self.0
            .serialize_uncompressed(&mut bytes)
            .map_err(|e| S::Error::custom(format!("{e:?}")))?;
        Serialize::serialize(&bytes, serializer)
    }
}

impl<'de> Deserialize<'de> for NsProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = <Vec<u8> as Deserialize>::deserialize(deserializer)?;
        <LargeRangeProof<F> as CanonicalDeserialize>::deserialize_uncompressed_unchecked(&*bytes)
            .map_err(|e| D::Error::custom(format!("{e:?}")))
            .map(NsProof)
    }
}

/// Dummy rollup payload commit
pub fn rollup_commit(payload: &Payload) -> RollupCommitment {
    let bytes: [u8; 32] = Sha256::digest(&payload.0).into();
    bytes.into()
}

pub fn compute_vid_param_hash(param: &VidParam) -> H256 {
    let bytes: [u8; 32] = Sha256::digest(bincode::serialize(param).unwrap()).into();
    bytes.into()
}

pub const SRS_DEGREE: usize = 2u64.pow(20) as usize + 2;

/// Construct a VID scheme given the number of storage nodes.
/// Copied from espresso-sequencer repo
pub fn vid_scheme(num_storage_nodes: u32, param: &VidParam) -> Vid {
    let recovery_threshold = 1 << num_storage_nodes.ilog2();

    Advz::new(num_storage_nodes, recovery_threshold, &param.0).unwrap_or_else(|err| {
        panic!("advz construction failure: (num_storage nodes,recovery_threshold)=({num_storage_nodes},{recovery_threshold}); \
                error: {err}")
  })
}
