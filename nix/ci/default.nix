{ inputs, ... }:
let
  baseSteps = [
    { uses = "actions/checkout@v6"; }
    {
      uses = "cachix/install-nix-action@v30";
      "with" = {
        nix_path = "nixpkgs=channel:nixos-unstable";
      };
    }
  ];
  defaultSteps = baseSteps ++ [
    {
      uses = "cachix/cachix-action@v17";
      "with" = {
        name = "quasi-coherent";
      };
    }
  ];
  cacheOutput = baseSteps ++ [
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
          # steps = defaultSteps ++ [
          steps = cacheOutput ++ [
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
        cachix-cache-deps = {
          steps = cacheOutput ++ [
            {
              name = "Cache build deps";
              run = "nix build";
            }
          ];
        };
      };
    };
  };
}
