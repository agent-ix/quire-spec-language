// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located checking refusals and their closed codes and causes.

use qsl_foundation::diagnostic::{Code, LimitKind};
use quire_exact::EffectiveId;
use quire_exact::Integer;
use quire_exact::{IllTyped, IllTypedCause};

/// The declaration a location belongs to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Origin {
    /// The body of the named function, at this zero-based declaration index.
    Body {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// The `decreases` measure of the named function.
    Measure {
        /// The declared name.
        function: String,
        /// The declaration index in the package.
        index: usize,
    },
    /// A standalone checked expression.
    Expression,
    /// A declared record, tuple or enum type, by its declared name. Its
    /// `declaration` occurrence is located here (FR-322). A type
    /// declaration carries no form spans, so this origin names no region
    /// (FR-096).
    TypeDeclaration {
        /// The declared name.
        name: String,
    },
}

/// A located expression: its declaration and the child-index path from that
/// declaration's root expression.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    /// The owning declaration.
    pub origin: Origin,
    /// Child indices from the root, as [`Expression::children`] numbers them.
    ///
    /// [`Expression::children`]: qsl_forms::Expression::children
    pub path: Vec<usize>,
}

impl Location {
    /// A child of this location.
    pub(crate) fn child(&self, index: usize) -> Self {
        let mut path = self.path.clone();
        path.push(index);
        Self {
            origin: self.origin.clone(),
            path,
        }
    }
}

/// An inclusive mathematical integer interval whose missing ends are
/// unbounded.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ProvedInterval {
    /// The lower end, if bounded.
    pub lower: Option<Integer>,
    /// The upper end, if bounded.
    pub upper: Option<Integer>,
}

/// The definedness obligation that could not be proved.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Obligation {
    /// A divisor may be zero: `unproved-nonzero`.
    Nonzero,
    /// `value(e)` without a proved `present(e)`: `unproved-presence`.
    Presence,
    /// An integer value may lie outside its required interval:
    /// `unproved-range`.
    Range {
        /// The required interval.
        required: Box<ProvedInterval>,
        /// The interval the accepted proof forms establish.
        proved: Box<ProvedInterval>,
    },
    /// A rational quotient may lie outside its `Rational[..]` domain:
    /// `unproved-range`.
    RationalRange,
    /// `reduce` without a proof of `size(c) >= 1`: `unproved-range`.
    NonemptyReduction {
        /// The proved `size(c)` interval.
        size: Box<ProvedInterval>,
    },
}

/// The FR-146 component obligation that failed, in check order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MeasureObligation {
    /// `missing-measure`.
    MissingMeasure,
    /// `measure-arity`.
    MeasureArity,
    /// `measure-kind`.
    MeasureKind,
    /// `nonnegative`.
    Nonnegative,
    /// `decrease`.
    Decrease,
}

impl MeasureObligation {
    /// The obligation spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingMeasure => "missing-measure",
            Self::MeasureArity => "measure-arity",
            Self::MeasureKind => "measure-kind",
            Self::Nonnegative => "nonnegative",
            Self::Decrease => "decrease",
        }
    }
}

/// The checking stage a declared checking limit bounds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckingStage {
    /// Name resolution and typing.
    Typing,
}

/// Which declared checking limit was reached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CheckingLimitKind {
    /// Admitted expression nodes per checked package.
    Nodes,
    /// Expression nesting depth.
    Depth,
    /// A checked-family declaration's own preimage byte length (QSL-153,
    /// `qsl_foundation::diagnostic::LimitKind::InputBytes`).
    InputBytes,
    /// Cumulative checking work: the checked-family contract meter's
    /// spend across every declaration checked against it (QSL-153,
    /// `qsl_foundation::diagnostic::LimitKind::WorkBudget`), and lowering's
    /// `declaration.check` charges.
    WorkBudget,
}

impl CheckingLimitKind {
    /// The T-4 [`LimitKind`] this crate-local kind names (QSL-236): the one
    /// bijection every construction site in this crate already assumed
    /// (`check::mod`'s own reverse mapping), now named once.
    pub(crate) const fn foundation_kind(self) -> LimitKind {
        match self {
            Self::Nodes => LimitKind::NodeCount,
            Self::Depth => LimitKind::NestingDepth,
            Self::InputBytes => LimitKind::InputBytes,
            Self::WorkBudget => LimitKind::WorkBudget,
        }
    }
}

