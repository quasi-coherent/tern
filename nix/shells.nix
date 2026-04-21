{ inputs, lib, ... }:
{
  perSystem =
    { self', pkgs, ... }:
    {
      devShells.default =
        let
          formatter = self'.formatter;
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe formatter} "$@"'';
          };
        in
        (inputs.crane.mkLib pkgs).devShell {
          inputsFrom = [ self'.packages.tern ];

          packages = [
            fmtt
            pkgs.cachix
            self'.packages.rust-stable.toolchain
          ];
        };
    };
}
