// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043: bounded evaluation of the emitted temporal operation graph.
//!
//! Evaluation is three-valued. `Unknown` means the observed prefix does not
//! determine the value; it becomes `Pending` with an `unsettled` basis at the
//! root and is never reported as `false`. A missing required valuation or a
//! missing history interval is an incomplete result, not an `Unknown` and not a
//! `false` atom.

use crate::protocol_artifact::wire as w;

use super::budget::{Dimension as Charge, Work};
use super::profile::Profile;
use super::result::{Completeness, Dimension, Error, Incomplete, Refusal, Subject, Support};
use super::trace::{Closure, Eviction, Trace};
use std::collections::BTreeSet;

/// Kleene three-valued truth over the observed prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    fn not(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}

/// One bounded binary temporal relation over an inclusive offset range.
#[derive(Clone, Copy)]
struct Relation {
    left: usize,
    right: usize,
    hi: i64,
    /// Evaluate the Boolean dual: `release` for `until`, `triggered` for `since`.
    dual: bool,
}

pub(super) struct Evaluator<'a> {
    pub profile: Profile,
    pub trace: &'a Trace,
    /// Trace position indices in admitted clock order.
    pub ordered: Vec<usize>,
    pub nodes: &'a [w::TemporalOperation],
    pub declaration: usize,
    pub instance: usize,
    /// Semantic trigger/origin identity for instance-specific valuations.
    pub instance_identity: &'a str,
    /// Checked atoms whose authored expression reads an immutable capture.
    /// Their valuation must be supplied under this instance identity.
    pub instance_bound_atoms: &'a BTreeSet<u32>,
    /// Progress in force for this binding, in the profile's clock domain. It is
    /// the trace's own assertion, or the progress a ledger retains for the
    /// binding; progress asserted under any other binding never reaches here.
    pub watermark: i64,
    /// Positions consulted while establishing the reported truth.
    pub support: Support,
    /// Distinct required valuation records touched by this obligation.
    pub retained_valuations: BTreeSet<(u32, usize)>,
}

enum FrameState {
    Enter,
    AwaitGroup,
    AwaitNot,
    AwaitBinaryLeft {
        right: usize,
        operator: w::TemporalBinary,
    },
    AwaitBinaryRight {
        left: Tri,
        operator: w::TemporalBinary,
    },
    Quantify {
        node: usize,
        offsets: Vec<i64>,
        next: usize,
        value: Tri,
        hi: i64,
        universal: bool,
    },
    AwaitQuantified {
        node: usize,
        offsets: Vec<i64>,
        next: usize,
        value: Tri,
        hi: i64,
        universal: bool,
    },
    Relate {
        relation: Relation,
        offsets: Vec<i64>,
        next: usize,
        value: Tri,
        every: Tri,
        any: Tri,
    },
    AwaitRight {
        relation: Relation,
        offsets: Vec<i64>,
        next: usize,
        value: Tri,
        every: Tri,
        any: Tri,
    },
    AwaitLeft {
        relation: Relation,
        offsets: Vec<i64>,
        next: usize,
        value: Tri,
        every: Tri,
        any: Tri,
        witness: Tri,
    },
}

struct Frame {
    node: usize,
    offset: i64,
    depth: usize,
    state: FrameState,
}

impl<'a> Evaluator<'a> {
    pub fn root(&mut self, node: usize, work: &mut Work) -> Result<Tri, Error> {
        self.evaluate(node, 0, 1, work)
    }

    fn locate(&self, node: usize, offset: i64) -> Subject {
        Subject {
            declaration: self.declaration,
            instance: self.instance,
            node: Some(node),
            position: self.position_at(offset),
            capture: None,
        }
    }

    /// The trace position an offset addresses, when one exists. A dense profile
    /// names one position per integer offset from the anchor; a finite window
    /// is sparse, so the offset names the admitted instant at that clock
    /// distance from the anchor and no other.
    fn position_at(&self, offset: i64) -> Option<usize> {
        if self.profile.requires_dense_positions() {
            return usize::try_from(offset)
                .ok()
                .and_then(|offset| self.ordered.get(offset).copied());
        }
        let coordinate = self.anchor_coordinate().checked_add(offset)?;
        self.ordered
            .iter()
            .copied()
            .find(|position| self.trace.positions[*position].coordinate == coordinate)
    }

