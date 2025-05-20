-- tern:noTransaction
CREATE INDEX CONCURRENTLY IF NOT EXISTS users_username_idx
  ON pg_test.users(username);
