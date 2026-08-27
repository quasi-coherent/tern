{ inputs, ... }:
let
  perSystem =
    {
      inputs',
      lib,
      pkgs,
      ...
    }:
    let
      root = ../.;
      stableToolchain = inputs'.fenix.packages.stable.toolchain;
      nightlyToolchain = inputs'.fenix.packages.latest.toolchain;
      crane = (inputs.crane.mkLib pkgs).overrideToolchain stableToolchain;
      craneNightly = (inputs.crane.mkLib pkgs).overrideToolchain nightlyToolchain;
      inherit (crane.crateNameFromCargoToml { cargoToml = ../Cargo.toml; }) pname version;

      cargoTomlAndLock = crane.fileset.cargoTomlAndLock root;
      src = crane.fileset.commonCargoSources root;

      # Shared arguments with the full workspace fileset.
      baseArgs = {
        inherit pname version;
        src = lib.fileset.toSource {
          inherit root;
          fileset = src;
        };
        strictDeps = true;
        cargoBuildExtraArgs = "--all-features --workspace";
      };
      # Build only dependencies (fileset is just Cargo.toml/Cargo.lock).
      cargoArtifacts = crane.buildDepsOnly {
        inherit (baseArgs) pname version cargoBuildExtraArgs;
        src = lib.fileset.toSource {
          inherit root;
          fileset = cargoTomlAndLock;
        };
        strictDeps = true;
      };
      # Tests and examples require .sql files to build.
      testSrc =
        lib.fileset.toSource {
          inherit root;
          fileset = lib.fileset.unions [
            src
            (lib.fileset.fileFilter (f: f.hasExt "sql") root)
          ];
        };

      args = baseArgs // {
        inherit cargoArtifacts;
      };

      tern = crane.buildPackage args;
    in
    {
      packages = {
        inherit tern;
        default = tern;

        ternDocs = craneNightly.cargoDoc (
          args
          // {
            cargoDocExtraArgs = "--no-deps";
            RUSTDOCFLAGS = "--cfg docsrs";
            doCheck = false;
          }
        );

        ternExamples = crane.buildPackage (
          args
          // {
            src = testSrc;
            cargoBuildExtraArgs = "--examples";
            doCheck = false;
          }
        );
      };

      checks = {
        ternTest = crane.cargoTest (
          args
          // {
            src = testSrc;
            doInstallCargoArtifacts = true;
          }
        );

        ternLint = crane.cargoClippy (
          args
          // {
            src = testSrc;
            cargoClippyExtraArgs = "--all-targets --keep-going -- -Dwarnings";
            doInstallCargoArtifacts = true;
          }
        );
      };
    };
in
{
  inherit perSystem;
}
