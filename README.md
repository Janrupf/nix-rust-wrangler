# Nix Rust Wrangler

This tool is a helper for managing Rust toolchain instances with declarative
configuration and on a per-project basis, while also staying compatible with IDE's
which expect Rust to be installed system wide.

## What can this do?

Nix Rust Wrangler provides seamless integration of the Rust toolchain commands into
flake environments, much like `nix-direnv` does for shells in general. Additionally,
Nix Rust Wrangler also supports the `+` syntax used by `rustup` to specify
toolchains. This makes for a better development experience inside of Nix flakes.

## Installing Nix Rust Wrangler

Depending on your setup, you may or may not want to install Nix Rust Wrangler system
wide. If you are using a development environment, which generally searches the
system PATH and does not integrate well with Nix flakes, you probably want this.

Note that installing Nix Rust Wrangler system wide will not provide impure
evaluation, unless you also explicitly install a Rust toolchain or Toolchain
collection system wide.

### System wide installation

Flake input:
```nix
inputs = {
  nix-rust-wrangler = {
    url = "github:Janrupf/nix-rust-wrangler";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

And then add the overlay to your nixpkgs:
```nix
nixpkgs.overlays = [
  inputs.nix-rust-wrangler.overlays.default
];
```

Alternatively, access the package directly:
```nix
environment.systemPackages = [
  inputs.nix-rust-wrangler.packages.${system}.default
];
```