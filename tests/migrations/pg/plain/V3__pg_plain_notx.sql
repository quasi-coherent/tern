-- tern:noTransaction,postgres
CREATE INDEX CONCURRENTLY tern_pg_plain_x_idx ON tern_pg_plain (x);
-- tern:begin_tx
INSERT INTO tern_pg_plain (x, y) VALUES (100, 'grouped');
INSERT INTO tern_pg_plain (x, y) VALUES (101, 'grouped');
-- tern:end_tx
