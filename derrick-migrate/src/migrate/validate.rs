use derrick_core::error::Error;
use derrick_core::types::{ExistingMigration, MigrationSource};

#[derive(Debug, Clone)]
pub struct Validate;

impl Validate {
    /// Verify that all applied migrations are coherent.
    pub fn run_validation(
        source: Vec<MigrationSource>,
        applied: Vec<ExistingMigration>,
    ) -> Result<(), Error> {
        let all = MigrationSource::order_by_asc(source);
        let existing = ExistingMigration::order_by_asc(applied);
        existing
            .iter()
            .enumerate()
            .map(|(ix, hist)| {
                let src = all.get(ix).ok_or(Error::VersionMissing(hist.version))?;
                Self::validate_pair(src, hist)?;
                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;

        Ok(())
    }

    // We expect these to be the same because it's called at an iteration of a loop
    // over source/applied pairs from the arrays, which have been sorted in
    // ascending order by version.
    fn validate_pair(source: &MigrationSource, applied: &ExistingMigration) -> Result<(), Error> {
        // The migration set is missing the applied migration
        if source.version != applied.version {
            return Err(Error::VersionMissing(applied.version));
        };
        Ok(())
    }
}
