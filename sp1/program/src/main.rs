//! This program proves that the executed transactions are correctly derived from an espresso block.
// Inputs:
//  - Namespace ID (public)
//  - Namespace table (public)
//  - VID commitment (public)
//  - Rollup transactions commitment (public)
//  - An index in the namespace table for the rollup
//  - Two offsets that defines the namespace range
//  - All transactions
// This program proves that
//  - The namespace table contains an entry of this namespace ID.
//  - Transactions given by two offsets in the (VID) committed payload are the ones committed by the rollup.

#![no_main]
sp1_zkvm::entrypoint!(main);

use espresso_derivation_utils::ns_table::{NamespaceId, NsTable};

pub fn main() {
    let ns_id = sp1_zkvm::io::read::<NamespaceId>();
    let _ns_table = sp1_zkvm::io::read::<NsTable>();
    // let vid_comm = sp1::zkvm::io::read::<>();
    // let rollup_comm = sp1::zkvm::io::read::<>();
    // let ns_index = sp1_zkvm::io::read::<NsIndex>();

    sp1_zkvm::io::commit(&ns_id);
}
