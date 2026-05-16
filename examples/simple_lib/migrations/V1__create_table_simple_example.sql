CREATE TABLE simple_example(
  id serial PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now(),
  x bigint,
  y text
);

ALTER TABLE simple_example
  ADD z integer NOT NULL DEFAULT 0;
