# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 3.1.0 - 2025-05-21

### Added
* [[#26]]: Method `Runner::run_apply` and CLI subcommand that accepts a target version to apply migrations up through.
  * Validation rule that local migrations be a superset of remote history with respect to version and name.
  * Ability of the migration parent module to be of the `mod.rs` form.
  * `Display` impl for `Report`.

### Changed
* [[#26]]: Deprecated `Runner::apply_all` in favor of `run_apply_all` to accept `dryrun` argument.  Deprecated
          `Runner::soft_apply` and CLI subcommand in favor of `run_soft_apply` to not accept a `start_version` input.
  * Logging: All usage of `log::info!` changed to `log::trace!`. CLI returns the report for the user to render
    if they want.

### Fixed
* [[#26]]: Ignore hidden files in migration directory during `MigrationSource` proc macro expansion.
  * Error produced with invalid macro attribute path.
  * Internal logic for splitting statments in a `no_tx` scenario allows block comments `/* ... */`.

## 3.0.0 - 2025-01-17

### Added
* [[#24]]: Add validation when parsing migration versions during proc macro expansion and during
           execution for `apply-all` and `dryrun`.  Reject duplicate versions and missing versions.

### Changed
* [[#24]]: The `Migration` interface is changed to return the built migration query for the `dryrun`
           command so that it can be displayed in the report.  Similarly, a report "content" field
           now contains the query that was applied rather than the file content in a report coming
           from an `apply-all` operation.

[#24]: https://github.com/quasi-coherent/tern/pull/24
[#26]: https://github.com/quasi-coherent/tern/pull/26
