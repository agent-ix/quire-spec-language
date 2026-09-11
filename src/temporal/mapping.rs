// SPDX-License-Identifier: AGPL-3.0-only
//! FR-045: native-to-TL mapping support classification.
//!
//! This is a total function of three inputs and nothing else: the admitted
//! declaration's selected profile, its reachable operator kinds, and the
//! surrounding-execution closure named in the request. Decision-scope closure is
//! a separate axis and is never substituted for it.
//! It consults no backend capability report, no installed TL version, no syntax
//! match and no historical result. It emits no TL formula, no valuation request
//! and no correspondence record: the emission half of the bridge remains blocked
//! on `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL capability.

use crate::protocol_artifact::wire as w;

use super::profile::Profile;
use super::trace::Closure;

/// Reviewed correspondence source this table restates.
pub const SUPPORT_TABLE: &str =
    "quire-specification/FR-095 @ 4d6230eb8aa9766ff3017360962f2d6368d74cb3";

/// A TL target named by the reviewed support table.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Target {
    /// `mltl.closed-trace/v1`.
    ClosedTrace,
    /// `mltl.online-prefix/v1`.
    OnlinePrefix,
}

impl Target {
    /// Exact TL profile identity named by the reviewed table.
    pub fn identity(self) -> &'static str {
        match self {
            Self::ClosedTrace => "mltl.closed-trace/v1",
            Self::OnlinePrefix => "mltl.online-prefix/v1",
        }
    }
}

/// A semantic dimension the current TL targets cannot express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unmatched {
    /// The finite-window boundary rule; index-based TL profiles extend atoms
    /// false beyond closure.
    FiniteWindow,
    /// Any bounded past operator, until a separately reviewed TL past profile
    /// exists.
    PastOperator,
}

/// Premises a supported mapping would still have to discharge. Naming them is
/// not discharging them, and this crate discharges none of them.
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

/// The classification of one mapping request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Support {
    Supported {
        target: Target,
        /// Reviewed source table this disposition was read from, so a later
        /// revision of that table is a visible change rather than silent
        /// staleness.
        table: &'static str,
        /// Extra condition the fixed-sample row attaches, when it applies.
        total_sample_valuation: bool,
        premises: &'static [&'static str],
    },
    Unsupported {
        /// Every unmatched dimension, not one summary cause.
        dimensions: Vec<Unmatched>,
    },
}

/// Which operator kinds a declaration's temporal graph reaches.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Operators {
    pub future: bool,
    pub past: bool,
}

impl Operators {
    /// Walk the emitted temporal arena. Every node is inspected once; the graph
    /// is a flat arena, so no traversal is required.
    pub fn of(nodes: &[w::Temporal]) -> Self {
        let mut result = Self::default();
        for node in nodes {
            match &node.operation {
                w::TemporalOperation::Unary { operator, .. } => match operator {
                    w::TemporalUnary::Eventually | w::TemporalUnary::Always => result.future = true,
                    w::TemporalUnary::Once | w::TemporalUnary::Historically => result.past = true,
                    w::TemporalUnary::Not => {}
                },
                w::TemporalOperation::Binary { operator, .. } => match operator {
                    w::TemporalBinary::Until | w::TemporalBinary::Release => result.future = true,
                    w::TemporalBinary::Since | w::TemporalBinary::Triggered => result.past = true,
                    w::TemporalBinary::And | w::TemporalBinary::Or | w::TemporalBinary::Implies => {
                    }
                },
                w::TemporalOperation::Constant { .. }
                | w::TemporalOperation::Holds { .. }
                | w::TemporalOperation::Group { .. } => {}
            }
        }
        result
    }
}

/// Classify one mapping request against the reviewed support table.
///
/// Unmatched dimensions accumulate: a bounded past operator under the
/// finite-window profile names both.
pub fn classify(profile: Profile, operators: Operators, surrounding_execution: Closure) -> Support {
    let mut dimensions = Vec::new();
    if profile == Profile::TimestampedWindow {
        dimensions.push(Unmatched::FiniteWindow);
    }
    if operators.past {
        dimensions.push(Unmatched::PastOperator);
    }
    if !dimensions.is_empty() {
        return Support::Unsupported { dimensions };
    }
    Support::Supported {
        target: match surrounding_execution {
            Closure::Closed => Target::ClosedTrace,
            Closure::Open => Target::OnlinePrefix,
        },
        table: SUPPORT_TABLE,
        total_sample_valuation: profile == Profile::FixedSample,
        premises: OUTSTANDING_PREMISES,
    }
}
