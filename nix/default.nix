{ inputs, ... }:
{
  imports = [
    inputs.treefmt-nix.flakeModule

    ./packages.nix
    ./shells.nix
  ];
}
