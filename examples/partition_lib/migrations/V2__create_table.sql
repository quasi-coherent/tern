-- This table will be range partitioned on the `created_at` timestamp column.
-- Each partition will be one week wide.
CREATE TABLE IF NOT EXISTS examples.partition_example (
  part_id uuid NOT NULL DEFAULT uuid_generate_v1mc(),
  created_at timestamptz(3) NOT NULL DEFAULT now(),
  username text,
  user_email text,
  user_age integer
) PARTITION BY RANGE(created_at);

-- Partitioned tables cannot have unique indices unless they contain the
-- partition key.
CREATE INDEX partition_eg_created_at_id_idx ON example.partitioned(created_at, part_id);

-- The pgpartman extension uses this to associate properties of child tables that
-- are not supported by pgsql itself.
CREATE TABLE examples.partition_example_template (
  LIKE examples.partition_example
  );

-- A primary key on the partitioned table only enforces uniqueness locally.  This
-- is a table property not covered by pgsql.
ALTER TABLE examples.partition_example_template
  ADD PRIMARY KEY (created_at, part_id);

-- Define the partitioning scheme that will be used.
--
-- By default this will create the next four partitions.
SELECT
  partman.create_parent(
    p_parent_table := 'example.partition_example',
    p_template_table := 'example.partition_example_template',
    p_control := 'created_at',
    p_interval := '1 week',
    p_type := 'range');
