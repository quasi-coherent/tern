use regex::Regex;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead as _, BufReader, Read};
use std::path::Path;
use std::str::FromStr as _;
use std::sync::OnceLock;

use crate::error::{TernError, TernResult};
use crate::query::{SqlDialect, Statement, parse};

/// Read a query and then split it.
#[derive(Default)]
pub(super) struct QueryReader {
    pub(super) header: Option<Header>,
    pub(super) stmts: std::vec::IntoIter<String>,
}

impl QueryReader {
    /// Build a `Query` from the contents of a file.
    pub(super) fn from_file<P: AsRef<Path>>(path: P) -> TernResult<Self> {
        // The header can be preceded by blank lines but nothing else.
        let tmp = File::open(&path)?;
        let mut lines = BufReader::new(tmp).lines();
        let Some(l) = lines.next().transpose()? else {
            return Err(TernError::QueryBuilder("empty".into()))?;
        };
        let hdr = Header::from_line(&l)?;
        let ff = File::open(path)?;
        Self::new(ff, hdr)
    }

    /// Build a `Query` from a raw SQL string.
    pub(super) fn from_sql<T: ?Sized + AsRef<str>>(
        sql: &T,
        header: Option<Header>,
    ) -> TernResult<Self> {
        let sql_str = sql.as_ref();
        // The header can be preceded by blank lines but nothing else.
        let hdr = if header.is_none() {
            let Some(l) = sql_str.lines().find(|l| !l.trim().is_empty()) else {
                return Err(TernError::QueryBuilder("empty".into()))?;
            };
            Header::from_line(&l)?
        } else {
            header
        };
        Self::new(sql_str.as_bytes(), hdr)
    }

    pub(super) fn no_tx(&self) -> bool {
        self.header.is_some_and(|h| h.no_tx)
    }

    pub(super) fn build(&mut self, buf: &mut BTreeSet<Statement>) {
        if self.no_tx() {
            self.build_no_tx(buf);
        } else {
            self.build_tx(buf);
        }
    }

    fn new<R: Read>(mut reader: R, header: Option<Header>) -> TernResult<Self> {
        let mut input = String::new();
        reader.read_to_string(&mut input)?;
        let dialect = header.and_then(|h| h.dialect);
        let stmts = parse::split_sql(&input, dialect).into_iter();
        Ok(Self { header, stmts })
    }

    fn build_tx(&mut self, buf: &mut BTreeSet<Statement>) {
        let mut stat = Statement::default();
        while let Some(sql) = self.stmts.next() {
            stat.push_sql(sql);
        }
        buf.insert(stat);
    }

    fn build_no_tx(&mut self, buf: &mut BTreeSet<Statement>) {
        let mut idx = 0_u32;
        while let Some(sql) = self.stmts.next() {
            let stat = Statement::new(idx, &sql);
            buf.insert(stat);
            idx += 1;
        }
    }
}

fn tern_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\-{2,}.*tern:([\w\,]*)").unwrap())
}

fn tern_annot(line: &str) -> Option<&str> {
    let re = tern_re();
    let caps = re.captures(line)?;
    let annot = caps.get(1)?;
    if annot.is_empty() {
        return None;
    }
    Some(annot.as_str())
}

// The acceptable ways to annotate the first line of a migration source.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Header {
    pub(super) no_tx: bool,
    pub(super) dialect: Option<SqlDialect>,
}

