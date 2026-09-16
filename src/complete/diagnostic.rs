// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located complete-source diagnostics over the crate's existing code type.

use crate::{Diagnostic, LocatedSpan, Phase, Source, SourceIdentity};

/// Compatibility name for the crate's pre-existing diagnostic code type. The
/// authority-bound complete-V1 catalog is deliberately not selected here.
pub type CompleteCode = crate::Code;

/// A located complete-source producer diagnostic. Catalog selection and typed
/// diagnostic causes are supplied by the later checked-package authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteDiagnostic {
    /// Phase that observed the failure.
    pub phase: Phase,
    /// Stable complete-V1 machine classification.
    pub code: CompleteCode,
    /// Exact caller-selected source identity and revision.
    pub source: SourceIdentity,
    /// Display path associated with the source.
    pub path: String,
    /// Located half-open source range.
    pub span: LocatedSpan,
    /// Deterministically ordered secondary source ranges relevant to the refusal.
    pub related: Vec<LocatedSpan>,
    /// Human-readable detail; never used to recover the code.
    pub message: String,
}

impl std::fmt::Display for CompleteDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for CompleteDiagnostic {}

impl CompleteDiagnostic {
    /// Convert a legacy producer diagnostic at the complete API boundary.
    pub(crate) fn from_legacy(diagnostic: Diagnostic) -> Self {
        Self {
            phase: diagnostic.phase,
            code: diagnostic.code,
            source: diagnostic.source,
            path: diagnostic.path,
            span: diagnostic.span,
            related: Vec::new(),
            message: diagnostic.message,
        }
    }
}

pub(crate) fn error(
    source: &Source,
    code: CompleteCode,
    phase: Phase,
    start: usize,
    end: usize,
    message: impl Into<String>,
) -> Box<CompleteDiagnostic> {
    Box::new(CompleteDiagnostic {
        phase,
        code,
        source: source.identity().clone(),
        path: source.path().into(),
        span: source
            .locate(crate::Span { start, end })
            .expect("internal offsets are UTF-8 boundaries"),
        related: Vec::new(),
        message: message.into(),
    })
}
