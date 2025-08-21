# partitioned

A standard index build in postgres locks out writes on the table until it is
done, so to avoid this interruption of service, an index can be created with the
keyword `CONCURRENTLY`.  For tables that are [partitioned][partitioned-table],
however, using this keyword is an error.  From the [documentation][pg-index-docs]:

> Concurrent builds for indexes on partitioned tables are currently not supported.
> However, you may concurrently build the index on each partition individually
> and then finally create the partitioned index non-concurrently in order to
> reduce the time where writes to the partitioned table will be locked out.
> In this case, building the partitioned index is a metadata only operation.

The issue with this approach, however, is that it is not known _a priori_ what
the names of the individual partitions are, and moreover, these partitions could
be created and detached periodically, so the collection of partitions to index
is different at any given time.

This example demonstrates an approach to address these challenges.

[partitioned-table]: https://www.postgresql.org/docs/current/ddl-partitioning.html
[pg-index-docs]: https://www.postgresql.org/docs/current/sql-createindex.html#SQL-CREATEINDEX-CONCURRENTLY
