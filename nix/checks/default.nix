{
  perSystem =
    { crane, self', ... }:
    let
      args = {
        src = crane.cleanCargoSource ../..;
        cargoArtifacts = self'.packages.tern-deps;
        strictDeps = true;
      };
    in
    {
      checks = {
        inherit (self'.packages)
          tern
          tern-cli
          tern-core
          tern-derive
          ;

        lint = crane.cargoClippy (
          args
          // {
            cargoClippyExtraArgs = "--all-targets -- -Dwarnings";
          }
        );

        test = crane.cargoTest (
          args
          // {
            cargoTestExtraArgs = "--all-targets";
          }
        );
      };
    };
}
