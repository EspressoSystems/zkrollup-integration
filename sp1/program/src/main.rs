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
    let ns_table = sp1_zkvm::io::read::<NsTable>();
    // let vid_comm = sp1::zkvm::io::read::<_>();
    // let rollup_comm = sp1::zkvm::io::read::<_>();
    let ns_index = sp1_zkvm::io::read::<u32>();
    let ns_range_start = sp1_zkvm::io::read::<u32>();
    let ns_range_end = sp1_zkvm::io::read::<u32>();
    // let pay_load = sp1_zkvm::io::read::<Payload>();

    let (id, start, end) = ns_table.read(ns_index).expect("Index out of bound.");
    assert!(id == ns_id);
    assert!(ns_range_start == start);
    assert!(ns_range_end == end);

    sp1_zkvm::io::commit(&ns_id);
    sp1_zkvm::io::commit(&ns_table);
}
