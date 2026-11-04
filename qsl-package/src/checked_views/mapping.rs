// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-527: native-to-TL mapping support classification over the checked
//! temporal clause.
//!
//! The disposition is a total function of the clause's selected profile, the
//! operator kinds in the view's flat `formula()` list and the
//! surrounding-execution closure. The classifier reads nothing else: no
//! backend capability report, no TL version, no formula bytes and no earlier
//! result. `formula()` lists the `quire.op.temporal.*` applications of the
//! clause, so every listed operator is reachable from the clause root and the
//! connectives `not`, `and`, `or` and `implies` carry no direction.

use qsl_semantics::check::TemporalProfile;
use quire_contract_model::{CheckedArtifactRef, CheckedNodeId, CheckedSelection};

use super::subject::{Activation, OperatorDirection, TemporalSubjectView};

/// The surrounding-execution closure a mapping request names. It selects the
/// TL row of FR-045's future rows; decision-scope closure is a separate axis
/// this function does not take.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Closure {
    /// The surrounding execution is complete: a closed trace. The legacy
    /// `temporal::Closure` spells this `Closed`; the Contract IR repoint maps it.
    Complete,
    /// The surrounding execution is an open prefix.
    Open,
}

/// A TL target named by the reviewed support table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// `mltl.closed-trace/v1`.
    ClosedTrace,
    /// `mltl.online-prefix/v1`.
    OnlinePrefix,
    /// `mltl.origin-complete-history/v1`: every bounded past operator alone,
    /// under either false-extension profile, whatever the surrounding closure.
    OriginCompleteHistory,
}

impl Target {
    /// The exact TL profile identity the table names.
    pub const fn identity(self) -> &'static str {
        match self {
            Self::ClosedTrace => "mltl.closed-trace/v1",
            Self::OnlinePrefix => "mltl.online-prefix/v1",
            Self::OriginCompleteHistory => "mltl.origin-complete-history/v1",
        }
    }
}

/// A semantic dimension the current TL targets cannot express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unmatched {
    /// The timestamped-event finite window; index-based TL profiles extend
    /// atoms false beyond closure.
    FiniteWindow,
    /// A bounded past and a bounded future operator both reachable; no TL
    /// profile evaluates both directions in one formula.
    MixedDirection,
    /// The infinite-trace profile has no finite-trace TL target.
    InfiniteTrace,
    /// Exact rational-time semantics (`quire.temporal.timed/v1`) have no
    /// target in the selected TL table.
    TimedProfile,
}

/// Premises a supported mapping would still have to discharge. Naming them is
/// not discharging them.
pub const OUTSTANDING_PREMISES: &[&str] = &[
    "total Boolean predicate projection",
    "source and clause identity",
    "model, type and predicate bindings",
    "evaluation anchor and immutable capture environment",
    "clock and observation binding",
    "interval",
    "closure and history premises",
    "result dimensions the selected TL wire does not encode",
];

/// The disposition of one mapping request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Support {
    /// The reviewed table names a TL target for this request.
    Supported {
        /// The TL target this request maps to.
        target: Target,
        /// The condition FR-045's fixed-sample future row attaches, and no
        /// other row.
        total_sample_valuation: bool,
        /// Premises a supported mapping would still have to discharge.
        premises: &'static [&'static str],
    },
    /// No current TL target can express this request.
    Unsupported {
        /// Every unmatched dimension, not one summary cause.
        dimensions: Vec<Unmatched>,
    },
}

/// What a classification retains from the native clause. Nothing is
/// substituted or reduced: these are the view's own values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Retained {
    /// The clause node.
    pub declaration: CheckedNodeId,
    /// The lock row of the selected temporal profile.
    pub profile: CheckedSelection,
    /// The clause's activation.
    pub activation: Activation,
}

/// The definition selection the package authenticates for one clause.
///
/// It names the package, the clause and the lock's `profile_selections` row
/// the clause's `temporal_profile` law selects. It carries no clock
/// parameters: classification takes no trace, so it authenticates no
/// parameter map. Only the classifier constructs one; its fields are private:
///
/// ```compile_fail,E0451
/// use qsl_package::checked_views::AuthenticatedSelection;
/// fn forge(clause: quire_contract_model::CheckedNodeId) -> AuthenticatedSelection {
///     AuthenticatedSelection {
///         package: "forged".into(),
///         clause,
///         definition: quire_contract_model::CheckedArtifactRef {
///             authority: "a".into(),
///             identity: "i".into(),
///         },
///     }
/// }
/// ```
///
/// It has no constructor function either:
///
/// ```compile_fail,E0599
/// use qsl_package::checked_views::AuthenticatedSelection;
/// fn forge(clause: quire_contract_model::CheckedNodeId) -> AuthenticatedSelection {
///     AuthenticatedSelection::new("forged", clause)
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedSelection {
    package: Box<str>,
    clause: CheckedNodeId,
    definition: CheckedArtifactRef,
}

