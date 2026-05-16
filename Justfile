system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

check *args:
    nix flake check {{args}}

ci:
    nix run .#render-workflows

docs:
    nix run .#tern-docs

update-rs:
    nix flake update fenix
