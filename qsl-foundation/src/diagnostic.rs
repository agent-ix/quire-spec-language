// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010: stable source-bound native diagnostics and standard error propagation.
//!
//! [`Locus`] is ADR-013 T-5's foundation diagnostic locus.
use crate::source::{LocatedSpan, Source, SourceIdentity, SourceReadCause, Span};

mod locus;
mod stage;
pub use locus::{InvalidJsonPointer, JsonPointer, Locus, UnresolvedLocus};
pub use stage::{LimitExceeded, LimitKind, LimitsField, StageFailure, StageWork, Staged, Stopped};

use std::collections::BTreeMap;

/// Native processing phase; successful syntax does not imply later execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    /// Immutable source intake and byte integrity.
    Source,
    /// Token recognition and delimiter/budget checks.
    Lex,
    /// Declaration and expression parsing.
    Parse,
    /// Admitted language/edition/profile selection.
    Profile,
    /// Token-preserving formatting.
    Format,
    /// Extracted-body correspondence validation or mapping.
    SourceMap,
    /// Exact formal import and native declaration resolution.
    Link,
    /// Native contextual typing and guarded proof checking.
    Check,
    /// Model-aware runtime population and invocation validation.
    Validate,
    /// Independent execution of a checked clause over validated inputs.
    Evaluate,
}

impl Phase {
    /// Stable phase label used by the native CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Lex => "lex",
            Self::Parse => "parse",
            Self::Profile => "profile",
            Self::Format => "format",
            Self::SourceMap => "source_map",
            Self::Link => "link",
            Self::Check => "check",
            Self::Validate => "validate",
            Self::Evaluate => "evaluate",
        }
    }
}

/// Stable native code vocabulary; see docs/native-error-codes.md.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum Code {
    /// A selected local file could not be read.
    IoError,
    /// The closed command request is malformed.
    InvalidRequest,
    /// A command's selected digest spelling is invalid.
    InvalidDigest,
    /// A command's selected identifier is invalid.
    InvalidIdentifier,
    /// A typed command result could not be serialized.
    OutputFailure,
    /// A real, admitted capability this build does not implement yet:
    /// checked native expressions outside the selected executable
    /// projection (`lowering`), or a `quire.checked-package/v2` emission
    /// path (`qsl-package`'s `emit`) not yet built.
    UnsupportedProjection,
    /// The existing IR rejected an executable derivation.
    ProjectionBinding,
    /// Checked native correspondence could not be preserved in the projection.
    InvalidProjectionCorrespondence,
    /// Source-only export cannot use extracted Markdown.
    ExtractionRequiresRun,
    /// Selected package bytes conflict with extracted compilation.
    ExtractionPackageConflict,
    /// Extracted compilation did not select exactly one authored binding.
    ExtractionClauseCount,
    /// Pinned Quire rejected the local semantic context.
    InvalidQuireContext,
    /// Required source identity, revision or path is absent.
    InvalidSourceIdentity,
    /// Supplied correspondence or query does not match the selected sources.
    InvalidSourceMap,
    /// Exact input bytes disagree with the supplied digest.
    SourceDigestMismatch,
    /// Input bytes are not valid UTF-8.
    InvalidUtf8,
    /// Source is malformed under the admitted grammar.
    InvalidSyntax,
    /// Recognized syntax is outside the admitted profile.
    UnsupportedConstruct,
    /// A native declaration or expression violates a fixed representability
    /// limit or required policy of the admitted profile, rather than naming
    /// a real capability outside it.
    UnrepresentableConstraint,
    /// Language label is not admitted.
    UnknownLanguage,
    /// Edition label is not admitted.
    UnknownEdition,
    /// Profile label is not admitted.
    UnknownProfile,
    /// Native package encoding, structure or reconstructed claims are invalid.
    InvalidPackage,
    /// Native artifact format selection is not implemented.
    UnknownWire,
    /// Native package requires an unknown or unavailable feature.
    UnknownRequiredFeature,
    /// A selected implementation budget prevented completion.
    ResourceExhausted,
    /// No supplied formal model matches a required package or alias.
    MissingImport,
    /// Selected dependency identity, revision or bytes differ from the artifact.
    StaleDependency,
    /// More than one formal candidate supplies the selected declaration.
    AmbiguousDeclaration,
    /// A required formal or lexical declaration is absent.
    MissingDeclaration,
    /// Import digest, context binding or clause identity is invalid.
    InvalidModelBinding,
    /// Native runtime artifact structure or selected runtime data is invalid.
    InvalidRuntimeInput,
    /// An invocation result or value is unavailable in the requested access form.
    WrongSnapshot,
    /// Native types, nominal identities or operator constraints disagree.
    IllTyped,
    /// An actual IR proof could not establish potentially evaluated definedness.
    UndefinedExpression,
    /// A target is absent from a declared complete population.
    DanglingReference,
    /// A required finite population is unavailable or declared incomplete.
    IncompletePopulation,
    /// A required artifact, observation or State root is unavailable.
    UnavailableObservation,
    /// Recorded created/deleted identities disagree with complete populations.
    PopulationDeltaMismatch,
    /// An observed change lies outside the immutable model's effect frame.
    FrameViolation,
    /// The caller requested cancellation before the next work unit.
    Cancelled,
    /// Execution encountered a violated established typing or input invariant.
    RuntimeInvariant,
    /// FR-151 dispatch linking found no, or several undominated, applicable
    /// candidates for a closed subtype.
    AmbiguousDispatch,
    /// FR-153: a population document's `modelIdentity`, a lookup key's
    /// universe, or a population member's declared type names something
    /// outside the bound closed environment.
    ForeignReference,
    /// FR-153: a formed population selection's count is outside its declared
    /// `allInstances<T>(p)` bound.
    CardinalityOutOfBound,
    /// ADR-013 O-01/QC-5: a second selection of a domain-package identity
    /// already selected at a different (or the same) version.
    DuplicateSelection,
    /// A stage-entry limit was reached (`stage_limit_exceeded`,
    /// catalog revision `1-draft.8`). The exhausted kind is a `SyntaxLimit`
    /// or [`LimitKind`] carried alongside, not part of this code.
    StageLimitExceeded,
    /// A document is not in its canonical form (QSpec FR-271): a domain
    /// package or observation document carries a number with no exact RFC
    /// 8785 spelling (FR-056, FR-106), or a checked-package/v2 wire's bytes
    /// are not RFC 8785 bytes.
    NoncanonicalWire,
}

