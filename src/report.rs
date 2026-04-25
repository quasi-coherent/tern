//! Displaying results of an operation.
//!
//! The module defines the [`Report`] type and its properties.  A `Report` value
//! contains the outcome of the operation with respect to each migration it
//! applied to, up to the end of the migration set or to the first failure.
//!
//! `Report` is where logging and format can be configured.
use chrono::{DateTime, Utc};
use display_json::{DebugAsJson, DisplayAsJsonPretty};
use serde::Serialize;
use std::fmt::{self, Display, Formatter};
use tern_core::migration::{Applied, MigrationId};
use tern_core::query::Query;

/// Report
pub struct Report;

/// The result of some operation with a migration.
#[derive(Clone, Serialize, DebugAsJson, DisplayAsJsonPretty)]
pub struct Outcome {
    version: i64,
    description: String,
    head: String,
    dryrun: bool,
    state: State,
    end_time: DateTime<Utc>,
    duration_millis: i64,
    error: Option<String>,
}

impl From<Applied> for Outcome {
    fn from(value: Applied) -> Self {
        Self {
            version: value.version(),
            description: value.description(),
            head: value.content().lines().take(5).collect(),
            dryrun: false,
            state: State::Applied,
            end_time: value.applied_at(),
            duration_millis: value.duration_millis(),
            error: None,
        }
    }
}

impl Outcome {
    #[allow(unused)]
    pub(crate) fn dryrun(id: &MigrationId, query: &Query) -> Self {
        Self {
            version: id.version(),
            description: id.description().to_string(),
            head: query.to_string().lines().take(5).collect(),
            dryrun: true,
            state: State::Skipped,
            end_time: Utc::now(),
            duration_millis: 0,
            error: None,
        }
    }
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, Serialize)]
enum State {
    Applied,
    Failed,
    SoftApplied,
    Skipped,
    Reverted,
}

impl Display for State {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let data = match self {
            Self::Applied => "Applied",
            Self::Failed => "Failed",
            Self::SoftApplied => "Soft Applied",
            Self::Skipped => "Skipped",
            Self::Reverted => "Reverted",
        };
        f.write_str(data)
    }
}
