{ pkgs
, lib
, ...
}:
let
  toolchainInstance = pkgs.callPackage ./toolchain-instance.nix {};
  versionLib = pkgs.callPackage ./version.nix {};
in
rec {
  inherit (toolchainInstance) mkToolchainInstance;
  inherit versionLib;

  /**
    Choose the highest toolchain version for a channel name.
   */
  highestVersionForChannel = toolchainInstances: channel:
    # Fold over the instances and choose the one which is the highest version
    # for a given name.
    lib.lists.foldl (current: new: if lib.lists.any (n: n == channel) new.names
      then if current == null
        then new
        else let
          order = builtins.compareVersions current.pkg.version new.pkg.version;
        in if order == -1
          then new
          else current
      else current
    ) null toolchainInstances;

  /**
    De-conflict a list of toolchain instances to channel names choosing their respective highest version.
   */
  buildChannelMap = toolchainInstances: let
    allNames = lib.lists.unique (lib.lists.flatten (map (instance: instance.names) toolchainInstances));
  in
    builtins.listToAttrs (map (name: lib.attrsets.nameValuePair name (highestVersionForChannel toolchainInstances name)) allNames);

  /**
    Build a directory that contains many toolchains.

    # Example

    ```nix
    mkToolchainCollection [
      # These may be created with deriveToolchainInstance or manually
      {
        version = "1.76.0";
        platform = "x86_64-unknown-linux-gnu",
        pkg = pkgs.rust-bin.stable."1.76.0".default;
        extraNames = ["default"];
      }
      { version = "1.74.0"; platform = "x86_64-unknown-linux-gnu", pkg = pkgs.rust-bin.stable."1.74.0".default; }

      # Packages are also allowed - however, automatically deriving the toolchain
      # instance is only possible for certain packages (e.g. from oxalica/rust-overlay)
      pkgs.rust-bin.stable."1.72.0".default
    ]
    ```
   */
  mkToolchainCollection = toolchainInstances:
  let
    realToolchainInstances = map deriveToolchainInstance toolchainInstances;
    channelMap = buildChannelMap realToolchainInstances;
    toolchainTreeMeta = {
      hostPlatform = pkgs.stdenv.hostPlatform.rust.rustcTarget;
    };
  in
    pkgs.runCommand "build-rust-toolchain-tree" {} ''
      mkdir $out
      cd $out

      ${
        lib.strings.concatStringsSep "\n" (
          lib.attrsets.mapAttrsToList (channel: instance:
            "ln -s ${lib.strings.escapeShellArg (builtins.toString instance.pkg)} ${lib.strings.escapeShellArg channel}"
          ) channelMap
        )
      }

      echo -n ${lib.strings.escapeShellArg (builtins.toJSON toolchainTreeMeta)} > collection.json
    '';

  /**
    Attempt to derive a toolchain instance from a package.

    Currently this is primarily meant to be used with packages from https://github.com/oxalica/rust-overlay.
   */
  deriveToolchainInstance = v:
    if v ? _type && v._type == "toolchain-instance"
      then v
    else if v ? availableComponents
      then deriveToolchainInstanceFromOxalicaRustOverlayPkg v
    else if (v ? version && v ? platform && v ? pkg)
      then deriveToolchainInstanceFromSpec v
    else
      # TODO: Somehow format the pkg argument
      throw "Don't know how to derive a rust toolchain from this argument";

  /**
    Derive a toolchain instance from an attrset with basic specifications about the toolchain.

    Example:
    ```nix
    deriveToolchainInstanceFromSpec {
      version = "1.68.2";
      platform = "x86_64-unknown-linux-gnu";
      pkg = your-rust-pkg;
    }
    ```
   */
  deriveToolchainInstanceFromSpec = spec: let
    names = map (versionName: "${versionName}-${spec.platform}") (versionLib.deriveChannelNames spec.version);
    extraNames = spec.extraNames or [];
  in
    mkToolchainInstance { names = names ++ extraNames; pkg = spec.pkg; };

  deriveToolchainInstanceFromOxalicaRustOverlayPkg = pkg:
  let
    availableComponents = pkg.passthru.availableComponents;

    # Attempt to find a component which we can derive metadata from
    component = availableComponents.rust or (lib.lists.findFirst
      (component: builtins.hasAttr "version" component && builtins.hasAttr "platform" component)
      throw "No component found which requires the provided metadata"
      availableComponents);
  in
    deriveToolchainInstanceFromSpec {
      version = component.version;
      platform = component.platform;
      inherit pkg;
    };
}
