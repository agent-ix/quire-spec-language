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

/// The typed cause of a checking refusal.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CheckCause {
    /// `refused { code: ill_typed }` with its FR-272 cause.
    IllTyped(IllTypedCause),
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
    /// `invalid_package` / `invalid-value` (finding #172-4): a supplied
    /// [`super::DispatchTable`] or [`super::DispatchOperation`] is
    /// malformed — an out-of-range table or function index, or a candidate
    /// whose signature does not match its dispatch operation's declared
    /// arity or types. Checked upfront in
    /// [`super::PackageDeclarations::check`], before any node is typed, so
    /// [`super::facts`]'s own call-graph walk can treat every table index it
    /// reads as already valid; reuses the already-catalogued `invalid-value`
    /// tag rather than minting a new one.
    InvalidDispatchDeclaration {
        /// What was malformed.
        detail: String,
    },
}

impl CheckCause {
    /// The refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::IllTyped(_) => Code::IllTyped,
            Self::MissingName(_) => Code::MissingDeclaration,
            Self::AmbiguousName { .. } => Code::AmbiguousDeclaration,
            Self::Unproved(_) | Self::UnprovedDecrease { .. } => Code::UndefinedExpression,
            Self::ResourceExhausted { .. } => Code::ResourceExhausted,
            Self::IeeeProfileNotAdmitted
            | Self::DefinitionCycle { .. }
            | Self::InvalidDispatchDeclaration { .. } => Code::InvalidPackage,
            Self::UnrepresentableBound => Code::UnrepresentableConstraint,
        }
    }

    /// The closed cause tag, where the code has one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::IllTyped(cause) => cause.tag(),
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
            Self::InvalidDispatchDeclaration { .. } => Some("invalid-value"),
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
