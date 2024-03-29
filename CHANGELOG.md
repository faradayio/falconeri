# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0-beta.12] - 2022-12-14

### Fixed

- Log much less from `falconeri_worker` by default, and make it configurable. This fixes an issue where the newer tracing code was causing the worker to log far too much.

## [1.0.0-beta.11] - 2022-12-14 [YANKED]

### Fixed

- This version hard-coded a very low logging level. It was yanked because the low logging level would have made it impossible to debug falconeri issues discovered in the field, and because it was never fully released.

## [1.0.0-beta.10] - 2022-12-02

### Fixed

- Prevent key constraint error when retrying failed datums ([Issue #33](https://github.com/faradayio/falconeri/issues/33)). But see [Issue #36](https://github.com/faradayio/falconeri/issues/36); we still don't do the right thing when output files are randomly named.
- Reduce odds of birthday paradox collision when naming jobs ([Issue #35](https://github.com/faradayio/falconeri/issues/35)).

## [1.0.0-beta.9] - 2022-10-24

### Fixed

- Hard-code PostgreSQL version to prevent it from getting accidentally upgraded by Kubernetes.

## [1.0.0-beta.8] - 2022-05-19

### Fixed

- Use correct file name to upload release assets (again).

## [1.0.0-beta.7] - 2022-05-19

### Fixed

- Use correct file name to upload release assets.

## [1.0.0-beta.6] - 2022-05-19

### Fixed

- Attempted to fix binary builds on Linux (yet again).

## [1.0.0-beta.5] - 2022-05-19

### Fixed

- Attempted to fix binary builds on Linux (again).

## [1.0.0-beta.4] - 2022-05-19

### Fixed

- Attempted to fix binary builds on Linux. Not even trying on the Mac.

## [1.0.0-beta.3] - 2022-05-17

### Fixed

- Work around issue where `--field-selector` didn't find all running pods, resulting in accidental worker terminations.

## [1.0.0-beta.2] - 2021-12-02

### Fixed

- Fix `job_timeout` conversion to `ttlActiveSeconds` in the Kubernetes YAML.

## [1.0.0-beta.1] - 2021-11-24

This release adds a "babysitter" process inside each `falconerid`. We use this to monitor jobs and datums, and detect and/or recover from various types of errors. Updating an existing cluster _should_ be fine, but it's likely to spend a minute or two detecting and marking problems with old jobs. So please exercise appropriate caution.

We plan to stabilize a `falconeri` 1.0 with approximately this feature set. It has been in production for years, and the babysitter was the last missing critical feature.

### Added

- If worker pod disappears off the cluster while processing a datum, detect this and set the datum to `status = Status::Error`. This is handled automatically by a "babysitter" thread in `falconerid`.
- Add support for `datum_tries` in the pipeline JSON. Set this to 2, 3, etc., to automatically retry failed datums. This is also handled by the babysitter.
- Periodically check to see whether a job has finished without being correctly marked as such. This is mostly intended to clean up existing clusters.
- Periodically check to see whether a Kubernetes job has unexpectedly disappeared, and mark the corresponding `falconeri` job as having failed.
- Add trace spans for most low-level database access.

### Fixed

- We now correctly update `updated_at` on all tables that have it.

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
