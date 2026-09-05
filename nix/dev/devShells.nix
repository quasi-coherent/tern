{
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

      chkk = pkgs.writeShellApplication {
        name = "chkk";
        runtimeInputs = with pkgs; [
          nix
          nix-fast-build
        ];
        text = ''
          cores="$(nproc)"
          system="$(nix eval --expr builtins.currentSystem --impure --raw)"
          nix-fast-build \
            --eval-max-memory-size 512 \
            --eval-workers "$cores" \
            --flake ".#checks.$system" \
            --no-link \
            --skip-cached \
            "$@"
        '';
      };
      fmtt = pkgs.writeShellApplication {
        name = "fmtt";
        text = ''${lib.getExe self'.formatter} "$@"'';
      };
    in
    {
      devShells.default = pkgs.mkShell {
        RUST_SRC_PATH = "${lib.getLib toolchain}/lib/rustlib/src/rust/library";
        packages = [
          chkk
          fmtt
          pkgs.cachix
          pkgs.cargo-expand
          pkgs.cargo-llvm-cov
          pkgs.cargo-machete
          pkgs.cargo-msrv
          pkgs.just
          pkgs.mariadb
          pkgs.mysql-shell
          pkgs.nixd
          pkgs.postgresql
          pkgs.sqlite
          toolchain
        ];
      };
    };
}
