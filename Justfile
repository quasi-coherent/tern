system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

check:
    nix flake check --keep-failed |& nom

ci:
    nix run .#render-workflows

docs:
    nix run .#tern-docs

update-rs:
    nix flake update fenix
