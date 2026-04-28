{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule

    ./checks.nix
    ./lib.nix
    ./shells.nix
    ./tern.nix
    ./treefmt.nix
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
