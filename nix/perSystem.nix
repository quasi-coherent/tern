{
  self,
  ...
}:
let
  perSystem =
    {
      inputs',
      lib,
      pkgs,
      self',
      ...
    }:
    let
      ternLib' = self.ternLib.forPkgs pkgs;
      rustTools = inputs'.fenix.packages.stable;
      craneLib = ternLib'.mkCrane {
        rustToolchain = rustTools.toolchain;
      };

      build = ternLib'.mkBuildArgs {
        cargoRoot = ../.;
        cargoBuildExtraArgs = "--all-features";
      };

      # All directories that contain migration files.
      #
      # The ordinary builders from crane will filter these from filesets because
      # they contain .sql and are in non-standard locations.
      allTestSources = [
        ../examples/simple_lib/migrations
        ../examples/partition_lib/migrations
        ../tests/migrations/mysql/plain
        ../tests/migrations/mysql/updown
        ../tests/migrations/pg/plain
        ../tests/migrations/pg/updown
        ../tests/migrations/sqlite/plain
        ../tests/migrations/sqlite/updown
      ];

      mkTestBuild =
        extraSources:
        ternLib'.mkBuildArgs {
          inherit extraSources;
          cargoRoot = ../.;
        };
    in
    {
      checks = {
        tern = craneLib.crane.buildPackage {
          inherit (build) cargoArtifacts;
          inherit (build.buildArgs)
            pname
            version
            src
            strictDeps
            cargoExtraArgs
            cargoBuildExtraArgs
            ;
        };

        tern-lib-tests = craneLib.crane.cargoTest {
          inherit (build) cargoArtifacts;
          inherit (build.buildArgs)
            pname
            version
            src
            strictDeps
            ;
          cargoTestExtraArgs = "--workspace --lib";
        };

        tern-examples =
          let
            testBuild = mkTestBuild [
              ../examples/simple_lib/migrations
              ../examples/partition_lib/migrations
            ];
          in
          craneLib.crane.buildPackage {
            inherit (testBuild) cargoArtifacts;
            inherit (testBuild.buildArgs)
              version
              src
              strictDeps
              ;
            pname = "tern-examples";
            pnameSuffix = "";
            cargoBuildExtraArgs = "--examples --all-features";
          };

        tern-clippy =
          let
            testBuild = mkTestBuild allTestSources;
          in
          craneLib.crane.cargoClippy {
            inherit (testBuild) cargoArtifacts;
            inherit (testBuild.buildArgs)
              pname
              version
              src
              strictDeps
              ;
            cargoClippyExtraArgs = "--workspace --all-features --all-targets --keep-going -- -Dwarnings";
          };
      };

      devShells.default =
        let
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
          cargo-doc = ternLib'.mkDocs {
            cargoRoot = ../.;
            rustdocFlags = "--cfg docsrs";
            cargoDocExtraArgs = "--no-deps --all-features --open";
          };
          integration-tests = import ./checks { inherit pkgs ternLib'; };
        in
        craneLib.crane.devShell {
          RUST_SRC_PATH = "${rustTools.rust-src}/lib/rustlib/src/rust/library";
          packages = [
            cargo-doc
            fmtt
            integration-tests.tern-doit
            pkgs.cachix
            pkgs.cargo-expand
            pkgs.cargo-llvm-cov
            pkgs.cargo-machete
            pkgs.cargo-msrv
            pkgs.just
            pkgs.mysql-shell
            pkgs.nixd
            pkgs.nix-output-monitor
            pkgs.postgresql_16
            rustTools.toolchain
          ];
        };

      treefmt = {
        projectRootFile = ".envrc";
        programs = {
          nixfmt = {
            enable = true;
            excludes = [ ".direnv" ];
          };
          rustfmt = {
            enable = true;
            # Nightly toolchain for rustfmt
            package = inputs'.fenix.packages.default.rustfmt;
          };
          taplo.enable = true;
          typos.enable = true;
        };
      };
    };
in
{
  inherit perSystem;
}
