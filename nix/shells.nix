{ inputs, lib, ... }:
{
  perSystem =
    { self', pkgs, ... }:
    {
      devShells.default =
        let
          rust-stable = self'.packages.rust-stable;
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
        in
        (inputs.crane.mkLib pkgs).devShell {
          inputsFrom = [ self'.packages.tern ];

          packages = [
            fmtt
            pkgs.cachix
            rust-stable.toolchain
          ];

          RUST_SRC_PATH = "${rust-stable.rust-src}/lib/rustlib/src/rust/library";
        };
    };
}
