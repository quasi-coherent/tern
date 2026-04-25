{
  inputs,
  lib,
  self,
  ...
}:
let
  tern-lib = import ./lib.nix { inherit inputs lib; };
in
{
  flake = {
    inherit tern-lib;

    flakeModules = {
      default = self.flakeModules.tern;
      tern = {
        imports = [
          ./options.nix
          (import ./modules/tern.nix { ternLib = tern-lib; })
        ];
      };
      db = import ./db { ternInputs = inputs; };
    };
  };
}
