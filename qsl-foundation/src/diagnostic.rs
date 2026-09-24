// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010: stable source-bound native diagnostics and standard error propagation.
//!
//! [`Locus`] is ADR-013 T-5's foundation diagnostic locus.
use crate::source::{LocatedSpan, Source, SourceIdentity, Span};

mod locus;
mod stage;
pub use locus::{InvalidJsonPointer, JsonPointer, Locus, UnresolvedLocus};
pub use stage::{LimitExceeded, LimitKind, StageFailure, Staged};

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
    /// FR-322 I2: an admitted `quire.checked-package/v2` wire's lock names
    /// one or more `dependency_selections`; this reader does not yet derive
    /// imports from a lock (QC-10, ADR-013 TK-08).
    UnsupportedDependencySelections,
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
            Self::UnsupportedDependencySelections => "unsupported_dependency_selections",
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
            Self::UnsupportedDependencySelections,
        ]
    }

    /// Resolve a known stable spelling; unknown codes remain explicit absence.
    pub fn from_code(value: &str) -> Option<Self> {
        Self::all()
            .iter()
            .copied()
            .find(|code| code.as_str() == value)
    }

    /// Whether this code records incomplete work rather than invalid input.
    pub fn is_incomplete(self) -> bool {
        matches!(
            self,
            Self::ResourceExhausted
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

    /// FR-301's single-code ladder: unsupported (21) before incomplete (22)
    /// before invalid or refused input (20). The sole source of this
    /// mapping; every exit-code site routes through it rather than
    /// re-deriving it from `is_unsupported`/`is_incomplete`.
    pub fn exit_code(self) -> u8 {
        if self.is_unsupported() {
            21
        } else if self.is_incomplete() {
            22
        } else {
            20
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
    /// Exact caller-selected diagnostic identity and revision labels.
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
}

/// The syntax resource ceiling a `resource_exhausted` refusal names
/// (NFR-001: a refusal names the limit kind, the bound and the span). Both
/// S1 parsers and both lexers refuse through this one type, so a caller
/// distinguishes the ceilings by variant, never by message text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SyntaxLimit {
    /// Bracket-pair nesting depth (NFR-001 "Nesting level"): `bound` is the
    /// selected nesting ceiling and the refusal's span is the opening
    /// bracket of pair `bound + 1`.
    NestingDepth {
        /// Selected nesting ceiling, in bracket pairs.
        bound: usize,
    },
    /// Token (complete-V1: retained CST leaf) ceiling.
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
            Self::NestingDepth { bound } => {
                write!(f, "nesting depth exceeds the ceiling of {bound} levels")
            }
            Self::Tokens { bound } => write!(f, "token ceiling of {bound} tokens exhausted"),
            Self::Nodes { bound } => write!(f, "syntax node ceiling of {bound} nodes exhausted"),
            Self::Work { bound } => write!(f, "parser work budget of {bound} steps exhausted"),
            Self::SourceBytes { bound } => {
                write!(f, "source byte ceiling of {bound} bytes exhausted")
            }
        }
    }
}

