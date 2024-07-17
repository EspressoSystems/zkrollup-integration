//! Define the payload for an espresso block.
use ark_bn254::Bn254;
use jf_vid::{advz::Advz, VidScheme};
use sha2::Sha256;
pub struct Payload(pub Vec<u8>);

/// Private type alias for the EC pairing type parameter for [`Advz`].
type E = Bn254;
/// Private type alias for the hash type parameter for [`Advz`].
type H = Sha256;
pub type VidCommitment = <Advz<E, H> as VidScheme>::Commit;
