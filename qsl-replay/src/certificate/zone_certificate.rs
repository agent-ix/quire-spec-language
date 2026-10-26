// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244's zone certificate and FR-245's check of it in the qualified core
//! (ADR-026 CF-1, CF-3, CF-4, CF-6).
//!
//! EN-6, the zone search in `qsl-analyze`, returns a [`ZoneCertificate`]
//! with every proof. [`check_zone_certificate`] verifies it with the core's
//! own zones ([`super::zone`]), reading nothing from the search but the
//! certificate. It re-admits the subject, rebuilds its timed reading and
//! recomputes every successor of every node it reaches from the initial
//! states:
//!
//! 1. the `Malformed` members: the scale, the LU bounds (each at least the
//!    bound the checker computes from the model), the nodes and the edges;
//! 2. the initial coverage: each initial symbolic state is covered by a
//!    named initial node of its discrete state (`InitialMissing`, at the
//!    initial discrete state);
//! 3. the nodes in node order: every symbolic successor has an edge of its
//!    transition identity and guard part to a node of its discrete state
//!    whose zone covers it under the aLU simulation with the certificate's
//!    bounds (`SuccessorMissing`); no node is bad unless it is `locked`, and
//!    from a `locked` node no quiescent node and no cycle that lets time
//!    diverge is reachable (`BadState`);
//! 4. `admitted`: a path of such edges from an initial node to a quiescent
//!    node, a terminal node under a tick source, or a cycle that lets time
//!    advance (`WitnessFails`, at its last node).
//!
//! The first failure is the rejection. A node no edge reaches from an
//! initial node is read nowhere and checked for nothing.
//!
//! Time divergence is checked by sufficient conditions the checker proves
//! itself, never by the search's evidence: a cycle lets time advance when,
//! under a tick source, it takes a tick, and under a dense source, some
//! clock it resets to `r` is guarded on it by `x >= c` (or `x > c`) with
//! `c > r` before its next reset, so each repetition takes at least
//! `c - r`. A strongly connected set of nodes lets no time diverge when
//! some clock that no edge inside it resets is bounded above at every
//! node of it. A certificate whose divergence needs more than these is
//! rejected, never accepted on trust.
//!
//! Supported items are a TT-1 invariant (`always holds(p)`) and the
//! deadlock-freedom item. A TT-2 window claim's certificate needs its claim
//! automaton in the core, and the liveness and time-lock-freedom forms
//! need their component and divergence parts (QSL-583): each is refused
//! [`ZoneCertificateRefusal::UnsupportedItem`].

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::fairness::{components, EdgeLabel, Graph, Identity};
use qsl_eval::simulation::model::{
    GuardedStep, ModelSimState, ModelSimulationRequest, ModelSystem, PlaceConstraint,
    TerminalClass, TimedSubjectRefusal, TimedSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::{ClockOp, TemporalProfile};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Cancel, Integer, Meter, Rational};
use quire_semantic_value::model_transition::ModelTransition;

use super::closure::CertificateRejection;
use super::zone::{LuBounds, Zone, ZoneError};
use super::{CertificateLocus, CertificateMember, CertificateRule};
use crate::model_check::item::ModelCheckItem;
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;

/// FR-244's certificate of a zone search's proof of the item `obligation`
/// names (ADR-013 O-09).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneCertificate<O> {
    /// The item it proves.
    pub obligation: O,
    /// The scale factor of every zone bound (FR-238): a bound `c` stands
    /// for `c / scale` time units.
    pub scale: Integer,
    /// Each clock place's `L` and `U`, in scaled units: the search uses one
    /// pair per place for every discrete state.
    pub lu_bounds: LuBounds,
    /// The nodes, in discovery order.
    pub nodes: Vec<CertNode>,
    /// The edges, by source node in discovery order and, from each node, in
    /// canonical transition order: one per convex part of an enabled
    /// step's guard (spec gap STD-493: FR-244 names one edge per transition
    /// identity).
    pub edges: Vec<CertEdge>,
    /// The node that holds each initial symbolic state, one per initial
    /// state that admits a valuation (spec gap STD-493: FR-244 names one).
    pub initial: Vec<usize>,
    /// The witness that the subject admits a time-divergent behaviour
    /// (ADR-026 TD-6).
    pub admitted: DivergenceWitness,
}

/// One certificate node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertNode {
    /// The digest of the model state's state key.
    pub state_key: DigestRecord,
    /// The claim-automaton state; 0 for an item that needs no automaton.
    pub automaton_state: usize,
    /// The zone, in scaled units, over the reference clock, the clock
    /// places and, for a TT-2 claim, the horizon clock.
    pub zone: Zone,
    /// Whether the node violates the item only on time-locked valuations,
    /// which FR-239 continues past.
    pub locked: bool,
}

