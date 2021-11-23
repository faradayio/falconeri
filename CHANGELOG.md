# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
