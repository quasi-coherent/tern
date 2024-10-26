# derrick

Work-in-progress crate for database migrations that enables Rust migrations to depend on context,
e.g., a migration to add an index to a partitioned PostgreSQL table might need knowledge of attached
partitions at runtime.
