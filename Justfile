system := `nix-instantiate --raw --strict --eval -E builtins.currentSystem`

check:
    nix flake check

fmt:
    nix run

write-ci:
    nix run .#render-workflows

update-rs:
    nix flake lock --update-input fenix
