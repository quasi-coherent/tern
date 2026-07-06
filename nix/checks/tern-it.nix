{
  buildArgs,
  cargoArtifacts,
  crane,
  docker,
  lib,
  writeShellApplication,
  writeTextFile,
}:
let
  dockerComposeYaml = writeTextFile {
    name = "docker-compose.yaml";
    text = builtins.readFile ../../docker-compose.yaml;
  };

  ternTests = crane.buildPackage (
    buildArgs
    // {
      inherit cargoArtifacts;
      pname = "tern-integration-tests";
      cargoExtraArgs = "--tests --features sqlx_postgres,sqlx_mysql,sqlx_sqlite";
      doCheck = false;
      # Test targets land in target/release not target/debug like normal because
      # crane sets `buildPhaseCargoCommand = "--profile release"` by default.
      installPhaseCommand = ''
        shopt -s nullglob
        mkdir -p $out/bin
        for f in target/release/deps/{pg,mysql,sqlite}-*; do
            case "$f" in
                *.d)        continue      ;;
                */pg-*)     prefix=pg     ;;
                */mysql-*)  prefix=mysql  ;;
                */sqlite-*) prefix=sqlite ;;
            esac
            install -m755 "$f" $out/bin/$prefix-test
        done
      '';
    }
  );

  tern-doit =
    let
      PG_DATABASE_URL = "postgres://tern:password@localhost:5433/tern";
      MYSQL_DATABASE_URL = "mysql://tern:password@localhost:3307/tern";
    in
    writeShellApplication {
      name = "tern-it";
      runtimeInputs = [
        docker
        ternTests
      ];
      text = ''
        set -e

        export PATH=${
          lib.makeBinPath [
            docker
            ternTests
          ]
        }:$PATH

        testbin=""
        case "$1" in
          mysql|pg|sqlite) testbin="$1"    ;;
          *) echo "invalid command $1"; exit 1 ;;
        esac

        docker compose up -f ${dockerComposeYaml} -d

        if [[ "$testbin" == "mysql" ]]; then
            MYSQL_DATABASE_URL="${MYSQL_DATABASE_URL}" mysql-test
        elif [[ "$testbin" == "pg" ]]; then
            PG_DATABASE_URL="${PG_DATABASE_URL}" pg-test
        else
            sqlite-test
        fi
      '';
    };
in
{
  inherit tern-doit ternTests;
}
