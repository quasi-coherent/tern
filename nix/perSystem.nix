{
  inputs,
  ...
}:
{
  imports = [
    ./crate.nix
    inputs.treefmt-nix.flakeModule
  ];

  perSystem =
    {
      inputs',
      lib,
      pkgs,
      self',
      ...
    }:
    let
      toolchain = inputs'.fenix.packages.stable.toolchain;
    in
    {
      devShells.default =
        let
          fmtt = pkgs.writeShellApplication {
            name = "fmtt";
            text = ''${lib.getExe self'.formatter} "$@"'';
          };
        in
        pkgs.mkShell {
          RUST_SRC_PATH = "${lib.getLib toolchain}/lib/rustlib/src/rust/library";
          packages = [
            fmtt
            pkgs.cachix
            pkgs.cargo-expand
            pkgs.cargo-llvm-cov
            pkgs.cargo-machete
            pkgs.cargo-msrv
            pkgs.just
            pkgs.mysql-shell
            pkgs.nixd
            pkgs.postgresql_16
            toolchain
          ];
        };

      treefmt = {
        projectRootFile = ".envrc";
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
