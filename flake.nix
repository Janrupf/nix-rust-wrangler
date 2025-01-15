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
    packagesOverlayInstance = (import ./package/overlay.nix);
  in (flake-utils.lib.eachDefaultSystem (system: let
    pkgs = import nixpkgs {
      inherit system;
      overlays = [ rustOverlayInstance packagesOverlayInstance ];
    };

    # Chicken-egg problem... wrangler needs rust to be built, so we choose
    # a static, predefined toolchain for that.
    wranglerRustToolchain = pkgs.rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" "clippy" ];
    };
  in rec {
    lib = pkgs.callPackage ./lib {};

    legacyPackages = pkgs;
    packages.rust-out = lib.mkToolchainCollection [
      ((lib.deriveToolchainInstance (pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "clippy" "rust-analyzer" ];
      })).addName "default")
    ];

    packages.default = pkgs.nix-rust-wrangler;

    devShells.default = pkgs.mkShell {
      NIX_RUST_WRANGLER_TOOLCHAIN_COLLECTION = packages.rust-out;
      buildInputs = [ pkgs.stdenv.cc pkgs.nix-rust-wrangler ];
    };

    util.rust-toolchain = wranglerRustToolchain;
  })) // {
    someAttr = { inner = false; };
  };
}
