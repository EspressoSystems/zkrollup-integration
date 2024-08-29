// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {stdJson} from "forge-std/StdJson.sol";
import {EspressoDerivation} from "../src/EspressoDerivation.sol";
import {SP1Verifier} from "@sp1-contracts/v1.1.0/SP1Verifier.sol";

struct SP1ProofFixtureJson {
    bytes proof;
    bytes publicValues;
    bytes32 vkey;
}

contract EspressoDerivationTest is Test {
    using stdJson for string;

    address verifier;
    EspressoDerivation public es;

    function loadFixture() public view returns (SP1ProofFixtureJson memory) {
        string memory root = vm.projectRoot();
        string memory path = string.concat(root, "/src/fixtures/fixture.json");
        string memory json = vm.readFile(path);
        bytes memory jsonBytes = json.parseRaw(".");
        return abi.decode(jsonBytes, (SP1ProofFixtureJson));
    }

    function setUp() public {
        SP1ProofFixtureJson memory fixture = loadFixture();

        verifier = address(new SP1Verifier());
        es = new EspressoDerivation(verifier, fixture.vkey);
    }

    function test_ValidDerivationProof() public view {
        SP1ProofFixtureJson memory fixture = loadFixture();

        es.verifyDerivationProof(fixture.proof, fixture.publicValues);
    }

    function testFail_InvalidDerivationProof() public view {
        SP1ProofFixtureJson memory fixture = loadFixture();

        // Create a fake proof.
        bytes memory fakeProof = new bytes(fixture.proof.length);

        es.verifyDerivationProof(fakeProof, fixture.publicValues);
    }
}