/// The one constructor for a syntax-ceiling refusal: code
/// `resource_exhausted`, the typed [`SyntaxLimit`] and a message rendered
/// from it, at `span`.
pub fn resource_exhausted(
    source: &Source,
    phase: Phase,
    span: Span,
    limit: SyntaxLimit,
) -> Box<Diagnostic> {
    let mut diagnostic = error(
        source,
        Code::ResourceExhausted,
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
    /// Whether incomplete work, rather than invalid input, caused this diagnostic.
    pub fn is_incomplete(&self) -> bool {
        self.code.is_incomplete()
    }
    /// Whether this diagnostic names a real, catalogued capability this
    /// build does not implement, rather than input that is itself invalid.
    pub fn is_unsupported(&self) -> bool {
        self.code.is_unsupported()
    }
    /// FR-301's exit code for this diagnostic alone, always one of
    /// {20, 21, 22} (asserted over `Code::all()` in
    /// tests/native_boundaries.rs). Within exactly that range, ascending
    /// exit code happens to be ascending severity (20 invalid, 21
    /// unsupported, 22 incomplete), so a caller combining several
    /// diagnostics into one report resolves the group's code by taking the
    /// numeric minimum over this method — see command/output.rs's `report`.
    /// This does not generalize past {20, 21, 22}; FR-301's full order
    /// (tool failure, invalid, unsupported, incomplete, violation, success)
    /// is not ascending-numeric across 0/10/20/21/22/30.
    pub fn exit_code(&self) -> u8 {
        self.code.exit_code()
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
            .map_err(|refusal| Box::new(Diagnostic::from(*refusal)))
    }

    /// Verify an independently supplied byte digest before constructing a mapped subject.
    pub fn read_verified(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        expected: crate::ByteDigest,
        byte_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        let source = Self::read(identity, path, bytes, byte_limit)?;
        if source.digest() != expected {
            return Err(error(
                &source,
                Code::SourceDigestMismatch,
                Phase::Source,
                0,
                0,
                "source bytes differ from the selected digest",
            ));
        }
        Ok(source)
    }
}

impl From<crate::source::SourceReadRefusal> for Diagnostic {
    fn from(refusal: crate::source::SourceReadRefusal) -> Self {
        use crate::source::SourceReadCause;
        let code = match refusal.cause {
            SourceReadCause::UnnamedSource => Code::InvalidSourceIdentity,
            SourceReadCause::ByteBudget => Code::ResourceExhausted,
            SourceReadCause::InvalidUtf8 => Code::InvalidUtf8,
            SourceReadCause::Nul => Code::InvalidSyntax,
        };
        Diagnostic {
            phase: Phase::Source,
            code,
            source: refusal.error.source,
            path: refusal.error.path,
            span: refusal.error.span,
            message: refusal.error.message,
            limit: None,
        }
    }
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
// S-5b (QSL-160) part one adds T-4's `LimitKind`, `LimitExceeded`,
// `Staged` and `StageFailure` in `stage`, with no `Locus` and no catalog
// code: `stage_limit_exceeded` is a revision `1-draft.6` code, and this
// build claims `1-draft.3` (Remaining work: QSL-236). `RefusalRecord`, the
// `Locus` on `LimitExceeded` and O-22's readers wait on a producer that can
// supply a `Locus` (Remaining work: QSL-233).

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
/// A catalog code in QSL is carried by a refusal record (O-17: each cause
/// type has a fixed category), so every code is `Category::Refusal` except:
/// `runtime_invariant`, which is `InternalFault`'s one code and category
/// `internal failure` (T-4); and `cancelled`, a caller cancellation, which
/// O-16's `incomplete` row names. `resource_exhausted` is a refusal: QSL
/// raises it only for a semantic maximum, which the catalog states is not a
/// caller work budget; an exhausted S6a work budget is the kernel
/// `Incomplete` outcome, which carries no catalog code. The `unsupported_*`
/// codes are refusals too: the catalog has no category column, and O-16's
/// evaluation column rules `unsupported` out of an evaluation outcome.
/// [`Code::is_incomplete`] and [`Code::is_unsupported`] are the native-v1
/// exit-code ladder (FR-301), not the O-16 category, so they may differ
/// from this table.
const CATALOG_CATEGORIES: [(&str, Category); 38] = [
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
    ("stage_limit_exceeded", Category::Refusal),
    ("unsupported_projection", Category::Refusal),
    ("invalid_capability", Category::Refusal),
    ("projection_binding", Category::Refusal),
    ("invalid_projection_correspondence", Category::Refusal),
    ("extraction-requires-run", Category::Refusal),
    ("extraction-package-conflict", Category::Refusal),
    ("extraction-clause-count", Category::Refusal),
    ("runtime_invariant", Category::InternalFailure),
];

/// FR-090-AC-5: the O-16 category of a catalog code, read from the code
/// alone. `None` for a code the catalog does not define: an unknown code has
/// no category, and this map never guesses one.
pub fn category_of(code: &CatalogCode) -> Option<Category> {
    CATALOG_CATEGORIES
        .iter()
        .find(|(spelling, _)| *spelling == code.code())
        .map(|&(_, category)| category)
}

/// ADR-013 O-16/O-17 (QSL-174): a family-owned evaluation-time refusal
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
}

/// ADR-013 O-16 (QSL-174): a family-owned evaluation-time undefined cause,
/// held only through this trait -- the undefined-category counterpart of
/// [`CatalogCoded`].
pub trait UndefinedCoded: std::fmt::Debug + Send + Sync + 'static {
    /// This cause's [`UndefinedRecord`]; its O-16 category is always
    /// `Category::Undefined`.
    fn undefined_record(&self) -> UndefinedRecord;
}

/// ADR-013 O-16 (QSL-174): the closed reason set of the
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

/// ADR-013 O-16 (QSL-174): the undefined-side counterpart of O-17's
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
    use super::{category_of, CatalogCode, Category, InternalFault, CATALOG_CATEGORIES};

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
