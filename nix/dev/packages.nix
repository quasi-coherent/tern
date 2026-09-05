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
      stableToolchain = inputs'.fenix.packages.stable.toolchain;
      nightlyToolchain = inputs'.fenix.packages.latest.toolchain;
      crane = (inputs.crane.mkLib pkgs).overrideToolchain stableToolchain;
      craneNightly = (inputs.crane.mkLib pkgs).overrideToolchain nightlyToolchain;

      root = ../../.;
      src = crane.cleanCargoSource root;

      # Shared arguments with the full workspace fileset.
      baseArgs = {
        inherit src;
        strictDeps = true;
      };
      cargoArtifacts = crane.buildDepsOnly baseArgs;

      # Tests and examples require .sql files to build.
      testSrc = lib.fileset.toSource {
        inherit root;
        fileset = lib.fileset.unions [
          (crane.fileset.commonCargoSources root)
          (lib.fileset.fileFilter (f: f.hasExt "sql") root)
        ];
      };

      args = baseArgs // {
        inherit cargoArtifacts;
        inherit (crane.crateNameFromCargoToml { cargoToml = ../../Cargo.toml; }) pname version;
        cargoBuildExtraArgs = "--all-features --workspace";
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
          }
        );

        ternExamples = crane.buildPackage (
          args
          // {
            src = testSrc;
            cargoBuildExtraArgs = "--examples";
          }
        );
      };

      checks = {
        ternTest = crane.cargoTest (
          args
          // {
            src = testSrc;
            doInstallCargoArtifacts = false;
          }
        );

        ternLint = crane.cargoClippy (
          args
          // {
            src = testSrc;
            cargoClippyExtraArgs = "--all-targets --keep-going -- -Dwarnings";
            doInstallCargoArtifacts = false;
          }
        );
      };
    };
in
{
  inherit perSystem;
}