impl Code {
    /// Stable code spelling, independent of the diagnostic message.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IoError => "io-error",
            Self::InvalidRequest => "invalid-request",
            Self::InvalidDigest => "invalid-digest",
            Self::InvalidIdentifier => "invalid-identifier",
            Self::OutputFailure => "output-failure",
            Self::UnsupportedProjection => "unsupported_projection",
            Self::ProjectionBinding => "projection_binding",
            Self::InvalidProjectionCorrespondence => "invalid_projection_correspondence",
            Self::ExtractionRequiresRun => "extraction-requires-run",
            Self::ExtractionPackageConflict => "extraction-package-conflict",
            Self::ExtractionClauseCount => "extraction-clause-count",
            Self::InvalidQuireContext => "invalid-quire-context",
            Self::InvalidSourceIdentity => "invalid_source_identity",
            Self::InvalidSourceMap => "invalid_source_map",
            Self::SourceDigestMismatch => "source_digest_mismatch",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InvalidSyntax => "invalid_syntax",
            Self::UnsupportedConstruct => "unsupported_construct",
            Self::UnrepresentableConstraint => "unrepresentable_constraint",
            Self::UnknownLanguage => "unknown_language",
            Self::UnknownEdition => "unknown_edition",
            Self::UnknownProfile => "unknown_profile",
            Self::InvalidPackage => "invalid_package",
            Self::UnknownWire => "unknown_wire",
            Self::UnknownRequiredFeature => "unknown_required_feature",
            Self::ResourceExhausted => "resource_exhausted",
            Self::MissingImport => "missing_import",
            Self::StaleDependency => "stale_dependency",
            Self::AmbiguousDeclaration => "ambiguous_declaration",
            Self::MissingDeclaration => "missing_declaration",
            Self::InvalidModelBinding => "invalid_model_binding",
            Self::InvalidRuntimeInput => "invalid_runtime_input",
            Self::WrongSnapshot => "wrong_snapshot",
            Self::IllTyped => "ill_typed",
            Self::UndefinedExpression => "undefined_expression",
            Self::DanglingReference => "dangling_reference",
            Self::IncompletePopulation => "incomplete_population",
            Self::UnavailableObservation => "unavailable_observation",
            Self::PopulationDeltaMismatch => "population_delta_mismatch",
            Self::FrameViolation => "frame_violation",
            Self::Cancelled => "cancelled",
            Self::RuntimeInvariant => "runtime_invariant",
            Self::AmbiguousDispatch => "ambiguous_dispatch",
            Self::ForeignReference => "foreign_reference",
            Self::CardinalityOutOfBound => "cardinality_out_of_bound",
            Self::DuplicateSelection => "duplicate_selection",
            Self::StageLimitExceeded => "stage_limit_exceeded",
            Self::NoncanonicalWire => "noncanonical_wire",
        }
    }

    /// Complete code vocabulary for enumeration and compatibility checks.
    pub fn all() -> &'static [Self] {
        &[
            Self::IoError,
            Self::InvalidRequest,
            Self::InvalidDigest,
            Self::InvalidIdentifier,
            Self::OutputFailure,
            Self::UnsupportedProjection,
            Self::ProjectionBinding,
            Self::InvalidProjectionCorrespondence,
            Self::ExtractionRequiresRun,
            Self::ExtractionPackageConflict,
            Self::ExtractionClauseCount,
            Self::InvalidQuireContext,
            Self::InvalidSourceIdentity,
            Self::InvalidSourceMap,
            Self::SourceDigestMismatch,
            Self::InvalidUtf8,
            Self::InvalidSyntax,
            Self::UnsupportedConstruct,
            Self::UnrepresentableConstraint,
            Self::UnknownLanguage,
            Self::UnknownEdition,
            Self::UnknownProfile,
            Self::InvalidPackage,
            Self::UnknownWire,
            Self::UnknownRequiredFeature,
            Self::ResourceExhausted,
            Self::MissingImport,
            Self::StaleDependency,
            Self::AmbiguousDeclaration,
            Self::MissingDeclaration,
            Self::InvalidModelBinding,
            Self::InvalidRuntimeInput,
            Self::WrongSnapshot,
            Self::IllTyped,
            Self::UndefinedExpression,
            Self::DanglingReference,
            Self::IncompletePopulation,
            Self::UnavailableObservation,
            Self::PopulationDeltaMismatch,
            Self::FrameViolation,
            Self::Cancelled,
            Self::RuntimeInvariant,
            Self::AmbiguousDispatch,
            Self::ForeignReference,
            Self::CardinalityOutOfBound,
            Self::DuplicateSelection,
            Self::StageLimitExceeded,
            Self::NoncanonicalWire,
        ]
    }

    /// Resolve a known stable spelling; unknown codes remain explicit absence.
    pub fn from_code(value: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
    }

    /// Whether this code records incomplete work rather than invalid input:
    /// a reached limit or budget, a cancellation, or an unavailable
    /// observation (ADR-029 CB-4's incomplete row, exit 22).
    pub fn is_incomplete(self) -> bool {
        matches!(
            self,
            Self::ResourceExhausted
                | Self::StageLimitExceeded
                | Self::Cancelled
                | Self::IncompletePopulation
                | Self::UnavailableObservation
        )
    }

    /// Whether this code records a well-formed request naming a real,
    /// catalogued capability this build does not implement, rather than
    /// input that is itself invalid. FR-301's contract reports these as
    /// unsupported (21), distinct from invalid or refused input (20).
    pub fn is_unsupported(self) -> bool {
        matches!(
            self,
            Self::UnsupportedProjection | Self::UnsupportedConstruct | Self::UnknownRequiredFeature
        )
    }

    /// This code's ADR-013 O-16 category, on QSpec FR-301's single-code
    /// ladder: `runtime_invariant` (an internal fault, ADR-013 T-4) is an
    /// internal failure; then unsupported before incomplete before invalid
    /// or refused input. Exit codes come from a category
    /// ([`Category::exit_code`], FR-285), and this is the code-to-category
    /// map an exit derived from a `Code` uses. It is not [`category_of`],
    /// the refusal-record map, which gives `Refusal` for the unsupported
    /// and incomplete codes this ladder separates.
    pub fn category(self) -> Category {
        if self == Self::RuntimeInvariant {
            Category::InternalFailure
        } else if self.is_unsupported() {
            Category::Unsupported
        } else if self.is_incomplete() {
            Category::Incomplete
        } else {
            Category::Refusal
        }
    }
}

impl std::fmt::Display for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A failed native phase with original source coordinates; never a Boolean result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    /// Phase that observed the failure.
    pub phase: Phase,
    /// Stable machine-readable classification.
    pub code: Code,
    /// The caller's four source labels (FR-001), exactly as offered.
    pub source: SourceIdentity,
    /// Display path; not a portable artifact identity or an OS path round trip.
    pub path: String,
    /// Half-open original byte range with one-based line/scalar positions.
    pub span: LocatedSpan,
    /// Contextual human-readable explanation; code carries stable classification.
    pub message: String,
    /// Set only by [`resource_exhausted`], so it never disagrees with
    /// `code`; read through [`Diagnostic::limit`].
    limit: Option<SyntaxLimit>,
    /// Set only for an `invalid_source_identity` refusal: `BlankLabel`
    /// (which label) or `EmptyPath` (`quire.native.diagnostics/v1`,
    /// FR-001); read through [`Diagnostic::identity_cause`].
    identity_cause: Option<SourceReadCause>,
}

