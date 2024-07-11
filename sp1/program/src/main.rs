//! This program proves that the executed transactions are correctly derived from an espresso block.
// Inputs:
//  - Namespace ID (public)
//  - Namespace table (public)
//  - VID commitment (public)
//  - Rollup transactions commitment (public)
//  - All transactions
// This program proves that
//  - The namespace table contains an entry of this namespace ID which specifies its byte range in the payload.
//  - Transactions given by two offsets in the (VID) committed payload are the ones committed by the rollup.

#![no_main]
sp1_zkvm::entrypoint!(main);

use espresso_derivation_utils::ns_table::NsTable;

pub fn main() {
    let ns_id = sp1_zkvm::io::read::<u32>();
    let ns_table = sp1_zkvm::io::read::<NsTable>();
    // let vid_comm = sp1::zkvm::io::read::<_>();
    // let rollup_comm = sp1::zkvm::io::read::<_>();
    // let pay_load = sp1_zkvm::io::read::<Payload>();

    let (ns_range_start, ns_range_end) = ns_table
        .scan_for_id(ns_id)
        .expect("Namespace ID not found.");

    std::println!("Byte range: ({}, {})", ns_range_start, ns_range_end);

    sp1_zkvm::io::commit(&ns_id);
    sp1_zkvm::io::commit(&ns_table);

    // Temporarily commit the range
    sp1_zkvm::io::commit(&ns_range_start);
    sp1_zkvm::io::commit(&ns_range_end);
}
