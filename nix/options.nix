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
      description = "Path to the Cargo.toml root.";
    };
    extraSources = mkOption {
      type = with types; listOf path;
      default = [ ];
      description = ''
        Paths to additional source files.  By default, *.rs, Cargo.toml, and
        Cargo.lock are used to construct the derivation's input fileset, and all
        other files are filtered out.

        So for instance, add any directory that contains migration SQL sources.
      '';
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
    testing = mkOption {
      type =
        with types;
        submodule {
          options = {
            enable = lib.mkEnableOption "emit testing packages";
            extraTestSources = mkOption {
              type = with types; listOf path;
              default = [ ];
              description = "Paths to additional test source files.";
            };
            cargoTestExtraArgs = mkOption {
              type = types.str;
              default = "";
              description = ''
                Additional flags to pass in the cargo test command.
              '';
            };
            basePgUrl = mkOption {
              type = nullOr str;
              default = null;
              description = "Base PostgreSQL DB URL required by tern-testing.";
            };
            baseMySqlUrl = mkOption {
              type = nullOr str;
              default = null;
              description = "Base MySQL DB URL required by tern-testing.";
            };
            baseSqliteUrl = mkOption {
              type = nullOr str;
              default = null;
              description = "Base SQLite DB required by tern-testing.";
            };
          };
        };
    };
  };
}
