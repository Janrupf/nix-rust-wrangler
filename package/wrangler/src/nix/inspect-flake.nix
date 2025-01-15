# Called using nix eval --apply
let
  targetSystem = "x86_64-linux";

  determineType = value:
    if value ? type
      then value.type
      else builtins.typeOf value;

  /**
    Apply a function to an attribute if it exists, otherwise return an empty set.
   */
  applyFlakeAttr = outputs: attr: f:
    if builtins.hasAttr attr outputs && builtins.hasAttr targetSystem (outputs.${attr})
      then f "${attr}.${targetSystem}" (outputs.${attr}.${targetSystem})
    else if builtins.hasAttr attr outputs
      then f attr (outputs.${attr})
    else
      {};

  /**
    Process the configuration object before it is converted to JSON.

    This is mainly used to prevent complete evaluation of some attributes
    in order to reduce the amount of work done per invocation.
   */
  processConfig = config:
    config //
    (if config ? toolchain then { toolchain = determineType config.toolchain; } else {}) //
    (if config ? toolchains
      then {
        toolchains = builtins.listToAttrs (map (name: {
          name = name;
          value = determineType (config.toolchains.${name});
        }) (builtins.attrNames config.toolchains));
      }
      else {});

in
outputs:
  (applyFlakeAttr outputs "devShells" (_: shells:
    (if shells ? default then
      { defaultDevShell = determineType shells.default; }
    else
      {}) //
    (if shells ? rustWrangler then
      { rustWranglerDevShell = determineType shells.rustWrangler; }
    else
      { })
    )
  ) //
  (applyFlakeAttr outputs "rustWrangler" (attrPath: rustWranglerConfig:
    {
      config = {
        at = attrPath;
        value = processConfig rustWranglerConfig;
      };
    }
  ))
