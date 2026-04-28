{ inputs, ... }:
let
  setup = [
    { uses = "actions/checkout@v6"; }
    {
      uses = "cachix/install-nix-action@v30";
      "with" = {
        nix_path = "nixpkgs=channel:nixos-unstable";
      };
    }
  ];
  cacheSetup = setup ++ [
    {
      uses = "cachix/cachix-action@v17";
      "with" = {
        name = "quasi-coherent";
        authToken = "\${{ secrets.CACHIX_AUTH_TOKEN }}";
      };
    }
  ];
in
{
  imports = [ inputs.actions-nix.flakeModules.default ];

  flake.actions-nix.workflows = {
    ".github/workflows/pr.yaml" = {
      on.pull_request.branches = [
        "master"
        "dev/*"
      ];
      concurrency = {
        cancel-in-progress = true;
        group = "\${{ github.workflow }}-\${{ github.event.pull_request.number || github.sha }}";
      };
      jobs = {
        nix-flake-check = {
          steps = setup ++ [
            {
              name = "Run flake checks";
              run = "nix -Lv flake check";
            }
          ];
        };
      };
    };
    ".github/workflows/cache.yaml" = {
      on.push.branches = [
        "master"
        "dev/*"
      ];
      jobs = {
        nix-flake-check-fast = {
          steps = setup ++ [
            {
              name = "Run flake checks";
              run = "nix flake check --no-build";
            }
          ];
        };
        cachix-cache-deps = {
          needs = [ "nix-flake-check-fast" ];
          steps = cacheSetup ++ [
            {
              name = "Cache build deps";
              run = "nix build .#tern-deps";
            }
          ];
        };
      };
    };
  };
}