/// One certificate edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertEdge {
    /// The source node.
    pub from: usize,
    /// The step's transition identity; `None` for an idle-tail stutter
    /// position of a TT-2 claim's automaton (FR-242).
    pub transition: Option<ModelTransition>,
    /// The convex part of the step's guard taken; `None` for a stutter.
    pub part: Option<usize>,
    /// Under a dense source, the piece of a TT-2 claim's horizon axis the
    /// position lies in, as constraints on the horizon clock; `None` with
    /// no automaton, past the formula's horizon, and under a tick source
    /// (spec gap STD-493: FR-244's edge names no piece).
    pub horizon: Option<Vec<PlaceConstraint>>,
    /// The node whose zone holds, or under the aLU simulation covers, the
    /// successor.
    pub to: usize,
}

/// A path of certificate edges to a divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivergenceWitness {
    /// The node the path starts at.
    pub start: usize,
    /// The edges of the path, by index.
    pub path: Vec<usize>,
    /// Where it ends.
    pub target: DivergentTarget,
}

/// A divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivergentTarget {
    /// Under a dense source, a quiescent node, from which time diverges by
    /// the idle tail.
    Quiescent(usize),
    /// Under a tick source, a terminal node, whose stutter step counts as
    /// a tick (FR-231 "Digital time").
    Terminal(usize),
    /// A cycle of edges from and back to the path's last node, each
    /// repetition of which lets time advance.
    Cycle(Vec<usize>),
}

/// FR-245's budgets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CertificateCheckLimits {
    /// The most certificate edges a check reads.
    pub max_certificate_edges: u64,
    /// The most certificate bytes a check reads.
    pub max_certificate_bytes: u64,
}

impl Default for CertificateCheckLimits {
    fn default() -> Self {
        Self {
            max_certificate_edges: 1 << 24,
            max_certificate_bytes: 1 << 30,
        }
    }
}

/// What the check needs beside the certificate: the subject, re-admitted
/// by the checker itself, the model whose timed reading the proof is over,
/// and the item.
pub struct ZoneCertificateRequest<'a> {
    /// FR-120's request of the subject.
    pub simulation: ModelSimulationRequest<'a>,
    /// The alias of the model whose time source and timed model the
    /// subject is read under (FR-230).
    pub model: &'a str,
    /// The item.
    pub item: ModelCheckItem<'a>,
}

/// What [`check_zone_certificate`] returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateVerdict {
    /// Every check passed: the item settles `Proved{ZoneCertified,
    /// Certified}` (FR-235).
    Accepted,
    /// A rule failed: the item settles `inconclusive`,
    /// `CertificateRejected`.
    Rejected(CertificateRejection),
    /// A budget or a cancellation stopped the check (V-7).
    Stopped(IncompleteCause),
}

/// Why [`check_zone_certificate`] checked nothing: the request, not the
/// certificate, is at fault. A refusal is never an acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateRefusal<O> {
    /// The certificate proves another item:
    /// `stale_dependency`/`content-mismatch`, naming both.
    ObligationMismatch {
        /// The certificate's obligation.
        certificate: Box<O>,
        /// The item's.
        item: Box<O>,
    },
    /// FR-106's admission of the subject failed.
    Admission(Box<AdmissionFailure>),
    /// The model's timed reading does not build (FR-231).
    TimedSubject(Box<TimedSubjectRefusal>),
    /// The item is not one this checker reads yet (see the module).
    UnsupportedItem,
    /// The clause's instances are not finite.
    Unbounded,
    /// A reading the check needs has no value: a guard, invariant, urgency,
    /// expansion or atom.
    Undecided,
    /// The checker broke an invariant of its own.
    Fault(&'static str),
}

impl<O> ZoneCertificateRefusal<O> {
    /// The refusal's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::ObligationMismatch { .. } => Code::StaleDependency,
            Self::UnsupportedItem => Code::UnsupportedConstruct,
            Self::Fault(_) => Code::RuntimeInvariant,
            Self::Admission(_) | Self::TimedSubject(_) | Self::Unbounded | Self::Undecided => {
                Code::InvalidRuntimeInput
            }
        }
    }
}

/// How a check ends early.
enum Halt<O> {
    Verdict(ZoneCertificateVerdict),
    Refusal(ZoneCertificateRefusal<O>),
}

impl<O> From<ZoneError> for Halt<O> {
    fn from(_: ZoneError) -> Self {
        Self::Refusal(ZoneCertificateRefusal::Fault("zone-shape"))
    }
}

type Step<T, O> = Result<T, Halt<O>>;

