{ ternInputs, ... }:
{
  imports = [
    ternInputs.den.flakeModules.default
    ternInputs.limavm-nix.flakeModules.den
    ./options.nix
    ./postgres.nix
    ./mysql.nix
  ];
}
