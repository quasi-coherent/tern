{ lib, ... }:
let
  perSystem =
    { crane, ... }:
    let
      # Following crane docs uses cleanCargoSource, which for us filters out the
      # directories that have migrations in them and we need those.  So we have
      # to pass an unfiltered root directory to crane and use filesets to add
      # additional things.
      root = ../.;

      src = lib.fileset.toSource {
        inherit root;

        fileset = lib.fileset.unions [
          # Captures all toml, rs, and Cargo.lock
          (crane.fileset.commonCargoSources ../.)
          ../examples/simple/migrations
          ../examples/dynamic/migrations
        ];
      };

      inherit (crane.crateNameFromCargoToml { inherit src; }) version;

      args = {
        inherit src version;
        strictDeps = true;
        cargoBuildExtraArgs = "--all-features";
      };

      # Creating a package of only the dependencies ensures that the derivation
      # exists to make it able to be cached.
      cargoArtifacts = crane.buildDepsOnly args;

      mkTernPackage =
        pname:
        crane.buildPackage {
          inherit pname cargoArtifacts;
          inherit (args)
            src
            version
            strictDeps
            cargoBuildExtraArgs
            ;
          cargoExtraArgs = "-p ${pname}";
        };

      tern = crane.buildPackage {
        inherit (args)
          src
          strictDeps
          version
          ;
        inherit cargoArtifacts;
        pname = "tern";
      };
    in
    {
      packages = {
        inherit tern;

        default = tern;
        tern-deps = cargoArtifacts;
        tern-cli = mkTernPackage "tern-cli";
        tern-core = mkTernPackage "tern-core";
        tern-derive = mkTernPackage "tern-derive";
      };
    };
in
{
  inherit perSystem;
}
