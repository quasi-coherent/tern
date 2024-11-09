# examples

```
> $ docker compose up -d
> $ cd pg-envvar-query && cargo b
> $ export DATABASE_URL=postgres://postgres:password@dockerhost:5433/postgres?sslmode=disable
> $ ./target/debug/pg-envvar-query migrate run
[2024-11-09T00:08:07Z INFO  derrick_migrate::report] DisplayMigration {
        version: 1,
        state: NewApplied,
        description: "create_table_dmd_test",
        sql: "-- derrick:noTransaction\nCREATE TABLE dmd_test(\n  id serial PRIMARY KEY,\n  created_at timestamptz NOT NULL DEFAULT now(),\n  x bigint,\n  y text\n);\n\nINSERT INTO dmd_test(x, y)\n  VALUES (123, 'qwer');...",
        transactional: NoTransaction,
        duration_ms: Duration(
            7,
        ),
        error_reason: None,
    }
[2024-11-09T00:08:07Z INFO  derrick_migrate::report] DisplayMigration {
        version: 2,
        state: NewApplied,
        description: "insert env value",
        sql: "INSERT INTO dmd_test(x, y) VALUES (25, 'quasicoherent');...",
        transactional: NoTransaction,
        duration_ms: Duration(
            1,
        ),
        error_reason: None,
    }
[2024-11-09T00:08:07Z INFO  derrick_migrate::report] DisplayMigration {
        version: 3,
        state: NewApplied,
        description: "create_dmd_test_y_idx",
        sql: "-- derrick:noTransaction\nCREATE INDEX CONCURRENTLY IF NOT EXISTS dmd_test_y_idx ON dmd_test(y);...",
        transactional: NoTransaction,
        duration_ms: Duration(
            3,
        ),
        error_reason: None,
    }
```
