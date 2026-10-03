// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-059/FR-060: diagnostics for the architecture-lint CLI.
use std::{fmt, io, path::Path};

/// The error classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Code {
    Usage,
    Io,
    CargoMetadata,
    InvalidMetadata,
    /// FR-060 T12-B/T12-C/T12-D's scan (`#[cfg(test)]` exclusion, token
    /// matching, debt-list function resolution) could not parse or tokenize
    /// a source file as Rust.
    SourceParse,
}

impl Code {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Io => "io",
            Self::CargoMetadata => "cargo-metadata-failed",
            Self::InvalidMetadata => "invalid-metadata",
            Self::SourceParse => "source-parse-failed",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub(crate) struct Error {
    pub(crate) code: Code,
    message: Box<str>,
}

impl Error {
    pub(crate) fn new(code: Code, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Self::new(Code::Io, format!("{source} ({})", path.display()))
    }

    pub(crate) fn source_parse(path: &Path, source: impl fmt::Display) -> Self {
        Self::new(Code::SourceParse, format!("{source} ({})", path.display()))
    }

    /// Distinguish usage/environment failure from a reported architecture
    /// violation. `run` decides the violation exit code (1) itself; this
    /// covers only the errors that stop the tool before it can report.
    pub(crate) fn exit_code(&self) -> u8 {
        match self.code {
            Code::Usage => 2,
            Code::Io | Code::CargoMetadata | Code::InvalidMetadata | Code::SourceParse => 3,
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
