{ lib, ... }:
let
  inherit (lib) getBin getExe;
in
{
  perSystem =
    {
      crane,
      nightlyRustTools,
      rustTools,
      pkgs,
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
          write-actions = pkgs.writeShellApplication {
            name = "write-actions";
            text = "${self'.packages.render-workflows}/bin/render-workflows";
          };
          docs =
            let
              cargoNightly = nightlyRustTools.cargo;
            in
            pkgs.writeShellApplication {
              name = "docs";
              text = ''${getBin cargoNightly}/bin/cargo doc -Zunstable-options --cfg=docsrs "$@"'';
            };
        in
        crane.devShell {
          inputsFrom = [ self'.packages.tern ];

          packages = [
            docs
            fmtt
            pkgs.cachix
            pkgs.expect
            pkgs.just
            pkgs.nixd
            pkgs.nix-output-monitor
            rustTools.toolchain
            write-actions
          ];

          RUST_SRC_PATH = "${rustTools.rust-src}/lib/rustlib/src/rust/library";
        };
    };
}
