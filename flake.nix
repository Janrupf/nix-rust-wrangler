{
  description = "Rustup but declarative";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs
  , flake-utils
  , rust-overlay
  , ... 
  }:
  let
    # We can re-use this across all nixpkgs instances
    rustOverlayInstance = (import rust-overlay);
  in (flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rustOverlayInstance ];
    };
  in rec {
    lib = pkgs.callPackage ./lib {};

    legacyPackages = pkgs;
    packages.rust-out = lib.mkToolchainTree [
      ((lib.deriveToolchainInstance pkgs.rust-bin.stable.latest.default).addName "default")
      (lib.deriveToolchainInstance pkgs.rust-bin.nightly.latest.default)
      (lib.deriveToolchainInstance pkgs.rust-bin.beta.latest.default)
    ];

    devShells.default = pkgs.mkShell {
      buildInputs = [ pkgs.python3 ];
    };
  }));
}
