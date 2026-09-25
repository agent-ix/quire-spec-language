// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located complete-source diagnostics over the crate's existing code type.

use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::source::provenance::SourceRegion;
use qsl_foundation::source::SourceReadCause;
use qsl_foundation::{Phase, Source, SourceIdentity, SyntaxLimit};

/// Compatibility name for the crate's pre-existing diagnostic code type. The
/// authority-bound complete-V1 catalog is deliberately not selected here.
pub type CompleteCode = qsl_foundation::Code;

/// The closed typed cause of a complete-source diagnostic, selected by its
/// producer at the failing operation under `quire.native.diagnostics/v1`
/// revision `1-draft.7`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompleteCause {
    /// `invalid_syntax`: a token the grammar does not admit at its position.
    UnexpectedToken,
    /// `invalid_syntax`: the source ends where the grammar requires more.
    UnexpectedEnd,
    /// `invalid_syntax`: bytes that form no token.
    InvalidToken,
    /// `invalid_syntax`: a string literal with a malformed escape.
    InvalidEscape,
    /// `unknown_language`, `unknown_edition` or `unknown_profile`: the
    /// selection is not supported by this consumer.
    UnsupportedSelection,
    /// `stale_dependency`: the selected version differs from the known one.
    RevisionMismatch,
    /// `stale_dependency`: the selected version is known with another digest.
    ByteDigestMismatch,
    /// `resource_exhausted`: the next charge exceeds its limit.
    InsufficientNextCharge,
    /// `missing_import`: no known definition has the selected identity.
    MissingSelection,
    /// `ambiguous_declaration`: one alias is declared twice in a namespace.
    AmbiguousName,
    /// `ambiguous_declaration`: one definition identity is selected at two
    /// distinct exact selections.
    ConflictingAuthority,
    /// `invalid_package`: a catalog holds one exact selection twice.
    DuplicateMember,
    /// `invalid_package`: a definition or model member is malformed.
    InvalidValue,
    /// `invalid_package`: the definition dependency graph has a cycle.
    DefinitionCycle,
    /// `invalid_package`: the resolved closure lacks a required facet.
    FeatureSetMismatch,
    /// `unknown_required_feature`: a capability name outside the inventory.
    UnknownFeature,
    /// `unknown_required_feature`: a known capability the closure does not
    /// provide.
    UnsupportedFeature,
    /// `invalid_model_binding`: the selected compiled model is absent or stale.
    WrongModelSelection,
    /// `invalid_model_binding`: one model identity is selected twice with
    /// distinct exact selections.
    ConflictingBinding,
    /// `cancelled`: the caller cancelled the request.
    CallerCancelled,
    /// `invalid_projection_correspondence`: a derived artifact lost its exact
    /// correspondence to the source.
    CorrespondenceLoss,
    /// `runtime_invariant`: an established evaluator or input invariant broke.
    EstablishedInvariantBroken,
    /// A retained host or source code with its original structured cause.
    Host(HostCause),
    /// `stage_limit_exceeded` (catalog revision `1-draft.7`): a
    /// [`SyntaxLimit`], named by its [`LimitKind`]. The token ceiling is
    /// `token-count-exceeded`.
    StageLimit(LimitKind),
}

/// The original structured cause of a retained host or source code.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum HostCause {
    /// `invalid_identifier`: a definition or model identity is empty or
    /// exceeds its bound.
    SelectionIdentity,
    /// `invalid_identifier`: a definition or model version is empty or exceeds
    /// its bound.
    SelectionVersion,
    /// `invalid_digest`: a selection digest is not canonical SHA-256.
    SelectionDigest,
    /// `invalid_source_identity`: a source label is empty or only
    /// whitespace, or the path is empty (FR-001).
    UnnamedSource,
    /// `invalid_source_identity`: an edit names a different predecessor.
    EditPredecessor,
    /// `invalid_source_identity`: a CST node belongs to another parsed source.
    ForeignNode,
    /// `invalid_source_identity`: a request is bound to another revision.
    RequestRevision,
    /// `invalid_source_map`: edits are unordered, overlapping or off a UTF-8
    /// boundary.
    EditRanges,
    /// `invalid_utf8`: the source bytes are not UTF-8.
    InvalidUtf8,
}