/// The syntax resource ceiling a `resource_exhausted` refusal names
/// (NFR-001: a refusal names the limit kind, the bound and the span). Both
/// S1 parsers and both lexers refuse through this one type, so a caller
/// distinguishes the ceilings by variant, never by message text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxLimit {
    /// Token (complete-V1: retained CST leaf) ceiling:
    /// `stage_limit_exceeded`/`token-count-exceeded` (catalog revision
    /// `1-draft.8`).
    Tokens {
        /// Selected token ceiling.
        bound: usize,
    },
    /// Syntax-node ceiling.
    Nodes {
        /// Selected syntax-node ceiling.
        bound: usize,
    },
    /// Parser work budget, in interpreter steps.
    Work {
        /// Step budget: a fixed number of steps per significant token of
        /// the unit, plus one token's worth for the end of input.
        bound: usize,
    },
    /// Source or output byte ceiling.
    SourceBytes {
        /// Selected byte ceiling.
        bound: usize,
    },
}

impl std::fmt::Display for SyntaxLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tokens { bound } => write!(f, "token ceiling of {bound} tokens exhausted"),
            Self::Nodes { bound } => write!(f, "syntax node ceiling of {bound} nodes exhausted"),
            Self::Work { bound } => write!(f, "parser work budget of {bound} steps exhausted"),
            Self::SourceBytes { bound } => {
                write!(f, "source byte ceiling of {bound} bytes exhausted")
            }
        }
    }
}

impl SyntaxLimit {
    /// The T-4 [`LimitKind`] this ceiling names (catalog revision
    /// `1-draft.8`).
    pub const fn stage_kind(self) -> LimitKind {
        match self {
            Self::Tokens { .. } => LimitKind::TokenCount,
            Self::Nodes { .. } => LimitKind::NodeCount,
            Self::Work { .. } => LimitKind::WorkBudget,
            Self::SourceBytes { .. } => LimitKind::InputBytes,
        }
    }
}

/// The one constructor for a syntax-ceiling refusal, at `span`: code
/// `stage_limit_exceeded` for every [`SyntaxLimit`] kind (catalog revision
/// `1-draft.8`), and a message rendered from `limit`.
pub fn resource_exhausted(
    source: &Source,
    phase: Phase,
    span: Span,
    limit: SyntaxLimit,
) -> Box<Diagnostic> {
    let mut diagnostic = error(
        source,
        Code::StageLimitExceeded,
        phase,
        span.start,
        span.end,
        limit.to_string(),
    );
    diagnostic.limit = Some(limit);
    diagnostic
}

// FR-010: thiserror infers `source` as an Error cause, but this public field
// carries source identity. Preserve the API and implement standard traits.
impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Diagnostic {}

impl Diagnostic {
    /// The resource ceiling a `resource_exhausted` refusal names; `None`
    /// for every other diagnostic.
    pub fn limit(&self) -> Option<SyntaxLimit> {
        self.limit
    }
    /// The `invalid_source_identity` cause (`BlankLabel` naming its label,
    /// or `EmptyPath`); `None` for every other diagnostic.
    pub fn identity_cause(&self) -> Option<SourceReadCause> {
        self.identity_cause
    }
    /// Whether incomplete work, rather than invalid input, caused this diagnostic.
    pub fn is_incomplete(&self) -> bool {
        self.code.is_incomplete()
    }
    /// Whether this diagnostic names a real, catalogued capability this
    /// build does not implement, rather than input that is itself invalid.
    pub fn is_unsupported(&self) -> bool {
        self.code.is_unsupported()
    }
}

/// Build a [`Diagnostic`] at a source region, locating `start..end` and
/// cloning the source's identity and path. `pub`: nearly every stage module
/// in the root crate (`parser`, `format`, `formal_source`, `checking`,
/// `native_model`, `linking`, `mapped`, `complete::*`, `runtime::*`, `lexer`),
/// and the `qsl-source` crate, is a real cross-crate call site.
pub fn error(
    source: &Source,
    code: Code,
    phase: Phase,
    start: usize,
    end: usize,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    Box::new(Diagnostic {
        code,
        phase,
        source: source.identity().clone(),
        path: source.path().into(),
        span: source
            .locate(Span { start, end })
            .expect("internal offsets are UTF-8 boundaries"),
        message: message.into(),
        limit: None,
        identity_cause: None,
    })
}

// ADR-011 §6.1: `source` (and `source_map`) precede `diagnostic` in layer F's
// module order, so neither may depend on `diagnostic`. These `impl Source`
// and `impl SourceMap` blocks live here, not in `source.rs`/`source_map.rs`,
// so that `source`/`source_map` construct only their own typed refusal
// (`SourceReadError`, `SourceMapError`) and `diagnostic` — the owner of the
// code vocabulary — maps that refusal onto a stable `Code`. Public callers
// see no difference: `Source::read`, `Source::read_verified`,
// `SourceMap::verify` and `SourceMap::map_span` keep their existing
// `Result<_, Box<Diagnostic>>` signatures.
impl crate::source::Source {
    /// Read original UTF-8 bytes without normalization, refusing more than
    /// `byte_limit` bytes. The caller's `byte_limit` is used as given.
    pub fn read(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        Self::read_typed(identity, path, bytes, byte_limit)
            .map_err(|refusal| source_refusal(*refusal, bytes, byte_limit))
    }

    /// Verify an independently supplied byte digest before constructing a mapped subject.
    pub fn read_verified(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        expected: crate::ByteDigest,
        byte_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        Self::read_verified_typed(identity, path, bytes, expected, byte_limit)
            .map_err(|refusal| source_refusal(*refusal, bytes, byte_limit))
    }
}