impl AuthenticatedSelection {
    /// The identity (name) of the package the selection was read from.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The clause node the selection is for.
    pub fn clause(&self) -> &CheckedNodeId {
        &self.clause
    }

    /// The registered temporal definition identity the row selects.
    pub fn definition_identity(&self) -> &str {
        &self.definition.identity
    }

    /// The definition artifact the row selects: authority and identity.
    pub fn definition_artifact(&self) -> &CheckedArtifactRef {
        &self.definition
    }
}

/// One classification: its disposition, the native subject it retains and the
/// selection the package authenticates for the clause.
///
/// `support` and `retained` are plain data. `selection` is sealed, so a
/// consumer that needs the disposition bound to it re-classifies from the
/// package the selection names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Classification {
    /// The disposition read from the reviewed support table.
    pub support: Support,
    /// The retained native subject, profile and activation.
    pub retained: Retained,
    selection: AuthenticatedSelection,
}

impl Classification {
    /// The selection the package authenticates for the clause, retained on
    /// every result, supported or not.
    pub fn selection(&self) -> &AuthenticatedSelection {
        &self.selection
    }
}

/// Why a clause was not classified.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MappingRefusal {
    /// The view's `profile_selections` row names a definition identity that
    /// is none of the five temporal profiles the `TemporalTrace` family
    /// admits. A package with no row at all is refused by the reader, so it
    /// never reaches here.
    #[error("the lock selects no temporal profile for the clause")]
    NoProfileSelection,
}

/// Classify the clause `subject` against FR-045's support table.
///
/// Unmatched dimensions accumulate: a mixed-direction request under the
/// finite-window profile names both. A past-only request under a
/// false-extension profile maps to the origin-complete-history target
/// whatever the closure; whether the history is complete is a bridge premise.
///
/// Refuses [`MappingRefusal::NoProfileSelection`] when the clause's profile row
/// names none of the five admitted temporal profiles.
///
/// Only the surrounding-execution closure is taken; a decision-scope closure
/// is a different type and does not compile here:
///
/// ```compile_fail,E0308
/// use qsl_package::checked_views::{mapping_support, TemporalSubjectView};
/// enum DecisionScope {
///     Complete,
///     Open,
/// }
/// fn call(view: &TemporalSubjectView<'_>) {
///     let _ = mapping_support(view, DecisionScope::Open);
/// }
/// ```
pub fn mapping_support(
    subject: &TemporalSubjectView<'_>,
    surrounding: Closure,
) -> Result<Classification, MappingRefusal> {
    let profile = TemporalProfile::from_identity(&subject.profile().definition.identity)
        .ok_or(MappingRefusal::NoProfileSelection)?;
    let (mut future, mut past) = (false, false);
    for used in subject.formula_graph() {
        match used.direction() {
            Some(OperatorDirection::Future) => future = true,
            Some(OperatorDirection::Past) => past = true,
            None => {}
        }
    }
    Ok(Classification {
        support: support(profile, future, past, surrounding),
        retained: Retained {
            declaration: subject.declaration().clone(),
            profile: subject.profile().clone(),
            activation: subject.activation().clone(),
        },
        selection: AuthenticatedSelection {
            package: subject.package().name().into(),
            clause: subject.declaration().clone(),
            definition: subject.profile().definition.clone(),
        },
    })
}

/// FR-045's table, in row order.
fn support(profile: TemporalProfile, future: bool, past: bool, surrounding: Closure) -> Support {
    let mut dimensions = Vec::new();
    match profile {
        TemporalProfile::InfiniteTrace => dimensions.push(Unmatched::InfiniteTrace),
        TemporalProfile::TimestampedEventFiniteWindow => dimensions.push(Unmatched::FiniteWindow),
        TemporalProfile::Timed => dimensions.push(Unmatched::TimedProfile),
        TemporalProfile::EventPositionFalseExtension
        | TemporalProfile::FixedSampleFalseExtension => {}
    }
    if past && future {
        dimensions.push(Unmatched::MixedDirection);
    }
    if !dimensions.is_empty() {
        return Support::Unsupported { dimensions };
    }
    if past {
        return Support::Supported {
            target: Target::OriginCompleteHistory,
            total_sample_valuation: false,
            premises: OUTSTANDING_PREMISES,
        };
    }
    Support::Supported {
        target: match surrounding {
            Closure::Complete => Target::ClosedTrace,
            Closure::Open => Target::OnlinePrefix,
        },
        total_sample_valuation: profile == TemporalProfile::FixedSampleFalseExtension && future,
        premises: OUTSTANDING_PREMISES,
    }
}