impl CompleteCause {
    /// The stable catalog tag. A host cause is tagged by its code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected-token",
            Self::UnexpectedEnd => "unexpected-end",
            Self::InvalidToken => "invalid-token",
            Self::InvalidEscape => "invalid-escape",
            Self::UnsupportedSelection => "unsupported-selection",
            Self::RevisionMismatch => "revision-mismatch",
            Self::ByteDigestMismatch => "byte-digest-mismatch",
            Self::InsufficientNextCharge => "insufficient-next-charge",
            Self::MissingSelection => "missing-selection",
            Self::AmbiguousName => "ambiguous-name",
            Self::ConflictingAuthority => "conflicting-authority",
            Self::DuplicateMember => "duplicate-member",
            Self::InvalidValue => "invalid-value",
            Self::DefinitionCycle => "definition-cycle",
            Self::FeatureSetMismatch => "feature-set-mismatch",
            Self::UnknownFeature => "unknown-feature",
            Self::UnsupportedFeature => "unsupported-feature",
            Self::WrongModelSelection => "wrong-model-selection",
            Self::ConflictingBinding => "conflicting-binding",
            Self::CallerCancelled => "caller-cancelled",
            Self::CorrespondenceLoss => "correspondence-loss",
            Self::EstablishedInvariantBroken => "established-invariant-broken",
            Self::Host(cause) => cause.code().as_str(),
            Self::StageLimit(kind) => kind.catalog_cause(),
        }
    }

    /// Whether the catalog admits this cause for `code`.
    pub fn is_cause_of(self, code: CompleteCode) -> bool {
        match self {
            Self::UnexpectedToken
            | Self::UnexpectedEnd
            | Self::InvalidToken
            | Self::InvalidEscape => code == CompleteCode::InvalidSyntax,
            Self::UnsupportedSelection => matches!(
                code,
                CompleteCode::UnknownLanguage
                    | CompleteCode::UnknownEdition
                    | CompleteCode::UnknownProfile
            ),
            Self::RevisionMismatch | Self::ByteDigestMismatch => matches!(
                code,
                CompleteCode::StaleDependency | CompleteCode::SourceDigestMismatch
            ),
            Self::InsufficientNextCharge => code == CompleteCode::ResourceExhausted,
            Self::MissingSelection => code == CompleteCode::MissingImport,
            Self::AmbiguousName | Self::ConflictingAuthority => {
                code == CompleteCode::AmbiguousDeclaration
            }
            Self::DuplicateMember
            | Self::InvalidValue
            | Self::DefinitionCycle
            | Self::FeatureSetMismatch => code == CompleteCode::InvalidPackage,
            Self::UnknownFeature | Self::UnsupportedFeature => {
                code == CompleteCode::UnknownRequiredFeature
            }
            Self::WrongModelSelection | Self::ConflictingBinding => {
                code == CompleteCode::InvalidModelBinding
            }
            Self::CallerCancelled => code == CompleteCode::Cancelled,
            Self::CorrespondenceLoss => code == CompleteCode::InvalidProjectionCorrespondence,
            Self::EstablishedInvariantBroken => code == CompleteCode::RuntimeInvariant,
            Self::Host(cause) => cause.code() == code,
            Self::StageLimit(_) => code == CompleteCode::StageLimitExceeded,
        }
    }
}

impl HostCause {
    /// The retained code this cause refines.
    pub fn code(self) -> CompleteCode {
        match self {
            Self::SelectionIdentity | Self::SelectionVersion => CompleteCode::InvalidIdentifier,
            Self::SelectionDigest => CompleteCode::InvalidDigest,
            Self::UnnamedSource
            | Self::EditPredecessor
            | Self::ForeignNode
            | Self::RequestRevision => CompleteCode::InvalidSourceIdentity,
            Self::EditRanges => CompleteCode::InvalidSourceMap,
            Self::InvalidUtf8 => CompleteCode::InvalidUtf8,
        }
    }
}

