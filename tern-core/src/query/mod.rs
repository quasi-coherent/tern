use regex::Regex;
use std::fmt::Write;
use std::io::Error as IoError;
use std::sync::OnceLock;

use crate::error::{Error, TernResult};

mod split;
use split::{Parser, SqlDialect};

/// A SQL query.
#[derive(Debug, Clone)]
pub struct Query(pub(crate) String);

impl Query {
    /// New `Query` from a string.
    pub fn new(sql: String) -> Self {
        Self(sql)
    }

    /// Return the underlying query text.
    pub fn sql(&self) -> &str {
        &self.0
    }

    /// Add another query to the end of this one.
    pub fn append(&mut self, other: Self) -> TernResult<()> {
        let mut buf = String::new();
        writeln!(buf, "{}", self.0)?;
        writeln!(buf, "{}", other.0)?;
        self.0 = buf;
        Ok(())
    }

    /// Split a query comprised of multiple statements into a collection of
    /// the atomic, single statements.
    ///
    /// This is necessary to honor the contract of "tern:noTransaction", since
    /// sending a query with multiple statements is treated as one prepared
    /// statement and ran in a transaction automatically.
    pub fn split_statements(&self) -> TernResult<Vec<String>> {
        let sql = self.0.as_bytes();
        let dialect = self.detect_dialect().unwrap_or(SqlDialect::Postgres);

        let mut parser = Parser::with_dialect(sql, sql.len(), dialect);
        let mut stats = Vec::new();

        while let Some(stat_bytes) =
            parser.read_statement().map_err(Error::split_err(stats.len()))?
        {
            let raw = String::from_utf8(stat_bytes)
                .map_err(IoError::other)
                .map_err(Error::split_err(stats.len()))?;

            // Drop "queries" that are only whitespace, like a trailing newline
            // at the end of the file, or newlines between queries.
            let stat = raw.trim();
            if !stat.is_empty() {
                stats.push(stat.to_string());
            }
        }

        Ok(stats)
    }

    fn detect_dialect(&self) -> Option<SqlDialect> {
        let mut first = self.0.lines().take(1);
        let l = first.next()?;
        let re = dialect_re();
        let caps = re.captures(l)?;
        Some(match caps.get(1)?.as_str() {
            "sqlite" => SqlDialect::Sqlite,
            "mysql" => SqlDialect::MySql,
            "postgres" => SqlDialect::Postgres,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn dialect_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r".*tern:noTransaction,?([a-z]*)").unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_dialect() {
        let x = "-- tern:noTransaction";
        let y = "-- tern:noTransaction,sqlite";
        let z = "-- Comment with tern:noTransaction,postgres in the middle";
        let t = "-- tern:noTransaction,dynamodb";
        let xx = Query::new(x.into());
        let yy = Query::new(y.into());
        let zz = Query::new(z.into());
        let tt = Query::new(t.into());
        assert_eq!(xx.detect_dialect(), None);
        assert_eq!(yy.detect_dialect(), Some(SqlDialect::Sqlite));
        assert_eq!(zz.detect_dialect(), Some(SqlDialect::Postgres));
        assert_eq!(tt.detect_dialect(), None);
    }

    #[test]
    fn handles_single() {
        const SQL: &str = "
SELECT
 blah,
 whatever, -- this column is way totally
 region_id,
 prod_discount_region_start_date_early,
 prod_discount_region_start_date,
 prod_discount_region_end_date
FROM
 prod_discount_date
WHERE
 whatever = 'way;totally' -- Way; very totally
 AND region_id = 5;
";
        let query = Query::new(SQL.into());
        let res = query.split_statements();
        assert!(res.is_ok());
        let mut stats = res.unwrap();
        assert_eq!(stats.len(), 1);
        let stat = stats.pop().unwrap();
        assert_eq!(&stat, SQL.trim());
    }

    #[test]
    fn empty_input() {
        let query = Query::new("".into());
        let res = query.split_statements().unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn no_trailing_semicolon() {
        let query = Query::new("SELECT 1".into());
        let res = query.split_statements().unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0], "SELECT 1");
    }

    #[test]
    fn whitespace_only_between_statements() {
        // Blank lines between statements should not produce empty entries.
        const SQL: &str = "SELECT 1;\n\n\nSELECT 2;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn semicolon_in_line_comment_not_a_terminator() {
        // A semicolon after -- should not split the statement.
        const SQL: &str = "SELECT 1 -- ignore; this\n, 2;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 1);
    }

    #[test]
    fn dollar_quoted_postgres() {
        // Semicolons inside $$ bodies must not split the statement.
        const SQL: &str = "-- tern:noTransaction,postgres
CREATE FUNCTION add(a int, b int) RETURNS int AS $$
BEGIN
  RETURN a + b; -- semicolon inside dollar body
END;
$$ LANGUAGE plpgsql;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 1);
    }

    #[test]
    fn dollar_quoted_with_tag_postgres() {
        // Named dollar-quote tag: $body$...$body$.
        const SQL: &str = "-- tern:noTransaction,postgres
DO $body$
BEGIN
  RAISE NOTICE 'step; one';
END;
$body$;

SELECT 1;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn mysql_backslash_escape() {
        // Backslash-escaped quote inside a string must not end the string early.
        const SQL: &str = "-- tern:noTransaction,mysql
INSERT INTO t (col) VALUES ('it\\'s fine');

SELECT 1;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 2);
    }

    #[test]
    fn semicolon_in_block_comment_not_a_terminator() {
        const SQL: &str = "SELECT 1 /* this; is ignored */;";
        let res = Query::new(SQL.into()).split_statements().unwrap();
        assert_eq!(res.len(), 1);
    }

    #[test]
    fn handles_multiple() {
        const SQL: &str = r#"
-- tern:noTransaction,postgres
SELECT
  column1 AS "asdf;lkh",
  column2
FROM
  the_table as a
/* Why
would anyone do this;
it's absurd
*/
JOIN
  the_other_table as b
USING (column3);

SELECT * INTO the_table_recent
FROM the_table
WHERE
  column1 != 'string--with--special/*characters*/and--terminator;'
  AND recent = true;
"#;
        let query = Query::new(SQL.into());
        let res = query.split_statements();
        assert!(res.is_ok_and(|ss| ss.len() == 2));
    }
}
