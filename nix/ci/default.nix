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
in
{
  imports = [ inputs.actions-nix.flakeModules.default ];

  flake.actions-nix.workflows = {
    ".github/workflows/main.yaml" = {
      on.push.branches = [ "master" ];
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
            inputs.actions-nix.lib.steps.runNixFlakeCheck
          ];
        };
        formatter = {
          steps = setup ++ [
            {
              name = "Run formatting check";
              run = "nix run";
            }
          ];
        };
      };
    };
    ".github/workflows/cache.yaml" = {
      on.push.branches = [ "master" ];
      jobs = {
        cachix-push = {
          steps = setup ++ [
            {
              uses = "cachix/cachix-action@v17";
              "with" = {
                name = "quasi-coherent";
                authToken = "'\${{ secrets.CACHIX_AUTH_TOKEN }}'";
              };
            }
          ];
        };
      };
    };
  };
}
