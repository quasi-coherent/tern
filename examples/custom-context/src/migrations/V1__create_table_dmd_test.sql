CREATE TABLE dmd_test(
  id serial PRIMARY KEY,
  created_at timestamptz NOT NULL DEFAULT now(),
  x bigint,
  y text
);

ALTER TABLE dmd_test
  ADD z integer NOT NULL DEFAULT 0;

INSERT INTO dmd_test(x, y)
  VALUES (5, 'asdf'),
(10, 'qwer'),
(15, 'zxcv');
