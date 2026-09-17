// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-012: stable diagnostics for the private fixture audit boundary.
use std::{collections::BTreeMap, fmt, io, path::Path};

/// Audit error categories; spellings are inventoried in docs/audit-error-codes.md.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Code {
    Usage,
    Io,
    InvalidJson,
    InvalidFixture,
    DigestMismatch,
    ProfileMismatch,
    IdentityMismatch,
    IdentityConflict,
    ForeignPath,
    ResourceExhausted,
}

impl Code {
    /// Stable spelling, independent of diagnostic prose.
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::Io => "io",
            Self::InvalidJson => "invalid-json",
            Self::InvalidFixture => "invalid-fixture",
            Self::DigestMismatch => "digest-mismatch",
            Self::ProfileMismatch => "profile-mismatch",
            Self::IdentityMismatch => "identity-mismatch",
            Self::IdentityConflict => "identity-content-conflict",
            Self::ForeignPath => "foreign-path",
            Self::ResourceExhausted => "resource-exhausted",
        }
    }

    /// The complete stable-code vocabulary, checked by the audit self-test.
    pub(crate) fn all() -> &'static [Self] {
        &[
            Self::Usage,
            Self::Io,
            Self::InvalidJson,
            Self::InvalidFixture,
            Self::DigestMismatch,
            Self::ProfileMismatch,
            Self::IdentityMismatch,
            Self::IdentityConflict,
            Self::ForeignPath,
            Self::ResourceExhausted,
        ]
    }

    /// Resolve a diagnostic code without interpreting human-readable prose.
    pub(crate) fn from_code(text: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|code| code.as_str() == text)
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Contextual error envelope; fixture failures never rely on enabled assertions.
#[derive(Debug, thiserror::Error)]
#[error("{code}: {message} {context:?}")]
pub(crate) struct Error {
    pub(crate) code: Code,
    message: Box<str>,
    context: BTreeMap<String, String>,
    #[source]
    source: Option<io::Error>,
}

impl Error {
    /// Construct a failed audit with stable classification and specific context.
    pub(crate) fn new(code: Code, message: impl Into<Box<str>>) -> Self {
        Self {
            code,
            message: message.into(),
            context: BTreeMap::new(),
            source: None,
        }
    }

    /// Attach the selected artifact path without converting OS arguments to UTF-8.
    pub(crate) fn at(mut self, path: &Path) -> Self {
        self.context
            .insert("path".into(), path.display().to_string());
        self
    }

    /// Retain the concrete I/O error at the file boundary.
    pub(crate) fn io(path: &Path, source: io::Error) -> Self {
        let mut error = Self::new(Code::Io, source.to_string()).at(path);
        error.source = Some(source);
        error
    }

    /// Distinguish failed content, command/I/O failure, and unavailable work.
    pub(crate) fn exit_code(&self) -> u8 {
        match self.code {
            Code::Usage | Code::Io => 2,
            Code::ResourceExhausted => 3,
            Code::InvalidJson
            | Code::InvalidFixture
            | Code::DigestMismatch
            | Code::ProfileMismatch
            | Code::IdentityMismatch
            | Code::IdentityConflict
            | Code::ForeignPath => 1,
        }
    }
}

/// One concrete error type for every audit mode.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// Check a fixture claim in debug and optimized binaries alike.
pub(crate) fn ensure(condition: bool, code: Code, message: impl Into<Box<str>>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::new(code, message))
    }
}