    fn evaluate(
        &mut self,
        node: usize,
        offset: i64,
        depth: usize,
        work: &mut Work,
    ) -> Result<Tri, Error> {
        let mut stack = vec![Frame {
            node,
            offset,
            depth,
            state: FrameState::Enter,
        }];
        let mut returned = None;
        loop {
            if let Some(value) = returned.take() {
                let Some(parent) = stack.last_mut() else {
                    return Ok(value);
                };
                let state = std::mem::replace(&mut parent.state, FrameState::Enter);
                match state {
                    FrameState::AwaitGroup => {
                        stack.pop();
                        returned = Some(value);
                    }
                    FrameState::AwaitNot => {
                        stack.pop();
                        returned = Some(value.not());
                    }
                    FrameState::AwaitBinaryLeft { right, operator } => {
                        let offset = parent.offset;
                        let depth = parent.depth.saturating_add(1);
                        parent.state = FrameState::AwaitBinaryRight {
                            left: value,
                            operator,
                        };
                        stack.push(Frame {
                            node: right,
                            offset,
                            depth,
                            state: FrameState::Enter,
                        });
                    }
                    FrameState::AwaitBinaryRight { left, operator } => {
                        stack.pop();
                        returned = Some(match operator {
                            w::TemporalBinary::And => left.and(value),
                            w::TemporalBinary::Or => left.or(value),
                            w::TemporalBinary::Implies => left.not().or(value),
                            _ => unreachable!("pointwise binary operator"),
                        });
                    }
                    FrameState::AwaitQuantified {
                        node,
                        offsets,
                        next,
                        value: accumulated,
                        hi,
                        universal,
                    } => {
                        parent.state = FrameState::Quantify {
                            node,
                            offsets,
                            next: next + 1,
                            value: if universal {
                                accumulated.and(value)
                            } else {
                                accumulated.or(value)
                            },
                            hi,
                            universal,
                        };
                    }
                    FrameState::AwaitRight {
                        relation,
                        offsets,
                        next,
                        value: accumulated,
                        every,
                        any,
                    } => {
                        let offset = offsets[next];
                        let depth = parent.depth.saturating_add(1);
                        parent.state = FrameState::AwaitLeft {
                            relation,
                            offsets,
                            next,
                            value: accumulated,
                            every,
                            any,
                            witness: value,
                        };
                        stack.push(Frame {
                            node: relation.left,
                            offset,
                            depth,
                            state: FrameState::Enter,
                        });
                    }
                    FrameState::AwaitLeft {
                        relation,
                        offsets,
                        next,
                        value: accumulated,
                        every,
                        any,
                        witness,
                    } => {
                        let step = if relation.dual {
                            any.or(witness)
                        } else {
                            every.and(witness)
                        };
                        parent.state = FrameState::Relate {
                            relation,
                            offsets,
                            next: next + 1,
                            value: if relation.dual {
                                accumulated.and(step)
                            } else {
                                accumulated.or(step)
                            },
                            every: every.and(value),
                            any: any.or(value),
                        };
                    }
                    _ => unreachable!("the parent frame awaits its child"),
                }
                continue;
            }

            let frame = stack.last_mut().expect("an unfinished frame");
            let node = frame.node;
            let offset = frame.offset;
            let depth = frame.depth;
            let state = std::mem::replace(&mut frame.state, FrameState::Enter);
            match state {
                FrameState::Enter => {
                    work.subject = self.locate(node, offset);
                    work.visit()?;
                    if work.legacy_depth_limit {
                        work.charge(Charge::Depth, depth)?;
                    }
                    let operation = self.nodes.get(node).cloned().ok_or(Refusal::Reference {
                        subject: work.subject,
                    })?;
                    match operation {
                        w::TemporalOperation::Constant { value } => {
                            stack.pop();
                            returned = Some(if value { Tri::True } else { Tri::False });
                        }
                        w::TemporalOperation::Holds { .. } => {
                            let value = self.atom(node, offset, work)?;
                            stack.pop();
                            returned = Some(value);
                        }
                        w::TemporalOperation::Group { value } => {
                            let child = index(&value, work)?;
                            stack.last_mut().unwrap().state = FrameState::AwaitGroup;
                            stack.push(Frame {
                                node: child,
                                offset,
                                depth: depth.saturating_add(1),
                                state: FrameState::Enter,
                            });
                        }
                        w::TemporalOperation::Unary {
                            operator,
                            interval,
                            value,
                        } => {
                            let child = index(&value, work)?;
                            if operator == w::TemporalUnary::Not {
                                stack.last_mut().unwrap().state = FrameState::AwaitNot;
                                stack.push(Frame {
                                    node: child,
                                    offset,
                                    depth: depth.saturating_add(1),
                                    state: FrameState::Enter,
                                });
                                continue;
                            }
                            let (lo, hi, universal) = match operator {
                                w::TemporalUnary::Eventually | w::TemporalUnary::Always => {
                                    let (lo, hi) = self.future(offset, &interval, work)?;
                                    (lo, hi, operator == w::TemporalUnary::Always)
                                }
                                w::TemporalUnary::Once | w::TemporalUnary::Historically => {
                                    let (lo, hi) = self.past(offset, &interval, work)?;
                                    (lo, hi, operator == w::TemporalUnary::Historically)
                                }
                                w::TemporalUnary::Not => unreachable!("handled above"),
                            };
                            stack.last_mut().unwrap().state = FrameState::Quantify {
                                node: child,
                                offsets: self.offsets(lo, hi, work)?,
                                next: 0,
                                value: if universal { Tri::True } else { Tri::False },
                                hi,
                                universal,
                            };
                        }
                        w::TemporalOperation::Binary {
                            operator,
                            interval,
                            left,
                            right,
                        } => {
                            let left = index(&left, work)?;
                            let right = index(&right, work)?;
                            match operator {
                                w::TemporalBinary::And
                                | w::TemporalBinary::Or
                                | w::TemporalBinary::Implies => {
                                    stack.last_mut().unwrap().state =
                                        FrameState::AwaitBinaryLeft { right, operator };
                                    stack.push(Frame {
                                        node: left,
                                        offset,
                                        depth: depth.saturating_add(1),
                                        state: FrameState::Enter,
                                    });
                                }
                                w::TemporalBinary::Until
                                | w::TemporalBinary::Release
                                | w::TemporalBinary::Since
                                | w::TemporalBinary::Triggered => {
                                    let past = matches!(
                                        operator,
                                        w::TemporalBinary::Since | w::TemporalBinary::Triggered
                                    );
                                    let (lo, hi) = if past {
                                        self.past(offset, &interval, work)?
                                    } else {
                                        self.future(offset, &interval, work)?
                                    };
                                    let relation = Relation {
                                        left,
                                        right,
                                        hi,
                                        dual: matches!(
                                            operator,
                                            w::TemporalBinary::Release
                                                | w::TemporalBinary::Triggered
                                        ),
                                    };
                                    self.admitted_order(lo, hi, work)?;
                                    let mut offsets = self.offsets(lo, hi, work)?;
                                    if past {
                                        offsets.reverse();
                                    }
                                    stack.last_mut().unwrap().state = FrameState::Relate {
                                        relation,
                                        offsets,
                                        next: 0,
                                        value: if relation.dual { Tri::True } else { Tri::False },
                                        every: Tri::True,
                                        any: Tri::False,
                                    };
                                }
                            }
                        }
                    }
                }
                FrameState::Quantify {
                    node,
                    offsets,
                    next,
                    value,
                    hi,
                    universal,
                } => {
                    if let Some(offset) = offsets.get(next).copied() {
                        stack.last_mut().unwrap().state = FrameState::AwaitQuantified {
                            node,
                            offsets,
                            next,
                            value,
                            hi,
                            universal,
                        };
                        stack.push(Frame {
                            node,
                            offset,
                            depth: depth.saturating_add(1),
                            state: FrameState::Enter,
                        });
                    } else {
                        stack.pop();
                        returned = Some(self.window(value, hi, universal));
                    }
                }
                FrameState::Relate {
                    relation,
                    offsets,
                    next,
                    value,
                    every,
                    any,
                } => {
                    if let Some(offset) = offsets.get(next).copied() {
                        stack.last_mut().unwrap().state = FrameState::AwaitRight {
                            relation,
                            offsets,
                            next,
                            value,
                            every,
                            any,
                        };
                        stack.push(Frame {
                            node: relation.right,
                            offset,
                            depth: depth.saturating_add(1),
                            state: FrameState::Enter,
                        });
                    } else {
                        stack.pop();
                        returned = Some(self.window(value, relation.hi, relation.dual));
                    }
                }
                _ => unreachable!("awaiting frames are handled with child results"),
            }
        }
    }

