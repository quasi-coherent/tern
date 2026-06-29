{
  den,
  config,
  lib,
  ...
}:
let
  inherit (lib) mkEnableOption mkOption types;
  cfg = config.tern.db;
in
{
  options.tern.db = {
    enable = mkEnableOption "db";
    hostname = mkOption {
      type = types.str;
      default = "terndb";
      description = "What to use for the networking.hostname value.";
    };
    lima = mkOption {
      type = types.deferredModule;
      default = { };
      description = "limavm-nix.lima options tree";
    };
    connStrs = mkOption {
      type = with types; attrsOf str;
      default = {
        PG_DATABASE_URL = lib.mkIf cfg.postgres.enable cfg.postgres.connStr;
        MYSQL_DATABASE_URL = lib.mkIf cfg.mysql.enable cfg.mysql.connStr;
      };
      readOnly = true;
      description = ''
        Mapping of environment variable key to db connection string.
      '';
    };
    portForwards = mkOption {
      type = with types; listOf (attrsOf str);
      default =
        [ ]
        ++ lib.optionals cfg.postgres.enable cfg.postgres.portForward
        ++ lib.optionals cfg.mysql.enable cfg.mysql.portForward;
      readOnly = true;
      internal = true;
      description = ''
        List of port forward mappings.
      '';
    };
  };

  config =
    let
      pgCfg = cfg.postgres;
      mysqlCfg = cfg.mysql;
    in
    lib.mkIf cfg.enable {
      den = {
        default = {
          includes = [
            den.batteries.define-user
            den.batteries.hostname
            den.batteries.inputs'
            den.batteries.self'
          ];
          nixos.system.stateVersion = "26.05";
        };

        aspects.tern = {
          nixos = { pkgs, ... }: {
            networking.hostname = cfg.tern.db.hostname;
            users.groups.tern = { };
            users.users.tern = {
              group = "tern";
              isSystemUser = true;
              shell = pkgs.zsh;
            };
          };

          limaGuest = {
            lima = cfg.lima // {
              portForwards = cfg.lima.portForwards ++ cfg.portForwards;
            };
          };

          includes = [
            den.batteries.limaPackages
          ]
          ++ lib.optionals pgCfg.enable den.aspects.postgres
          ++ lib.optionals mysqlCfg.enable den.aspects.mysql;
        };
      };
    };
}