impl From<LimitKind> for CheckingLimitKind {
    /// The reverse of `CheckingLimitKind::foundation_kind` (QSL-236, L6):
    /// `check::mod`'s `StageFailure::Limit` arm named this bijection inline
    /// as a `match`, matched exhaustively rather than a `_` catch-all (PR
    /// #262 review, coordinator round 3, finding 4) so a `LimitKind` this
    /// crate does not yet expect forces a real decision here, not a guess.
    /// `NodeCount` maps onto the pre-existing `Self::Nodes` (both name "how
    /// many expression nodes"); `InputBytes` and `WorkBudget` have no
    /// pre-existing counterpart in this older `Typer`-era enum, so QSL-153
    /// added one each. Named once here rather than duplicated at that call
    /// site.
    fn from(kind: LimitKind) -> Self {
        match kind {
            LimitKind::NestingDepth => Self::Depth,
            LimitKind::NodeCount => Self::Nodes,
            LimitKind::InputBytes => Self::InputBytes,
            LimitKind::WorkBudget => Self::WorkBudget,
        }
    }
}

/// FR-272's closed `wrong_snapshot` cause list this crate decides for
/// `pre(...)` (native-diagnostics.md: `wrong-observation`, `wrong-invocation`,
/// `wrong-anchor` or `forbidden-pre-read`). Only the two causes `pre(...)`
/// itself can name are represented; `wrong-observation` and
/// `wrong-invocation` are catalogued for the native runtime's own
/// snapshot-selection refusals (`src/runtime`), not this crate.
///
/// The two represented causes split by *layer*, not by clause: every
/// checking-time refusal this crate's `Typer` decides over `pre(...)`'s own
/// syntax -- wrong clause, ineligible operand, or a captured `let` alias
/// (FR-208 applies FR-042 to invariants/preconditions and names this same
/// cause explicitly) -- is [`Self::ForbiddenPreRead`]. [`Self::WrongAnchor`]
/// is reserved for the one case the checker cannot see at all: a
/// postcondition it did admit, evaluated at runtime
/// (`evaluate.rs`'s `select_anchor`) over a population value with no
/// attached pre binding.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WrongSnapshotCause {
    /// A postcondition's `pre(...)` evaluated over a `Value::Population`
    /// with no attached pre anchor -- the caller supplied a population
    /// admitted through `admit_binding` directly rather than through
    /// `admit_invocation` (the only constructor that attaches one). A
    /// runtime-only cause: the checker has no way to see, at checking time,
    /// which admission path a caller's runtime argument will take.
    WrongAnchor,
    /// `pre(...)` written where the checked declaration does not admit it
    /// at all (anywhere but an operation's postcondition -- shared-
    /// grammar.md's "self, result and pre(...) are caller-side anchor
    /// operations, unavailable as implicit ambient state inside a reusable
    /// predicate"), or admitted but refused over its own operand: a bare
    /// `Name`, literal, or a value already captured/computed outside this
    /// `pre(...)`'s own operand (FR-042's Behavior clause: "Pre is refused
    /// ... on bare parameters/constants/captures"; FR-042-AC-3's `let s =
    /// self in pre(s.version)` analogue for a `let`-aliased state root) --
    /// re-anchoring any of these would silently do nothing rather than
    /// replay a real read under the pre observation.
    ForbiddenPreRead,
    /// FR-063, TC-387: exists only so `--cfg seam_probe` makes the
    /// `ProtocolClause` snapshot cause's `catalog_code()` match
    /// non-exhaustive. Never constructed outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

impl WrongSnapshotCause {
    /// The closed FR-272 cause tag. Not a seam-probe location: it owns the
    /// probe variant's own arm.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WrongAnchor => "wrong-anchor",
            Self::ForbiddenPreRead => "forbidden-pre-read",
            #[cfg(seam_probe)]
            Self::__SeamProbe => "__seam_probe__",
        }
    }
}

