// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ISP1Verifier} from "@sp1-contracts/ISP1Verifier.sol";

/// @title Fibonacci.
/// @author Espresso System
/// @notice This contract implements a simple example of verifying the proof of a computing a
///         fibonacci number.
contract EspressoDerivation {
    /// @notice The address of the SP1 verifier contract.
    /// @dev This can either be a specific SP1Verifier for a specific version, or the
    ///      SP1VerifierGateway which can be used to verify proofs for any version of SP1.
    ///      For the list of supported verifiers on each chain, see:
    ///      https://github.com/succinctlabs/sp1-contracts/tree/main/contracts/deployments
    address public verifier;

    /// @notice The verification key for the program.
    bytes32 public vkey;

    constructor(address _verifier, bytes32 _vkey) {
        verifier = _verifier;
        vkey = _vkey;
    }

    /// @notice Verify a derivation proof from a batch of Espresso blocks.
    /// @param proof The encoded proof.
    /// @param publicValues The encoded public values.
    function verifyDerivationProof(bytes calldata proof, bytes calldata publicValues)
        public
        view
    {
        ISP1Verifier(verifier).verifyProof(vkey, publicValues, proof);
    }
}
