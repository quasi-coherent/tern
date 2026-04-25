{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule

    ./ci
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
        projectRootFile = "flake.nix";
        programs = {
          rustfmt.enable = true;
          nixfmt.enable = true;
          typos.enable = true;
        };
        settings.formatter.rustfmt = {
          options = [
            "--config-path"
            "rustfmt.toml"
          ];
        };
      };
    };
}
