{ inputs, ... }:
let
  root = ../.;
in
{
  perSystem =
    { pkgs, inputs', ... }:
    let
      crane' = inputs.crane.mkLib pkgs;

      # Latest build of the stable toolchain.
      rustTools = inputs'.fenix.packages.stable;

      crane = crane'.overrideToolchain rustTools.toolchain;

      # Building docs requires `--cfg=docsrs`.
      # `--cfg=docsrs` requires the nightly toolchain.
      # We require `--cfg=docsrs` so we require the nightly toolchain.
      nightlyRustTools = inputs'.fenix.packages.minimal;

      # *.rs, Cargo.toml, Cargo.lock
      src = crane.cleanCargoSource root;

      # The `pname` and `version` for the package derivation.
      workspace = crane.crateNameFromCargoToml { inherit src; };
    in
    {
      _module.args = {
        inherit
          crane
          nightlyRustTools
          rustTools
          src
          workspace
          ;

        # Build workspace dependencies, which includes everything in ../crates,
        # so it gets an entry in the store and cachix can cache it:
        cargoArtifacts = crane.buildDepsOnly {
          inherit (workspace) pname version;
          inherit src;
          cargoBuildExtraArgs = "--all-features";
          strictDeps = true;
        };
      };
    };
}
