#![allow(deprecated)]

use std::fmt;

use thiserror::Error;

use crate::Diagnostic;

/// Convenience [Diagnostic] that can be used as an "anonymous" wrapper for
/// Errors. This is intended to be paired with [IntoDiagnostic].
#[deprecated(since = "1.1.0", note = "please use `WrapErr::wrap_err` instead")]
#[derive(Debug, Error)]
#[error("{}", self.error)]
pub struct DiagnosticError {
    #[source]
    error: Box<dyn std::error::Error + Send + Sync + 'static>,
    code: String,
}

impl DiagnosticError {
    /// Return a reference to the inner Error type.
    #[deprecated(since = "1.1.0", note = "please use `WrapErr::wrap_err` instead")]
    pub fn inner(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        &*self.error
    }
}

impl Diagnostic for DiagnosticError {
    fn code<'a>(&'a self) -> Box<dyn std::fmt::Display + 'a> {
        Box::new(&self.code)
    }
}

/**
Convenience trait that adds a `.into_diagnostic()` method that converts a type to a `Result<T, DiagnosticError>`.
*/
#[deprecated(since = "1.1.0", note = "please use `WrapErr::wrap_err` instead")]
pub trait IntoDiagnostic<T, E> {
    /// Converts [Result]-like types that return regular errors into a
    /// `Result` that returns a [Diagnostic].
    #[deprecated(since = "1.1.0", note = "please use `WrapErr::wrap_err` instead")]
    fn into_diagnostic(self, code: impl fmt::Display) -> Result<T, DiagnosticError>;
}

impl<T, E: std::error::Error + Send + Sync + 'static> IntoDiagnostic<T, E> for Result<T, E> {
    fn into_diagnostic(self, code: impl fmt::Display) -> Result<T, DiagnosticError> {
        self.map_err(|e| DiagnosticError {
            error: Box::new(e),
            code: format!("{}", code),
        })
    }
}
