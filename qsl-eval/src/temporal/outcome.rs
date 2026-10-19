// SPDX-License-Identifier: AGPL-3.0-or-later
//! What the temporal evaluator returns: an outcome that settles the clause
//! on the trace, and the typed refusals of a trace it cannot read.

use qsl_foundation::diagnostic::Category;
use qsl_foundation::{CatalogCode, InternalFault};
use qsl_semantics::check::TemporalProfile;
use quire_exact::Incomplete;
use quire_semantic_value::fairness::FairnessConstraint;

use super::letters::UndefinedCause;
use super::position::{PositionRefusal, TemporalPosition};
use super::window::{WindowLimitReached, WindowRefusal};

/// A claim that is not defined on the trace (ADR-018 UE-2): where it first
/// reads an undefined atom, and why.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedEvaluation {
    /// The first position at which an atom the clause reads is undefined.
    pub at: TemporalPosition,
    /// The first undefined atom at `at`, in clause-node order.
    pub cause: UndefinedCause,
}

/// The settlement of a clause on one trace (FR-327 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalOutcome {
    /// `Completed(true)`: every activation evaluated true. Evidence for this
    /// trace only (`tested`), never `proved` (ADR-014 section 8).
    Holds,
    /// `Completed(false)`: the first activation, in position order, that
    /// evaluated false.
    Violated {
        /// The position of that activation.
        at: TemporalPosition,
    },
    /// The claim is not defined on the trace; it does not hold on it.
    Undefined(UndefinedEvaluation),
    /// A finite prefix that neither shows a bad prefix nor can show more
    /// (FR-328): O-16 inconclusive.
    Pending,
    /// The clause has a fairness set and the trace carries no enabledness
    /// to check it against (FR-328, FR-329): O-16 unsupported, wire cause
    /// `unsupported_projection`/`missing-fairness-premise`.
    MissingFairnessPremise {
        /// The first constraint of the checked fairness set.
        constraint: FairnessConstraint,
    },
    /// The work budget ran out; no truth value (ADR-014 TR-5).
    Incomplete(Incomplete),
    /// A window aggregate outgrew `max_window_values` (FR-194; ADR-018
    /// V-7); no truth value.
    WindowLimitReached(WindowLimitReached),
}

impl TemporalOutcome {
    /// The ADR-013 O-16 category of this outcome (FR-327 "Results").
    pub fn category(&self) -> Category {
        match self {
            Self::Holds => Category::Success,
            Self::Undefined(UndefinedEvaluation {
                cause: UndefinedCause::InsufficientData(_),
                ..
            }) => Category::Undefined,
            Self::Violated { .. } | Self::Undefined(_) => Category::Violation,
            Self::Pending => Category::Inconclusive,
            Self::MissingFairnessPremise { .. } => Category::Unsupported,
            Self::Incomplete(_) | Self::WindowLimitReached(_) => Category::Incomplete,
        }
    }
}

/// Why a trace cannot be read at all.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalRefusal {
    /// A finite trace holds no observation (FR-327 "Inputs").
    #[error("a finite trace holds at least one observation")]
    EmptyTrace,
    /// A lasso with an empty loop (FR-329 "Lasso shape").
    #[error("a lasso holds at least one loop position")]
    EmptyLoop,
    /// A lasso's prefix and loop together exceed `u64` positions.
    #[error("a lasso's prefix and loop exceed u64 positions")]
    TraceTooLong,
    /// A lasso under a bounded profile: only the infinite-trace profile
    /// reads one (FR-329).
    #[error("a lasso is read under the infinite-trace profile only, not {profile:?}")]
    LassoUnderBoundedProfile {
        /// The clause's profile.
        profile: TemporalProfile,
    },
    /// The profile reads timestamped instants, which a position trace does
    /// not carry.
    #[error("{profile:?} reads timestamped instants, which this trace does not carry")]
    ProfileNotEvaluated {
        /// The clause's profile.
        profile: TemporalProfile,
    },
    /// A position is outside the represented trace (FR-327 "Positions").
    #[error(transparent)]
    Position(#[from] PositionRefusal),
    /// A window aggregate cannot be read on the trace.
    #[error(transparent)]
    Window(#[from] WindowRefusal),
    /// An invariant of the checked clause or the evaluator broke.
    #[error("temporal evaluation broke invariant {0:?}")]
    Fault(InternalFault),
}

impl TemporalRefusal {
    /// The catalog code of this refusal: `invalid_runtime_input`/`invalid-value`
    /// for a malformed trace or position (FR-327, FR-329),
    /// `unsupported_construct`/`expression-form` for a profile or trace shape
    /// the evaluator does not read, and
    /// `runtime_invariant`/`established-invariant-broken` for a fault.
    pub fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::EmptyTrace | Self::EmptyLoop | Self::TraceTooLong | Self::Window(_) => {
                CatalogCode::new("invalid_runtime_input", "invalid-value")
            }
            Self::LassoUnderBoundedProfile { .. } | Self::ProfileNotEvaluated { .. } => {
                CatalogCode::new("unsupported_construct", "expression-form")
            }
            Self::Position(refusal) => refusal.catalog_code(),
            Self::Fault(fault) => fault.catalog_code(),
        }
    }
}