/// [`CheckCause::ResourceExhausted`]'s payload (QSL-236): the checking
/// stage, the limit kind reached, its declared bound and the counter value
/// the refused step would have reached.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct StageLimitCause {
    /// The checking stage.
    pub stage: CheckingStage,
    /// The limit reached.
    pub kind: CheckingLimitKind,
    /// The declared limit.
    pub limit: u64,
    /// The counter value the refused step would have reached.
    pub actual: u128,
}

/// The typed cause of a checking refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CheckCause {
    /// `refused { code: ill_typed }` with its FR-272 cause.
    IllTyped(IllTypedCause),
    /// `refused { code: wrong_snapshot }` with its FR-272 cause: `pre(...)`
    /// written outside its admitted clause, or over an ineligible operand.
    WrongSnapshot(WrongSnapshotCause),
    /// `missing_declaration` / `missing-name`.
    MissingName(String),
    /// `ambiguous_declaration` / `ambiguous-name`, listing every locus of the
    /// name.
    AmbiguousName {
        /// The ambiguous name.
        name: String,
        /// Every declaring locus.
        loci: Vec<Location>,
    },
    /// `undefined_expression` with the cause of its obligation.
    Unproved(Obligation),
    /// `undefined_expression` / `unproved-decrease` for one recursive
    /// component.
    UnprovedDecrease {
        /// The component cycle, first and last entries equal. The refusal
        /// location is the call edge the obligation is about.
        cycle: Vec<String>,
        /// The failed obligation.
        obligation: MeasureObligation,
    },
    /// `stage_limit_exceeded`/`<kind>-exceeded` (QSL-236): a declared
    /// checking limit was reached at this node. Boxed: `actual`'s `u128`
    /// would otherwise make every `CheckCause` pay for this one variant's
    /// widest field (clippy `result_large_err` on `CheckRefusal`), the same
    /// reason `InternalFault` below is boxed.
    ResourceExhausted(Box<StageLimitCause>),
    /// An IEEE conversion or arithmetic in a package with no admitted IEEE
    /// profile:
    /// `invalid_package`.
    IeeeProfileNotAdmitted,
    /// A derived collection bound exceeds the representable cardinality range.
    UnrepresentableBound,
    /// `invalid_package` / `definition-cycle` (FR-151, TC-196 D08): an
    /// FR-146 call-graph strongly connected component containing an FR-151
    /// dispatch edge. Operation bodies and contract clauses have no
    /// `decreases` form, so such a component is refused outright, never
    /// measure-checked.
    DefinitionCycle {
        /// Every dispatch edge in the component, ascending by caller then
        /// callee source-declaration order (FR-151's own listing order).
        edges: Vec<(String, String)>,
    },
    /// `invalid_package` / `invalid-value`: a supplied [`super::DispatchTable`]
    /// or [`super::DispatchOperation`] is malformed — an out-of-range table
    /// or function index, or a candidate whose signature does not match its
    /// dispatch operation's declared arity or types. Checked upfront in
    /// [`super::PackageDeclarations::check`], before any node is typed, so
    /// `facts`'s own call-graph walk can treat every table index it
    /// reads as already valid; reuses the already-catalogued `invalid-value`
    /// tag rather than minting a new one. Boxed (QSL-236): this was already
    /// `CheckCause`'s widest variant at 56 bytes; adding `ResourceExhausted`'s
    /// `StageLimitCause` payload cost `CheckCause` its niche-packed
    /// discriminant, so `CheckRefusal` crossed clippy's `result_large_err`
    /// threshold. Boxing this pre-existing variant (not `ResourceExhausted`,
    /// already boxed) is the fix, same precedent as `InternalFault` below.
    InvalidDispatchDeclaration(Box<InvalidDispatchDeclaration>),
    /// `missing_declaration` / `missing-selection` (FR-093): a lowered
    /// operation or leaf needs a profile law whose `DefinitionRef` the
    /// package's lock evidence does not supply. `check` writes no law from a
    /// constant (ADR-011 §2.4), so the node is not built.
    MissingSelection {
        /// The law role the lock evidence does not select.
        role: super::node_key::LawRole,
    },
    /// `unknown_required_feature` / `unsupported-feature` (FR-092 "Recursion
    /// groups", FR-092-OQ-1): the nodes of a recursion group cannot be keyed
    /// apart from another group's, so `check` refuses the group instead of
    /// keying it. `loci` names every in-group declaration, in declaration
    /// order.
    UnsupportedFeature {
        /// Every in-group declaration's region.
        loci: Vec<Location>,
    },
    /// `runtime_invariant` / `established-invariant-broken` (FR-094
    /// "Refusals", ADR-013 T-4): keying a node needed a fact that type
    /// admission, intake or the check stage's own unit scope guarantees, and
    /// it was not there. A fault in `check`, not in the input; the node gets
    /// no key. Boxed: a fault names a `DeclarationKey` or a `UnitId`, larger
    /// than every other cause.
    InternalFault(Box<KeyFault>),
    /// `invalid_package`: a node preimage that cannot be encoded (an empty
    /// or non-identifier name, a number RFC 8785 cannot render exactly, a
    /// body nested past the preimage depth bound).
    NodePreimage(super::node_key::NodeKeyRefusal),
}

