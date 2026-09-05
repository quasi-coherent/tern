{ lib, ... }:
let
  inherit (lib) mkOption types;

  ternApp = types.submodule { };
in
{
  options.tern = {
    cargoToml = mkOption {
      type = types.path;
      description = "Path to the tern app's Cargo.toml";
    };

  };
}
