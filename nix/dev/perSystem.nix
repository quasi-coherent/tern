{
  inputs,
  lib,
  self,
  ...
}:
let
  perSystem =
    {
      pkgs,
      self',
      ...
    }:
    let
      tern-lib = self.tern-lib.forPkgs pkgs;
      system = pkgs.stdenvNoCC.hostPlatform.system;
      rustTools = inputs.fenix.packages.${system}.stable;
      craneLib = tern-lib.mkCrane {
        rustToolchain = rustTools.toolchain;
      };
      build = tern-lib.mkBuildArgs {
        cargoRoot = ../../.;
        cargoBuildExtraArgs = "--all-features";
      };
    in
    {
      apps = {
        default = {
          meta = "Format project source";
          program = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
        };
      };

      treefmt = {
        projectRootFile = ".git/config";
        programs = {
          nixfmt = {
            enable = true;
            excludes = [ ".direnv" ];
          };
          rustfmt = {
            enable = true;
            package = craneLib.crane.rustfmt;
          };
          taplo.enable = true;
          typos.enable = true;
        };
      };

      devShells.default =
        let
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
        in
        craneLib.crane.devShell {
          RUST_SRC_PATH = "${rustTools.rust-src}/lib/rustlib/src/rust/library";
          packages = [
            fmtt
            pkgs.bacon
            pkgs.cachix
            pkgs.expect
            pkgs.just
            pkgs.nixd
            pkgs.nix-output-monitor
            pkgs.postgresql
            rustTools.toolchain
          ];
        };
    };
in
{
  inherit perSystem;
}
