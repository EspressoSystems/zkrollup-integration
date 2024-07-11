//! Definitions and utilities for namespace table of an espresso block.
//! Most of contents are "unwrapped" from espresso-sequencer repo.
use serde::{Deserialize, Serialize};

/// Byte lengths for the different items that could appear in a namespace table.
const NUM_NSS_BYTE_LEN: usize = 4;
const NS_OFFSET_BYTE_LEN: usize = 4;

// Byte length for namespace IDs.
const NS_ID_BYTE_LEN: usize = 4;

/// Type definition for a namespace table.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NsTable(pub Vec<u8>);

impl NsTable {
    /// Number of entries in the namespace table.
    ///
    /// Defined as the maximum number of entries that could fit in the namespace
    /// table, ignoring what's declared in the table header.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        u32::from_le_bytes(self.0[..NUM_NSS_BYTE_LEN].try_into().unwrap())
    }

    /// Read from namespace table given an index.
    ///
    /// Return None if there's no corresponding entry, or a triple
    /// (id, start, end) which specifies the namespacd ID and its range in the
    /// payload [start, end).
    pub fn read(&self, index: u32) -> Option<(u32, u32, u32)> {
        if index >= self.len() {
            None
        } else {
            Some(self.read_unchecked(index))
        }
    }

    /// Read from namespace table given an index without range check.
    ///
    /// Return a triple (id, start, end) which specifies the namespacd ID and its
    /// range [start, end) in the payload.
    pub fn read_unchecked(&self, index: u32) -> (u32, u32, u32) {
        let pos = index as usize * (NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN) + NUM_NSS_BYTE_LEN;
        let id = u32::from_le_bytes(self.0[pos..pos + NS_ID_BYTE_LEN].try_into().unwrap());
        let end = u32::from_le_bytes(
            self.0[pos + NS_ID_BYTE_LEN..pos + NS_OFFSET_BYTE_LEN + NS_ID_BYTE_LEN]
                .try_into()
                .unwrap(),
        );
        let start = if index == 0 {
            0u32
        } else {
            u32::from_le_bytes(self.0[pos - NS_OFFSET_BYTE_LEN..pos].try_into().unwrap())
        };
        (id, start, end)
    }

    /// Read from namespace table given a namespace ID.
    ///
    /// Return None if given ID is not present, or a tuple (start, end) specifying
    /// its bytes range [start, end) in the payload.
    pub fn scan_for_id(&self, id: u32) -> Option<(u32, u32)> {
        let mut pos = NUM_NSS_BYTE_LEN;
        let mut last_offset = 0u32;
        for _ in 0..self.len() {
            let cur_id = u32::from_le_bytes(self.0[pos..pos + NS_ID_BYTE_LEN].try_into().unwrap());
            let cur_offset = u32::from_le_bytes(
                self.0[pos + NS_ID_BYTE_LEN..pos + NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN]
                    .try_into()
                    .unwrap(),
            );
            if id == cur_id {
                return Some((last_offset, cur_offset));
            }
            last_offset = cur_offset;
            pos += NS_ID_BYTE_LEN + NS_OFFSET_BYTE_LEN;
        }
        None
    }
}