/// The invariant [`CheckCause::InternalFault`] names, with the value that
/// broke it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum KeyFault {
    /// A `Reference<T>`'s `EffectiveId` is no `type_identities` value of an
    /// admitted domain package's effective view.
    UnknownEffectiveId(EffectiveId),
    /// A `DeclarationKey`'s `package` is no admitted domain package's
    /// identity.
    UnadmittedPackage(crate::model::key::DeclarationKey),
    /// A `DeclarationKey`'s `node` is empty.
    EmptyNode(crate::model::key::DeclarationKey),
    /// A `DeclarationKey` names no record of its admitted domain package.
    UnknownDeclaration(crate::model::key::DeclarationKey),
    /// A domain package record of a kind no checked node names.
    UnnamedRecordKind(crate::model::key::DeclarationKey),
    /// A compound `UnitId` the check stage's unit scope does not hold.
    UnheldUnit(quire_exact::UnitId),
    /// A `Population<T>[N]`-typed node whose object type `T` no population
    /// binding names.
    UntargetedPopulation,
    /// A literal of a value kind the `Value` family's checker never builds
    /// as a literal.
    UnbuiltLiteral,
    /// A `ValueType::Composite` handle names no declaration of the check's
    /// type environment.
    UnknownComposite(quire_exact::NodeKey),
    /// A recursion group handed to the group keying is empty, repeats a
    /// member, or names no member at a group position.
    InvalidGroup,
    /// A node names a draft that dependency order never keyed.
    UnresolvedDraft,
    /// A record value's slots and its declaration's fields differ in
    /// number.
    RecordSlotCount,
    /// An IEEE arithmetic node whose left operand is not an IEEE type.
    UntypedIeeeOperand,
    /// Two admitted domain packages share this identity: a check selects
    /// one version of each (FR-094, ADR-013 O-01).
    DuplicateModelSelection(String),
    /// An admitted enum declaration or member whose nominal preimage has no
    /// RFC 8785 encoding.
    NonCanonicalNominal(quire_exact::NodeKey),
}

impl KeyFault {
    /// The violated invariant's stable identifier (ADR-013 T-4).
    pub fn invariant(&self) -> &'static str {
        match self {
            Self::UnknownEffectiveId(_) => "reference-target-admitted",
            Self::UnadmittedPackage(_) => "declaration-package-admitted",
            Self::EmptyNode(_) => "declaration-node-nonempty",
            Self::UnknownDeclaration(_) => "declaration-record-present",
            Self::UnnamedRecordKind(_) => "record-kind-named",
            Self::UnheldUnit(_) => "compound-unit-held",
            Self::UntargetedPopulation => "population-target-bound",
            Self::UnbuiltLiteral => "literal-kind-built",
            Self::UnknownComposite(_) => "composite-declared",
            Self::InvalidGroup => "recursion-group-well-formed",
            Self::UnresolvedDraft => "draft-keyed-in-dependency-order",
            Self::RecordSlotCount => "record-slots-match-fields",
            Self::UntypedIeeeOperand => "ieee-operand-typed",
            Self::DuplicateModelSelection(_) => "one-model-version-per-identity",
            Self::NonCanonicalNominal(_) => "nominal-preimage-canonical",
        }
    }
}

