use std::collections::BTreeSet;
use std::fmt::{self, Display, Formatter};

mod builder;
pub use builder::{QueryBuilder, QueryReader};

mod split;
pub use split::SqlDialect;

use crate::context::MigrationExecutor;
use crate::error::{TernError, TernResult};

// Placeholder for queries that don't exist yet.
const PENDING_QUERY: &str = "-- Query pending resolution.\nSELECT 1;";

/// The query to send.
///
/// A `Query` is an indexed collection of [`Statement`]s.
#[derive(Clone, Debug)]
pub struct Query {
    inner: BTreeSet<Statement>,
    no_tx: bool,
}

impl Query {
    /// Return a builder to push SQL to.
    ///
    /// `QueryBuilder` builds a [`Query`] by converting it to a [`QueryReader`]
    /// that a `Query` can read the constituent statements from.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use tern_core::query::{Query, QueryBuilder};
    /// # fn f() -> tern_core::error::TernResult<()> {
    /// let mut builder = Query::builder();
    /// builder.push_sql("CREATE TABLE blah (c1 text);");
    /// builder.push_sql("CREATE INDEX blah_idx ON blah (c1);");
    ///
    /// // Create a reader that reads into a non-transactional query:
    /// let reader = builder.into_reader().with_notx();
    /// let query = reader.read_into()?;
    ///
    /// assert_eq!(query.size(), 2);
    /// #     Ok(())
    /// # }
    /// ```
    pub fn builder() -> QueryBuilder {
        QueryBuilder::default()
    }

    /// Placeholder for a query that resolves at a later time.
    pub fn pending() -> Self {
        let stat = Statement(0, PENDING_QUERY.into());
        let mut inner = BTreeSet::new();
        inner.insert(stat);
        Self { inner, no_tx: false }
    }

    /// Return the number of `Statement`s in this query.
    pub fn size(&self) -> usize {
        if !self.no_tx {
            return 1;
        }
        self.inner.len()
    }

    /// Return this query as a single prepared statement.
    pub fn statement(&self) -> Statement {
        let sql =
            self.inner.iter().map(|st| st.raw()).collect::<Vec<_>>().join("\n");
        Statement(0, sql)
    }

    /// Override the `Query` if necessary to force it to be applied outside of a
    /// transaction.
    pub fn force_notx(self) -> Self {
        Self { no_tx: true, ..self }
    }

    /// Returns whether this `Query` is to run in a database transaction.
    pub fn in_tx(&self) -> bool {
        !self.no_tx
    }

    // Send with the executor.
    //
    // This is what we call in practice and it's arranged to make it essentially
    // impossible to do anything else.  This is to guarantee that the right
    // method to send the query is called (`send_tx` versus `send_notx`, "up"
    // versus "down").
    pub(crate) async fn send_with<E: MigrationExecutor + ?Sized>(
        &self,
        exec: &mut E,
    ) -> TernResult<()> {
        if self.no_tx {
            let stat = self.statement();
            let sql = stat.raw();
            log::trace!(sql:%, transaction = true; "send statement");
            exec.send_tx(&stat.1).await?;
        } else {
            let tot = self.size();
            for stat in self.inner.iter() {
                let sql = stat.raw();
                let idx = stat.0;
                log::trace!(
                    sql:%,
                    idx:%,
                    tot:%,
                    transaction = false;
                    "send statement",
                );
                exec.send_notx(sql)
                    .await
                    .map_err(|e| TernError::stat(e, idx))?;
            }
        }
        Ok(())
    }

    fn new(inner: BTreeSet<Statement>, no_tx: bool) -> Self {
        Self { inner, no_tx }
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let n = self.size();
        if self.no_tx {
            self.inner.iter().try_for_each(|st| {
                let num = st.0;
                let q = &st.1;
                writeln!(f, "[Statement {num}/{n}]")?;
                writeln!(f, "{q}")
            })
        } else {
            let st = self.statement();
            let q = &st.1;
            writeln!(f, "{q}")
        }
    }
}

/// A SQL statement.
///
/// A `Statement` is one or more individual SQL expressions that are sent in one
/// prepared statement.
#[derive(Clone, Debug, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct Statement(pub(crate) u32, pub(crate) String);

impl Statement {
    fn init() -> Self {
        Self::default()
    }

    pub(crate) fn raw(&self) -> &str {
        &self.1
    }

    pub(super) fn push_sql<T: AsRef<str>>(&mut self, sql: T) {
        self.1.push_str(sql.as_ref());
    }

    /// Reset for the next statement, returning the current.
    fn take(&mut self) -> Statement {
        let idx = self.0;
        let stat = std::mem::take(&mut self.1);
        let this = Statement(idx, stat);
        self.0 += 1;
        this
    }
}

#[cfg(test)]

mod tests {

    use super::*;

    #[test]
    fn dollar_quoted_postgres() {
        // Semicolons inside $$ bodies must not split the statement.
        const SQL: &str = "
CREATE FUNCTION add(a int, b int) RETURNS int AS $$
BEGIN
  RETURN a + b; -- semicolon inside dollar body
END;
$$ LANGUAGE plpgsql;";
        let res = QueryReader::from_sql(SQL).and_then(|r| r.read_into());
        assert!(res.is_ok_and(|q| q.in_tx() && q.size() == 1))
    }

    #[test]
    fn dollar_quoted_with_tag_postgres() {
        const SQL: &str = "
-- tern:noTransaction,postgres
DO $body$
BEGIN
  RAISE NOTICE 'step; one';
END;
$body$;
SELECT 1;
";
        let res = QueryReader::from_sql(SQL).and_then(|r| r.read_into());
        assert!(res.is_ok_and(|q| q.size() == 2));
    }

    #[test]
    fn mysql_backslash_escape() {
        // Backslash-escaped quote inside a string must not end the string
        // early.
        const SQL: &str = "
-- tern:noTransaction,mysql
INSERT INTO t (col) VALUES ('it\\'s fine');
SELECT 1;";
        let res = QueryReader::from_sql(SQL).and_then(|r| r.read_into());
        assert!(res.is_ok_and(|q| q.size() == 2));
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

        let res = QueryReader::from_sql(SQL).and_then(|r| r.read_into());
        assert!(res.is_ok_and(|q| q.size() == 2));
    }
}
