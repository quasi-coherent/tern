{ lib, ... }:
let
  inherit (lib) getExe;
in
{
  perSystem =
    {
      crane,
      pkgs,
      rustPkgs,
      self',
      ...
    }:
    {
      devShells.default =
        let
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${getExe self'.formatter} "$@"'';
          };
          watch-expand = pkgs.writeShellApplication {
            name = "watch-expand";
            text = ''cargo watch -- cargo expand "$@"'';
          };
          workflow-gen = pkgs.writeShellApplication {
            name = "workflow-gen";
            text = "${self'.packages.render-workflows}/bin/render-workflows";
          };
        in
        crane.devShell {
          inputsFrom = [ self'.packages.tern ];

          packages = [
            fmtt
            pkgs.cachix
            pkgs.expect
            pkgs.just
            pkgs.nixd
            pkgs.nix-output-monitor
            rustPkgs.toolchain
            watch-expand
            workflow-gen
          ];

          RUST_SRC_PATH = "${rustPkgs.rust-src}/lib/rustlib/src/rust/library";
        };
    };
}
