system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

chk *args:
    nix flake check {{args}}

bld *args:
    nix build .#packages.{{system}}.default {{args}}

fmtt *args:
    fmtt {{args}}

reload *args:
    nix build .#devShells.{{system}}.default {{args}}

upd-rs:
    nix flake update fenix
