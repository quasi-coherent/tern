{ inputs, ... }:
{
  imports = [
    inputs.flake-parts.flakeModules.partitions
    ./flake-parts.nix
  ];

  partitionedAttrs.apps = "dev";
  partitionedAttrs.checks = "dev";
  partitionedAttrs.devShells = "dev";
  partitionedAttrs.formatter = "dev";
  partitions.dev.extraInputsFlake = ./internal;
  partitions.dev.module = { inputs, ... }: {
    imports = [
      inputs.treefmt-nix.flakeModule
      ./perSystem.nix
    ];
  };
}
