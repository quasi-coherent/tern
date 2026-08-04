{ ternLib }:
let
  perSystem =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      ternConfig = config.tern;
      ternTestingConfig = ternConfig.testing;
      mkBuildArgs = ternLib.forPkgs pkgs;

      tern = pkgs.callPackage ./tern.nix { inherit ternConfig mkBuildArgs; };
      tern-testing = pkgs.callPackage ./tern-testing.nix {
        inherit ternConfig ternTestingConfig mkBuildArgs;
      };
    in
    {
      packages = {
        inherit tern;
        default = tern;
      };
    }
    // lib.mkIf ternTestingConfig.enable {
      packages = tern-testing.tern-doit;
      checks = tern-testing.tern-it;
    };
in
{
  inherit perSystem;
}
