{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule

    ./actions.nix
    ./checks.nix
    ./crate.nix
    ./lib.nix
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
      apps.default = {
        meta = "Format project source";
        program = pkgs.writeShellApplication {
          name = "fmtt";
          text = ''${lib.getExe self'.formatter} "$@"'';
        };
      };

      treefmt = {
        projectRootFile = ".envrc";
        programs = {
          nixfmt = {
            enable = true;
            excludes = [ ".direnv" ];
          };
          rustfmt.enable = true;
          taplo.enable = true;
          typos.enable = true;
        };
      };
    };
}
