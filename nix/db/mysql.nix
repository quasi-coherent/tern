{
  config,
  lib,
  ...
}:
let
  inherit (lib) mkEnableOption mkOption types;
  cfg = config.tern.db.mysql;
in
{
  options.tern.db.mysql = {
    enable = mkEnableOption "mysql";
    package = mkOption {
      type = with types; nullOr package;
      default = null;
      description = "The package being used by mysql.";
    };
    port = mkOption {
      type = types.port;
      default = 3306;
      description = ''
        Port on the VM host to forward the guest mysql server host port.
      '';
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
      default = "mysql://localhost:${cfg.port}/tern?user=tern";
      description = "MySQL connection string.";
      readOnly = true;
    };
    portForward = mkOption {
      type = with types; attrsOf port;
      default = {
        guestPort = 3306;
        hostPort = cfg.port;
      };
      readOnly = true;
      internal = true;
    };
  };

  config = {
    den.aspects.mysql = lib.mkIf cfg.enable {
      nixos.services = {
        mysql = {
          enable = true;
          ensureUsers = [
            {
              name = "tern";
              ensurePermissions."tern.*" = "ALL PRIVILEGES";
            }
          ];
          ensureDatabases = [ "tern" ];
          initialScript = cfg.initScript;
        }
        // lib.mkIf (!(isNull cfg.package)) { inherit (cfg) package; };
      };
    };
  };
}
