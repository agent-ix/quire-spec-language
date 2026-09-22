// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located complete-source diagnostics over the crate's existing code type.

use qsl_foundation::source::SourceReadCause;
use qsl_foundation::{LocatedSpan, Phase, Source, SourceIdentity};

/// Compatibility name for the crate's pre-existing diagnostic code type. The
/// authority-bound complete-V1 catalog is deliberately not selected here.
pub type CompleteCode = qsl_foundation::Code;

/// The closed typed cause of a complete-source diagnostic, selected by its
/// producer at the failing operation under `quire.native.diagnostics/v1`
/// revision `1-draft.3`.
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
    /// `invalid_source_identity`: the identity, revision or path is empty.
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

/// Read exact source bytes at the complete API boundary, refusing with a typed
/// cause.
pub(crate) fn read_source(
    identity: SourceIdentity,
    path: impl Into<String>,
    bytes: &[u8],
    byte_limit: usize,
) -> Result<Source, Box<CompleteDiagnostic>> {
    Source::read_typed(identity, path, bytes, byte_limit).map_err(|refusal| {
        let (code, cause, phase) = match refusal.cause {
            SourceReadCause::UnnamedSource => (
                CompleteCode::InvalidSourceIdentity,
                CompleteCause::Host(HostCause::UnnamedSource),
                Phase::Source,
            ),
            SourceReadCause::ByteBudget => (
                CompleteCode::ResourceExhausted,
                CompleteCause::InsufficientNextCharge,
                Phase::Source,
            ),
            SourceReadCause::InvalidUtf8 => (
                CompleteCode::InvalidUtf8,
                CompleteCause::Host(HostCause::InvalidUtf8),
                Phase::Source,
            ),
            SourceReadCause::Nul => (
                CompleteCode::InvalidSyntax,
                CompleteCause::InvalidToken,
                Phase::Source,
            ),
        };
        Box::new(CompleteDiagnostic {
            phase,
            code,
            cause,
            source: refusal.error.source,
            path: refusal.error.path,
            span: refusal.error.span,
            related: Vec::new(),
            message: refusal.error.message,
        })
    })
}

pub(crate) fn error(
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
        span: source
            .locate(qsl_foundation::Span { start, end })
            .expect("internal offsets are UTF-8 boundaries"),
        related: Vec::new(),
        message: message.into(),
    })
}
