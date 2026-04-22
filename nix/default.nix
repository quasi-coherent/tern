{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule
    ./packages.nix
    ./shells.nix
  ];

  perSystem =
    {
      lib,
      pkgs,
      self',
      ...
    }:
    {
      apps.format = {
        meta = "Format project source";
        program = pkgs.writeShellApplication {
          name = "fmtt";
          text = ''${lib.getExe self'.formatter} "$@"'';
        };
      };
    };
}
