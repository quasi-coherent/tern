{
  lib,
  ...
}:
let
  inherit (lib) mkOption types;
in
{
  options.tern = {
    cargoRoot = mkOption {
      type = types.path;
      description = "Path to the tern project Cargo.toml.";
    };
    extraSources = mkOption {
      type = with types; listOf path;
      default = [ ];
      description = ".";
    };
    rustToolchain = mkOption {
      type = with types; nullOr package;
      default = null;
      description = ''
        The Rust toolchain to use when building the project.
        Defaults to the current stable toolchain.
      '';
    };
    cargoExtraArgs = mkOption {
      type = types.str;
      default = "--locked";
      description = ''
        Additional flags to pass in the cargo invocation.
      '';
    };
    cargoBuildExtraArgs = mkOption {
      type = types.str;
      default = "";
      description = ''
        Additional flags to pass in the cargo build command.
      '';
    };
  };
}