/// A located complete-source producer diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteDiagnostic {
    /// Phase that observed the failure.
    pub phase: Phase,
    /// Stable complete-V1 machine classification.
    pub code: CompleteCode,
    /// Closed typed cause, admitted by the catalog for `code`.
    pub cause: CompleteCause,
    /// The caller's four source labels, exactly as offered.
    pub source: SourceIdentity,
    /// Display path associated with the source.
    pub path: String,
    /// ADR-013 O-12: the refusal's region under the source's `RawSourceRef`,
    /// which holds bytes only; a renderer derives line and column from the
    /// source (`Source::render`, FR-001). `None` only for an S0 refusal
    /// FR-001 locates at no region: an unnamed source or input beyond the
    /// byte ceiling.
    pub region: Option<SourceRegion>,
    /// Deterministically ordered secondary source regions relevant to the refusal.
    pub related: Vec<SourceRegion>,
    /// Human-readable detail; never used to recover the code.
    pub message: String,
    /// Set only by [`resource_exhausted`], so it never disagrees with
    /// `code`; read through [`CompleteDiagnostic::limit`].
    limit: Option<SyntaxLimit>,
}

impl CompleteDiagnostic {
    /// The byte range of the refusal's region, or `None` when it has none.
    pub fn byte_span(&self) -> Option<qsl_foundation::Span> {
        let region = self.region.as_ref()?;
        Some(qsl_foundation::Span {
            start: usize::try_from(region.start()).ok()?,
            end: usize::try_from(region.end()).ok()?,
        })
    }

    /// The resource ceiling a `resource_exhausted` refusal names; `None`
    /// for every other diagnostic.
    pub fn limit(&self) -> Option<SyntaxLimit> {
        self.limit
    }
}

impl std::fmt::Display for CompleteDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for CompleteDiagnostic {}

/// Read exact source bytes at the complete API boundary, refusing with a typed
/// cause.
// Widened to `pub`: the root crate's `complete::parse_with_catalog` calls it
// across the crate boundary (ADR-011 §7.3 X-3).
pub fn read_source(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    byte_limit: usize,
) -> Result<Source, Box<CompleteDiagnostic>> {
    Source::read_typed(identity, path, bytes, byte_limit).map_err(|refusal| {
        let limit = (refusal.cause == SourceReadCause::ByteBudget)
            .then_some(SyntaxLimit::SourceBytes { bound: byte_limit });
        let (code, cause) = match refusal.cause {
            SourceReadCause::UnnamedSource => (
                CompleteCode::InvalidSourceIdentity,
                CompleteCause::Host(HostCause::UnnamedSource),
            ),
            // QSL-236: the source's own byte ceiling is `SyntaxLimit::SourceBytes`,
            // one of the four kinds the catalog admits.
            SourceReadCause::ByteBudget => (
                CompleteCode::StageLimitExceeded,
                CompleteCause::StageLimit(LimitKind::InputBytes),
            ),
            SourceReadCause::InvalidUtf8 => (
                CompleteCode::InvalidUtf8,
                CompleteCause::Host(HostCause::InvalidUtf8),
            ),
            SourceReadCause::Bom | SourceReadCause::Nul => {
                (CompleteCode::InvalidSyntax, CompleteCause::InvalidToken)
            }
            SourceReadCause::DigestMismatch => (
                CompleteCode::SourceDigestMismatch,
                CompleteCause::ByteDigestMismatch,
            ),
        };
        Box::new(CompleteDiagnostic {
            phase: Phase::Source,
            code,
            cause,
            source: refusal.error.source,
            path: refusal.error.path,
            region: refusal.error.region,
            related: Vec::new(),
            message: refusal.error.message,
            limit,
        })
    })
}