    /// The inclusive future offset range an interval selects from `offset`,
    /// under checked arithmetic. Overflow refuses before evaluation.
    fn future(
        &self,
        offset: i64,
        interval: &w::Nullable<w::Interval>,
        work: &mut Work,
    ) -> Result<(i64, i64), Error> {
        let (lower, upper) = bounds(interval, work)?;
        let lo = offset.checked_add(lower).ok_or_else(|| horizon(work))?;
        let hi = offset.checked_add(upper).ok_or_else(|| horizon(work))?;
        charge_horizon(hi, work)?;
        Ok((lo, hi))
    }

    /// The inclusive past offset range an interval selects from `offset`.
    fn past(
        &self,
        offset: i64,
        interval: &w::Nullable<w::Interval>,
        work: &mut Work,
    ) -> Result<(i64, i64), Error> {
        let (lower, upper) = bounds(interval, work)?;
        let lo = offset.checked_sub(upper).ok_or_else(|| horizon(work))?;
        let hi = offset.checked_sub(lower).ok_or_else(|| horizon(work))?;
        charge_horizon(upper, work)?;
        Ok((lo, hi))
    }

    /// The offsets an inclusive range quantifies over. A false-extension
    /// profile is dense in its own domain, so every integer offset is a
    /// position; a finite window is sparse and ranges only over admitted
    /// instants.
    fn offsets(&mut self, lo: i64, hi: i64, work: &mut Work) -> Result<Vec<i64>, Error> {
        if lo > hi {
            return Ok(Vec::new());
        }
        if self.profile.requires_dense_positions() {
            // A false-extension profile is dense, so the range names one
            // position per integer offset. A span too large to address is a
            // position-count stop, not a horizon-arithmetic stop: the bound
            // composition itself did not overflow.
            let span = hi
                .checked_sub(lo)
                .and_then(|span| span.checked_add(1))
                .and_then(|span| usize::try_from(span).ok());
            let Some(span) = span else {
                return Err(exhausted(Charge::Positions, usize::MAX, work));
            };
            work.charge(Charge::Positions, span)?;
            return Ok((lo..=hi).collect());
        }
        let anchor = self.anchor_coordinate();
        let mut selected = Vec::new();
        for &position in &self.ordered {
            work.charge(Charge::Positions, 1)?;
            let distance = self.trace.positions[position]
                .coordinate
                .checked_sub(anchor)
                .ok_or_else(|| horizon(work))?;
            if distance >= lo && distance <= hi {
                selected.push(distance);
            }
        }
        Ok(selected)
    }

