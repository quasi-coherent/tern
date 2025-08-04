use crate::context::MigrationContext;
use crate::error::TernResult;
use crate::source::migration::AppliedMigration;

use futures_core::Future;
use regex::Regex;
use std::fmt::Write;

/// A helper trait for [`Migration`].
///
/// The user implements this for Rust migrations.
/// [`Migration`]: crate::source::migration::Migration
pub trait QueryBuilder {
    /// The context for running the migration this query is for.
    type Ctx: MigrationContext;

    /// Asynchronously produce the migration query.
    fn build(&self, ctx: &mut Self::Ctx) -> impl Future<Output = TernResult<Query>> + Send;
}

/// Library of "administrative" queries that are needed during a migration run.
pub trait QueryRepository {
    /// The query that creates the schema history table or does nothing if it
    /// already exists.
    fn create_history_if_not_exists_query(history_table: &str) -> Query;

    /// The query that drops the history table if requested.
    fn drop_history_query(history_table: &str) -> Query;

    /// The query to update the schema history table with an applied migration.
    fn insert_into_history_query(history_table: &str, applied: &AppliedMigration) -> Query;

    /// The query to return all rows from the schema history table.
    fn select_star_from_history_query(history_table: &str) -> Query;

    /// Query to insert or update a record in the history table.
    fn upsert_history_query(history_table: &str, applied: &AppliedMigration) -> Query;
}

/// A SQL query.
#[derive(Debug, Clone)]
pub struct Query(pub(crate) String);

impl Query {
    pub fn new<T: Into<String>>(sql: T) -> Self {
        Self(sql.into())
    }

    /// Return the serialization of the SQL making the query.
    pub fn sql(&self) -> &str {
        &self.0
    }

    /// Insert another query before this one.
    pub fn prepend(&mut self, other: Self) -> TernResult<()> {
        let mut buf = String::new();
        writeln!(buf, "{}", other.0)?;
        writeln!(buf, "{}", self.0)?;
        self.0 = buf;
        Ok(())
    }

    /// Add another query to the end of this one.
    pub fn append(&mut self, other: Self) -> TernResult<()> {
        let mut buf = String::new();
        writeln!(buf, "{}", self.0)?;
        writeln!(buf, "{}", other.0)?;
        self.0 = buf;
        Ok(())
    }

    /// Split a query comprised of multiple statements.
    ///
    /// For queries having `no_tx = true`, a migration comprised of multiple,
    /// separate SQL statements needs to be broken up so that the statements can
    /// run sequentially.  Otherwise, many backends will run the collection of
    /// statements in a transaction automatically, which breaches the `no_tx`
    /// contract.
    ///
    /// _Warning_: This is sensitive to the particular character sequence used in
    /// writing comments.  Only `--` and C-style `/* ... */` are treated
    /// correctly because this is valid comment syntax in any of the supported
    /// backends.  A line starting with `#`, for instance, will not be treated as
    /// a comment since this is only valid in MySql.  Unsupported comment format
    /// can have unpredictable and undesirable outcome.
    pub fn split_statements(&self) -> TernResult<Vec<String>> {
        let mut statements = Vec::new();
        self.sanitize()
            .lines()
            .try_fold(String::new(), |mut buf, line| {
                if line.trim().is_empty() {
                    return Ok(buf);
                }
                writeln!(buf, "{line}")?;
                // If the line ends with `;` this is the end of the statement, so
                // push the accumulated buffer to the vector and start a new one.
                if line.ends_with(";") {
                    statements.push(buf);
                    Ok::<String, std::fmt::Error>(String::new())
                } else {
                    Ok(buf)
                }
            })?;

        Ok(statements)
    }

    fn sanitize(&self) -> String {
        let block_comment = Regex::new(r"\/\*(?s).*?(?-s)\*\/").unwrap();
        let sql = self
            .sql()
            .trim()
            .lines()
            .filter(|line| {
                let line = line.trim();
                !line.starts_with("--") || line.is_empty()
            })
            .map(|line| {
                // Remove trailing comments: "SELECT a -- like this"
                let mut stripped = line.to_string();
                let offset = stripped.find("--").unwrap_or(stripped.len());
                stripped.replace_range(offset.., "");
                stripped.trim_end().to_string()
            })
            .collect::<Vec<_>>()
            .join("\n");
        let stripped = block_comment.replace_all(&sql, "");

        if !stripped.ends_with(";") {
            format!("{stripped};")
        } else {
            stripped.to_string()
        }
    }
}

impl std::fmt::Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::Query;

    const SQL_IN1: &str = "
-- This is a comment.
SELECT
  *
FROM
  the_schema.the_table
WHERE
  everything = 'is_good'
";
    const SQL_OUT1: &str = "SELECT
  *
FROM
  the_schema.the_table
WHERE
  everything = 'is_good';
";
    const SQL_IN2: &str = "
-- tern:noTransaction
SELECT count(e.*),
  e.x,
  e.y -- This is the column called `y`
FROM /* A comment block can even be like this */ the_table
  as e
JOIN another USING (id)
/*
This is a multi
line
comment
*/
WHERE false;

SELECT a
from x
-- Asdfsdfsdfsdfsdsdf /* Unnecessary comment */
where false

;
";
    const SQL_OUT21: &str = "SELECT count(e.*),
  e.x,
  e.y
FROM  the_table
  as e
JOIN another USING (id)
WHERE false;
";

    const SQL_OUT22: &str = "SELECT a
from x
where false
;
";

    #[test]
    fn split_one() {
        let q1 = Query::new(SQL_IN1.to_string());
        let res1 = q1.split_statements();
        assert!(res1.is_ok());
        let split1 = res1.unwrap();
        assert_eq!(split1, vec![SQL_OUT1.to_string()]);
    }

    #[test]
    fn split_two() {
        let q2 = Query::new(SQL_IN2.to_string());
        let res2 = q2.split_statements();
        assert!(res2.is_ok());
        let split2 = res2.unwrap();
        assert_eq!(split2, vec![SQL_OUT21.to_string(), SQL_OUT22.to_string()]);
    }
}