/// Build a located diagnostic at `source`'s exact `start..end` byte span.
// Widened to `pub`: the root crate's `complete::parse_with_catalog` calls it
// across the crate boundary (ADR-011 §7.3 X-3).
pub fn error(
    source: &Source,
    code: CompleteCode,
    cause: CompleteCause,
    phase: Phase,
    start: usize,
    end: usize,
    message: impl Into<String>,
) -> Box<CompleteDiagnostic> {
    Box::new(CompleteDiagnostic {
        phase,
        code,
        cause,
        source: source.identity().clone(),
        path: source.path().into(),
        region: Some(
            source
                .region(qsl_foundation::Span { start, end })
                .expect("internal offsets are UTF-8 boundaries"),
        ),
        related: Vec::new(),
        message: message.into(),
        limit: None,
    })
}

/// The one constructor for a complete-V1 syntax-ceiling refusal, at `span`:
/// `stage_limit_exceeded`/[`CompleteCause::StageLimit`] naming every
/// [`SyntaxLimit`] kind's catalog cause (revision `1-draft.7`), and a
/// message rendered from `limit`.
pub fn resource_exhausted(
    source: &Source,
    phase: Phase,
    span: qsl_foundation::Span,
    limit: SyntaxLimit,
) -> Box<CompleteDiagnostic> {
    let mut diagnostic = error(
        source,
        CompleteCode::StageLimitExceeded,
        CompleteCause::StageLimit(limit.stage_kind()),
        phase,
        span.start,
        span.end,
        limit.to_string(),
    );
    diagnostic.limit = Some(limit);
    diagnostic
}

#[cfg(test)]
mod tests {
    use super::{resource_exhausted, CompleteCause, CompleteCode};
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::LimitKind;
    use qsl_foundation::{Phase, Source, SourceIdentity, Span, SyntaxLimit};

    fn source() -> Source {
        Source::read(
            SourceIdentity::new("agent-ix", "test:diagnostic", "git", "1"),
            "diagnostic.native",
            b"x",
            qsl_foundation::source::MAX_SOURCE_BYTES,
        )
        .expect("test source")
    }

    /// Every `SyntaxLimit` kind reports `stage_limit_exceeded/<kind>-exceeded`
    /// (catalog revision `1-draft.7`); the token ceiling is
    /// `token-count-exceeded`.
    #[trace("TC-113", "FR-035-AC-5")]
    #[test]
    fn resource_exhausted_reports_the_kind_that_maps_to_the_catalog() {
        let source = source();
        let span = Span { start: 0, end: 1 };
        let cases = [
            (
                SyntaxLimit::NestingDepth { bound: 4 },
                CompleteCode::StageLimitExceeded,
                "nesting-depth-exceeded",
            ),
            (
                SyntaxLimit::Nodes { bound: 4 },
                CompleteCode::StageLimitExceeded,
                "node-count-exceeded",
            ),
            (
                SyntaxLimit::Work { bound: 4 },
                CompleteCode::StageLimitExceeded,
                "work-budget-exceeded",
            ),
            (
                SyntaxLimit::SourceBytes { bound: 4 },
                CompleteCode::StageLimitExceeded,
                "input-bytes-exceeded",
            ),
            (
                SyntaxLimit::Tokens { bound: 4 },
                CompleteCode::StageLimitExceeded,
                "token-count-exceeded",
            ),
        ];
        for (limit, code, cause) in cases {
            let diagnostic = resource_exhausted(&source, Phase::Parse, span, limit);
            assert_eq!(diagnostic.code, code, "{limit:?}");
            assert_eq!(diagnostic.cause.as_str(), cause, "{limit:?}");
            assert!(diagnostic.cause.is_cause_of(diagnostic.code), "{limit:?}");
            assert_eq!(diagnostic.limit(), Some(limit));
        }
        assert!(matches!(
            resource_exhausted(
                &source,
                Phase::Parse,
                span,
                SyntaxLimit::Tokens { bound: 4 }
            )
            .cause,
            CompleteCause::StageLimit(LimitKind::TokenCount)
        ));
    }
}
