//! This program is for fast testing, bench, profiling of logic in SP1

#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use sp1_test_program::{fibonacci, PublicValuesStruct};

fn main() {
    // Read an input to the program.
    //
    // Behind the scenes, this compiles down to a custom system call which handles
    // reading inputs from the prover.
    let n = sp1_zkvm::io::read::<u32>();

    let mut a: u32 = 0;
    let mut b: u32 = 0;
    // compute the same fib number 2 times (very silly), just to demonstrate nested cycle tracking
    println!("cycle-tracker-start: dummy loop");
    for _ in 0..2 {
        println!("cycle-tracker-start: fib compute");
        // Compute the n'th fibonacci number using a function from the workspace lib
        // crate.
        (a, b) = fibonacci(n);
        println!("cycle-tracker-end: fib compute");
    }
    println!("cycle-tracker-end: dummy loop");

    // Encode the public values of the program.
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct { n, a, b });

    // Commit to the public values of the program. The final proof will have a
    // commitment to all the bytes that were committed to.
    sp1_zkvm::io::commit_slice(&bytes);
}
