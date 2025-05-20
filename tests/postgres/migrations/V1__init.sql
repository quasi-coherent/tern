CREATE SCHEMA IF NOT EXISTS pg_test;

CREATE TABLE IF NOT EXISTS pg_test.users (
    id serial PRIMARY KEY,
    inserted_at timestamptz NOT NULL DEFAULT now(),
    first_name text NOT NULL,
    last_name text NOT NULL,
    email text,
    age int
);
