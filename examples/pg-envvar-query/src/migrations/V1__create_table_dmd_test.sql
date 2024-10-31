CREATE TABLE dmd_test(
  id serial PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now(),
  x bigint,
  y text
);
