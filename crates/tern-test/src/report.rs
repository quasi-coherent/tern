//! Collecting and rendering the outcome of testing.
use std::fmt::Write as _;
use std::time::{Duration, Instant};

use console::Style;
use futures_core::future::Future;
use tern_core::error::TernResult;

/// How a recorded step ended.
#[derive(Clone, Debug)]
pub enum StepStatus {
    /// The step succeeded.
    Passed,
    /// The step failed with this message.
    Failed(String),
    /// The step ran but did not have any property checking ran.
    Unverified,
}

/// One recorded step of a test run.
#[derive(Clone, Debug)]
pub struct StepReport {
    label: String,
    status: StepStatus,
    duration: Duration,
}

impl StepReport {
    /// A step that succeeded.
    pub fn passed(label: impl Into<String>, duration: Duration) -> Self {
        Self { label: label.into(), status: StepStatus::Passed, duration }
    }

    /// A step that failed with `message`.
    pub fn failed(
        label: impl Into<String>,
        message: impl Into<String>,
        duration: Duration,
    ) -> Self {
        Self {
            label: label.into(),
            status: StepStatus::Failed(message.into()),
            duration,
        }
    }

    /// A step that ran without anything certifying it.
    pub fn unverified(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            status: StepStatus::Unverified,
            duration: Duration::ZERO,
        }
    }

    fn render(&self, width: usize, out: &mut String) {
        let dim = Style::new().dim();
        let label = format!("{:<width$}", self.label);
        let line = match &self.status {
            StepStatus::Passed => format!(
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
            StepStatus::Unverified => format!(
                "  {} {}",
                Style::new().yellow().bold().apply_to("○"),
                Style::new().yellow().apply_to(&self.label),
            ),
        };
        let _ = writeln!(out, "{line}");
    }
}

/// The accumulated record of tests.
pub struct TestReport {
    name: String,
    dbname: String,
    url: String,
    keep: bool,
    steps: Vec<StepReport>,
    started: Instant,
}

impl TestReport {
    /// Start a new report.
    pub fn new(name: &str, dbname: &str, url: &str, keep: bool) -> Self {
        Self {
            name: name.to_string(),
            dbname: dbname.to_string(),
            url: url.to_string(),
            keep,
            steps: Vec::new(),
            started: Instant::now(),
        }
    }

    /// Record a step directly.
    pub fn push(&mut self, step: StepReport) {
        self.steps.push(step);
    }

    /// Await the step `fut` and append its report with name `label`.
    pub async fn step<T>(
        &mut self,
        label: impl Into<String>,
        fut: impl Future<Output = TernResult<T>>,
    ) -> TernResult<T> {
        let label = label.into();
        let start = Instant::now();
        let res = fut.await;
        let duration = start.elapsed();
        self.steps.push(match &res {
            Ok(_) => StepReport::passed(label, duration),
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
            dim.apply_to(format!("db {}", &self.dbname)),
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
        let kept_msg =
            if self.keep { "database kept" } else { "database dropped" };
        let footer = format!(
            "  {} {outcome} {}\n",
            dim.apply_to("──"),
            dim.apply_to(format!(
                "in {:.1?} · {kept_msg}",
                self.started.elapsed()
            )),
        );
        if self.keep {
            let url_msg = format!(
                "  {} {}\n    {}\n",
                Style::new().cyan().bold().apply_to("●"),
                Style::new()
                    .cyan()
                    .bold()
                    .apply_to(format!("database kept: {}", &self.dbname)),
                Style::new().cyan().apply_to(&self.url),
            );
            format!("{steps}{footer}{url_msg}")
        } else {
            format!("{steps}{footer}")
        }
    }
}
