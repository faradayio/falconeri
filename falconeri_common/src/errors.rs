//! Error-handling code.

use std::fmt;

use anyhow::Error;

/// Support for displaying an error with a complete list of causes, and an
/// optional backtrace.
pub trait DisplayCausesAndBacktraceExt {
    /// Display the error and its causes, plus a backtrace (if available).
    fn display_causes_and_backtrace(&self) -> DisplayCauses<'_>;

    /// Display the error and its causes.
    fn display_causes_without_backtrace(&self) -> DisplayCauses<'_>;
}

impl DisplayCausesAndBacktraceExt for Error {
    fn display_causes_and_backtrace(&self) -> DisplayCauses<'_> {
        DisplayCauses {
            err: self,
            show_backtrace: true,
        }
    }

    fn display_causes_without_backtrace(&self) -> DisplayCauses<'_> {
        DisplayCauses {
            err: self,
            show_backtrace: false,
        }
    }
}

/// Helper type used to display errors.
pub struct DisplayCauses<'a> {
    /// The error to display.
    err: &'a Error,

    /// Should we show the backtrace?
    show_backtrace: bool,
}

impl fmt::Display for DisplayCauses<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ERROR: {}", self.err)?;
        let mut source = self.err.source();
        while let Some(next) = source {
            writeln!(f, "  caused by: {}", next)?;
            source = next.source();
        }

        if self.show_backtrace {
            write!(f, "{}", self.err.backtrace())?;
        }
        Ok(())
    }
}
