# zkrollup-integration
Tools for zkRollups to integrate with Espresso

## Getting Started

To enter the development shell: `nix develop`

### SP1 stack

To build the ELF executable for your program and generate the proof, you will have to run outside the nix dev-shell.
For contract developments, you can enter nix shell to use necessary tools.

```
# this will first rebuild the program to elf, then generate plonky3 proof and verify it
just sp1-prove

# this will generate a proof for solidity, and creates fixture for contract verifier
just sp1-prove --evm
```