/// The native-v1 rendering of an S0 refusal over the offered `bytes`. A
/// refusal with a region renders it over those bytes (FR-001). One with no
/// region renders byte 0, because this lane-private `Diagnostic` requires a
/// span: retained native-v1 debt (ADR-013 §6, FR-001 "Where an S0 refusal
/// is located"), not the canonical refusal's location.
fn source_refusal(
    refusal: crate::source::SourceReadRefusal,
    bytes: &[u8],
    byte_limit: usize,
) -> Box<Diagnostic> {
    let code = match refusal.cause {
        SourceReadCause::BlankLabel { .. } | SourceReadCause::EmptyPath => {
            Code::InvalidSourceIdentity
        }
        SourceReadCause::ReferenceInvariant => Code::RuntimeInvariant,
        // The source's own byte ceiling is `SyntaxLimit::SourceBytes`,
        // one of the four kinds the catalog admits.
        SourceReadCause::ByteBudget => Code::StageLimitExceeded,
        SourceReadCause::InvalidUtf8 => Code::InvalidUtf8,
        SourceReadCause::Bom | SourceReadCause::Nul => Code::InvalidSyntax,
        SourceReadCause::DigestMismatch => Code::SourceDigestMismatch,
    };
    // L3: only a `ByteBudget` refusal names a `SyntaxLimit` --
    // every other cause's `limit` stays `None`.
    let limit = (refusal.cause == SourceReadCause::ByteBudget)
        .then_some(SyntaxLimit::SourceBytes { bound: byte_limit });
    let origin = crate::source::Position {
        byte: 0,
        line: 1,
        column: 1,
    };
    let span = refusal
        .error
        .region
        .as_ref()
        .and_then(|region| crate::source::render_offered(bytes, region))
        .unwrap_or(LocatedSpan {
            start: origin,
            end: origin,
        });
    Box::new(Diagnostic {
        phase: Phase::Source,
        code,
        source: refusal.error.source,
        path: refusal.error.path,
        span,
        message: refusal.error.message,
        limit,
        identity_cause: matches!(
            refusal.cause,
            SourceReadCause::BlankLabel { .. } | SourceReadCause::EmptyPath
        )
        .then_some(refusal.cause),
    })
}

impl crate::source_map::SourceMap {
    /// Validate a complete monotonic map against immutable exact source bytes.
    /// Both sources can be loaded with Source::read_verified when consuming pinned artifacts.
    pub fn verify(
        original: Source,
        body: Source,
        region: crate::source::Span,
        segments: Vec<crate::source_map::Segment>,
        layout: crate::source_map::Layout,
        segment_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        Self::verify_typed(original, body, region, segments, layout, segment_limit)
            .map_err(|error| Box::new(Diagnostic::from(*error)))
    }

    /// Map a span from the exact body source. Discontiguous regions stay separate;
    /// concatenating their original bytes reproduces the body region exactly.
    /// A zero-width boundary selects the following segment, except EOF uses the last end.
    pub fn map_span(
        &self,
        source: &Source,
        span: crate::source::Span,
    ) -> Result<Vec<LocatedSpan>, Box<Diagnostic>> {
        self.map_span_typed(source, span)
            .map_err(|error| Box::new(Diagnostic::from(*error)))
    }
}

impl From<crate::source_map::SourceMapError> for Diagnostic {
    fn from(error: crate::source_map::SourceMapError) -> Self {
        use crate::source_map::SourceMapErrorCause;
        let code = match error.cause {
            SourceMapErrorCause::SegmentBudget => Code::ResourceExhausted,
            SourceMapErrorCause::InvalidMap => Code::InvalidSourceMap,
        };
        Diagnostic {
            phase: Phase::SourceMap,
            code,
            source: error.source,
            path: error.path,
            span: error.span,
            message: error.message,
            limit: None,
            identity_cause: None,
        }
    }
}

// ADR-013 slice S-5 (agent-ix/quire-spec-language#213), part one of a split
// slice ("S-5a"; the owner's ruling on QSL-26, 2026-09-20). This is the start
// of this module's move to the ADR-011 §6.1 foundation `diagnostic` module
// (M-1: "codes, typed causes, locus"), which lands piecemeal across #213's
// slices. `Code`, `Phase` and `Diagnostic` above are the pre-existing
// native-v1 lane-private catalog (ADR-013 §6: "Box<Diagnostic> 45-variant
// Code"); they retain no canonical authority and gain no new consumer (R-09),
// and nothing below converts to or from them.
//
// S-5b adds T-4's `LimitKind`, `LimitExceeded`, `Staged` and
// `StageFailure` in `stage`, and O-17's `RefusalRecord` below, each naming
// its position by T-5's `Locus`. This build claims catalog revision
// `1-draft.8`, whose `stage_limit_exceeded` has one cause per `LimitKind`.

/// ADR-013 O-16: the outcome category every evaluation, negotiation and proof
/// result maps into. Exactly the eight values ADR-013 §3 O-16's category
/// table names in its first column; not a kernel type (`quire-exact` carries
/// no category), and this crate is its only owner. Category-mapping
/// functions from each family's own outcome (C-08, C-09, C-23) match this
/// enum exhaustively with no `_` arm, so a ninth category fails every one of
/// them to compile rather than silently falling into an existing row.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Category {
    /// A completed value, or a claim's `Completed(false)` (a violation is
    /// its own, separate category below).
    Success,
    /// A claim's `Completed(false)`: a false predicate, not a refusal.
    Violation,
    /// The operation has no mathematical value (kernel `Undefined`).
    Undefined,
    /// The operation is defined but its result, or the request itself, is
    /// not admitted.
    Refusal,
    /// A well-formed request naming a real, catalogued capability this
    /// build, backend or solver does not supply.
    Unsupported,
    /// A named charge was unavailable and no partial value exists: timeout,
    /// cancellation or bound exhaustion.
    Incomplete,
    /// Neither proved nor refuted -- including a vacuous proof and a replay
    /// parity disagreement.
    Inconclusive,
    /// A runtime, evaluator or mapping invariant broke; never a `Refusal`.
    InternalFailure,
}

impl Category {
    /// Every value, in the ADR-013 O-16 category table's row order.
    pub const ALL: [Self; 8] = [
        Self::Success,
        Self::Violation,
        Self::Undefined,
        Self::Refusal,
        Self::Unsupported,
        Self::Incomplete,
        Self::Inconclusive,
        Self::InternalFailure,
    ];

