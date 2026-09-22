// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-014/030/031: explicit native/formal identities and exact source correspondence.
//! This binding checks coordinates; assigning authored identities is the caller's role.

use quire_contract_ir::{SourceIdentity as IrIdentity, SourceLocation, SourceSpan};

use crate::{Code, Diagnostic, Phase, Position, Source, Span};

/// Explicit caller-selected native and formal identities, before source bytes are read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIdentities {
    /// Opaque native source identity and revision labels.
    pub native: crate::SourceIdentity,
    /// Validated formal document identity and revision.
    pub formal: IrIdentity,
}

/// Immutable correspondence between one exact native source and a formal identity.
/// Native labels remain opaque; no revision conversion or global registry is implied.
#[derive(Clone, Debug)]
pub struct FormalSource {
    source: Source,
    identity: IrIdentity,
}

impl FormalSource {
    /// Assign an explicit, already-validated formal identity to this native source.
    /// The caller owns the assignment; this constructor does not authenticate it.
    pub fn new(source: Source, identity: IrIdentity) -> Self {
        Self { source, identity }
    }

    /// Retained original source, including opaque native labels and exact byte digest.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Separately supplied formal document identity and positive revision.
    pub fn identity(&self) -> &IrIdentity {
        &self.identity
    }

    /// Map an exact-source span into IR coordinates using the existing source index.
    /// Foreign labels, path or digest and invalid UTF-8 ranges refuse without a locus.
    pub fn to_ir(&self, source: &Source, span: Span) -> Result<SourceSpan, Box<FormalSourceError>> {
        if source.identity() != self.source.identity()
            || source.path() != self.source.path()
            || source.digest() != self.source.digest()
        {
            return Err(self.failure("span request differs from the bound native source"));
        }
        let located = self
            .source
            .locate(span)
            .ok_or_else(|| self.failure("invalid native source span"))?;
        SourceSpan::new(self.location(located.start)?, self.location(located.end)?)
            .map_err(|upstream| self.upstream_failure(upstream))
    }

    /// Validate all formal endpoint coordinates against bytes before returning a span.
    /// Structurally valid IR spans with foreign identities or false coordinates refuse.
    pub fn to_native(&self, span: &SourceSpan) -> Result<Span, Box<FormalSourceError>> {
        if span.source() != self.identity() {
            return Err(self.failure("formal span belongs to a different source identity"));
        }
        let native = Span {
            start: usize::try_from(span.start().byte_offset())
                .map_err(|_| self.failure("formal start offset is not representable"))?,
            end: usize::try_from(span.end().byte_offset())
                .map_err(|_| self.failure("formal end offset is not representable"))?,
        };
        // Reconstruct through the same indexed coordinate authority and compare every
        // field, including both line/column pairs, instead of trusting IR monotonicity.
        if self.to_ir(&self.source, native)? != *span {
            return Err(self.failure("formal coordinates disagree with the bound source bytes"));
        }
        Ok(native)
    }

    fn location(&self, position: Position) -> Result<SourceLocation, Box<FormalSourceError>> {
        let line = u32::try_from(position.line)
            .map_err(|_| self.failure("native line is not representable in IR"))?;
        let column = u32::try_from(position.column)
            .map_err(|_| self.failure("native column is not representable in IR"))?;
        let byte = u64::try_from(position.byte)
            .map_err(|_| self.failure("native byte offset is not representable in IR"))?;
        SourceLocation::new(self.identity.clone(), line, column, byte)
            .map_err(|upstream| self.upstream_failure(upstream))
    }

    fn failure(&self, message: &str) -> Box<FormalSourceError> {
        Box::new(FormalSourceError {
            diagnostic: crate::diagnostic::error(
                &self.source,
                Code::InvalidSourceMap,
                Phase::SourceMap,
                0,
                0,
                message,
            ),
            upstream: None,
        })
    }

    fn upstream_failure(&self, upstream: quire_contract_ir::Diagnostic) -> Box<FormalSourceError> {
        let mut error = self.failure("formal source constructor rejected mapped coordinates");
        error.upstream = Some(Box::new(upstream));
        error
    }
}

/// A [`FormalSource`] coordinate-mapping refusal. `diagnostic` (ADR-011 §6.1) does not
/// import `quire_contract_ir`, so the exact IR constructor refusal that caused this
/// failure, when there was one, is carried here rather than inside [`Diagnostic`].
#[derive(Clone, Debug)]
pub struct FormalSourceError {
    /// Stable code, native source locus and human-readable explanation.
    pub diagnostic: Box<Diagnostic>,
    /// The IR coordinate constructor's own refusal, when that specific step failed.
    pub upstream: Option<Box<quire_contract_ir::Diagnostic>>,
}

impl std::fmt::Display for FormalSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.diagnostic, formatter)
    }
}

impl std::error::Error for FormalSourceError {}

impl From<FormalSourceError> for Diagnostic {
    fn from(error: FormalSourceError) -> Self {
        *error.diagnostic
    }
}
