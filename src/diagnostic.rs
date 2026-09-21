// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-010: stable source-bound native diagnostics and standard error propagation.
use crate::source::{LocatedSpan, Source, SourceIdentity, Span};

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
    /// Checked native expressions are outside the selected executable projection.
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
    /// Related formal declarations, sorted by identity and source location.
    pub related: Vec<crate::linking::DeclarationLocation>,
    /// Structured upstream formal diagnostic, when that operation failed.
    pub upstream: Option<Box<quire_contract_ir::Diagnostic>>,
    /// Exact programmatic input location, separate from authored source coordinates.
    pub runtime: Option<Box<crate::runtime::RuntimeLocation>>,
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

pub(crate) fn error(
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
        related: Vec::new(),
        upstream: None,
        runtime: None,
    })
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
// Held back as S-5b, not built here: `RefusalRecord` and `LimitExceeded`
// both carry the foundation `Locus` (ADR-013 T-5), which is slice S-4's row
// and has not landed; `LimitKind`'s only consumer is `LimitExceeded`, so it
// stays with it. O-22's version-refusal readers all construct a
// `RefusalRecord` too -- for a wire reader specifically, ADR-013 T-5 assigns
// `Locus::Artifact{digest, pointer}`, which additionally needs the O-18
// digest record (S-2's row, gated on quire-specification#138) -- so O-22
// stays in S-5b in full, blocked on both S-4 and S-2.

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

#[cfg(test)]
mod foundation_tests {
    use super::{CatalogCode, Category, InternalFault};

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