    /// Coordinate of the activation anchor. The first admitted position in
    /// clock order is offset zero.
    fn anchor_coordinate(&self) -> i64 {
        self.ordered
            .first()
            .map(|position| self.trace.positions[*position].coordinate)
            .unwrap_or_default()
    }

    /// Whether progress establishes this clock through the inclusive upper
    /// endpoint `hi`, measured from the anchor in the profile's own clock
    /// domain, over complete valuations.
    ///
    /// A fixed-sample or timestamped progress assertion advances the clock with
    /// no business event; an event-position clock does not advance during
    /// silence, so its watermark stays a retained premise and establishes
    /// nothing.
    fn established(&self, hi: i64) -> bool {
        self.profile.advances_on_progress()
            && self.trace.completeness == Completeness::Complete
            && self
                .anchor_coordinate()
                .checked_add(hi)
                .is_some_and(|deadline| self.watermark >= deadline)
    }

    /// Settle a quantification over a finite window. A window ranges only over
    /// admitted instants, so the polarity a later instant could still overturn
    /// — `false` for an existential range, including the empty one, and `true`
    /// for a universal one — holds only where progress establishes the window
    /// through its inclusive upper endpoint. A dense profile has no later
    /// instant to admit, so its own boundary rule decides instead.
    fn window(&self, value: Tri, hi: i64, universal: bool) -> Tri {
        let decisive = if universal { Tri::False } else { Tri::True };
        if value == decisive || self.profile.requires_dense_positions() || self.established(hi) {
            return value;
        }
        Tri::Unknown
    }