    /// A stable display spelling. ADR-013 O-16 fixes the eight values and
    /// states that category values "compare lexically on their wire
    /// strings", but names no QSpec wire contract for this QSL-internal
    /// type, so this spelling is QSL's own and carries no QSpec authority.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Violation => "violation",
            Self::Undefined => "undefined",
            Self::Refusal => "refusal",
            Self::Unsupported => "unsupported",
            Self::Incomplete => "incomplete",
            Self::Inconclusive => "inconclusive",
            Self::InternalFailure => "internal-failure",
        }
    }

    /// FR-285 (ADR-029 CB-4): the one exit function, QSpec FR-301's code
    /// for an item of this category. Undefined folds to 10 with violation;
    /// inconclusive is a proof or `analyze` item's, which did not settle
    /// its claim and so is incomplete (22). A supplied-trace clause still
    /// pending at the trace's end takes [`Self::trace_exit_code`] instead.
    pub const fn exit_code(self) -> u8 {
        match self {
            Self::Success => 0,
            Self::Violation | Self::Undefined => 10,
            Self::Refusal => 20,
            Self::Unsupported => 21,
            Self::Incomplete | Self::Inconclusive => 22,
            Self::InternalFailure => 30,
        }
    }

    /// FR-285's row for a supplied-trace clause (FR-283): one still pending
    /// when the trace ends is inconclusive but observed no violation, so it
    /// exits 0. Every other category exits as [`Self::exit_code`] states.
    pub const fn trace_exit_code(self) -> u8 {
        match self {
            Self::Inconclusive => 0,
            other => other.exit_code(),
        }
    }

    /// FR-285's severity fold: the most severe of `codes` under QSpec
    /// FR-301's order 30, 20, 21, 22, 10, 0, or `None` when `codes` is
    /// empty. A code outside that set ranks most severe, so it is never
    /// hidden behind a known one.
    pub fn most_severe(codes: impl IntoIterator<Item = u8>) -> Option<u8> {
        const ORDER: [u8; 6] = [30, 20, 21, 22, 10, 0];
        codes.into_iter().min_by_key(|code| {
            ORDER
                .iter()
                .position(|known| known == code)
                .map_or(0, |rank| rank + 1)
        })
    }
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// ADR-013 O-17: one `quire.native.diagnostics/v1` code and cause, exactly as
/// the `quire.native.diagnostics/v1` catalog spells them. Every
/// `catalog_code()` across every stage returns this same
/// type (C-15); there is no conversion back to a typed cause, and no
/// consumer reads the catalog's message -- only the code, the cause and the
/// structured fields the catalog defines for that pair. Equality is lexical
/// on both fields, matching ADR-013 O-17's stated equality kind.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct CatalogCode {
    code: &'static str,
    cause: &'static str,
}

impl CatalogCode {
    /// Construct a code/cause pair. Callers name codes and causes exactly as
    /// the catalog spells them (ADR-013 R-07: a typed-cause-to-code
    /// conversion never invents a code the catalog does not define).
    pub const fn new(code: &'static str, cause: &'static str) -> Self {
        Self { code, cause }
    }

    /// The catalog's top-level code, e.g. `"runtime_invariant"`.
    pub const fn code(&self) -> &'static str {
        self.code
    }

    /// The catalog's cause under that code, e.g.
    /// `"established-invariant-broken"`.
    pub const fn cause(&self) -> &'static str {
        self.cause
    }
}

impl std::fmt::Display for CatalogCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.code, self.cause)
    }
}

/// FR-259 B6: `quire-canonical` could not reserve memory for a read or an
/// encoding. Intake, observation digest admission and the library package
/// identity read each report this one outcome, carrying the size in bytes of
/// the reservation that failed (`requested`). Memory is not a configured
/// limit, so the outcome names no bound and no setting.
pub const ALLOCATION_FAILED: CatalogCode =
    CatalogCode::new("resource_exhausted", "allocation-failed");

/// ADR-013 T-4: a broken runtime, evaluator or mapping invariant, raised
/// from a compiler stage, the I2 reader, `replay` or `route` (and, for
/// evaluation, `CheckedPackage::call`). Names the stage and the violated
/// invariant, each by a stable identifier -- not a display string (ADR-013
/// R-05) and not itself a closed enum: T-4 gives `LimitKind` alone the
/// "closed enum" qualifier, and the set of stages and invariants that can
/// raise a fault is expected to grow as later slices add stages, without
/// widening this type. `InternalFault` maps to exactly one catalog code
/// (`runtime_invariant`/`established-invariant-broken`, confirmed by the
/// catalog's revision `1-draft.6` note) and exactly one category
/// (`Category::InternalFailure`), and is never a `Refusal` (ADR-013 T-4).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct InternalFault {
    stage: &'static str,
    invariant: &'static str,
}

impl InternalFault {
    /// `stage` and `invariant` are stable identifiers the raising site
    /// names, never derived from a display string or message (ADR-013 R-05).
    pub const fn new(stage: &'static str, invariant: &'static str) -> Self {
        Self { stage, invariant }
    }

    /// The stage, reader or module that raised this fault.
    pub const fn stage(&self) -> &'static str {
        self.stage
    }

    /// The stable identifier of the violated invariant.
    pub const fn invariant(&self) -> &'static str {
        self.invariant
    }

    /// Always `runtime_invariant`/`established-invariant-broken` (ADR-013
    /// T-4): an internal fault is never a `Refusal` and never any other
    /// catalog code.
    pub fn catalog_code(&self) -> CatalogCode {
        CatalogCode::new("runtime_invariant", "established-invariant-broken")
    }

    /// Always `Category::InternalFailure` (ADR-013 O-16).
    pub fn category(&self) -> Category {
        Category::InternalFailure
    }
}

/// Every code of the `quire.native.diagnostics/v1` "Required distinguishing
/// causes" table with its ADR-013 O-16 category, in the catalog's row order.
///
/// This is the refusal-record map: the category a code has when a refusal
/// record carries it (O-17: each cause type has a fixed category). Every
/// code is `Category::Refusal` except `runtime_invariant`, which is
/// `InternalFault`'s one code and category `internal failure` (T-4), and
/// `cancelled` and `stage_limit_exceeded` (a reached stage limit), which
/// O-16's `incomplete` row names and [`Code::is_incomplete`] agrees with.
/// `resource_exhausted` is the caller work-budget code (the catalog's
/// `insufficient-next-charge`; a semantic maximum is not a work budget). It
/// is a refusal here, when a refusal record carries it (ADR-013's read-only
/// ceiling row). When a denied charge stops a run instead, the outcome is
/// incomplete and still carries `resource_exhausted` (ADR-014 B-2; FR-101's
/// `Outcome::Stopped`), and that outcome's own category map says so rather
/// than reading this table. The `unsupported_*`
/// codes are refusals too: the catalog has no category column, and O-16's
/// evaluation column rules `unsupported` out of an evaluation outcome.
/// This table is only the category a refusal record reports (ADR-013
/// O-17). No exit code is derived from it: exits come from the O-16
/// category ([`Category::exit_code`]), which for a native code is
/// [`Code::category`]. That map separates the unsupported
/// codes (`unsupported_construct`, `unknown_required_feature`,
/// `unsupported_projection`) and the incomplete ones (`resource_exhausted`,
/// `incomplete_population`, `unavailable_observation`) that this table
/// files under `Refusal`.
const CATALOG_CATEGORIES: [(&str, Category); 49] = [
    ("invalid_syntax", Category::Refusal),
    ("unsupported_construct", Category::Refusal),
    ("unknown_language", Category::Refusal),
    ("unknown_edition", Category::Refusal),
    ("unknown_profile", Category::Refusal),
    ("unknown_wire", Category::Refusal),
    ("unknown_required_feature", Category::Refusal),
    ("missing_import", Category::Refusal),
    ("missing_declaration", Category::Refusal),
    ("stale_dependency", Category::Refusal),
    ("source_digest_mismatch", Category::Refusal),
    ("ambiguous_declaration", Category::Refusal),
    ("invalid_package", Category::Refusal),
    ("invalid_model_binding", Category::Refusal),
    ("ill_typed", Category::Refusal),
    ("undefined_expression", Category::Refusal),
    ("ambiguous_dispatch", Category::Refusal),
    ("cardinality_out_of_bound", Category::Refusal),
    ("inexact_decimal", Category::Refusal),
    ("decimal_out_of_domain", Category::Refusal),
    ("division_pair_out_of_domain", Category::Refusal),
    ("modulo_out_of_domain", Category::Refusal),
    ("text_length_out_of_domain", Category::Refusal),
    ("integer_out_of_domain", Category::Refusal),
    ("rational_out_of_domain", Category::Refusal),
    ("ieee_not_exact", Category::Refusal),
    ("ieee_nan_payload_not_representable", Category::Refusal),
    ("ieee_rational_out_of_domain", Category::Refusal),
    ("wrong_snapshot", Category::Refusal),
    ("invalid_runtime_input", Category::Refusal),
    ("unavailable_observation", Category::Refusal),
    ("incomplete_population", Category::Refusal),
    ("foreign_reference", Category::Refusal),
    ("dangling_reference", Category::Refusal),
    ("population_delta_mismatch", Category::Refusal),
    ("frame_violation", Category::Refusal),
    ("resource_exhausted", Category::Refusal),
    ("cancelled", Category::Incomplete),
    ("duplicate_selection", Category::Refusal),
    ("stage_limit_exceeded", Category::Incomplete),
    ("noncanonical_wire", Category::Refusal),
    ("unsupported_projection", Category::Refusal),
    ("invalid_capability", Category::Refusal),
    ("projection_binding", Category::Refusal),
    ("invalid_projection_correspondence", Category::Refusal),
    ("extraction-requires-run", Category::Refusal),
    ("extraction-package-conflict", Category::Refusal),
    ("extraction-clause-count", Category::Refusal),
    ("runtime_invariant", Category::InternalFailure),
];