impl Header {
    fn from_line(line: &str) -> TernResult<Option<Self>> {
        let Some(annot) = tern_annot(line) else {
            return Ok(None);
        };
        let no_tx = annot.contains("noTransaction");
        // For both "noTransaction,pg" and "pg,noTransaction":
        let dstr = annot.replace("noTransaction", "").replace(",", "");
        if dstr.is_empty() {
            return Ok(Some(Self { no_tx, dialect: None }));
        }
        let dialect = SqlDialect::from_str(&dstr)?;
        Ok(Some(Self { no_tx, dialect: Some(dialect) }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollar_quoted_postgres() {
        const SQL: &str = "
CREATE FUNCTION add(a int, b int) RETURNS int AS $$
BEGIN
  RETURN a + b; -- semicolon inside dollar body
END;
$$ LANGUAGE plpgsql;";
        // No `tern:` annotation means no header.
        let l = SQL.lines().find(|l| !l.trim().is_empty()).unwrap();
        let res = Header::from_line(l);
        assert!(res.is_ok());
        let header = res.unwrap();
        assert!(header.is_none())
    }

    #[test]
    fn mysql_backslash_escape() {
        const SQL: &str = "
-- tern:noTransaction,mysql
INSERT INTO t (col) VALUES ('it\\'s fine');
SELECT 1;";
        let l = SQL.lines().find(|l| !l.trim().is_empty()).unwrap();
        let res = Header::from_line(l);
        assert!(res.is_ok());
        let header = res.unwrap();
        assert!(
            header.is_some_and(
                |h| h.no_tx && h.dialect == Some(SqlDialect::MySql)
            )
        )
    }

    #[test]
    fn handles_multiple_no_tx() {
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
  AND recent = true;"#;
        let l = SQL.lines().find(|l| !l.trim().is_empty()).unwrap();
        let res = Header::from_line(l);
        assert!(res.is_ok());
        let header = res.unwrap();
        assert!(
            header.is_some_and(
                |h| h.no_tx && h.dialect == Some(SqlDialect::Postgres)
            )
        )
    }

    fn split(sql: &str, dialect: SqlDialect) -> Vec<String> {
        super::parse::split_sql(sql, Some(dialect))
    }

    #[test]
    fn splits_simple_statements() {
        let stmts = split("SELECT 1;\nSELECT 2;", SqlDialect::Postgres);
        assert_eq!(stmts, vec!["SELECT 1;", "SELECT 2;"]);
    }

    #[test]
    fn drops_trailing_whitespace_chunk() {
        // Whitespace after the final `;` must not become a statement.
        let stmts = split("SELECT 1;\n\n  \n", SqlDialect::Postgres);
        assert_eq!(stmts, vec!["SELECT 1;"]);
    }

    #[test]
    fn keeps_comment_only_chunk() {
        // Comment-only chunks can carry `tern:` annotations.
        let stmts = split("SELECT 1;\n-- tern:end_tx\n", SqlDialect::Postgres);
        assert_eq!(stmts, vec!["SELECT 1;", "-- tern:end_tx"]);
    }

    #[test]
    fn semicolons_in_strings_and_comments() {
        let stmts = split(
            "SELECT 'a;b', \"c;d\" FROM t; -- trailing; comment\n\
             /* block; comment */ SELECT 2;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[1].ends_with("SELECT 2;"));
    }

    #[test]
    fn nested_block_comment_postgres() {
        let stmts = split(
            "/* outer /* inner; */ still; comment */\nSELECT 1;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 1);
    }

    #[test]
    fn dollar_quote_anonymous() {
        let stmts = split(
            "CREATE FUNCTION add(a int, b int) RETURNS int AS $$\n\
             BEGIN\n  RETURN a + b;\nEND;\n$$ LANGUAGE plpgsql;\n\
             SELECT 1;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("RETURN a + b;"));
    }

    #[test]
    fn dollar_quote_tagged() {
        let stmts = split(
            "DO $body$\nBEGIN\n  RAISE NOTICE 'step; one';\nEND;\n$body$;\n\
             SELECT 1;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].ends_with("$body$;"));
    }

    #[test]
    fn dollar_parameter_is_not_a_quote() {
        let stmts = split(
            "PREPARE p AS SELECT $1;\nEXECUTE p('x');",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn estring_backslash_escape_postgres() {
        let stmts = split(
            "INSERT INTO t VALUES (E'it\\'s; fine');\nSELECT 1;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn backslash_escape_mysql() {
        let stmts = split(
            "INSERT INTO t (col) VALUES ('it\\'s; fine');\nSELECT 1;",
            SqlDialect::MySql,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn backtick_and_hash_comment_mysql() {
        let stmts = split(
            "SELECT `a;b` FROM t; # inline; comment\nSELECT 2;",
            SqlDialect::MySql,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn bare_begin_end_block_is_one_statement() {
        let stmts = split(
            "BEGIN;\nUPDATE t SET a = 1;\nDELETE FROM u;\nEND;\nSELECT 1;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].starts_with("BEGIN;"));
        assert!(stmts[0].ends_with("END;"));
    }

    #[test]
    fn begin_commit_block_is_one_statement() {
        let stmts = split(
            "BEGIN;\nUPDATE t SET a = 1;\nCOMMIT;\nSELECT 1;",
            SqlDialect::Sqlite,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn sqlite_trigger_body() {
        let stmts = split(
            "CREATE TRIGGER trg AFTER INSERT ON t\n\
             BEGIN\n\
               UPDATE t SET n = n + 1;\n\
               INSERT INTO log VALUES (new.id);\n\
             END;\n\
             CREATE INDEX t_idx ON t (n);",
            SqlDialect::Sqlite,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].ends_with("END;"));
    }

    #[test]
    fn mysql_trigger_with_end_if() {
        let stmts = split(
            "CREATE TRIGGER trg BEFORE INSERT ON t FOR EACH ROW\n\
             BEGIN\n\
               IF NEW.a IS NULL THEN\n\
                 SET NEW.a = 0;\n\
               END IF;\n\
             END;\n\
             SELECT 1;",
            SqlDialect::MySql,
        );
        assert_eq!(stmts.len(), 2);
        assert!(stmts[0].contains("END IF;"));
    }

    #[test]
    fn end_and_its_closer_split_across_lines() {
        // The `END`/`IF` pair straddles a line boundary; the suspended
        // decision must carry over so `IF` isn't read as a new opener.
        let stmts = split(
            "CREATE TRIGGER trg BEFORE INSERT ON t FOR EACH ROW\n\
             BEGIN\n\
               IF NEW.a IS NULL THEN\n\
                 SET NEW.a = 0;\n\
               END\n\
               IF;\n\
             END;\n\
             SELECT 1;",
            SqlDialect::MySql,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn case_end_in_select_is_balanced() {
        let stmts = split(
            "SELECT CASE WHEN a THEN 1 ELSE 2 END AS x FROM t;\nSELECT 2;",
            SqlDialect::Postgres,
        );
        assert_eq!(stmts.len(), 2);
    }

    #[test]
    fn if_not_exists_is_not_a_block() {
        let stmts = split(
            "CREATE TABLE IF NOT EXISTS t (a int);\n\
             DROP TABLE IF EXISTS u;\n\
             SELECT 1;",
            SqlDialect::Sqlite,
        );
        assert_eq!(stmts.len(), 3);
    }

    #[test]
    fn no_trailing_terminator_flushes_at_eof() {
        let stmts = split("SELECT 1;\nSELECT 2", SqlDialect::Postgres);
        assert_eq!(stmts, vec!["SELECT 1;", "SELECT 2"]);
    }
}
