{ inputs, ... }:
{
  imports = [ inputs.treefmt-nix.flakeModule ];

  perSystem = { inputs', pkgs, ... }: {
    treefmt = {
      projectRootFile = "flake.nix";
      programs = {
        mdformat.enable = true;
        nixfmt = {
          enable = true;
          package = pkgs.nixfmt-rs;
        };
        rustfmt = {
          enable = true;
          package = inputs'.fenix.packages.default.rustfmt;
        };
        taplo.enable = true;
        typos.enable = true;
      };
      settings = {
        "cargo-sort-derives" = {
          command = pkgs.cargo-sort-derives;
          options = [
            "--color"
            "never"
          ];
          includes = [ "*.rs" ];
        };
        global.exclude = [ ".direnv/*" ];
      };
    };
  };
}