/// FR-090-AC-5: the category a refusal record carrying this catalog code
/// reports, read from the code alone (`CATALOG_CATEGORIES`, the ADR-013
/// O-17 refusal-record map). Never an exit code's source: exits come from
/// the O-16 category ([`Category::exit_code`]). `None` for a code the catalog does not define: an
/// unknown code has no category, and this map never guesses one.
pub fn category_of(code: &CatalogCode) -> Option<Category> {
    catalog_category(code.code())
}

/// The refusal-record category of the catalog code spelled `code`
/// ([`category_of`]); `None` for a spelling the catalog does not define.
pub fn catalog_category(code: &str) -> Option<Category> {
    CATALOG_CATEGORIES
        .iter()
        .find(|(spelling, _)| *spelling == code)
        .map(|&(_, category)| category)
}

/// ADR-013 O-16/O-17: a family-owned evaluation-time refusal
/// cause, held only through this trait so the layer-3 `check` core and this
/// crate itself never name the concrete cause type -- "a cause belongs to
/// the family whose construct produces it" (ADR-013 O-16). The supertraits
/// make a `FamilyResult`/`FamilyOutcome` built from a `Box<dyn CatalogCoded>`
/// `Debug`, able to cross a thread, and free of any borrow (ADR-013 O-16
/// "Representation").
pub trait CatalogCoded: std::fmt::Debug + Send + Sync + 'static {
    /// This cause's catalog code (O-17's method); its O-16 category is
    /// always `Category::Refusal`.
    fn catalog_code(&self) -> CatalogCode;

    /// The structured payload the catalog row requires for the cause
    /// [`Self::catalog_code`] names (ADR-013 O-17, FR-096), with the same
    /// map shape as [`UndefinedRecord::fields`]: one entry for each required
    /// payload item other than a location, valued by the item's rendering.
    /// A location item is the record's [`Locus`], not a field. Read from the
    /// cause's own variant, never from a message.
    ///
    /// `None` for a cause FR-096's key table has no row for: it has no
    /// fields to give, and an empty map would pass for a cause whose row
    /// requires none.
    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>>;

    /// This cause as O-17's [`RefusalRecord`], raised at `locus`: its code,
    /// its fields, and category refusal. `None` when
    /// [`Self::catalog_fields`] is.
    fn refusal_record(&self, locus: Option<Locus>) -> Option<RefusalRecord> {
        Some(RefusalRecord::new(
            self.catalog_code(),
            self.catalog_fields()?,
            locus,
        ))
    }
}

