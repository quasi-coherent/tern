//! Testing utilities.
use crate::app::TernApp;

/// `TernTest` implements a testing suite for a `TernApp`.
pub trait TernTest {
    /// The app being tested.
    type App: TernApp;

    /// Get a mutable reference to the [`TernApp`].
    fn app_mut(&mut self) -> &mut Self::App;
}
