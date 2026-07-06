//! Collecting and presenting results of an operation.
use chrono::{DateTime, Utc};
use console::Style;
use std::fmt::{self, Display, Formatter};
use std::time::Duration;
use tern_core::error::{ErrorKind, MigrationError, TernError, TernResult};
use tern_core::migration::MigrationData;

/// The result of an operation.
pub type OpResult = Result<OpComplete, OpError>;

#[derive(Clone, Debug, Default)]
pub(crate) struct CollectOp(Vec<OpSuccess>);

impl CollectOp {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn try_push<R>(
        &mut self,
        res: TernResult<R>,
    ) -> Result<(), OpError>
    where
        OpSuccess: From<R>,
    {
        match res.map(OpSuccess::from) {
            Ok(v) => {
                self.0.push(v);
                Ok(())
            },
            Err(e) => {
                let partial = std::mem::take(&mut self.0);
                Err(OpError::new(partial, e))
            },
        }
    }

    pub(crate) fn ok(self) -> OpResult {
        Ok(OpComplete(self.0))
    }
}

/// A successful operation applied to a collection of migrations.
#[derive(Clone, Debug, Default)]
pub struct OpComplete(Vec<OpSuccess>);

impl IntoIterator for OpComplete {
    type IntoIter = iter::IterOp;
    type Item = OpSuccess;

    fn into_iter(mut self) -> Self::IntoIter {
        self.0.sort_by_key(|v| -v.version);
        iter::IterOp(self.0)
    }
}

impl FromIterator<OpSuccess> for OpComplete {
    fn from_iter<T: IntoIterator<Item = OpSuccess>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl From<OpSuccess> for OpComplete {
    fn from(value: OpSuccess) -> Self {
        Self(vec![value])
    }
}

impl Display for OpComplete {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // An empty result is silent.
        if self.0.is_empty() {
            return Ok(());
        }
        let total = self.0.iter().map(|v| v.duration).sum::<Duration>();
        let migrations = match self.0.len() {
            1 => "1 migration".to_string(),
            n => format!("{n} migrations"),
        };
        writeln!(
            f,
            "{}",
            Style::new()
                .bold()
                .apply_to(format!("{migrations} in {total:.2?}"))
        )?;
        self.0.iter().try_for_each(|v| write!(f, "{v}"))
    }
}

/// A successful operation applied to one migration.
#[derive(Clone, Debug)]
pub struct OpSuccess {
    version: i64,
    description: String,
    query: String,
    end_time: DateTime<Utc>,
    duration: Duration,
}

impl OpSuccess {
    // Returns the same value but where `query` is truncated.
    fn truncated(&self) -> OpSuccess {
        let mut lines = self.query.lines().take(4).collect::<Vec<_>>();
        let query = if lines.pop().is_none() {
            lines.join("\n")
        } else {
            let snip = lines.join("\n");
            format!("{snip}\n...truncated...")
        };
        Self {
            version: self.version,
            description: self.description.clone(),
            query,
            end_time: self.end_time,
            duration: self.duration,
        }
    }
}

impl From<&MigrationData> for OpSuccess {
    fn from(value: &MigrationData) -> Self {
        Self {
            version: value.version(),
            description: value.description().into(),
            query: value.content().into(),
            end_time: value.applied_at(),
            duration: value
                .duration_millis()
                .try_into()
                .map(Duration::from_millis)
                .unwrap_or_default(),
        }
    }
}

impl From<MigrationData> for OpSuccess {
    fn from(value: MigrationData) -> Self {
        Self {
            version: value.version(),
            description: value.description().into(),
            query: value.content().into(),
            end_time: value.applied_at(),
            duration: value
                .duration_millis()
                .try_into()
                .map(Duration::from_millis)
                .unwrap_or_default(),
        }
    }
}

impl Display for OpSuccess {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let dim = Style::new().dim();
        writeln!(
            f,
            "{} {} {}",
            Style::new().green().bold().apply_to("✓"),
            Style::new().bold().apply_to(format!("V{}", self.version)),
            self.description,
        )?;
        self.query
            .lines()
            .try_for_each(|l| writeln!(f, "    {}", dim.apply_to(l)))?;
        writeln!(
            f,
            "    {}",
            dim.apply_to(format!(
                "finished {} · {:.1?}",
                self.end_time.format("%Y-%m-%d %H:%M:%S UTC"),
                self.duration
            ))
        )
    }
}

/// An operation that failed.
#[derive(Debug, thiserror::Error)]
pub struct OpError {
    partial: Vec<OpSuccess>,
    error: TernError,
}

impl OpError {
    /// New `OpError`.
    pub fn new(partial: Vec<OpSuccess>, error: TernError) -> Self {
        Self { partial, error }
    }

    /// Return the partial results.
    pub fn partial(&self) -> &[OpSuccess] {
        self.partial.as_slice()
    }

    /// Consume this type, returning the inner error.
    pub fn into_inner(self) -> TernError {
        self.error
    }
}

impl Display for OpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.partial.iter().try_for_each(|v| write!(f, "{}", v.truncated()))?;
        writeln!(
            f,
            "{} {}",
            Style::new().red().bold().apply_to("✗"),
            Style::new().red().apply_to(&self.error)
        )
    }
}

impl From<TernError> for OpError {
    fn from(value: TernError) -> Self {
        Self::new(Vec::new(), value)
    }
}

impl MigrationError for OpError {
    fn message(&self) -> String {
        self.to_string()
    }

    fn kind(&self) -> ErrorKind {
        ErrorKind::Ops
    }
}

#[doc(hidden)]
pub mod iter {
    use super::*;

    pub struct IterOp(pub(super) Vec<OpSuccess>);

    impl Iterator for IterOp {
        type Item = OpSuccess;

        fn next(&mut self) -> Option<Self::Item> {
            // IntoIterator sorted in reverse.
            self.0.pop()
        }
    }
}