fn reject<T, O>(rule: CertificateRule, at: CertificateLocus) -> Step<T, O> {
    Err(Halt::Verdict(ZoneCertificateVerdict::Rejected(
        CertificateRejection { rule, at },
    )))
}

fn malformed<T, O>(member: CertificateMember) -> Step<T, O> {
    reject(
        CertificateRule::Malformed,
        CertificateLocus::CertificateMember(member),
    )
}

fn at_node(node: usize) -> CertificateLocus {
    CertificateLocus::ZoneNode {
        node: u64::try_from(node).unwrap_or(u64::MAX),
    }
}

fn undecided<T, O, E>(_: E) -> Step<T, O> {
    Err(Halt::Refusal(ZoneCertificateRefusal::Undecided))
}

/// FR-245: check `certificate` for `request`'s item, whose obligation is
/// `item`, under `limits`, polling `cancel` at every node.
///
/// # Errors
///
/// [`ZoneCertificateRefusal`] when the certificate names another item, the
/// subject does not admit, its timed reading does not build, the item is
/// not supported, or a reading has no value.
pub fn check_zone_certificate<O: Clone + Eq>(
    request: ZoneCertificateRequest<'_>,
    item: &O,
    certificate: &ZoneCertificate<O>,
    limits: CertificateCheckLimits,
    cancel: &Cancel,
) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<O>> {
    if certificate.obligation != *item {
        return Err(ZoneCertificateRefusal::ObligationMismatch {
            certificate: Box::new(certificate.obligation.clone()),
            item: Box::new(item.clone()),
        });
    }
    let system = ModelSystem::new(request.simulation)
        .map_err(|failure| ZoneCertificateRefusal::Admission(Box::new(failure)))?;
    let timed = TimedSystem::new(&system, request.model)
        .map_err(|refusal| ZoneCertificateRefusal::TimedSubject(Box::new(refusal)))?;
    let claim = match request.item {
        ModelCheckItem::Clause { clause, letters } => {
            let Some(atom) = qsl_eval::automaton::invariant_atom(clause) else {
                return Err(ZoneCertificateRefusal::UnsupportedItem);
            };
            if !matches!(
                clause.profile(),
                TemporalProfile::InfiniteTrace | TemporalProfile::Timed
            ) {
                return Err(ZoneCertificateRefusal::UnsupportedItem);
            }
            let instances = system
                .instances(&clause.over().1)
                .map_err(|_| ZoneCertificateRefusal::Unbounded)?;
            Some(Claim {
                atom,
                letters,
                instances,
            })
        }
        ModelCheckItem::DeadlockFreedom => None,
        ModelCheckItem::RaceFreedom => return Err(ZoneCertificateRefusal::UnsupportedItem),
    };
    let mut checker = Checker {
        timed: &timed,
        claim,
        certificate,
        cancel,
        states: vec![None; certificate.nodes.len()],
        successors: BTreeMap::new(),
        valid: BTreeSet::new(),
    };
    match checker.check(limits) {
        Ok(()) => Ok(ZoneCertificateVerdict::Accepted),
        Err(Halt::Verdict(verdict)) => Ok(verdict),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

/// A TT-1 claim: its atom, read through `letters` for each instance.
struct Claim<'a> {
    atom: qsl_semantics::check::FormulaNodeId,
    letters: &'a dyn qsl_eval::model_subject::ModelLetters,
    instances: Vec<qsl_eval::model_subject::ModelInstance>,
}

/// One recomputed symbolic successor of a node.
struct Successor {
    step: GuardedStep,
    part: usize,
    digest: DigestRecord,
    zone: Zone,
}

struct Checker<'c, 'a, O> {
    timed: &'c TimedSystem<'c, 'a>,
    claim: Option<Claim<'c>>,
    certificate: &'c ZoneCertificate<O>,
    cancel: &'c Cancel,
    /// The model state of each node an edge reaches from an initial node.
    states: Vec<Option<ModelSimState>>,
    /// Each reached node's recomputed successors.
    successors: BTreeMap<usize, Vec<Successor>>,
    /// The edges whose successor is real and covered by their target.
    valid: BTreeSet<usize>,
}

impl<O> Checker<'_, '_, O> {
    fn check(&mut self, limits: CertificateCheckLimits) -> Step<(), O> {
        self.poll()?;
        self.budgets(limits)?;
        self.members()?;
        self.initial()?;
        self.reach()?;
        for node in 0..self.certificate.nodes.len() {
            self.poll()?;
            if self.states[node].is_some() {
                self.node(node)?;
            }
        }
        self.admitted()
    }

    fn poll(&self) -> Step<(), O> {
        if self.cancel.poll() {
            let Some(source) = self.cancel.cause() else {
                return Err(Halt::Refusal(ZoneCertificateRefusal::Fault(
                    "a-polled-cancellation-has-a-cause",
                )));
            };
            return Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
        Ok(())
    }

    /// `max_certificate_edges` and `max_certificate_bytes`, before any
    /// recomputation.
    fn budgets(&self, limits: CertificateCheckLimits) -> Step<(), O> {
        let stop = |limit, value| {
            Err(Halt::Verdict(ZoneCertificateVerdict::Stopped(
                IncompleteCause::LimitReached { limit, value },
            )))
        };
        let edges = u64::try_from(self.certificate.edges.len()).unwrap_or(u64::MAX);
        if edges > limits.max_certificate_edges {
            return stop(
                ModelCheckLimit::CertificateMaxEdges,
                limits.max_certificate_edges,
            );
        }
        if certificate_bytes(self.certificate) > limits.max_certificate_bytes {
            return stop(
                ModelCheckLimit::CertificateMaxBytes,
                limits.max_certificate_bytes,
            );
        }
        Ok(())
    }

    /// The zone dimension: the reference clock and the clock places.
    fn dim(&self) -> usize {
        self.timed.places().len() + 1
    }

    /// The `Malformed` members, in member order.
    fn members(&self) -> Step<(), O> {
        let certificate = self.certificate;
        if certificate.scale <= Integer::zero() {
            return malformed(CertificateMember::Scale);
        }
        let Some((lower, upper)) = computed_bounds(self.timed, &certificate.scale) else {
            return malformed(CertificateMember::Scale);
        };
        let at_least = |listed: &[Option<Integer>], computed: &[Option<Integer>]| {
            listed.len() == computed.len()
                && listed
                    .iter()
                    .zip(computed)
                    .all(|(listed, computed)| match (listed, computed) {
                        (_, None) => true,
                        (None, Some(_)) => false,
                        (Some(listed), Some(computed)) => listed >= computed,
                    })
        };
        if !at_least(&certificate.lu_bounds.lower, &lower)
            || !at_least(&certificate.lu_bounds.upper, &upper)
        {
            return malformed(CertificateMember::Bounds);
        }
        let nodes = certificate.nodes.len();
        let node_ok = |node: &CertNode| node.zone.dim() == self.dim() && node.automaton_state == 0;
        if !certificate.nodes.iter().all(node_ok)
            || certificate.initial.iter().any(|&node| node >= nodes)
            || certificate.admitted.start >= nodes
        {
            return malformed(CertificateMember::Nodes);
        }
        let edges = certificate.edges.len();
        let edge_ok = |edge: &CertEdge| {
            edge.from < nodes
                && edge.to < nodes
                && edge.transition.is_some()
                && edge.part.is_some()
                && edge.horizon.is_none()
        };
        let target_ok = match &certificate.admitted.target {
            DivergentTarget::Quiescent(node) | DivergentTarget::Terminal(node) => *node < nodes,
            DivergentTarget::Cycle(cycle) => cycle.iter().all(|&edge| edge < edges),
        };
        if !certificate.edges.iter().all(edge_ok)
            || certificate.admitted.path.iter().any(|&edge| edge >= edges)
            || !target_ok
        {
            return malformed(CertificateMember::Edges);
        }
        Ok(())
    }

    /// The scaled integer of `value`, which the certificate's scale makes
    /// exact once its bounds are checked.
    fn scaled(&self, value: &Rational) -> Integer {
        scaled(value, &self.certificate.scale).unwrap_or_else(Integer::zero)
    }

    fn constrain(&self, zone: &mut Zone, constraints: &[PlaceConstraint]) -> Step<(), O> {
        for constraint in constraints {
            let clock = constraint.place() + 1;
            let bound = self.scaled(constraint.bound());
            let below = bound.neg();
            match constraint.op() {
                ClockOp::Less => zone.constrain(clock, 0, &bound, true)?,
                ClockOp::LessOrEqual => zone.constrain(clock, 0, &bound, false)?,
                ClockOp::Equal => {
                    zone.constrain(clock, 0, &bound, false)?;
                    zone.constrain(0, clock, &below, false)?;
                }
                ClockOp::GreaterOrEqual => zone.constrain(0, clock, &below, false)?,
                ClockOp::Greater => zone.constrain(0, clock, &below, true)?,
            }
        }
        Ok(())
    }

    /// Whether time may elapse at `state`.
    fn elapses(&self, state: &ModelSimState) -> Step<bool, O> {
        Ok(
            self.timed.tick_period().is_none()
                && !self.timed.is_urgent(state).or_else(undecided)?,
        )
    }

    /// `zone` within `state`'s invariants, time let elapse unless it is
    /// urgent, and within them again.
    fn settle(&self, state: &ModelSimState, zone: &mut Zone) -> Step<(), O> {
        let invariants = self.timed.invariants(state).or_else(undecided)?;
        self.constrain(zone, &invariants)?;
        if !zone.is_empty() && self.elapses(state)? {
            zone.up();
            self.constrain(zone, &invariants)?;
        }
        Ok(())
    }

    fn digest(&self, state: &ModelSimState) -> Step<DigestRecord, O> {
        self.timed
            .system()
            .state_key(state)
            .map(|(_, digest)| digest)
            .or_else(undecided)
    }

    /// Each initial symbolic state is covered by a named initial node of
    /// its discrete state.
    fn initial(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        for start in self.timed.initial() {
            self.poll()?;
            let state = start.discrete().clone();
            let digest = self.digest(&state)?;
            let mut zone = Zone::zero(self.dim());
            self.settle(&state, &mut zone)?;
            if zone.is_empty() {
                continue;
            }
            let mut covered = None;
            for &node in &certificate.initial {
                let listed = &certificate.nodes[node];
                if listed.state_key == digest
                    && zone.alu_covered_by(&listed.zone, &certificate.lu_bounds)?
                {
                    covered = Some(node);
                    break;
                }
            }
            let Some(node) = covered else {
                return reject(
                    CertificateRule::InitialMissing,
                    CertificateLocus::ModelState(digest),
                );
            };
            self.states[node].get_or_insert(state);
        }
        Ok(())
    }

    /// The model state of every node an edge reaches from an initial node,
    /// each reached node's successors, and the edges that hold.
    fn reach(&mut self) -> Step<(), O> {
        let certificate = self.certificate;
        let mut leaving: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (index, edge) in certificate.edges.iter().enumerate() {
            leaving.entry(edge.from).or_default().push(index);
        }
        let mut queue: Vec<usize> = (0..self.states.len())
            .filter(|&node| self.states[node].is_some())
            .collect();
        let mut next = 0;
        while let Some(&node) = queue.get(next) {
            next += 1;
            self.poll()?;
            let Some(state) = self.states[node].clone() else {
                continue;
            };
            let successors = self.successors_of(&state, &certificate.nodes[node].zone)?;
            for &index in leaving.get(&node).map_or(&[][..], Vec::as_slice) {
                let edge = &certificate.edges[index];
                let target = &certificate.nodes[edge.to];
                let mut holds = false;
                for successor in &successors {
                    if Some(successor.step.transition()) == edge.transition.as_ref()
                        && Some(successor.part) == edge.part
                        && successor.digest == target.state_key
                        && successor
                            .zone
                            .alu_covered_by(&target.zone, &certificate.lu_bounds)?
                    {
                        holds = true;
                        if self.states[edge.to].is_none() {
                            self.states[edge.to] = Some(successor.step.post().clone());
                            queue.push(edge.to);
                        }
                        break;
                    }
                }
                if holds {
                    self.valid.insert(index);
                }
            }
            self.successors.insert(node, successors);
        }
        Ok(())
    }

    /// The symbolic successors of `(state, zone)`: one per enabled step and
    /// guard part whose zone is not empty.
    fn successors_of(&self, state: &ModelSimState, zone: &Zone) -> Step<Vec<Successor>, O> {
        let mut found = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            let target = self.timed.invariants(step.post()).or_else(undecided)?;
            for (part, constraints) in step.parts().iter().enumerate() {
                let mut next = zone.clone();
                self.constrain(&mut next, constraints)?;
                if next.is_empty() {
                    continue;
                }
                for (place, value) in step.resets() {
                    next.reset(place + 1, &self.scaled(value))?;
                }
                self.constrain(&mut next, &target)?;
                if next.is_empty() {
                    continue;
                }
                self.settle(step.post(), &mut next)?;
                if next.is_empty() {
                    continue;
                }
                let digest = self.digest(step.post())?;
                found.push(Successor {
                    step: step.clone(),
                    part,
                    digest,
                    zone: next,
                });
            }
        }
        Ok(found)
    }

    /// Node `node`'s successors are all covered, and it is bad only when
    /// it is `locked` and no time diverges after it.
    fn node(&self, node: usize) -> Step<(), O> {
        let leaving: BTreeSet<usize> = self
            .certificate
            .edges
            .iter()
            .enumerate()
            .filter(|(index, edge)| edge.from == node && self.valid.contains(index))
            .map(|(index, _)| index)
            .collect();
        for successor in self.successors.get(&node).map_or(&[][..], Vec::as_slice) {
            let covered = leaving.iter().any(|&index| {
                let edge = &self.certificate.edges[index];
                Some(successor.step.transition()) == edge.transition.as_ref()
                    && Some(successor.part) == edge.part
            });
            if !covered {
                return reject(CertificateRule::SuccessorMissing, at_node(node));
            }
        }
        let listed = &self.certificate.nodes[node];
        if listed.locked {
            if self.diverges_after(node)? {
                return reject(CertificateRule::BadState, at_node(node));
            }
        } else if self.bad(node)? {
            return reject(CertificateRule::BadState, at_node(node));
        }
        Ok(())
    }

    fn state(&self, node: usize) -> Step<&ModelSimState, O> {
        self.states[node]
            .as_ref()
            .ok_or(Halt::Refusal(ZoneCertificateRefusal::Fault(
                "a-reached-node",
            )))
    }

    /// Whether `node` violates the item at some valuation of its zone.
    fn bad(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        let Some(claim) = &self.claim else {
            return self.deadlocked(node);
        };
        for instance in &claim.instances {
            let mut meter = Meter::new(self.timed.system().meter_limits());
            match claim
                .letters
                .atom(claim.atom, instance, state, &mut meter)
                .or_else(undecided)?
            {
                AtomReading::True => {}
                AtomReading::False | AtomReading::Undefined(_) => return Ok(true),
            }
        }
        Ok(false)
    }

    /// FR-232 over the zone: a quiescent node some of whose valuations
    /// enable no step after any delay, as the search reads it.
    fn deadlocked(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        if !self.timed.is_quiescent(state).or_else(undecided)? {
            return Ok(false);
        }
        let mut enabling = Vec::new();
        for step in self.timed.guarded_steps(state).or_else(undecided)? {
            let target = self.timed.invariants(step.post()).or_else(undecided)?;
            // A reset clock's bound is constant over delays; a false one
            // disables the step.
            let reset_breaks = target.iter().any(|bound| {
                step.resets()
                    .iter()
                    .rev()
                    .find(|(place, _)| *place == bound.place())
                    .is_some_and(|(_, value)| !bound.op().holds(value, bound.bound()))
            });
            if reset_breaks {
                continue;
            }
            let reset: Vec<usize> = step.resets().iter().map(|(place, _)| *place).collect();
            let unreset: Vec<PlaceConstraint> = target
                .into_iter()
                .filter(|bound| !reset.contains(&bound.place()))
                .collect();
            for part in step.parts() {
                let mut zone = Zone::universe(self.dim());
                self.constrain(&mut zone, part)?;
                self.constrain(&mut zone, &unreset)?;
                if !zone.is_empty() {
                    zone.down();
                    enabling.push(zone);
                }
            }
        }
        if enabling.is_empty() {
            return Ok(matches!(
                self.timed.system().terminal_class(state, false),
                Ok(TerminalClass::Deadlocked)
            ));
        }
        let mut uncovered = vec![self.certificate.nodes[node].zone.clone()];
        for cover in &enabling {
            let mut rest = Vec::new();
            for piece in &uncovered {
                rest.extend(piece.minus(cover)?);
            }
            uncovered = rest;
            if uncovered.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether time can diverge after `node`: over valid edges it reaches a
    /// quiescent node (a terminal one under a tick source), or a strongly
    /// connected set whose divergence the checker cannot rule out.
    fn diverges_after(&self, node: usize) -> Step<bool, O> {
        let mut reached = vec![node];
        let mut seen: BTreeSet<usize> = BTreeSet::from([node]);
        let mut at = 0;
        while let Some(&from) = reached.get(at) {
            at += 1;
            for (index, edge) in self.certificate.edges.iter().enumerate() {
                if edge.from == from && self.valid.contains(&index) && seen.insert(edge.to) {
                    reached.push(edge.to);
                }
            }
        }
        for &node in &reached {
            if self.idles(node)? {
                return Ok(true);
            }
        }
        let mut graph = Graph::new();
        let vertices: Vec<_> = reached
            .iter()
            .map(|_| graph.add_vertex(std::iter::empty::<Identity>()))
            .collect();
        let position: BTreeMap<usize, usize> = reached
            .iter()
            .enumerate()
            .map(|(position, &node)| (node, position))
            .collect();
        let mut inside = Vec::new();
        for (index, edge) in self.certificate.edges.iter().enumerate() {
            if let (true, Some(&from), Some(&to)) = (
                self.valid.contains(&index),
                position.get(&edge.from),
                position.get(&edge.to),
            ) {
                graph
                    .add_edge(vertices[from], vertices[to], EdgeLabel::Stutter)
                    .map_err(|_| Halt::Refusal(ZoneCertificateRefusal::Fault("graph")))?;
                inside.push(index);
            }
        }
        let within: BTreeSet<_> = vertices.iter().copied().collect();
        let found = components(&graph, &within, &Cancel::new())
            .map_err(|_| Halt::Refusal(ZoneCertificateRefusal::Fault("components")))?;
        for component in found.iter().filter(|component| component.has_edge()) {
            let members: BTreeSet<usize> = component
                .vertices()
                .iter()
                .map(|vertex| reached[vertex.index()])
                .collect();
            let edges: Vec<usize> = inside
                .iter()
                .copied()
                .filter(|&index| {
                    let edge = &self.certificate.edges[index];
                    members.contains(&edge.from) && members.contains(&edge.to)
                })
                .collect();
            if !self.zeno(&members, &edges)? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether a behaviour may idle at `node` forever: under a dense source
    /// a quiescent state where time elapses, under a tick source one that
    /// enables no step, whose stutter ticks (FR-231).
    fn idles(&self, node: usize) -> Step<bool, O> {
        let state = self.state(node)?;
        Ok(match self.timed.tick_period() {
            Some(_) => self
                .successors
                .get(&node)
                .is_none_or(|successors| successors.is_empty()),
            None => self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?,
        })
    }

    /// The step an edge takes, from its source's successors.
    fn step_of(&self, index: usize) -> Step<&GuardedStep, O> {
        let edge = &self.certificate.edges[index];
        self.successors
            .get(&edge.from)
            .and_then(|successors| {
                successors.iter().find(|successor| {
                    Some(successor.step.transition()) == edge.transition.as_ref()
                        && Some(successor.part) == edge.part
                })
            })
            .map(|successor| &successor.step)
            .ok_or(Halt::Refusal(ZoneCertificateRefusal::Fault(
                "a-valid-edge-has-a-step",
            )))
    }

    /// Whether no time diverges inside the strongly connected `members`
    /// with the `edges` among them: under a dense source, some clock that
    /// none of `edges` resets is bounded above at every member; under a
    /// tick source, none of `edges` ticks.
    fn zeno(&self, members: &BTreeSet<usize>, edges: &[usize]) -> Step<bool, O> {
        if self.timed.tick_period().is_some() {
            for &index in edges {
                let step = self.step_of(index)?;
                if self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero()) {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
        let mut reset = BTreeSet::new();
        for &index in edges {
            reset.extend(
                self.step_of(index)?
                    .resets()
                    .iter()
                    .map(|(place, _)| *place),
            );
        }
        for place in 0..self.timed.places().len() {
            if reset.contains(&place) {
                continue;
            }
            let mut bounded = true;
            for &member in members {
                if self.certificate.nodes[member]
                    .zone
                    .bound(place + 1, 0)?
                    .is_none()
                {
                    bounded = false;
                    break;
                }
            }
            if bounded {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// `admitted`: a path of valid edges from an initial node to a
    /// divergent target.
    fn admitted(&self) -> Step<(), O> {
        let witness = &self.certificate.admitted;
        let mut at = witness.start;
        let fails = |at: usize| reject(CertificateRule::WitnessFails, at_node(at));
        if !self.certificate.initial.contains(&at) || self.states[at].is_none() {
            return fails(at);
        }
        for &index in &witness.path {
            let edge = &self.certificate.edges[index];
            if edge.from != at || !self.valid.contains(&index) {
                return fails(at);
            }
            at = edge.to;
        }
        let holds = match &witness.target {
            DivergentTarget::Quiescent(node) => {
                *node == at && self.timed.tick_period().is_none() && self.idles(at)?
            }
            DivergentTarget::Terminal(node) => {
                *node == at && self.timed.tick_period().is_some() && self.idles(at)?
            }
            DivergentTarget::Cycle(cycle) => self.advancing_cycle(at, cycle)?,
        };
        if holds {
            Ok(())
        } else {
            fails(at)
        }
    }

    /// Whether `cycle` is a cycle of valid edges from and back to `at`
    /// each repetition of which lets time advance: it ticks, under a tick
    /// source; under a dense source, some clock guarded on it by a lower
    /// bound above the value its last reset before the guard set it.
    fn advancing_cycle(&self, at: usize, cycle: &[usize]) -> Step<bool, O> {
        let mut node = at;
        for &index in cycle {
            let edge = &self.certificate.edges[index];
            if edge.from != node || !self.valid.contains(&index) {
                return Ok(false);
            }
            node = edge.to;
        }
        if cycle.is_empty() || node != at {
            return Ok(false);
        }
        let steps: Vec<(&GuardedStep, usize)> = cycle
            .iter()
            .map(|&index| {
                let part = self.certificate.edges[index].part.unwrap_or_default();
                self.step_of(index).map(|step| (step, part))
            })
            .collect::<Step<_, O>>()?;
        if self.timed.tick_period().is_some() {
            return Ok(steps.iter().any(|(step, _)| {
                self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero())
            }));
        }
        let length = steps.len();
        for (position, (step, part)) in steps.iter().enumerate() {
            let Some(constraints) = step.parts().get(*part) else {
                continue;
            };
            for constraint in constraints {
                let strict_lower = match constraint.op() {
                    ClockOp::Greater => true,
                    ClockOp::GreaterOrEqual | ClockOp::Equal => false,
                    ClockOp::Less | ClockOp::LessOrEqual => continue,
                };
                // The last reset of the clock before this guard, going back
                // around the cycle; this edge's own resets come after it.
                let reset = (1..=length).find_map(|back| {
                    let (earlier, _) = steps[(position + length - back) % length];
                    earlier
                        .resets()
                        .iter()
                        .rev()
                        .find(|(place, _)| *place == constraint.place())
                        .map(|(_, value)| value.clone())
                });
                let Some(reset) = reset else {
                    continue;
                };
                let bound = constraint.bound();
                if bound > &reset || (strict_lower && bound == &reset) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// `value` in units of `1 / scale`; `None` when the scale does not make it
/// an integer.
fn scaled(value: &Rational, scale: &Integer) -> Option<Integer> {
    let product = value.mul(&Rational::from_integer(scale.clone()));
    (product.denominator().as_big() == &num_bigint::BigInt::from(1))
        .then(|| Integer::from_big(product.numerator().as_big().clone()))
}

/// The LU bounds the checker computes from the model: per clock place, the
/// largest constant a guard, invariant or reset compares it with from
/// below and from above, scaled; `None` when the scale makes one not an
/// integer.
#[allow(
    clippy::type_complexity,
    reason = "one list of lower and one of upper bounds"
)]
fn computed_bounds(
    timed: &TimedSystem<'_, '_>,
    scale: &Integer,
) -> Option<(Vec<Option<Integer>>, Vec<Option<Integer>>)> {
    let model = timed.model();
    let fields = model.clocks().len();
    let mut lower: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut upper: Vec<Vec<Rational>> = vec![Vec::new(); fields];
    let mut record = |clock: usize, op: ClockOp, bound: &Rational| {
        if clock >= fields {
            return;
        }
        if matches!(
            op,
            ClockOp::Greater | ClockOp::GreaterOrEqual | ClockOp::Equal
        ) {
            lower[clock].push(bound.clone());
        }
        if matches!(op, ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal) {
            upper[clock].push(bound.clone());
        }
    };
    for guard in model.guards() {
        for part in guard.parts() {
            for constraint in part.clocks() {
                record(
                    constraint.clock().index(),
                    constraint.op(),
                    constraint.bound(),
                );
            }
        }
    }
    for invariant in model.invariants() {
        for constraint in invariant.bounds() {
            record(
                constraint.clock().index(),
                constraint.op(),
                constraint.bound(),
            );
        }
    }
    for resets in model.resets() {
        for reset in resets.resets() {
            record(reset.clock().index(), ClockOp::Equal, reset.value());
        }
    }
    let max = |values: &[Rational]| -> Option<Option<Integer>> {
        match values.iter().max() {
            Some(value) => scaled(value, scale).map(Some),
            None => Some(None),
        }
    };
    let mut lows = Vec::new();
    let mut highs = Vec::new();
    for place in timed.places() {
        let clock = place.clock().index();
        lows.push(max(lower.get(clock)?)?);
        highs.push(max(upper.get(clock)?)?);
    }
    Some((lows, highs))
}

/// The bytes a check reads of `certificate`: each node's key and zone
/// cells, each edge, the LU bounds and the witness, at their integers'
/// magnitudes.
fn certificate_bytes<O>(certificate: &ZoneCertificate<O>) -> u64 {
    let integer = |value: &Integer| value.magnitude_bits() / 8 + 1;
    let mut bytes = integer(&certificate.scale);
    for bound in certificate
        .lu_bounds
        .lower
        .iter()
        .chain(&certificate.lu_bounds.upper)
        .flatten()
    {
        bytes = bytes.saturating_add(integer(bound));
    }
    for node in &certificate.nodes {
        bytes = bytes.saturating_add(40);
        let dim = node.zone.dim();
        for i in 0..dim {
            for j in 0..dim {
                let cell = match node.zone.bound(i, j) {
                    Ok(Some((value, _))) => integer(&value) + 1,
                    _ => 1,
                };
                bytes = bytes.saturating_add(cell);
            }
        }
    }
    let edges = u64::try_from(certificate.edges.len()).unwrap_or(u64::MAX);
    let witness = u64::try_from(certificate.admitted.path.len()).unwrap_or(u64::MAX);
    bytes
        .saturating_add(edges.saturating_mul(48))
        .saturating_add(witness.saturating_mul(8))
}
