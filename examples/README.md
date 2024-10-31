# examples

```
> $ docker compose up -d
> $ export DATABASE_URL=postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable
> $ cargo b
> $

[2024-10-27T20:21:44Z DEBUG derrick_migrate::backends::sqlx::postgres] running `create table if exists` query
[2024-10-27T20:21:44Z INFO  sqlx::postgres::notice] relation "_derrick_migrations" already exists, skipping
[2024-10-27T20:21:44Z DEBUG sqlx::query] summary="CREATE TABLE IF NOT …" db.statement="\n\nCREATE TABLE IF NOT EXISTS _derrick_migrations(\n  version bigint PRIMARY KEY,\n  description text NOT NULL,\n  content text NOT NULL,\n  duration_sec bigint NOT NULL,\n  applied_at timestamptz NOT NULL DEFAULT now()\n);\n" rows_affected=0 rows_returned=0 elapsed=1.104629ms elapsed_secs=0.001104629
[2024-10-27T20:21:44Z DEBUG derrick_migrate::backends::sqlx::postgres] running select query
[2024-10-27T20:21:44Z DEBUG sqlx::query] summary="SELECT version, description, content, …" db.statement="\n\nSELECT\n  version,\n  description,\n  content,\n  duration_sec,\n  applied_at\nFROM\n  _derrick_migrations\nORDER BY\n  version;\n" rows_affected=0 rows_returned=0 elapsed=1.92359ms elapsed_secs=0.00192359
[2024-10-27T20:21:44Z INFO  derrick_migrate::runner] validated migration set
...
...
```
