{
  lib,
  mkBuildArgs,
  ternConfig,
  ternTestingConfig,
  writeShellApplication,
}:
let
  inherit (ternTestingConfig)
    basePgUrl
    baseMySqlUrl
    baseSqliteUrl
    ;

  extraSources = ternConfig.extraSources ++ ternTestingConfig.extraTestSources;

  build = mkBuildArgs {
    inherit (ternConfig)
      cargoRoot
      rustToolchain
      cargoExtraArgs
      cargoBuildExtraArgs
      ;
    inherit extraSources;
  };

  inherit (build) buildArgs;
  inherit (buildArgs) pname;

  tern-it = build.crane.buildPackage {
    inherit (ternTestingConfig) cargoTestExtraArgs;
    inherit (build) cargoArtifacts;
    inherit (buildArgs)
      version
      src
      strictDeps
      cargoExtraArgs
      ;
    pname = "${pname}-it";
    doCheck = false;
  };

  tern-doit =
    let
      pgUrl = basePgUrl ? "";
      mysqlUrl = baseMySqlUrl ? "";
      sqliteUrl = baseSqliteUrl ? "";
    in
    writeShellApplication {
      name = "${pname}-doit";
      runtimeInputs = [ tern-it ];
      text = ''
        set -e
        export PATH=${lib.getBin tern-it}:$PATH
        PG_DATABASE_URL="${pgUrl}"
        MYSQL_DATABASE_URL="${mysqlUrl}"
        SQLITE_DATABASE_URL="${sqliteUrl}"
        export PG_DATABASE_URL
        export MYSQL_DATABASE_URL
        export SQLITE_DATABASE_URL
        exec ${pname}-it "$@"
      '';
    };
in
{
  inherit tern-it tern-doit;
}
