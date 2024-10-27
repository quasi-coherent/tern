# derrick

Work-in-progress crate for database migrations that enables Rust migrations to depend on a runtime
context, e.g., a migration to add an index to a partitioned PostgreSQL table might need to know
currently attached partitions before the query can be defined.
