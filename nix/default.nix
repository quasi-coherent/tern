{ inputs, lib, ... }:
{
  imports = [
    inputs.flake-parts.flakeModules.partitions
    ./flakeModules.nix
  ];

  partitions.dev = {
    extraInputsFlake = ./dev;
    module = ./dev;
  };
  partitionedAttrs = lib.genAttrs [ "checks" "devShells" "formatter" "packages" ] (_: "dev");
}
