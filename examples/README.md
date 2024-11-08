# examples

```
> $ docker compose up -d
> $ cd pg-envvar-query && cargo b
> $ export DATABASE_URL=postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable
> $ ./target/debug/pg-envvar-query migrate run
[2024-11-08T23:34:07Z INFO  derrick_migrate_cli] DisplayMigration {
        version: 1,
        state: NewApplied,
        description: "create_table_dmd_test",
        sql: "CREATE TABLE dmd_test(\n  id serial PRIMARY KEY,\n  created_at timestamptz NOT NULL DEFAULT now(),\n  x bigint,\n  y text\n);\n\nINSERT INTO dmd_test(x, y)\n  VALUES (123, 'qwer');...",
        transactional: InTransaction,
        duration_ms: Duration(
            7,
        ),
        error_reason: None,
    }
...
...
...
> $ psql "postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable"
psql (14.13 (Homebrew))
Type "help" for help.

postgres=# \dt
                List of relations
 Schema |        Name         | Type  |  Owner
--------+---------------------+-------+----------
 public | _derrick_migrations | table | postgres
 public | dmd_test            | table | postgres
(2 rows)

postgres=# select * from dmd_test
postgres-# ;
 id |          created_at           | x  |       y
----+-------------------------------+----+---------------
  1 | 2024-10-31 23:47:52.380714+00 | 25 | quasicoherent
(1 row)
...
```
