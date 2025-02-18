# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
