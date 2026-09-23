// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located checking refusals and their closed codes and causes.

use qsl_foundation::diagnostic::Code;
use quire_exact::Integer;
use quire_exact::NodeKey;
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
}

/// A located expression: its declaration and the child-index path from that
/// declaration's root expression.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Location {
    /// The owning declaration.
    pub origin: Origin,
    /// Child indices from the root, as [`Expression::children`] numbers them.
    ///
    /// [`Expression::children`]: crate::forms::Expression::children
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
    /// `crate::family::StageLimitKind::InputBytes`).
    InputBytes,
    /// A checked-family declaration's own preimage field-write count
    /// (QSL-153, `crate::family::StageLimitKind::WorkBudget`).
    WorkBudget,
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
}

impl WrongSnapshotCause {
    /// The closed FR-272 cause tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WrongAnchor => "wrong-anchor",
            Self::ForbiddenPreRead => "forbidden-pre-read",
        }
    }
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
    /// `resource_exhausted` / `insufficient-next-charge`: a declared checking
    /// limit was reached at this node.
    ResourceExhausted {
        /// The checking stage.
        stage: CheckingStage,
        /// The limit reached.
        kind: CheckingLimitKind,
        /// The declared limit.
        limit: u64,
    },
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
    /// tag rather than minting a new one.
    InvalidDispatchDeclaration(InvalidDispatchDeclaration),
    /// `invalid_package` / `invalid-value`: a supplied `PackageDeclarations::
    /// model_correspondence` entry repeats a `NodeKey` an earlier entry
    /// already named (PR #300 review round 2, MEDIUM-3). The same "built by
    /// the caller (the `model` bridge); the checker only records/resolves
    /// against it" division as `dispatch_operations`/`dispatch_tables`
    /// above, validated the same way and reusing the same already-catalogued
    /// `invalid-value` tag, not a new one.
    ///
    /// This crate does not yet validate that a correspondence entry's
    /// `NodeKey` names a node this package's own checking actually admitted
    /// (the review's own "refuse or diagnose nodes not in the graph"):
    /// `model_correspondence`'s keys are checked relation/model node keys
    /// (`FrameSubjects`'s own doc), a node category with no check-stage
    /// producer yet (FR-340 frame syntax, FR-088-CON-2, `Remaining work:
    /// #210`) -- validating against `type_nodes`/function identities, the
    /// only node categories `check` can enumerate today, would refuse every
    /// legitimate future frame-subject entry as readily as a genuinely
    /// malformed one, since neither category is the one a real entry names.
    InvalidModelCorrespondence(InvalidModelCorrespondence),
}

/// [`CheckCause::InvalidModelCorrespondence`]'s own typed detail.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum InvalidModelCorrespondence {
    /// Two entries name the same `NodeKey`. ADR-013 O-04's correspondence is
    /// one node id to one `DeclarationKey`; a second entry for a node
    /// already recorded is refused rather than silently overwriting the
    /// first (R-05: no entry's arrival order is load-bearing).
    DuplicateNode {
        /// The repeated node key.
        node: quire_exact::NodeKey,
    },
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
        /// The receiver type both operations declare.
        receiver_type: NodeKey,
        /// The shared member name.
        member: String,
    },
    /// A [`super::ResolvedSignatures`] entry is keyed by an index past the
    /// package's own `functions`, so it stands in for no declaration.
    ResolvedSignatureOutOfRange {
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
            Self::ResourceExhausted { .. } => Code::ResourceExhausted,
            Self::IeeeProfileNotAdmitted
            | Self::DefinitionCycle { .. }
            | Self::InvalidDispatchDeclaration(_)
            | Self::InvalidModelCorrespondence(_) => Code::InvalidPackage,
            Self::UnrepresentableBound => Code::UnrepresentableConstraint,
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
            Self::ResourceExhausted { .. } => Some("insufficient-next-charge"),
            Self::DefinitionCycle { .. } => Some("definition-cycle"),
            Self::InvalidDispatchDeclaration(_) | Self::InvalidModelCorrespondence(_) => {
                Some("invalid-value")
            }
            Self::IeeeProfileNotAdmitted | Self::UnrepresentableBound => None,
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
