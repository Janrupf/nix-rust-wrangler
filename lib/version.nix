{ lib
, ...
}:
let
  removePreReleaseFromBeta = unversioned:
    if lib.strings.hasPrefix "beta" unversioned
    then lib.strings.concatStrings (builtins.match "(beta)\\.[[:digit:]]+(-.+)" unversioned)
    else unversioned;
in
{
  /**
    Derive the Rust channel names from a rust version.
   */
  deriveChannelNames = version:
  let
    # Normalize and process version
    versionPadded3 = lib.versions.pad 3 version;
    splitVersion = lib.versions.splitVersion versionPadded3;

    # Build the string prefix that is the numeric part (ie. "1.68.0")
    firstNonNumericComponentIdx = (lib.lists.findFirstIndex
      (s: (builtins.match "[[:digit:]]+" s) == null)
      (builtins.length splitVersion)
      splitVersion);
    numericPrefix = lib.strings.concatStringsSep "." (lib.lists.take firstNonNumericComponentIdx splitVersion);

    # Get the name without any rust version prefix. For regular releases, this will produce an empty
    # string. However, for beta's and nightlies this still contains the channel and date.
    unversionedName = removePreReleaseFromBeta (
      lib.strings.removePrefix "-" (lib.strings.removePrefix numericPrefix versionPadded3)
    );
  in (
    [versionPadded3] ++ # The padded version is always kept, since its unique in every case

    # If the version has a 0 as the minor, alias it to a version which has no minor at all
    (if (builtins.elemAt splitVersion 2) == "0"
      then ["${builtins.elemAt splitVersion 0}.${builtins.elemAt splitVersion 1}${
        if unversionedName != "" then "-${unversionedName}" else ""
      }"]
      else []) ++

    # If the version is a nightly or beta, the name contains the date. Add it as an alias
    # which does not contain the prefixed the rust version.
    (let
      isBeta = lib.strings.hasPrefix "beta" unversionedName;
      isNightly = lib.strings.hasPrefix "nightly" unversionedName;
    in if
      (isBeta && unversionedName != "beta") ||
      (isNightly && unversionedName != "nightly")
      then (
        [unversionedName] ++
        (lib.lists.optional isBeta "beta") ++
        (lib.lists.optional isNightly "nightly")
      ) else ["stable"]
    )
  );
}
