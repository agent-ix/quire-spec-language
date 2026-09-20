// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-059/FR-060/FR-061: stable diagnostics for the architecture-lint CLI.
use std::{fmt, io, path::Path};

/// Stable error classification. Spellings never change once released.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Code {
    Usage,
    Io,
    CargoMetadata,
    InvalidMetadata,
    InvalidLockfile,
    /// A local clone's resolved head does not match its remote `main` --
    /// #249 review round 2 H-2: a stale clone must fail loudly, never report
    /// a silently different (and possibly clean-looking) answer than the
    /// same command run against a fresh one.
    Stale,
}

impl Code {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Io => "io",
            Self::CargoMetadata => "cargo-metadata-failed",
            Self::InvalidMetadata => "invalid-metadata",
            Self::InvalidLockfile => "invalid-lockfile",
            Self::Stale => "stale-clone",
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

    pub(crate) fn at(self, path: &Path) -> Self {
        Self::new(self.code, format!("{} ({})", self.message, path.display()))
    }

    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        Self::new(Code::Io, format!("{source} ({})", path.display()))
    }

    /// Distinguish usage/environment failure from a reported architecture
    /// violation. `run` decides the violation exit code (1) itself; this
    /// covers only the errors that stop the tool before it can report.
    pub(crate) fn exit_code(&self) -> u8 {
        match self.code {
            Code::Usage => 2,
            Code::Io | Code::CargoMetadata | Code::InvalidMetadata | Code::InvalidLockfile => 3,
            Code::Stale => 4,
        }
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
