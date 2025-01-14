{ lib
, ...
}:
let
  # Add the overrideAttrs function (and helpers) to an extensible
  addOverrideAttrs = extensible: extensible // rec {
    overrideAttrs = f: addOverrideAttrs (extensible.extend (lib.toExtension f));
    addName = name: overrideAttrs (final: prev: { names = prev.names ++ [ name ]; });
  };
in
{
  /**
    Create an instance of a toolchain based on the package to link and the names to
    link with.
   */
  mkToolchainInstance = { pkg, names }: addOverrideAttrs (lib.makeExtensible (attrs: {
    inherit pkg;
    inherit names;

    _type = "toolchain-instance";
  }));
}