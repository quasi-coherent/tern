{ inputs, lib, ... }:
let
  root = ../.;
in
{
  perSystem =
    { pkgs, inputs', ... }:
    let
      crane' = inputs.crane.mkLib pkgs;
    in
    {
      _module.args = rec {
        # Building docs requires `--cfg=docsrs`.
        # `--cfg=docsrs` requires the nightly toolchain.
        # We require `--cfg=docsrs` so we require the nightly toolchain.
        craneNightly = crane'.overrideToolchain inputs'.fenix.packages.minimal.toolchain;
        crane = crane'.overrideToolchain inputs'.fenix.packages.stable.toolchain;

        # The root crate name and version.
        manifest = crane.crateNameFromCargoToml { src = crane.cleanCargoSource root; };

        # Has *.rs, all Cargo.toml plus the Cargo.lock.
        ternSrc = crane.fileset.commonCargoSources root;

        # `ternSrc` but with additional assets: building some targets requires
        # the directory with migrations, which contains .sql, which are filtered
        # out for not usually being Rust source.
        ternSrcExtra = lib.fileset.toSource {
          inherit root;
          fileset = lib.fileset.unions [
            ternSrc
            ../examples/simple_lib/migrations
            ../tests/migrations
          ];
        };

        # Build just dependencies for the cache.
        cargoArtifacts =
          let
            # The smallest possible fileset that can build the workspace deps:
            cargoTomlAndLock = crane.fileset.cargoTomlAndLock root;
          in
          crane.buildDepsOnly {
            inherit (manifest) pname version;
            src = lib.fileset.toSource {
              inherit root;
              fileset = cargoTomlAndLock;
            };
            cargoBuildExtraArgs = "--all-features";
            strictDeps = true;
          };

        mkTernPackage =
          pname:
          crane.buildPackage {
            inherit (manifest) version;
            inherit pname cargoArtifacts;
            src = ternSrc;
            strictDeps = true;
            cargoBuildExtraArgs = "--all-features -p ${pname}";
          };

        mkTernPackage' =
          pname: version:
          crane.buildPackage {
            inherit pname version cargoArtifacts;
            src = ternSrc;
            strictDeps = true;
            cargoBuildExtraArgs = "--all-features -p ${pname}";
          };
      };
    };
}
