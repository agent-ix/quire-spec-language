// SPDX-License-Identifier: AGPL-3.0-only
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
struct Relation {
    left: usize,
    right: usize,
    lo: i64,
    hi: i64,
    /// Evaluate the Boolean dual: `release` for `until`, `triggered` for `since`.
    dual: bool,
    /// Walk from the greatest offset toward the least, as a past operator does.
    past: bool,
}

pub(super) struct Evaluator<'a> {
    pub profile: Profile,
    pub trace: &'a Trace,
    /// Trace position indices in admitted clock order.
    pub ordered: Vec<usize>,
    pub nodes: &'a [w::Temporal],
    pub declaration: usize,
    pub instance: usize,
    /// Positions consulted while establishing the reported truth.
    pub support: Support,
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
        }
    }

    /// The trace position holding a dense offset, when one exists.
    fn position_at(&self, offset: i64) -> Option<usize> {
        usize::try_from(offset)
            .ok()
            .and_then(|offset| self.ordered.get(offset).copied())
    }

    fn evaluate(
        &mut self,
        node: usize,
        offset: i64,
        depth: usize,
        work: &mut Work,
    ) -> Result<Tri, Error> {
        work.visit()?;
        work.charge(Charge::Depth, depth)?;
        work.subject = self.locate(node, offset);
        let operation =
            self.nodes
                .get(node)
                .map(|entry| &entry.operation)
                .ok_or(Refusal::Reference {
                    subject: work.subject,
                })?;
        match operation {
            // A source constant keeps its authored value at every offset,
            // including beyond a closed decision scope.
            w::TemporalOperation::Constant { value } => {
                Ok(if *value { Tri::True } else { Tri::False })
            }
            // An atomic valuation is distinct from a constant, and a grouping
            // node preserves that distinction rather than erasing it.
            w::TemporalOperation::Holds { .. } => self.atom(node, offset, work),
            w::TemporalOperation::Group { value } => {
                self.evaluate(index(value, work)?, offset, depth.saturating_add(1), work)
            }
            w::TemporalOperation::Unary {
                operator,
                interval,
                value,
            } => {
                let inner = index(value, work)?;
                let depth = depth.saturating_add(1);
                match operator {
                    // Pointwise temporal negation, not an untimed reading.
                    w::TemporalUnary::Not => Ok(self.evaluate(inner, offset, depth, work)?.not()),
                    w::TemporalUnary::Eventually | w::TemporalUnary::Always => {
                        let (lo, hi) = self.future(offset, interval, work)?;
                        let universal = matches!(operator, w::TemporalUnary::Always);
                        self.quantify(inner, lo, hi, universal, depth, work)
                    }
                    w::TemporalUnary::Once | w::TemporalUnary::Historically => {
                        let (lo, hi) = self.past(offset, interval, work)?;
                        let universal = matches!(operator, w::TemporalUnary::Historically);
                        self.quantify(inner, lo, hi, universal, depth, work)
                    }
                }
            }
            w::TemporalOperation::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let (left, right) = (index(left, work)?, index(right, work)?);
                let depth = depth.saturating_add(1);
                match operator {
                    w::TemporalBinary::And => Ok(self
                        .evaluate(left, offset, depth, work)?
                        .and(self.evaluate(right, offset, depth, work)?)),
                    w::TemporalBinary::Or => Ok(self
                        .evaluate(left, offset, depth, work)?
                        .or(self.evaluate(right, offset, depth, work)?)),
                    w::TemporalBinary::Implies => Ok(self
                        .evaluate(left, offset, depth, work)?
                        .not()
                        .or(self.evaluate(right, offset, depth, work)?)),
                    // Release and triggered are the Boolean duals of until and
                    // since over the same range and lower-bound convention.
                    w::TemporalBinary::Until
                    | w::TemporalBinary::Release
                    | w::TemporalBinary::Since
                    | w::TemporalBinary::Triggered => {
                        let past = matches!(
                            operator,
                            w::TemporalBinary::Since | w::TemporalBinary::Triggered
                        );
                        let (lo, hi) = if past {
                            self.past(offset, interval, work)?
                        } else {
                            self.future(offset, interval, work)?
                        };
                        self.relation(
                            Relation {
                                left,
                                right,
                                lo,
                                hi,
                                dual: matches!(
                                    operator,
                                    w::TemporalBinary::Release | w::TemporalBinary::Triggered
                                ),
                                past,
                            },
                            depth,
                            work,
                        )
                    }
                }
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
            let span = hi
                .checked_sub(lo)
                .and_then(|span| span.checked_add(1))
                .ok_or_else(|| horizon(work))?;
            let span = usize::try_from(span).map_err(|_| horizon(work))?;
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

    fn quantify(
        &mut self,
        node: usize,
        lo: i64,
        hi: i64,
        universal: bool,
        depth: usize,
        work: &mut Work,
    ) -> Result<Tri, Error> {
        let offsets = self.offsets(lo, hi, work)?;
        if offsets.is_empty() {
            return Ok(self.empty_window(hi, universal, work));
        }
        let mut value = if universal { Tri::True } else { Tri::False };
        for offset in offsets {
            let step = self.evaluate(node, offset, depth, work)?;
            value = if universal {
                value.and(step)
            } else {
                value.or(step)
            };
        }
        Ok(value)
    }

    /// An empty quantification range. Under the finite-window profile the
    /// empty-existential `false` and empty-universal `true` are emitted only
    /// where the authority establishes completeness through the inclusive
    /// upper endpoint.
    fn empty_window(&self, hi: i64, universal: bool, _work: &mut Work) -> Tri {
        let complete =
            self.trace.completeness == Completeness::Complete && self.trace.watermark >= hi;
        if !complete {
            return Tri::Unknown;
        }
        if universal {
            Tri::True
        } else {
            Tri::False
        }
    }

    /// `left until[a,b] right` and `left since[a,b] right`, together with their
    /// Boolean duals `release` and `triggered`.
    ///
    /// `until` is `OR_j ( AND_{i<j} left(i) AND right(j) )`. `release` is its
    /// Boolean dual `not((not left) until (not right))`, which expands to
    /// `AND_j ( OR_{i<j} left(i) OR right(j) )`. Writing the dual out this way
    /// keeps both under one lower-bound convention rather than two. For a past
    /// operator the interval's lower bound is its greatest offset, so the walk
    /// runs backwards and the same expansion applies unchanged.
    fn relation(
        &mut self,
        relation: Relation,
        depth: usize,
        work: &mut Work,
    ) -> Result<Tri, Error> {
        let Relation {
            left,
            right,
            lo,
            hi,
            dual,
            past,
        } = relation;
        self.admitted_order(lo, hi, work)?;
        let mut offsets = self.offsets(lo, hi, work)?;
        if past {
            offsets.reverse();
        }
        if offsets.is_empty() {
            return Ok(self.empty_window(hi, dual, work));
        }
        let mut result = if dual { Tri::True } else { Tri::False };
        let mut every = Tri::True;
        let mut any = Tri::False;
        for offset in offsets {
            let witness = self.evaluate(right, offset, depth, work)?;
            let step = if dual {
                any.or(witness)
            } else {
                every.and(witness)
            };
            result = if dual {
                result.and(step)
            } else {
                result.or(step)
            };
            let held = self.evaluate(left, offset, depth, work)?;
            every = every.and(held);
            any = any.or(held);
        }
        Ok(result)
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
            return Ok(self.outside());
        }

        let Some(position) = self.position_at(offset) else {
            return Ok(self.outside());
        };
        if self.trace.evicted.iter().any(|evicted| {
            matches!(
                evicted,
                Eviction::Valuation { node, coordinate }
                    if *node == leaf
                        && *coordinate == self.trace.positions[position].coordinate
            )
        }) {
            return Err(Incomplete {
                dimension: Dimension::Valuation,
                subject,
            }
            .into());
        }
        match self.trace.positions[position].valuations.get(&leaf) {
            Some(true) => {
                self.support.push(position);
                Ok(Tri::True)
            }
            Some(false) => {
                self.support.push(position);
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
    fn outside(&self) -> Tri {
        if self.trace.completeness == Completeness::Incomplete {
            return Tri::Unknown;
        }
        match (self.trace.decision_scope, self.profile.extends_false()) {
            (Closure::Closed, true) => Tri::False,
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

fn horizon(work: &Work) -> Error {
    super::budget::Exhaustion {
        dimension: Charge::Horizon,
        used: work.usage.horizon,
        requested: usize::MAX,
        limit: work.limits.horizon,
        subject: work.subject,
    }
    .into()
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
