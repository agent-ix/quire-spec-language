// SPDX-License-Identifier: AGPL-3.0-or-later
//! Located checking refusals and their closed codes and causes.

use super::super::comparison::{IllTyped, IllTypedCause};
use super::super::integer::Integer;
use crate::diagnostic::Code;

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
    /// [`Expression::children`]: super::Expression::children
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
}

/// FR-272's closed `wrong_snapshot` cause list this crate decides for
/// `pre(...)` (native-diagnostics.md: `wrong-observation`, `wrong-invocation`,
/// `wrong-anchor` or `forbidden-pre-read`). Only the two causes a `pre(...)`
/// checking refusal can actually name are represented; `wrong-observation`
/// and `wrong-invocation` are catalogued for the native runtime's own
/// snapshot-selection refusals (`src/runtime`), not this checker.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum WrongSnapshotCause {
    /// `pre(...)` written where the checked declaration does not admit it
    /// (anywhere but an operation's postcondition) -- shared-grammar.md's
    /// "self, result and pre(...) are caller-side anchor operations,
    /// unavailable as implicit ambient state inside a reusable predicate",
    /// generalized to every non-postcondition clause kind FR-042 refuses
    /// `pre(...)` in (invariants, preconditions, a plain function body).
    WrongAnchor,
    /// `pre(...)`'s own operand has no eligible state read for it to anchor
    /// (FR-042's Behavior clause: "Pre is refused ... on bare parameters/
    /// constants/captures"): a bare `Name`, literal, or a value already
    /// captured/computed outside this `pre(...)`'s own operand (so
    /// re-anchoring it would silently do nothing rather than replay it under
    /// the pre observation).
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
            Self::IeeeProfileNotAdmitted => Code::InvalidPackage,
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