/// Which dispatch-table function slot [`InvalidDispatchDeclaration`] names.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DispatchFunctionRole {
    /// A candidate's own operation body.
    Body,
    /// A candidate's own runtime-evaluated effective precondition.
    Precondition,
    /// A precondition clause reachable from a candidate's static FR-146
    /// redefinition ancestry.
    PreconditionClause,
}

/// The typed detail of [`CheckCause::InvalidDispatchDeclaration`]: what
/// exactly is malformed about a supplied [`super::DispatchTable`] or
/// [`super::DispatchOperation`], with the actual field/index path
/// (`native-diagnostics.md`'s `invalid_package`/`invalid-value` row) rather
/// than a free-form summary.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum InvalidDispatchDeclaration {
    /// A `NodeKind::Dispatch` node's own `operation` index names no
    /// operation in the package's own `dispatch_operations`.
    OperationOutOfRange {
        /// The out-of-range operation index.
        operation: usize,
    },
    /// A dispatch operation names a `table` index past the package's own
    /// `dispatch_tables`.
    TableOutOfRange {
        /// The dispatch operation's own member name.
        member: String,
        /// The out-of-range table index.
        table: usize,
    },
    /// A dispatch-table function index names no function in the package's
    /// own `functions`.
    FunctionOutOfRange {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The out-of-range function index.
        index: usize,
    },
    /// A candidate function's declared parameter count does not match the
    /// dispatch operation's own arity (the receiver plus every declared
    /// argument).
    Arity {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
        /// The function's own declared parameter count.
        declared: usize,
        /// The receiver plus the dispatch operation's own argument count.
        expected: usize,
    },
    /// A candidate function's parameter types (after the receiver) do not
    /// match the dispatch operation's declared argument types.
    ParameterType {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
    },
    /// A candidate function's declared result type does not match the
    /// result its role requires (the operation's own result for a body,
    /// `Boolean` for a precondition or precondition clause).
    ResultType {
        /// Which slot this index was read from.
        role: DispatchFunctionRole,
        /// The function index, valid in `functions` but wrongly shaped.
        index: usize,
    },
    /// Two dispatch operations in the package's own `dispatch_operations`
    /// share a `(receiver_type, member)` pair. A well-formed package exposes
    /// each static receiver type/member name once; a duplicate means two
    /// bridge calls (see `crate::check::checked_dispatch`) were merged for
    /// overlapping roots, and name lookup at a call site would otherwise
    /// silently pick whichever entry happens to come first.
    DuplicateOperation {
        /// The receiver type both operations declare, by its effective
        /// identity.
        receiver_type: EffectiveId,
        /// The shared member name.
        member: String,
    },
    /// A [`super::ResolvedSignatures`] entry is keyed by an index past the
    /// package's own `functions`, so it stands in for no declaration.
    ResolvedSignatureOutOfRange {
        /// The out-of-range function index.
        index: usize,
    },
    /// A `PackageDeclarations::model_clauses` entry is keyed by an index
    /// past the package's own `functions`, so it owns no clause function.
    ModelClauseOutOfRange {
        /// The out-of-range function index.
        index: usize,
    },
}

