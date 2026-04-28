{ lib, ... }:
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
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
        in
        crane.devShell {
          inputsFrom = [ self'.packages.tern ];

          packages = [
            fmtt
            pkgs.cachix
            pkgs.just
            rustPkgs.toolchain
          ];

          RUST_SRC_PATH = "${rustPkgs.rust-src}/lib/rustlib/src/rust/library";
        };
    };
}
