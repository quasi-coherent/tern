{
  config,
  lib,
  ...
}:
let
  inherit (lib) mkEnableOption mkOption types;
  cfg = config.tern.db.postgres;
in
{
  options.tern.db.postgres = {
    enable = mkEnableOption "postgres";
    package = mkOption {
      type = with types; nullOr package;
      default = null;
      description = "The package being used by postgres.";
    };
    port = mkOption {
      type = types.port;
      default = 5432;
      description = ''
        Port on the VM host to forward the guest pgsql server host port.
      '';
    };
    extensions = mkOption {
      type = with types; listOf package;
      default = [ ];
      description = "List of pgsql extensions to enable.";
    };
    initScript = mkOption {
      type = with types; nullOr path;
      default = null;
      description = ''
        A file containing SQL statements to execute on first startup.
      '';
    };
    connStr = mkOption {
      type = types.str;
      default = "postgres://tern@localhost:${cfg.port}/tern";
      description = "Postgres connection string.";
      readOnly = true;
    };
    portForward = mkOption {
      type = with types; attrsOf port;
      default = {
        guestPort = 5432;
        hostPort = cfg.port;
      };
      readOnly = true;
      internal = true;
    };
  };

  config = {
    den.aspects.pgsql = lib.mkIf cfg.enable {
      nixos.services = {
        postgresql = {
          enable = true;
          ensureUsers = [
            {
              name = "tern";
              ensureClauses = {
                superuser = true;
                createrole = true;
                createdb = true;
              };
              ensureDBOwnership = true;
            }
          ];
          ensureDatabases = [ "tern" ];
          extensions = cfg.extensions;
          initialScript = cfg.initScript;
        }
        // lib.mkIf (!(isNull cfg.package)) { inherit (cfg) package; };
      };
    };
  };
}