    /// Order-sensitive operators refuse when two participating positions share
    /// a clock coordinate without one admitted order authority covering both.
    /// Trace insertion order is never substituted.
    fn admitted_order(&self, lo: i64, hi: i64, work: &mut Work) -> Result<(), Error> {
        let anchor = self.anchor_coordinate();
        let participating: Vec<usize> = self
            .ordered
            .iter()
            .copied()
            .filter(|position| {
                let distance = self.trace.positions[*position].coordinate - anchor;
                distance >= lo && distance <= hi
            })
            .collect();
        for (index, left) in participating.iter().enumerate() {
            for right in participating.iter().skip(index + 1) {
                if self.trace.positions[*left].coordinate == self.trace.positions[*right].coordinate
                    && !self.trace.comparable(*left, *right)
                {
                    return Err(Refusal::Order {
                        subject: work.subject,
                    }
                    .into());
                }
            }
        }
        Ok(())
    }

    /// An atomic valuation at one offset, under the selected profile's
    /// closed-boundary rule.
    fn atom(&mut self, node: usize, offset: i64, work: &mut Work) -> Result<Tri, Error> {
        work.charge(Charge::Valuations, 1)?;
        let subject = self.locate(node, offset);
        let leaf = u32::try_from(node).map_err(|_| Refusal::Reference { subject })?;

        if offset < 0 {
            // A past offset needs history through the computed lower boundary,
            // or an authoritative execution origin. A bare cutoff does not
            // authorize false extension.
            if !self.trace.authoritative_origin {
                return Err(Incomplete {
                    dimension: Dimension::History,
                    subject,
                }
                .into());
            }
            return Ok(self.outside(offset));
        }

        let Some(position_index) = self.position_at(offset) else {
            return Ok(self.outside(offset));
        };
        let valuation_record = (leaf, position_index);
        if !self.retained_valuations.contains(&valuation_record) {
            let demand = self.retained_valuations.len().saturating_add(1);
            work.charge(Charge::Retention, demand)?;
            self.retained_valuations.insert(valuation_record);
        }
        let coordinate = self.trace.positions[position_index].coordinate;
        for evicted in &self.trace.evicted {
            work.visit()?;
            if matches!(
                evicted,
                Eviction::Valuation { node, coordinate: evicted_coordinate }
                    if *node == leaf && *evicted_coordinate == coordinate
            ) {
                return Err(Incomplete {
                    dimension: Dimension::Valuation,
                    subject,
                }
                .into());
            }
        }
        let position = &self.trace.positions[position_index];
        let instance_value = position
            .instance_valuations
            .get(self.instance_identity)
            .and_then(|valuations| valuations.get(&leaf));
        let value = if self.instance_bound_atoms.contains(&leaf) {
            instance_value
        } else {
            instance_value.or_else(|| position.valuations.get(&leaf))
        };
        match value {
            Some(true) => {
                self.support.push(position_index);
                Ok(Tri::True)
            }
            Some(false) => {
                self.support.push(position_index);
                Ok(Tri::False)
            }
            // A leaf absent from a required position's map is a missing
            // valuation, never a false one.
            None => Err(Incomplete {
                dimension: Dimension::Valuation,
                subject,
            }
            .into()),
        }
    }

