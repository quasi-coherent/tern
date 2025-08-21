CREATE TABLE example.partitioned(
  id uuid NOT NULL DEFAULT uuid_generate_v1mc(),
  created_at timestamptz(3) NOT NULL DEFAULT now(),
  name text,
  email text,
  age int
)
PARTITION BY RANGE (created_at);

-- Partitioned tables cannot have unique indices unless they contain the
-- partition key.
CREATE INDEX partitioned_created_at_id_idx ON example.partitioned(created_at, id);

-- The `pg_partman` extension uses this template table to manage certain properties that aren't inherently
-- done by postgres; see the documentation at
-- https://www.github.com/pgpartman/pg_partman/blob/master/doc/pg_partman.md#child-table-property-inheritance
CREATE TABLE example.partitioned_template(
  LIKE example.partitioned
);

ALTER TABLE example.partitioned_template
  ADD PRIMARY KEY (created_at, id);

-- Define the partitioning schema that will be used.
--
-- By default this will create the next four partitions.
SELECT
  partman.create_parent(p_parent_table := 'example.partitioned', p_template_table :=
    'example.partitioned_template', p_control := 'created_at', p_interval := '1 week', p_type :=
    'range');
