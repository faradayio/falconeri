# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- If worker pod disappears off the cluster while processing a datum, detect this and set the datum to `status = Status::Error`. This is handled automatically by a "babysitter" thread in `falconerid`.
- Periodically check to see whether a job has finished.
- Add support for `datum_tries` in the pipeline JSON. Set this to 2, 3, etc., to automatically retry failed datums. This is also handled by the babysitter.
- Add trace spans for most low-level database access.

## [0.2.13] - 2021-11-23

### Added

- Wrote some basic developer documentation to supplement the `justfile`s.
- Allow specifying `--falconerid-log-level` for `falconeri deploy`. This uses standard `RUST_LOG` syntax, as described in the CLI help. 

### Fixed

- Cleaned up tracing output a bit.
- Switched to using `rustls` for HTTPS. Database connections still indirectly require OpenSSL thanks to `libpq`.

## [0.2.12] - 2021-11-22

### Fixed

- Attempt to fix TravisCI binary releases.

## [0.2.11] - 2021-11-22

### Added

- Don't show interactive progress bar when uploading outputs.
- Support `job_timeout` in pipeline schemas. This allows you to specify when an entire job should be stopped, even if it isn't done. Values include "300s", "2h", "2d", etc.
- Add much better tracing support when `RUST_LOG=trace` is passed.

### Changed

- We update most of our dependencies, including Rust libraries and our Docker base images. But this shouldn't affect normal use.

### Fixed

- Set `ttlSecondsAfterFinished` to 1 day so that old jobs don't hang around forever on the backplane wasting storage.
