{ inputs, ... }:
let
  # Actions reference.
  actions = {
    checkout = "actions/checkout@v6";
    cachix = "cachix/cachix-action@v17";
    install-nix = "cachix/install-nix-action@v30";
    cargo-docs-rs = "dtolnay/install@cargo-docs-rs";
  };

  # Step definitions.
  steps = {
    checkout = {
      uses = actions.checkout;
    };

    install-nix = {
      uses = actions.install-nix;
      "with".nix_path = "nixpkgs=channel:nixos-unstable";
    };

    cachix-read = {
      uses = actions.cachix;
      "with".name = "quasi-coherent";
    };

    cachix-write = {
      uses = actions.cachix;
      "with" = {
        name = "quasi-coherent";
        authToken = "\${{ secrets.CACHIX_AUTH_TOKEN }}";
      };
    };

    flake-check = {
      name = "nix flake check";
      run = "nix -Lv flake check";
    };

    cargo-docs-rs = {
      uses = actions.cargo-docs-rs;
    };

    check-docs = {
      name = "check docs";
      run = "cargo docs-rs";
    };
  };

  concurrency = {
    group = "\${{ github.workflow }}-\${{ github.head_ref || github.ref_name }}";
    cancel-in-progress = "\${{ github.event_name == 'pull_request' }}";
  };

  setupSteps = [
    steps.checkout
    steps.install-nix
    steps.cachix-read
  ];
in
{
  imports = [ inputs.actions-nix.flakeModules.default ];

  flake.actions-nix = {
    defaultValues.jobs = {
      timeout-minutes = 60;
      runs-on = "ubuntu-24.04";
    };
    workflows = {
      ".github/workflows/main.yaml" = {
        inherit concurrency;
        on.pull_request.branches = [ "master" ];
        jobs.flake-check.steps = setupSteps ++ [
          steps.flake-check
          steps.cargo-docs-rs
          steps.check-docs
        ];
      };

      ".github/workflows/dev.yaml" = {
        inherit concurrency;
        on.pull_request.branches = [ "dev/*" ];
        jobs.flake-check = {
          "if" = "github.event.pull_request.author_association == 'OWNER'";
          steps = setupSteps ++ [
            steps.cachix-write
            steps.flake-check
            steps.cargo-docs-rs
            steps.check-docs
          ];
        };
      };

      ".github/workflows/cache.yaml" = {
        inherit concurrency;
        on.push.branches = [
          "master"
          "dev/*"
        ];
        jobs.cache-build.steps = setupSteps ++ [
          steps.cachix-write
          {
            name = "nix build";
            run = "nix build";
          }
        ];
      };
    };
  };
}
