{ lib
, pkgs
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
}: let
  pkg = rustPlatform.buildRustPackage rec {
    pname = "nix-rust-wrangler";
    version = "0.1.0";

    src = ./.;

    useFetchCargoVendor = true;
    cargoHash = "sha256-TOTTlazj9HxqjqgZJQL5Z05cV+0+maDWeC3E9/tq6wg=";

    postInstall = ''
      cd $out/bin

      ${lib.strings.concatStringsSep "\n"
        (map (alias: "ln -s nix-rust-wrangler ${lib.strings.escapeShellArg alias}") toolAliases)
      }
    '';
  };
in
  pkg // {
    withDefaultToolchain = toolchainName: pkgs.stdenv.mkDerivation {
      pname = "nix-rust-wrangler-with-${toolchainName}";
      version = pkg.version;

      nativeBuildInputs = [ pkgs.makeWrapper ];
      buildInputs = [ pkg ];

      phases = [ "buildPhase" ];

      buildPhase = ''
        runHook preBuild

        mkdir -p $out/bin

        for tool in ${pkg}/bin/*; do
          makeWrapper $tool $out/bin/$(basename $tool) \
            --set RUSTUP_TOOLCHAIN ${toolchainName}
        done

        runHook postBuild
      '';
    };
  }
