//! Definitions and utilities for namespace table of an espresso block.

use serde::{Deserialize, Serialize};

/// Type definition for a namespace table.
/// TODO: Fill in details.
#[derive(Serialize, Deserialize)]
pub struct NsTable;

/// Type definition for namespace id.
pub type NamespaceId = u32;

impl AsRef<[u8]> for NsTable {
    fn as_ref(&self) -> &[u8] {
        &[]
    }
}