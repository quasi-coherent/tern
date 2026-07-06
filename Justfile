system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

chk *args:
    nix flake check {{args}}

bld *args:
    nix build .#checks.{{system}}.tern {{args}}

fmtt *args:
    fmtt {{args}}

upd-rs:
    nix flake update fenix