    /// An atomic valuation outside the admitted positions. An incomplete input
    /// applies no boundary rule, even where the scope is labelled closed.
    ///
    /// An open scope extends nothing on its own. A fixed-sample or timestamped
    /// progress assertion, however, advances the clock through the offset with
    /// no business event, so over complete valuations the silent offset carries
    /// a false atom and the bounded obligation settles at its deadline. An
    /// event-position clock does not advance during silence and stays unknown.
    fn outside(&self, offset: i64) -> Tri {
        if self.trace.completeness == Completeness::Incomplete {
            return Tri::Unknown;
        }
        match (self.trace.decision_scope, self.profile.extends_false()) {
            (Closure::Closed, true) => Tri::False,
            (Closure::Open, true) if self.established(offset) => Tri::False,
            (Closure::Closed, false) | (Closure::Open, _) => Tri::Unknown,
        }
    }
}

fn index(handle: &w::Handle, work: &mut Work) -> Result<usize, Error> {
    usize::try_from(handle.index).map_err(|_| {
        Refusal::Reference {
            subject: work.subject,
        }
        .into()
    })
}

/// A stop whose requested increment cannot be represented, so it can never be
/// charged. Usage and the effective ceiling are reported unchanged.
fn exhausted(dimension: Charge, requested: usize, work: &Work) -> Error {
    let (used, limit) = match dimension {
        Charge::Positions => (work.usage.positions, work.limits.positions),
        Charge::Horizon => (work.usage.horizon, work.limits.horizon),
        Charge::Valuations => (work.usage.valuations, work.limits.valuations),
        Charge::Instances => (work.usage.instances, work.limits.instances),
        Charge::Captures => (work.usage.captures, work.limits.captures),
        Charge::Retention => (work.usage.retention, work.limits.retention),
        Charge::Visits => (work.usage.visits, work.limits.visits),
        Charge::Depth => (work.usage.depth, work.limits.depth),
    };
    super::budget::Exhaustion {
        dimension,
        used,
        requested,
        limit,
        subject: work.subject,
    }
    .into()
}

/// Checked interval arithmetic overflowed; the composed horizon is outside the
/// i64 domain and no evaluation is attempted.
fn horizon(work: &Work) -> Error {
    exhausted(Charge::Horizon, usize::MAX, work)
}

fn charge_horizon(bound: i64, work: &mut Work) -> Result<(), Error> {
    let magnitude = usize::try_from(bound.unsigned_abs()).map_err(|_| horizon(work))?;
    work.charge(Charge::Horizon, magnitude)?;
    Ok(())
}

/// A timed operator carries an interval; an untimed connective carries none.
fn bounds(interval: &w::Nullable<w::Interval>, work: &mut Work) -> Result<(i64, i64), Error> {
    let Some(interval) = interval.0.as_ref() else {
        return Err(Refusal::Binding {
            dimension: Dimension::Interval,
            subject: work.subject,
        }
        .into());
    };
    let lower = integer(&interval.lower, work)?;
    let upper = integer(&interval.upper, work)?;
    if lower < 0 || upper < lower {
        return Err(Refusal::Binding {
            dimension: Dimension::Interval,
            subject: work.subject,
        }
        .into());
    }
    Ok((lower, upper))
}

