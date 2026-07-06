use futures_core::future::Future;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::time::Duration;
use tern_core::error::TernResult;
use tern_test::TestExecutor;
use tern_test::property::PropertySet;
use tern_test::report::StepReport;

use crate::app::TernApp;
use crate::testing::{TernTest, TestCase, TestConfig};

/// Run the test body against a [`TernTest`].
pub fn run_test<T, F, Fut, G>(config: TestConfig, f: F, body: G)
where
    T: TernApp,
    T::Exec: TestExecutor,
    F: FnMut(T::Exec) -> Fut,
    Fut: Future<Output = TernResult<T>>,
    G: AsyncFnOnce(&mut TernTest<T>) -> TernResult<()>,
{
    let Some(url) = config.get_url() else {
        eprintln!(
            "Environment variable `TERN_TEST_DB_URL` unset \
              and no alternative was provided",
        );
        return;
    };
    let name = config.get_name();
    let keep = config.should_keep_db();
    let rt = tern_test::runtime();
    let mut testing =
        rt.block_on(TernTest::new(name, url, keep, f)).unwrap_or_else(|e| {
            panic!("tern test `{name}`: failed test setup: {e}")
        });

    let outcome =
        catch_unwind(AssertUnwindSafe(|| rt.block_on(body(&mut testing))));
    let (app, testdb, mut report) = testing.into_parts();
    // Drop the app to release the connection it holds.
    drop(app);
    outcome.as_ref().err().map(|p| {
        report.push(StepReport::failed(
            "panic",
            panic_message(p.as_ref()),
            Duration::ZERO,
        ));
    });

    if !keep && let Err(e) = rt.block_on(testdb.drop_with::<T::Exec>()) {
        log::warn!("failed to drop test DB {}: {e}", testdb.testdb_name());
    };
    println!("{}", report.render());

    match outcome {
        Err(panic) => resume_unwind(panic),
        Ok(Err(e)) => panic!("tern test `{name}` failed: {e}"),
        _ => {},
    }
}

/// Run one test case.
///
/// The properties are evaluated lazily, so tests without them incur no penalty.
pub fn run_test_case<T, F, Fut, P, G>(
    config: TestConfig,
    case: TestCase,
    f: F,
    properties: G,
) where
    T: TernApp,
    T::Exec: TestExecutor,
    F: FnMut(T::Exec) -> Fut,
    Fut: Future<Output = TernResult<T>>,
    P: PropertySet<T>,
    G: FnOnce() -> P,
{
    run_test(config, f, async move |t: &mut TernTest<T>| match case {
        TestCase::ApplyAll => t.apply_all().await,
        TestCase::UpDown { version } => {
            let props = properties();
            match props.property(version) {
                Some(p) => t.check_property(version, p).await,
                _ => t.check_structural(version).await,
            }
        },
    });
}

fn panic_message(p: &dyn std::any::Any) -> String {
    p.downcast_ref::<&str>()
        .map(ToString::to_string)
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("test panicked"))
}
