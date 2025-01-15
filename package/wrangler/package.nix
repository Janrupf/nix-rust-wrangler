{ lib
, rustPlatform
, toolAliases ? [
    "rustc"
    "rustdoc"
    "cargo"
    "rust-lldb"
    "rust-gdb"
    "rust-gdbgui"
    "rls"
    "cargo-clippy"
    "clippy-driver"
    "cargo-miri"
    "rust-analyzer"
    "rustfmt"
    "cargo-fmt"
    # "rustup" # This breaks CLion, oh no...
  ]
, ...
}:
rustPlatform.buildRustPackage rec {
  pname = "nix-rust-wrangler";
  version = "0.1.0";

  src = ./.;

  cargoHash = "sha256-TCEzrCtDpOsGuXp231Y4uOlu+6wNoXVp/GvVT4klbWI=";

  postInstall = ''
    cd $out/bin

    ${lib.strings.concatStringsSep "\n"
      (map (alias: "ln -s nix-rust-wrangler ${lib.strings.escapeShellArg alias}") toolAliases)
    }
  '';
}
