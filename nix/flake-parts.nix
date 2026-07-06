{
  inputs,
  lib,
  self,
  ...
}:
let
  ternLib = import ./lib.nix { inherit inputs lib; };
in
{
  flake = {
    inherit ternLib;

    flakeModules = {
      default = self.flakeModules.tern;
      tern = {
        imports = [
          ./options.nix
          (import ./modules/tern.nix { inherit ternLib; })
        ];
      };
    };
  };
}
