{
  perSystem =
    { craneLib, self', ... }:
    let
      inherit (self'.packages) tern-deps;

      src = craneLib.cleanCargoSource ../.;
      cargoArtifacts = tern-deps;
    in
    {
      checks = {
        inherit (self'.packages)
          tern
          tern-cli
          tern-core
          tern-derive
          ;

        lint = craneLib.cargoClippy {
          inherit src cargoArtifacts;
          cargoClippyExtraArgs = "--all-targets -- -Dwarnings";
        };

        test = craneLib.cargoTest {
          inherit src cargoArtifacts;
          cargoTestExtraArgs = "--all-targets";
        };
      };
    };
}
