{ lib, ... }:
let
  perSystem =
    { crane, ... }:
    let
      root = ../.;

      # The smallest fileset contains the Cargo.toml and the Cargo.lock file to
      # build only the workspace deps.
      cargoTomlAndLock = crane.fileset.cargoTomlAndLock root;
      # The set of *.rs, Cargo.toml, Cargo.lock files.
      src = crane.fileset.commonCargoSources root;

      inherit (crane.crateNameFromCargoToml { src = crane.cleanCargoSource ../.; }) pname version;

      args = {
        # Override if building with `-p`.
        inherit pname version;
        src = lib.fileset.toSource {
          inherit root;
          fileset = src;
        };
        strictDeps = true;
        cargoBuildExtraArgs = "--all-features";
      };

      # Build only dependencies so that they can be cached for everything else.
      cargoArtifacts = crane.buildDepsOnly {
        inherit (args) pname version cargoBuildExtraArgs;

        src = lib.fileset.toSource {
          inherit root;
          fileset = cargoTomlAndLock;
        };
        strictDeps = true;
      };

      mkTernPackage =
        pname:
        crane.buildPackage {
          inherit pname cargoArtifacts;
          inherit (args)
            src
            version
            strictDeps
            ;
          cargoBuildExtraArgs = "--all-features -p ${pname}";
        };

      # Individual crates as flake outputs.
      tern-cli = mkTernPackage "tern-cli";
      tern-core = mkTernPackage "tern-core";
      tern-derive = mkTernPackage "tern-derive";

      # The root package.
      tern = crane.buildPackage {
        inherit cargoArtifacts;
        inherit (args)
          src
          pname
          version
          strictDeps
          ;
      };
    in
    {
      packages = {
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          ;

        default = tern;
      };

      checks = {
        # Build the crates as part of `nix flake check` for convenience.
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          ;

        tern-clippy = crane.cargoClippy {
          inherit (args)
            src
            strictDeps
            pname
            version
            ;

          inherit cargoArtifacts;
          cargoClippyExtraArgs = "--all-features --all-targets -- -Dwarnings";
        };

        tern-test = crane.cargoTest {
          inherit (args)
            strictDeps
            pname
            version
            ;
          inherit cargoArtifacts;

          cargoTestExtraArgs = "--all-features --all-targets";
          # We need to add the migration directories for the examples since
          # everywhere else they're filtered out and we need them to compile
          # tests (which includes `examples`).
          src = lib.fileset.toSource {
            inherit root;
            fileset = lib.fileset.unions [
              src
              ../examples/dynamic/migrations
              ../examples/simple/migrations
            ];
          };
        };
      };
    };
in
{
  inherit perSystem;
}
