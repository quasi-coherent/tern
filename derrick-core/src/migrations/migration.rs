use super::source::{MigrationQuery, MigrationSource};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use std::borrow::Cow;

/// A migration that can be applied.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version.
    pub version: i64,
    /// Migration description.
    pub description: Cow<'static, str>,
    /// The file content of the migration.
    pub content: Cow<'static, str>,
    /// The query to run.
    pub sql: Cow<'static, str>,
    /// If true, not ran in a transaction.
    pub no_tx: bool,
}

impl Migration {
    pub fn new(source: &MigrationSource, query: MigrationQuery) -> Self {
        Self {
            version: source.version,
            description: Cow::Owned(source.description.replace("_", " ")),
            content: Cow::Owned(source.content.to_string()),
            sql: Cow::Owned(query.sql().to_string()),
            no_tx: query.no_tx(),
        }
    }

    pub fn new_applied(&self, duration_sec: i64) -> AppliedMigration {
        AppliedMigration {
            duration_sec,
            description: self.description.to_string(),
            content: STANDARD.encode(self.content.as_ref()),
            version: self.version,
        }
    }
}

/// A migration that was just applied.
#[derive(Debug, Clone)]
pub struct AppliedMigration {
    /// Migration version.
    pub version: i64,
    /// Migration description.
    pub description: String,
    /// The base64 encoding of the migration file
    /// when it was applied.
    pub content: String,
    /// Apply duration.
    pub duration_sec: i64,
}

impl AppliedMigration {
    pub fn order_by_asc(mut applied: Vec<AppliedMigration>) -> Vec<AppliedMigration> {
        applied.sort_by_key(|m| m.version);
        applied
    }
}