/// FR-096-AC-8: a kernel [`quire_exact::Refusal`] as O-17's
/// [`RefusalRecord`] raised at `locus`, for each of the twelve kernel causes
/// the key table gives a code and fields. The code and cause are the
/// refusal's own [`quire_exact::Refusal::code`]/[`cause`](quire_exact::Refusal::cause);
/// the fields are read from the variant, never from a message: each domain
/// or width is spelled exactly as the catalog spells it (FR-096 "A kernel
/// value refusal carries what its record renders"), and
/// `CardinalityOutOfBound`/`ForeignReference` keep their own fields.
///
/// `None` for `CheckedInvariant`: it is an [`InternalFault`], never a
/// refusal record (a record is always category refusal).
#[deny(clippy::wildcard_enum_match_arm)]
pub fn kernel_refusal_record(
    refusal: &quire_exact::Refusal,
    locus: Option<Locus>,
) -> Option<RefusalRecord> {
    use quire_exact::{CollectionKind, InexactTarget, Refusal};
    let code = CatalogCode::new(refusal.code()?, refusal.cause()?);
    let fields = match refusal {
        Refusal::CardinalityOutOfBound {
            kind, bound, count, ..
        } => {
            let collection = match kind {
                CollectionKind::Sequence => "sequence",
                CollectionKind::Set => "set",
                CollectionKind::Bag => "bag",
                CollectionKind::OrderedSet => "ordered-set",
            };
            BTreeMap::from([
                ("collection", collection.to_owned()),
                (
                    "bound",
                    format!("[{}, {}]", bound.minimum(), bound.maximum()),
                ),
                ("count", count.to_string()),
            ])
        }
        Refusal::ForeignReference { required, supplied } => BTreeMap::from([
            ("required", required.to_string()),
            ("supplied", supplied.to_string()),
        ]),
        Refusal::InexactDecimal { target } => BTreeMap::from([(
            "expected",
            match target {
                InexactTarget::Decimal(decimal) => spell_decimal(decimal),
                InexactTarget::Integer(interval) => spell_int(interval),
            },
        )]),
        Refusal::DecimalOutOfDomain { target } => {
            BTreeMap::from([("expected", spell_decimal(target))])
        }
        Refusal::DivisionPairOutOfDomain { domain, .. } | Refusal::ModuloOutOfDomain { domain } => {
            BTreeMap::from([("expected", spell_int(domain))])
        }
        Refusal::IntegerOutOfDomain { target } => BTreeMap::from([("expected", spell_int(target))]),
        Refusal::TextLengthOutOfDomain { target } => BTreeMap::from([(
            "expected",
            format!(
                "Text[{}, {}; {}]",
                target.min(),
                target.max(),
                target.profile().as_str()
            ),
        )]),
        Refusal::RationalOutOfDomain { target } | Refusal::IeeeRationalOutOfDomain { target } => {
            BTreeMap::from([("expected", spell_rational(target))])
        }
        Refusal::IeeeNotExact { target, would_be } => BTreeMap::from([
            ("expected", target.as_str().to_owned()),
            (
                "flags",
                would_be
                    .iter()
                    .map(quire_exact::IeeeFlag::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ]),
        Refusal::IeeeNanPayloadNotRepresentable { target, source } => BTreeMap::from([
            ("expected", target.as_str().to_owned()),
            ("actual", source.as_str().to_owned()),
        ]),
        Refusal::CheckedInvariant => return None,
    };
    Some(RefusalRecord::new(code, fields, locus))
}

/// `Int[lo, hi]`, each bound as QSpec FR-038 writes an integer.
fn spell_int(interval: &quire_exact::IntegerInterval) -> String {
    format!("Int[{}, {}]", interval.lower(), interval.upper())
}

/// `Decimal[lo, hi; smin, smax]`; no rounding mode is written.
fn spell_decimal(decimal: &quire_exact::DecimalType) -> String {
    format!(
        "Decimal[{}, {}; {}, {}]",
        decimal.lower(),
        decimal.upper(),
        decimal.min_scale(),
        decimal.max_scale()
    )
}

/// `Rational[lo, hi; dmin, dmax]`.
fn spell_rational(domain: &quire_exact::RationalDomain) -> String {
    let (numerator, denominator) = (domain.numerator(), domain.denominator());
    format!(
        "Rational[{}, {}; {}, {}]",
        numerator.lower(),
        numerator.upper(),
        denominator.lower(),
        denominator.upper()
    )
}

/// ADR-013 O-17 (FR-096): a refusal as a consumer outside its producer
/// reads it -- the catalog code, the O-16 category (always
/// [`Category::Refusal`]), the T-5 [`Locus`] where the refusal was raised,
/// and the structured fields the catalog row requires for that code, other
/// than a location (a location is the locus). The producer reads the fields
/// from its typed cause, never from a message.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefusalRecord {
    code: CatalogCode,
    locus: Option<Locus>,
    fields: BTreeMap<&'static str, String>,
}

impl RefusalRecord {
    /// A refusal with catalog code `code` and catalog fields `fields`,
    /// raised at `locus` (`None` where no producer can know a position,
    /// FR-096).
    pub fn new(
        code: CatalogCode,
        fields: BTreeMap<&'static str, String>,
        locus: Option<Locus>,
    ) -> Self {
        Self {
            code,
            locus,
            fields,
        }
    }

    /// The cause's catalog code.
    pub fn code(&self) -> CatalogCode {
        self.code
    }

    /// Always [`Category::Refusal`] (ADR-013 O-17: a `CatalogCoded` cause's
    /// category is fixed).
    pub fn category(&self) -> Category {
        Category::Refusal
    }

    /// Where the refusal was raised, when known.
    pub fn locus(&self) -> Option<&Locus> {
        self.locus.as_ref()
    }

    /// The cause's catalog fields, keyed by the catalog's payload names.
    pub fn fields(&self) -> &BTreeMap<&'static str, String> {
        &self.fields
    }
}

/// ADR-013 O-16: a family-owned evaluation-time undefined cause,
/// held only through this trait -- the undefined-category counterpart of
/// [`CatalogCoded`].
pub trait UndefinedCoded: std::fmt::Debug + Send + Sync + 'static {
    /// This cause's [`UndefinedRecord`]; its O-16 category is always
    /// `Category::Undefined`.
    fn undefined_record(&self) -> UndefinedRecord;
}

/// ADR-013 O-16: the closed reason set of the
/// `quire.native.diagnostics/v1` "Undefined reasons" table -- the catalog
/// states this is not a refusal code or cause. Grows by one variant each
/// time a family adds a new evaluation-time undefined result, the same way
/// a family-dispatch cause set grows.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum UndefinedReason {
    /// FR-151 dispatch: an FR-151 dispatched call's selected method's
    /// effective precondition evaluated to `false`.
    PreconditionFalse,
    /// FR-153: a `lookup<T>(p, r) absent undefined` query's reference `r`
    /// names no member of the population bound to `p`.
    AbsentKey,
}

impl UndefinedReason {
    /// The catalog's own reason spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PreconditionFalse => "precondition-false",
            Self::AbsentKey => "absent-key",
        }
    }
}

impl std::fmt::Display for UndefinedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// ADR-013 O-16: the undefined-side counterpart of O-17's
/// `RefusalRecord` (held back for #213 S-5b): `reason` plus the catalog's
/// own structured payload for that reason. [`UndefinedCoded::undefined_record`]
/// returns this record, not the bare reason, because the catalog requires
/// the payload and a consumer outside the producing family holds only the
/// trait object -- without the record it could reach the payload only by
/// downcasting to the family's own cause type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedRecord {
    /// The catalog's closed undefined reason.
    pub reason: UndefinedReason,
    /// The catalog's structured payload for `reason`, keyed by the
    /// catalog's own field names.
    pub fields: std::collections::BTreeMap<&'static str, String>,
}

#[cfg(test)]
mod foundation_tests {

    use super::{
        category_of, resource_exhausted, CatalogCode, Category, Code, InternalFault, Phase, Source,
        SourceIdentity, Span, SyntaxLimit, CATALOG_CATEGORIES,
    };
    use ix_trace_rs::trace;

    /// FR-285-AC-1 (TC-769 step 1): each of the nine table rows maps to its
    /// code, with undefined folding to 10 beside violation, an inconclusive
    /// proof or `analyze` item at 22 and a supplied-trace clause pending at
    /// the trace's end at 0.
    #[trace("TC-769", "FR-285-AC-1")]
    #[test]
    fn tc_769_exit_function_maps_every_table_row() {
        let rows = [
            Category::Success.exit_code(),
            Category::Violation.exit_code(),
            Category::Undefined.exit_code(),
            Category::Refusal.exit_code(),
            Category::Unsupported.exit_code(),
            Category::Incomplete.exit_code(),
            Category::Inconclusive.exit_code(),
            Category::Inconclusive.trace_exit_code(),
            Category::InternalFailure.exit_code(),
        ];
        assert_eq!(rows, [0, 10, 10, 20, 21, 22, 22, 0, 30]);
        for category in Category::ALL {
            if category != Category::Inconclusive {
                assert_eq!(
                    category.trace_exit_code(),
                    category.exit_code(),
                    "{category}"
                );
            }
        }
    }

