system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

check *args:
    nix flake check {{args}}

ci:
    nix run .#render-workflows

doc:
    nix run .#tern-doc

update-rs:
    nix flake update fenix
