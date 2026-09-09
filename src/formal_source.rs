// SPDX-License-Identifier: AGPL-3.0-only
//! FR-014: exact source correspondence for caller-selected Contract IR identities.
//! This binding checks coordinates; assigning authored identities is the caller's role.

use quire_contract_ir::{SourceIdentity as IrIdentity, SourceLocation, SourceSpan};

use crate::{Code, Diagnostic, Phase, Position, Source, Span};

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
    pub fn to_ir(&self, source: &Source, span: Span) -> Result<SourceSpan, Box<Diagnostic>> {
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
    pub fn to_native(&self, span: &SourceSpan) -> Result<Span, Box<Diagnostic>> {
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

    fn location(&self, position: Position) -> Result<SourceLocation, Box<Diagnostic>> {
        let line = u32::try_from(position.line)
            .map_err(|_| self.failure("native line is not representable in IR"))?;
        let column = u32::try_from(position.column)
            .map_err(|_| self.failure("native column is not representable in IR"))?;
        let byte = u64::try_from(position.byte)
            .map_err(|_| self.failure("native byte offset is not representable in IR"))?;
        SourceLocation::new(self.identity.clone(), line, column, byte)
            .map_err(|upstream| self.upstream_failure(upstream))
    }

    fn failure(&self, message: &str) -> Box<Diagnostic> {
        crate::diagnostic::error(
            &self.source,
            Code::InvalidSourceMap,
            Phase::SourceMap,
            0,
            0,
            message,
        )
    }

    fn upstream_failure(&self, upstream: quire_contract_ir::Diagnostic) -> Box<Diagnostic> {
        let mut diagnostic = self.failure("formal source constructor rejected mapped coordinates");
        diagnostic.upstream = Some(Box::new(upstream));
        diagnostic
    }
}
