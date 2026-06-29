{ self, ... }:
{
  perSystem =
    {
      pkgs,
      ...
    }:
    let
      tern-lib = self.tern-lib.forPkgs pkgs;

      build = tern-lib.mkBuildArgs {
        cargoRoot = ../../.;
        cargoBuildExtraArgs = "--all-features";
      };

      inherit (build) crane buildArgs cargoArtifacts;

      ternTestArgs =
        let
          build = tern-lib.mkBuildArgs {
            cargoRoot = ../../.;
            extraSources = [
              ../../tests/migrations/migrations01
              ../../examples/simple_lib/migrations
              ../../examples/partition_lib/migrations
            ];
          };
        in
        build.buildArgs;
    in
    {
      checks = {
        tern = crane.buildPackage {
          inherit cargoArtifacts;
          inherit (buildArgs)
            pname
            version
            src
            strictDeps
            cargoExtraArgs
            cargoBuildExtraArgs
            ;
        };

        tern-clippy = crane.cargoClippy {
          inherit cargoArtifacts;
          inherit (ternTestArgs)
            pname
            version
            src
            strictDeps
            ;
          cargoClippyExtraArgs = "--all-features --all-targets --keep-going -- -Dwarnings";
        };

        tern-test = crane.cargoTest {
          inherit cargoArtifacts;
          inherit (ternTestArgs)
            pname
            version
            src
            strictDeps
            ;
          cargoTestExtraArgs = "--all-features --all-targets";
        };

        tern-examples = crane.buildPackage {
          inherit cargoArtifacts;
          inherit (ternTestArgs)
            version
            src
            strictDeps
            ;
          pname = "tern-examples";
          pnameSuffix = "";
          cargoBuildExtraArgs = "--examples --all-features";
        };
      };
    };
}