/// An admitted interval bound. The reader has already checked the numeric
/// domain; a non-integer bound here is an inadmissible graph, not a refusal the
/// caller can repair by supplying a different trace.
fn integer(value: &w::Integer, work: &mut Work) -> Result<i64, Error> {
    match value.checked() {
        Ok(crate::protocol_artifact::ProtocolNumber::Integer(value)) => Ok(value.value()),
        Ok(crate::protocol_artifact::ProtocolNumber::Rational(_)) | Err(_) => {
            Err(Refusal::Binding {
                dimension: Dimension::Interval,
                subject: work.subject,
            }
            .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::temporal::{ClockBinding, Evidence, Execution, Limits, Position};

    /// TC-124 / NFR-008-AC-2: the first unaffordable visit identifies the
    /// declaration and node that requested it, rather than a prior subject.
    #[test]
    fn tc_124_visit_exhaustion_names_the_current_subject() {
        let trace = Trace {
            clock: ClockBinding {
                name: "orders".into(),
                profile_identity: Profile::EventPosition.identity().into(),
                parameters: BTreeMap::new(),
            },
            positions: Vec::new(),
            anchor: "origin".into(),
            triggers: Vec::new(),
            trigger_evidence: Evidence::Admitted,
            trigger_scope: Closure::Closed,
            decision_scope: Closure::Closed,
            surrounding_execution: Closure::Open,
            execution: Execution::Completed,
            completeness: Completeness::Complete,
            authoritative_origin: true,
            watermark: 0,
            evicted: Vec::new(),
        };
        let mut evaluator = Evaluator {
            profile: Profile::EventPosition,
            trace: &trace,
            ordered: Vec::new(),
            nodes: &[],
            declaration: 37,
            instance: 11,
            instance_identity: "execution:fixture",
            instance_bound_atoms: &BTreeSet::new(),
            watermark: 0,
            support: Support::default(),
            retained_valuations: BTreeSet::new(),
        };
        let mut work = Work::new(Limits {
            visits: 0,
            ..Limits::default()
        });

        let Err(Error::Exhausted(exhaustion)) = evaluator.root(0, &mut work) else {
            panic!("zero visit budget must stop before inspecting the node");
        };
        assert_eq!(exhaustion.dimension, Charge::Visits);
        assert_eq!(exhaustion.subject.declaration, 37);
        assert_eq!(exhaustion.subject.instance, 11);
        assert_eq!(exhaustion.subject.node, Some(0));
    }

    /// A failed peak-retention charge does not insert the new demand, while a
    /// repeated successful demand for the same record is charged only once.
    #[test]
    fn retention_demand_is_charged_before_insert_and_deduplicated() {
        let trace = Trace {
            clock: ClockBinding {
                name: "orders".into(),
                profile_identity: Profile::EventPosition.identity().into(),
                parameters: BTreeMap::new(),
            },
            positions: vec![Position {
                coordinate: 0,
                order: None,
                valuations: BTreeMap::from([(0, true)]),
                instance_valuations: BTreeMap::new(),
            }],
            anchor: "origin".into(),
            triggers: Vec::new(),
            trigger_evidence: Evidence::Admitted,
            trigger_scope: Closure::Closed,
            decision_scope: Closure::Closed,
            surrounding_execution: Closure::Open,
            execution: Execution::Completed,
            completeness: Completeness::Complete,
            authoritative_origin: true,
            watermark: 0,
            evicted: Vec::new(),
        };
        let instance_bound_atoms = BTreeSet::new();
        fn make_evaluator<'a>(
            trace: &'a Trace,
            instance_bound_atoms: &'a BTreeSet<u32>,
        ) -> Evaluator<'a> {
            Evaluator {
                profile: Profile::EventPosition,
                trace,
                ordered: vec![0],
                nodes: &[],
                declaration: 0,
                instance: 0,
                instance_identity: "execution:fixture",
                instance_bound_atoms,
                watermark: 0,
                support: Support::default(),
                retained_valuations: BTreeSet::new(),
            }
        }

        let mut stopped = make_evaluator(&trace, &instance_bound_atoms);
        let mut no_retention = Work::new(Limits {
            retention: 0,
            ..Limits::default()
        });
        assert!(matches!(
            stopped.atom(0, 0, &mut no_retention),
            Err(Error::Exhausted(exhaustion)) if exhaustion.dimension == Charge::Retention
        ));
        assert!(stopped.retained_valuations.is_empty());
        assert_eq!(no_retention.usage.retention, 0);

        let mut repeated = make_evaluator(&trace, &instance_bound_atoms);
        let mut one_retained = Work::new(Limits {
            retention: 1,
            ..Limits::default()
        });
        assert_eq!(repeated.atom(0, 0, &mut one_retained), Ok(Tri::True));
        assert_eq!(repeated.atom(0, 0, &mut one_retained), Ok(Tri::True));
        assert_eq!(repeated.retained_valuations.len(), 1);
        assert_eq!(one_retained.usage.retention, 1);
    }
}
