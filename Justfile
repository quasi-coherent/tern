system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

chkk *args:
    nix flake check {{args}}

bldd *args:
    nix build .#packages.{{system}}.default {{args}}

fmtt *args:
    fmtt {{args}}

reload *args:
    nix build .#devShells.{{system}}.default {{args}}

upds *args:
    nix flake update nixpkgs,fenix {{args}}
