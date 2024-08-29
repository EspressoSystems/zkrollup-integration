//! Definitions of necessary parts of an espresso block.

use primitive_types::H256;

pub mod header;
pub mod payload;

pub type RollupCommitment = H256;