    /// FR-285-AC-2 (TC-769 step 2): a multi-item outcome exits with the most
    /// severe item code under the order 30, 20, 21, 22, 10, 0, in any item
    /// order.
    #[trace("TC-769", "FR-285-AC-2")]
    #[test]
    fn tc_769_multi_item_outcomes_exit_with_the_most_severe_code() {
        let cases: [(&[u8], u8); 6] = [
            (&[10, 22], 22),
            (&[0, 30, 20], 30),
            (&[21, 22], 21),
            (&[20, 21], 20),
            (&[0, 10], 10),
            (&[0], 0),
        ];
        for (codes, expected) in cases {
            assert_eq!(
                Category::most_severe(codes.iter().copied()),
                Some(expected),
                "{codes:?}"
            );
            assert_eq!(
                Category::most_severe(codes.iter().rev().copied()),
                Some(expected),
                "{codes:?} reversed"
            );
        }
        assert_eq!(Category::most_severe(std::iter::empty()), None);
    }

    /// `Code::from_code` is the typed reverse lookup of `Code::as_str`,
    /// round-tripping every code, and `None` for an unrecognized spelling.
    #[test]
    fn from_code_round_trips_every_code_and_refuses_an_unknown_one() {
        for code in Code::all() {
            assert_eq!(Code::from_code(code.as_str()), Some(*code));
        }
        assert_eq!(Code::from_code("not_a_real_code"), None);
    }

    /// FR-001-AC-11: the native `Diagnostic` of `Source::read` carries
    /// `blank-label` with its label, or `empty-path`, on
    /// `invalid_source_identity`.
    #[ix_trace_rs::trace("TC-424", "FR-001-AC-11")]
    #[test]
    fn the_native_diagnostic_carries_the_identity_cause() {
        use crate::source::{SourceLabel, SourceReadCause};
        let read = |identity, path| {
            *Source::read(identity, path, b"x", crate::source::MAX_SOURCE_BYTES)
                .expect_err("refused")
        };
        let blank = read(SourceIdentity::new("a", "u", " ", "1"), "");
        let empty = read(SourceIdentity::new("a", "u", "git", "1"), "");
        assert_eq!(
            blank.identity_cause(),
            Some(SourceReadCause::BlankLabel {
                label: SourceLabel::RevisionNamespace
            })
        );
        assert_eq!(empty.identity_cause(), Some(SourceReadCause::EmptyPath));
        assert!(matches!(
            (blank.code, empty.code),
            (Code::InvalidSourceIdentity, Code::InvalidSourceIdentity)
        ));
    }

    fn source() -> Source {
        Source::read(
            SourceIdentity::new("agent-ix", "test:diagnostic", "git", "1"),
            "diagnostic.native",
            b"x",
            crate::source::MAX_SOURCE_BYTES,
        )
        .expect("test source")
    }

    /// Every `SyntaxLimit` kind reports `Code::StageLimitExceeded`, and
    /// names its catalog `LimitKind` (revision `1-draft.8`): the token
    /// ceiling is `token-count-exceeded`. The native-v1 `Diagnostic` has no
    /// typed cause field, so the kind is named in the rendered message.
    #[test]
    fn resource_exhausted_reports_the_kind_that_maps_to_the_catalog() {
        let source = source();
        let span = Span { start: 0, end: 1 };
        let cases = [
            (SyntaxLimit::Nodes { bound: 4 }, Code::StageLimitExceeded),
            (SyntaxLimit::Work { bound: 4 }, Code::StageLimitExceeded),
            (
                SyntaxLimit::SourceBytes { bound: 4 },
                Code::StageLimitExceeded,
            ),
            (SyntaxLimit::Tokens { bound: 4 }, Code::StageLimitExceeded),
        ];
        for (limit, code) in cases {
            let diagnostic = resource_exhausted(&source, Phase::Parse, span, limit);
            assert_eq!(diagnostic.code, code, "{limit:?}");
            assert_eq!(diagnostic.limit(), Some(limit));
        }
        assert_eq!(
            SyntaxLimit::Tokens { bound: 4 }
                .stage_kind()
                .catalog_cause(),
            "token-count-exceeded"
        );
    }

    /// FR-090-AC-5's map: one row per catalog code, the internal-fault code
    /// is `internal failure`, and an unknown code has no category.
    #[test]
    fn category_of_reads_the_code_and_refuses_an_unknown_one() {
        let mut spellings: Vec<&str> = CATALOG_CATEGORIES.iter().map(|(code, _)| *code).collect();
        spellings.sort_unstable();
        spellings.dedup();
        assert_eq!(spellings.len(), CATALOG_CATEGORIES.len());
        assert_eq!(
            category_of(&InternalFault::new("S6a", "x").catalog_code()),
            Some(Category::InternalFailure)
        );
        assert_eq!(
            category_of(&CatalogCode::new("wrong_snapshot", "wrong-anchor")),
            Some(Category::Refusal)
        );
        assert_eq!(
            category_of(&CatalogCode::new("cancelled", "caller-cancelled")),
            Some(Category::Incomplete)
        );
        assert_eq!(
            category_of(&CatalogCode::new("not_a_catalog_code", "x")),
            None
        );
    }

    #[test]
    fn category_has_exactly_the_adr_013_o16_eight_values() {
        let spellings: Vec<&str> = Category::ALL
            .iter()
            .map(|category| category.as_str())
            .collect();
        assert_eq!(
            spellings,
            vec![
                "success",
                "violation",
                "undefined",
                "refusal",
                "unsupported",
                "incomplete",
                "inconclusive",
                "internal-failure",
            ]
        );
        // Every match over `Category` in this crate must be exhaustive; a
        // ninth variant added here without a corresponding compile error
        // elsewhere would mean some consumer grew a `_` arm.
        for category in Category::ALL {
            match category {
                Category::Success
                | Category::Violation
                | Category::Undefined
                | Category::Refusal
                | Category::Unsupported
                | Category::Incomplete
                | Category::Inconclusive
                | Category::InternalFailure => {}
            }
        }
    }

    #[test]
    fn catalog_code_display_is_code_slash_cause() {
        let code = CatalogCode::new("runtime_invariant", "established-invariant-broken");
        assert_eq!(code.code(), "runtime_invariant");
        assert_eq!(code.cause(), "established-invariant-broken");
        assert_eq!(
            code.to_string(),
            "runtime_invariant/established-invariant-broken"
        );
    }

    #[test]
    fn internal_fault_never_reports_a_refusal_code_or_category() {
        let fault = InternalFault::new("S3", "checked-node-not-in-model-correspondence");
        assert_eq!(fault.stage(), "S3");
        assert_eq!(
            fault.invariant(),
            "checked-node-not-in-model-correspondence"
        );
        assert_eq!(
            fault.catalog_code(),
            CatalogCode::new("runtime_invariant", "established-invariant-broken")
        );
        assert_eq!(fault.category(), Category::InternalFailure);
    }
}
