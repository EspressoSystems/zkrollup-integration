# List of commands
default:
    just --list

# Build SP1 program under sp1/program
@sp1-build:
    echo "Rebuilding SP1 program ..."
    cd sp1/program && cargo prove build --verbose
    mv elf/riscv32im-succinct-zkvm-elf sp1/program/elf/fib-riscv32im-succinct-zkvm-elf && rm -rf elf/
    echo "... done"

# Generate and verify SP1 proof
@sp1-prove *args: sp1-build
    echo "Proving & Verifying SP1 program ..."
    RUST_LOG=info cargo run --bin sp1-prove --release -- {{args}}
    echo "... done"

@sp1-test-contracts:
    echo "Testing SP1 contracts"
    cd sp1/contracts && forge test -v
