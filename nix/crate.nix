{ lib, inputs, ... }:
let
  perSystem =
    {
      crane,
      inputs',
      pkgs,
      ...
    }:
    let
      root = ../.;

      # The smallest fileset contains the Cargo.toml and the Cargo.lock file to
      # build only the workspace deps.
      cargoTomlAndLock = crane.fileset.cargoTomlAndLock root;

      # Add the set of all *.rs, Cargo.toml, Cargo.lock files.
      src = crane.fileset.commonCargoSources root;

      # `src` but with additional assets.
      #
      # The test migrations contain sql files and these are filtered out of
      # the source usually, so we have to explicitly include the directory that
      # contains the migrations, else only the ones that happen to be written in
      # Rust survive, and you get errors about how you can't start a migration
      # set at version 6.
      #
      # Also do this for the examples because `cargo t` builds both test and
      # example targets.
      srcWithExtras = lib.fileset.toSource {
        inherit root;

        fileset = lib.fileset.unions [
          src
          ../examples/simple_lib/migrations
          ../tests/migrations
        ];
      };

      # Workspace crate name and version.  Important to remember that `version`
      # is shared by all member crates here but that doesn't need to be true.
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

      # Package the workspace members.
      tern = mkTernPackage "tern";
      tern-cli = mkTernPackage "tern-cli";
      tern-core = mkTernPackage "tern-core";
      tern-derive = mkTernPackage "tern-derive";
      tern-executor = mkTernPackage "tern-executor";
    in
    {
      packages = {
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          tern-executor
          ;

        # nix build
        default = tern;

        tern-docs =
          let
            # The `crane` input is built against the stable toolchain of fenix,
            # but `--cfg=docsrs` requires the nightly toolchain. And we require
            # `--cfg=docsrs`.
            nightly = inputs'.fenix.packages.latest;
            craneNightly = (inputs.crane.mkLib pkgs).overrideToolchain nightly.toolchain;
          in
          craneNightly.cargoDoc {
            inherit (args)
              src
              strictDeps
              pname
              version
              ;
            inherit cargoArtifacts;
            cargoExtraArgs = "-Zunstable-options --cfg=docsrs";
          };
      };

      checks = {
        # Build all workspace members as part of checks.
        inherit
          tern
          tern-cli
          tern-core
          tern-derive
          tern-executor
          ;

        tern-clippy = crane.cargoClippy {
          inherit (args)
            strictDeps
            pname
            version
            ;
          inherit cargoArtifacts;
          src = srcWithExtras;
          cargoClippyExtraArgs = "--all-features --all-targets -- -Dwarnings";
        };

        tern-test = crane.cargoTest {
          inherit (args)
            strictDeps
            pname
            version
            ;
          inherit cargoArtifacts;
          src = srcWithExtras;
          cargoTestExtraArgs = "--all-features --all-targets";
        };
      };
    };
in
{
  inherit perSystem;
}
