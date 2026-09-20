// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043: the caller-supplied observation trace.
//!
//! Agent F owns observation transport, storage, replay and completeness
//! authority. Everything here is a caller-constructed input. The completeness
//! assertion, the admitted order and the authoritative-origin claim are trusted
//! as supplied and retained as premises; they are not verified by this
//! evaluator, so a later contradiction can identify the results that depended
//! on them.

use std::collections::BTreeMap;

pub use super::result::{Closure, Completeness, Execution};

/// The clock binding a trace asserts, checked against the declaration's
/// admitted definition entry and emitted clock binding name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockBinding {
    /// Emitted clock binding name, without the `clock:` requirement prefix.
    pub name: String,
    /// Asserted registered temporal profile identity.
    pub profile_identity: String,
    /// Asserted registered temporal profile revision.
    pub profile_revision: String,
    /// Declared clock parameters this profile requires — sample period, epoch
    /// and unit, timestamp unit, or sequence authority. The emitted body
    /// carries none of them, so they are retained premises that participate in
    /// result identity rather than values this evaluator can authenticate.
    /// Recorded as remaining work on compiler #38.
    pub parameters: BTreeMap<String, String>,
}

/// An admitted order over positions sharing a clock coordinate. Two keys from
/// different authorities are not comparable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderKey {
    /// Exact admitted order authority; never an ingestion or transport label.
    pub authority: String,
    /// Order value within the authority's key space.
    pub key: i64,
}

/// One admitted trace position.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Position {
    /// Clock coordinate in the selected profile's domain: an event position, a
    /// sample offset, or a timestamp in the declared unit.
    pub coordinate: i64,
    /// Admitted causal or sequence order, where the authority supplies one.
    pub order: Option<OrderKey>,
    /// Valuation for each `holds` leaf, keyed by its index in the
    /// declaration's temporal arena. A leaf absent from this map at a required
    /// position is a missing valuation, never a false one.
    pub valuations: BTreeMap<u32, bool>,
}

/// Whether trigger evidence was admitted, absent or refused. Absent and refused
/// are distinct dispositions and neither is an inactive scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Evidence {
    /// Trigger evidence was supplied and admitted.
    Admitted,
    /// No trigger evidence was supplied.
    Missing,
    /// Trigger evidence was supplied but refused.
    Refused,
}

/// One declared capture's supplied input state at an anchor. Establishing the
/// value is agent F's binding contract; classifying the disposition is this
/// evaluator's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureInput {
    /// An established typed value, with the anchor it was read at.
    Value {
        /// The anchor the value was read at.
        anchor: String,
        /// The established value, retained verbatim.
        value: String,
    },
    /// No input was supplied for this capture.
    Missing,
    /// The input was supplied but carried no value.
    Null,
    /// The input's type does not match the capture's declared type.
    WrongType,
    /// The input was read at an earlier revision of its source.
    Stale,
}

/// One admitted semantic trigger, or one admitted whole-execution origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trigger {
    /// Semantic trigger-event identity, or execution-origin identity. This
    /// alone keys the obligation instance.
    pub identity: String,
    /// Opaque delivery receipt identity. Repeated delivery of one semantic
    /// trigger carries distinct receipts and creates no second instance.
    pub receipt: String,
    /// The anchor this delivery was read at.
    pub anchor: String,
    /// Opaque payload, compared only to detect a conflicting redelivery under
    /// one semantic trigger identity.
    pub payload: String,
    /// Activation guard valuation at the anchor. `None` where the guard could
    /// not be established; an unestablished guard is not a false guard.
    /// Ignored where the declaration has no guard.
    pub guard: Option<bool>,
    /// Supplied input for each declared capture, in the declaration's authored
    /// capture order.
    pub captures: Vec<CaptureInput>,
}

/// A complete caller-constructed observation trace for one declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Trace {
    /// The clock binding this trace asserts.
    pub clock: ClockBinding,
    /// Admitted positions. Order in this vector is not an admitted order and is
    /// never consulted as one.
    pub positions: Vec<Position>,
    /// The authored activation anchor this trace binds to.
    pub anchor: String,
    /// Admitted triggers or execution origins.
    pub triggers: Vec<Trigger>,
    /// Whether trigger evidence was admitted, absent or refused.
    pub trigger_evidence: Evidence,
    /// Trigger-scope closure, independent of the decision scope.
    pub trigger_scope: Closure,
    /// Decision-scope progress and closure.
    pub decision_scope: Closure,
    /// Surrounding-execution progress and closure. Closing the decision scope
    /// does not close this.
    pub surrounding_execution: Closure,
    /// Assessment-execution disposition.
    pub execution: Execution,
    /// Input completeness. An incomplete input applies neither false extension
    /// nor finite-window empty truth, even where the scope is labelled closed.
    pub completeness: Completeness,
    /// Whether the lower boundary is an authoritative execution origin. A mere
    /// history cutoff is not, and cannot decide a past operator.
    pub authoritative_origin: bool,
    /// Progress watermark in the profile's clock domain.
    pub watermark: i64,
    /// Retained valuation and capture records this evaluation may still read.
    /// A record required by an unsettled obligation but listed here as evicted
    /// produces an explicit incomplete result naming it.
    pub evicted: Vec<Eviction>,
}

/// A retained record the caller reports as no longer available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Eviction {
    /// The valuation for one temporal leaf at one clock coordinate.
    Valuation {
        /// Index of the temporal leaf in the declaration's arena.
        node: u32,
        /// Clock coordinate the evicted valuation was recorded at.
        coordinate: i64,
    },
    /// One declared capture of one obligation instance.
    Capture {
        /// Semantic identity of the obligation instance.
        instance: String,
        /// Index of the declared capture in the authored list.
        capture: usize,
    },
}

impl Trace {
    /// Positions ordered by clock coordinate, then by admitted order key where
    /// one authority supplies both. Insertion order is never consulted.
    pub(super) fn ordered(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.positions.len()).collect();
        indices.sort_by(|left, right| {
            let (left, right) = (&self.positions[*left], &self.positions[*right]);
            left.coordinate.cmp(&right.coordinate).then_with(|| {
                match (left.order.as_ref(), right.order.as_ref()) {
                    (Some(left), Some(right)) if left.authority == right.authority => {
                        left.key.cmp(&right.key)
                    }
                    _ => std::cmp::Ordering::Equal,
                }
            })
        });
        indices
    }

    /// Whether two positions at one coordinate carry a usable admitted order.
    /// Missing keys on either side, or keys from different authorities, do not.
    pub(super) fn comparable(&self, left: usize, right: usize) -> bool {
        match (
            self.positions[left].order.as_ref(),
            self.positions[right].order.as_ref(),
        ) {
            (Some(left), Some(right)) => left.authority == right.authority,
            _ => false,
        }
    }
}
