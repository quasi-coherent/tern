{
  perSystem =
    { crane, self', ... }:
    let
      args = {
        src = crane.cleanCargoSource ../..;
        cargoArtifacts = self'.packages.tern-deps;
        strictDeps = true;
      };
      defaultSelect = "--all-features --all-targets";
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
            cargoClippyExtraArgs = "${defaultSelect} -- -Dwarnings";
          }
        );

        test = crane.cargoTest (
          args
          // {
            cargoTestExtraArgs = "${defaultSelect}";
          }
        );
      };
    };
}
