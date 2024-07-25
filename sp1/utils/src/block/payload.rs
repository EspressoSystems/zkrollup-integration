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
pub type VidCommitment = <Vid as VidScheme>::Commit;

/// Type of common data for VID scheme
pub type VidCommon = <Vid as VidScheme>::Common;

/// Public parameters to setup the VID scheme
/// Manual (de)serialization to avoid the expensive validity check.
#[derive(Debug)]
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

/// Namespace Proof type
pub type NsProof = LargeRangeProof<<UnivariateKzgPCS<E> as PolynomialCommitmentScheme>::Evaluation>;

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
