//! Define the payload for an espresso block.
use ark_bn254::Bn254;
use jf_pcs::{
    prelude::UnivariateUniversalParams, univariate_kzg::UnivariateKzgPCS,
    PolynomialCommitmentScheme,
};
use jf_utils::canonical;
use jf_vid::{
    advz::{payload_prover::LargeRangeProof, Advz},
    VidScheme,
};
use primitive_types::H256;
use serde::{Deserialize, Serialize};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct VidParam(#[serde(with = "canonical")] pub UnivariateUniversalParams<E>);

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
