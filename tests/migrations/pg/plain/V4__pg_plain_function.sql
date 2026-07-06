CREATE FUNCTION tern_pg_plain_count() RETURNS bigint AS $body$
BEGIN
  -- Semicolons inside the dollar-quoted body must not split this statement.
  RETURN (SELECT count(*) FROM tern_pg_plain);
END;
$body$ LANGUAGE plpgsql;