impl CheckCause {
    /// The refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::IllTyped(_) => Code::IllTyped,
            Self::WrongSnapshot(_) => Code::WrongSnapshot,
            Self::MissingName(_) => Code::MissingDeclaration,
            Self::AmbiguousName { .. } => Code::AmbiguousDeclaration,
            Self::Unproved(_) | Self::UnprovedDecrease { .. } => Code::UndefinedExpression,
            Self::ResourceExhausted(_) => Code::StageLimitExceeded,
            Self::IeeeProfileNotAdmitted
            | Self::DefinitionCycle { .. }
            | Self::InvalidDispatchDeclaration(_)
            | Self::NodePreimage(_) => Code::InvalidPackage,
            Self::UnrepresentableBound => Code::UnrepresentableConstraint,
            Self::MissingSelection { .. } => Code::MissingDeclaration,
            Self::UnsupportedFeature { .. } => Code::UnknownRequiredFeature,
            Self::InternalFault(_) => Code::RuntimeInvariant,
        }
    }

    /// The closed cause tag, where the code has one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::IllTyped(cause) => cause.tag(),
            Self::WrongSnapshot(cause) => Some(cause.as_str()),
            Self::MissingName(_) => Some("missing-name"),
            Self::AmbiguousName { .. } => Some("ambiguous-name"),
            Self::Unproved(Obligation::Nonzero) => Some("unproved-nonzero"),
            Self::Unproved(Obligation::Presence) => Some("unproved-presence"),
            Self::Unproved(
                Obligation::Range { .. }
                | Obligation::RationalRange
                | Obligation::NonemptyReduction { .. },
            ) => Some("unproved-range"),
            Self::UnprovedDecrease { .. } => Some("unproved-decrease"),
            Self::ResourceExhausted(cause) => Some(cause.kind.foundation_kind().catalog_cause()),
            Self::DefinitionCycle { .. } => Some("definition-cycle"),
            Self::InvalidDispatchDeclaration(_) => Some("invalid-value"),
            Self::MissingSelection { .. } => Some("missing-selection"),
            Self::UnsupportedFeature { .. } => Some("unsupported-feature"),
            Self::InternalFault(_) => Some("established-invariant-broken"),
            Self::IeeeProfileNotAdmitted | Self::UnrepresentableBound | Self::NodePreimage(_) => {
                None
            }
        }
    }
}

/// A located checking refusal, made before any charge.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("{} at {location:?}: {cause:?}", cause.code().as_str())]
pub struct CheckRefusal {
    /// Where the refusal originates.
    pub location: Location,
    /// The typed cause.
    pub cause: CheckCause,
}

impl CheckRefusal {
    pub(crate) fn ill_typed(location: &Location, cause: IllTypedCause) -> Self {
        Self {
            location: location.clone(),
            cause: CheckCause::IllTyped(cause),
        }
    }

    /// A kernel type refusal as its FR-272 cause: an unordered enum ordering
    /// is `operator-ineligible`, every other incompatibility `type-mismatch`.
    pub(crate) fn from_ill_typed(location: &Location, refusal: IllTyped) -> Self {
        let cause = match refusal.cause {
            IllTypedCause::TypeMismatch
            | IllTypedCause::OperatorIneligible
            | IllTypedCause::AmbiguousLiteral => refusal.cause,
            IllTypedCause::UnorderedEnumOrdering => IllTypedCause::OperatorIneligible,
            IllTypedCause::DistinctTextProfiles
            | IllTypedCause::DistinctEnumDeclarations
            | IllTypedCause::IncompatibleDimensions
            | IllTypedCause::AffineUnitArithmetic
            | IllTypedCause::DistinctUnits
            | IllTypedCause::MalformedDecimalType
            | IllTypedCause::DistinctIeeeWidths
            | IllTypedCause::IeeeWithExactOperand
            | IllTypedCause::IeeeToNonRationalExact => IllTypedCause::TypeMismatch,
        };
        Self::ill_typed(location, cause)
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckCause, CheckingLimitKind, CheckingStage, StageLimitCause};
    use qsl_foundation::diagnostic::Code;

    /// QSL-236: every `CheckingLimitKind` reports `stage_limit_exceeded`
    /// with its own `<kind>-exceeded` cause, and carries the bound and
    /// actual counter it was built with.
    #[test]
    fn resource_exhausted_reports_stage_limit_exceeded_per_kind() {
        let cases = [
            (CheckingLimitKind::Nodes, "node-count-exceeded"),
            (CheckingLimitKind::Depth, "nesting-depth-exceeded"),
            (CheckingLimitKind::InputBytes, "input-bytes-exceeded"),
            (CheckingLimitKind::WorkBudget, "work-budget-exceeded"),
        ];
        for (kind, cause) in cases {
            let refused = CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                stage: CheckingStage::Typing,
                kind,
                limit: 10,
                actual: 11,
            }));
            assert_eq!(refused.code(), Code::StageLimitExceeded, "{kind:?}");
            assert_eq!(refused.cause(), Some(cause), "{kind:?}");
            let CheckCause::ResourceExhausted(exceeded) = refused else {
                unreachable!()
            };
            assert_eq!(exceeded.limit, 10);
            assert_eq!(exceeded.actual, 11);
        }
    }
}
