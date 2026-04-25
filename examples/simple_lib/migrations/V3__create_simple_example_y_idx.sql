-- tern:noTransaction
CREATE INDEX CONCURRENTLY IF NOT EXISTS simple_example_y_idx ON simple_example (y);

-- To update pg_stats with the new index.
-- This and the index build happen independently outside of a transaction.
VACUUM ANALYZE simple_example;
