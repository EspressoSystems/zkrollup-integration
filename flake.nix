{
  description = "Dev env for zkRollup integration with Espresso";

  nixConfig = {
    extra-substituters = [ "https://espresso-systems-private.cachix.org" ];
    extra-trusted-public-keys = [
      "espresso-systems-private.cachix.org-1:LHYk03zKQCeZ4dvg3NctyCq88e44oBZVug5LpYKjPRI="
    ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    pre-commit-hooks = {
      url = "github:cachix/pre-commit-hooks.nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-compat.follows = "flake-compat";
      };
    };
    foundry.url =
      "github:shazow/foundry.nix/monthly"; # Use monthly branch for permanent releases
    solc-bin.url = "github:EspressoSystems/nix-solc-bin";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    { self
    , nixpkgs
    , flake-utils
    , flake-compat
    , pre-commit-hooks
    , foundry
    , solc-bin
    , rust-overlay
    , ...
    }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      overlays =
        [ foundry.overlay solc-bin.overlays.default (import rust-overlay) ];
      pkgs = import nixpkgs { inherit system overlays; };
      nightlyRustToolchain = pkgs.rust-bin.nightly.latest.minimal.override {
        extensions = [
          "clippy"
          "llvm-tools-preview"
          "rustc-dev"
          "rust-analyzer"
          "rustfmt"
        ];
      };

    in
    with pkgs; {
      checks = {
        pre-commit-check = pre-commit-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            check-format = {
              enable = true;
              files = "\\.rs$";
              entry = "cargo fmt -- --check";
            };
            cargo-clippy = {
              enable = true;
              description = "Lint Rust code.";
              entry = "cargo-clippy --workspace -- -D warnings";
              files = "\\.rs$";
              pass_filenames = false;
            };
            cargo-sort = {
              enable = true;
              description = "Ensure Cargo.toml are sorted";
              entry = "cargo sort sp1/program sp1/script";
              pass_filenames = false;
            };
            spell-check = {
              enable = true;
              description = "Spell check";
              entry = "typos";
              pass_filenames = false;
            };
            nixpkgs-fmt.enable = true;
          };
        };
      };

      devShells.default =
        let
          # nixWithFlakes allows pre v2.4 nix installations to use
          # flake commands (like `nix flake update`)
          nixWithFlakes = pkgs.writeShellScriptBin "nix" ''
            exec ${pkgs.nixFlakes}/bin/nix --experimental-features "nix-command flakes" "$@"
          '';
          solc = pkgs.solc-bin.latest;
        in
        mkShell {
          buildInputs = [
            git
            nixWithFlakes
            entr
            typos
            just

            # Ethereum contracts, solidity, ...
            foundry-bin
            solc

            # Rust
            nightlyRustToolchain
          ] ++ lib.optionals stdenv.isDarwin
            [ darwin.apple_sdk.frameworks.SystemConfiguration ];
          shellHook = ''
            export RUST_BACKTRACE=full
            export RUST_LOG=info
            export PATH="$PATH:$(pwd)/target/debug:$(pwd)/target/release"
            # Prevent cargo aliases from using programs in `~/.cargo` to avoid conflicts with local rustup installations.
            export CARGO_HOME=$HOME/.cargo-nix

            # Ensure `cargo fmt` uses `rustfmt` from nightly.
            export RUSTFMT="${nightlyRustToolchain}/bin/rustfmt"
          '' + self.checks.${system}.pre-commit-check.shellHook;
          FOUNDRY_SOLC = "${solc}/bin/solc";
          CARGO_TARGET_DIR = "target/nix_rustc";

        };
    });
}
