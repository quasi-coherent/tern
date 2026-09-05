//! Collecting and rendering the outcome of testing.
use console::Style;
use futures_core::future::Future;
use std::fmt::Write as _;
use std::time::{Duration, Instant};
use tern_core::error::TernResult;

use crate::conn::TestConn;

/// How a recorded step ended.
#[derive(Clone, Debug)]
pub enum StepStatus {
    /// The step succeeded.
    Checked,
    /// The step failed with this message.
    Failed(String),
    /// The step ran successfully but no checks were associated to it.
    Ran,
}

/// One step in a test suite.
#[derive(Clone, Debug)]
pub struct StepReport {
    label: String,
    status: StepStatus,
    duration: Duration,
}

impl StepReport {
    /// A step that passed the checks.
    pub fn checked<T: Into<String>>(label: T, duration: Duration) -> Self {
        Self { label: label.into(), status: StepStatus::Checked, duration }
    }

    /// The step succeeded with no checks ran.
    pub fn ran<T: Into<String>>(label: T, duration: Duration) -> Self {
        Self { label: label.into(), status: StepStatus::Ran, duration }
    }

    /// A step that failed with `message`.
    pub fn failed<T, S>(label: T, message: S, duration: Duration) -> Self
    where
        T: Into<String>,
        S: Into<String>,
    {
        Self {
            label: label.into(),
            status: StepStatus::Failed(message.into()),
            duration,
        }
    }

    fn render(&self, width: usize, out: &mut String) {
        let dim = Style::new().dim();
        let label = format!("{:<width$}", self.label);
        let line = match &self.status {
            StepStatus::Checked => format!(
                "  {} {label}  {}",
                Style::new().green().bold().apply_to("✓"),
                dim.apply_to(format!("{:.1?}", self.duration)),
            ),
            StepStatus::Failed(msg) => format!(
                "  {} {label}  {}\n      {}",
                Style::new().red().bold().apply_to("✗"),
                dim.apply_to(format!("{:.1?}", self.duration)),
                Style::new().red().apply_to(msg),
            ),
            StepStatus::Ran => format!(
                "  {} {}",
                Style::new().yellow().bold().apply_to("○"),
                Style::new().yellow().apply_to(&self.label),
            ),
        };
        let _ = writeln!(out, "{line}");
    }
}

/// The accumulated record of tests.
#[derive(Clone, Debug)]
pub struct TestReport {
    name: String,
    conn: TestConn,
    steps: Vec<StepReport>,
    started: Instant,
}

impl TestReport {
    /// Start a new report.
    pub fn new(name: &str, conn: &TestConn) -> Self {
        Self {
            name: name.to_string(),
            conn: conn.clone(),
            steps: Vec::new(),
            started: Instant::now(),
        }
    }

    /// Push the record of a step in the test suite.
    pub fn push(&mut self, step: StepReport) {
        self.steps.push(step);
    }

    /// Await the step `fut` and append its report with name `label`.
    pub async fn step<S, T, Fut>(&mut self, label: S, fut: Fut) -> TernResult<T>
    where
        S: Into<String>,
        Fut: Future<Output = TernResult<T>>,
    {
        let start = Instant::now();
        let res = fut.await;
        let duration = start.elapsed();
        self.steps.push(match &res {
            Ok(_) => StepReport::checked(label, duration),
            Err(e) => StepReport::failed(label, e.to_string(), duration),
        });
        res
    }

    /// Whether any recorded step failed.
    pub fn failed(&self) -> bool {
        self.steps.iter().any(|s| matches!(s.status, StepStatus::Failed(_)))
    }

    /// Render the report.
    pub fn render(&self) -> String {
        let dim = Style::new().dim();
        let width = self
            .steps
            .iter()
            .map(|s| s.label.chars().count())
            .max()
            .unwrap_or_default();
        let header = format!(
            "{} {}\n  {}\n",
            Style::new().bold().apply_to("tern test ▸"),
            Style::new().bold().apply_to(&self.name),
            dim.apply_to(format!("db {}", &self.conn.testdb_name())),
        );
        let steps = self.steps.iter().fold(header, |mut acc, s| {
            s.render(width, &mut acc);
            acc
        });
        let outcome = if self.failed() {
            Style::new().red().bold().apply_to("failed")
        } else {
            Style::new().green().bold().apply_to("passed")
        };
        let kept_msg = if self.conn.preserved() {
            "database kept"
        } else {
            "database dropped"
        };
        let footer = format!(
            "  {} {outcome} {}\n",
            dim.apply_to("──"),
            dim.apply_to(format!(
                "in {:.1?} · {kept_msg}",
                self.started.elapsed()
            )),
        );
        if self.conn.preserved() {
            let url_msg = format!(
                "  {} {}\n    {}\n",
                Style::new().cyan().bold().apply_to("●"),
                Style::new().cyan().bold().apply_to(format!(
                    "database kept: {}",
                    &self.conn.testdb_name()
                )),
                Style::new().cyan().apply_to(&**self.conn.test_conn()),
            );
            format!("{steps}{footer}{url_msg}")
        } else {
            format!("{steps}{footer}")
        }
    }
}
