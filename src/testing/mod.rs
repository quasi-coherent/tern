//! Testing utilities.
pub use tern_test::property::Properties;

use crate::app::TernApp;

/// `TernTest` wires a user-defined test struct to the app under test and the
/// property set that certifies it.
///
/// Implement this by deriving it:
///
/// ```rust,ignore
/// #[derive(TernTest)]
/// #[tern(properties = "MyAppTest::properties")] // optional
/// pub struct MyAppTest {
///     #[tern(app)]
///     app: MyApp, // the type that derives `TernApp`
/// }
/// ```
///
/// The derive emits only the accessors below.  The test *runner* — database
/// provisioning, applying/reverting migrations, checking properties, and
/// rendering a report — is a separate layer built on top of this trait.
pub trait TernTest {
    /// The app under test (the type that derives `TernApp`).
    type App: TernApp;

    /// Mutable access to the app under test.
    fn app_mut(&mut self) -> &mut Self::App;

    /// The property set certifying this app's up/down pairs.
    fn properties(&mut self) -> Properties<Self::App>;
}
