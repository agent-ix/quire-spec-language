// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-338 (ADR-018 PC-3, PC-5, LA-3): the closure certificate EN-1 returns
//! with a safety proof, and [`check_closure`], the core's checker of it.
//!
//! The checker reads nothing from the engine but the certificate. It
//! re-admits the request's subject through `ModelSystem` (FR-120), rebuilds
//! each instance's canonical safety translation with `qsl-eval`'s code, and
//! recomputes every successor itself. It accepts exactly when, for every
//! instance of the item:
//!
//! - every initial product state is in the certificate (`InitialMissing`);
//! - every successor of a reached product state is in it
//!   (`SuccessorMissing`), so the reached set is closed under expansion;
//! - no reached product state is bad (`BadState`): its monitor state rejects,
//!   its letter is undefined, its expansion records `ContractUndetermined`,
//!   or, for the deadlock-freedom item alone, it is deadlocked (ADR-018
//!   DL-6).
//!
//! A subject with no initial state is refused: no behaviour is examined, so
//! no certificate of it is accepted. Admission polls the cancel handle at
//! every certificate entry and reads at most `max_states` entries per
//! instance. Before any key is used for membership, each certificate
//! state's key is admitted against one instance's context; a key no
//! instance admits (malformed, non-canonical, another item's, profile's or
//! activation's) rejects the whole certificate `ShapeMismatch` at the least
//! refused state, by model-state digest octets then key octets. The checker
//! expands only model states the certificate holds, so its work is bounded
//! by the certificate. A budget or a cancellation stops it with the typed
//! `Stopped` cause, which is never an acceptance.
//!
//! Over a protocol subject whose memory model runs under a bound (ADR-025
//! MB-2), the checker expands under the request's bound. When a reached
//! state's expansion cut a step at it (MB-3), the reached set is closed
//! only under the bound: a check that otherwise passes returns
//! `MemoryBoundReached` with the distinct bound-limited model states, never
//! an acceptance (MB-4), and a state whose only steps were cut is a
//! boundary state, never a deadlock.
//!
//! Over an `activation on each` protocol the checker expands under the
//! request's `max_live_instances` (ADR-027 AE-4) alike: a reached state that
//! left an `activate` step unexpanded makes the reached set closed only
//! under the instance bound, and a check that otherwise passes returns
//! `InstanceBoundReached` with the distinct instance-limited model states
//! (after `MemoryBoundReached`), never an acceptance.
//!
//! TP-1 and deadlock freedom have no future interval, so FR-338's
//! non-progress acceptance check does not apply: they keep pure bad-prefix
//! closure. A bounded `eventually[0,u]` clause (TP-2) is closed over its
//! bounded monitor's states and counted distance. Over a protocol subject the
//! steps that do not count are recorded between the product states that hold
//! its debt open, and a component of them with a cycle fair under the
//! subject's scheduler constraints and the clause's own rejects the
//! certificate `WitnessFails` at the least product state of any fair
//! component, since that cycle is a behaviour that never meets the interval.
//! The search reads each state's enabled steps from the expansion the run
//! already charged and adds no state or edge of its own; the fairness
//! kernels poll `cancel`. Remaining work: QSL-878 gives that search its own
//! typed auxiliary-exhaustion limit (FR-338-AC-6).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::automaton::{
    AutomatonStateKey, BoundedKind, BoundedState, BoundedTranslation, SafetyItem, SafetyState,
    SafetyTranslation,
};
use qsl_eval::model_subject::{ModelLetters, ModelSubject, ProtocolSubject};
use qsl_eval::simulation::model::{ModelFinding, ModelSimState, ModelSystem, TerminalClass};
use qsl_eval::simulation::protocol::{
    activates_on_each, check_protocol_bounds, live_instance_bound, ProtocolFairnessError,
    ProtocolRefusal, ProtocolSystem, Sc, SubjectMemory,
};
use qsl_eval::simulation::{
    Cut, EncodingRefusal, Expansion, ExpansionStop, NotSimulated, SubjectSystem, TransitionSystem,
    ZeroProgress, ZeroProgressError, BOUND_POSITIONS,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::InternalFault;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Cancel, Meter, ScalarLimits};
use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::certificate::check::{
    admit, expansion_halt, finish, poll, stopped, widen, Budgets, Checker, Halt, Step,
};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::identity::ClauseRootRefusal;
use crate::model_check::item::{Instance, ItemRefusal, ModelCheckItem, Translation};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};
use crate::proof_result::{IncompleteCause, InstanceBound, MemoryBound, MemoryBoundMember};

/// One product state of a certificate: its model state's
/// `quire.simulation.state-key/v1` digest and its automaton state's
/// canonical key.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProductStateRef {
    /// The model state's state-key digest.
    pub model_state: DigestRecord,
    /// The translation's canonical key of the automaton state.
    pub automaton_state: AutomatonStateKey,
}

impl Serialize for ProductStateRef {
    /// `{"model_state": {"domain", "digest"}, "automaton_state": <base64url
    /// of the key octets>}`; base64url is FR-331's transport of the octets.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use base64::Engine as _;
        let mut object = serializer.serialize_struct("ProductStateRef", 2)?;
        object.serialize_field("model_state", &DigestWire::of(&self.model_state))?;
        let octets = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(self.automaton_state.as_bytes());
        object.serialize_field("automaton_state", &octets)?;
        object.end()
    }
}

/// A digest record's wire object: `{"domain", "digest"}`.
#[derive(Serialize)]
pub(crate) struct DigestWire {
    domain: &'static str,
    digest: String,
}

impl DigestWire {
    pub(crate) fn of(record: &DigestRecord) -> Self {
        Self {
            domain: record.domain().as_str(),
            digest: record.hex(),
        }
    }
}

/// FR-338's closure certificate: the product states EN-1 explored.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClosureCertificate {
    /// The product states, in any order.
    pub states: Vec<ProductStateRef>,
}

/// A certificate rule that failed and where (wire `certificate-rejected{rule,
/// at}`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertificateRejection {
    /// The failed rule.
    pub rule: CertificateRule,
    /// Where it failed.
    pub at: CertificateLocus,
}

/// What a temporal certificate checker returns (FR-338 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalCertificateCheck {
    /// Every required check completed and passed.
    Accepted,
    /// A certificate rule failed.
    Rejected(CertificateRejection),
    /// A budget or a cancellation stopped the check before it completed:
    /// `LimitReached{limit, value}` or `Cancelled{source}`.
    Stopped(IncompleteCause),
    /// The check closed, but at `states` distinct reached model states the
    /// memory model cut a step at `bound` (ADR-025 MB-3, MB-4): the reached
    /// set is closed only under the bound, so it is never an acceptance.
    MemoryBoundReached {
        /// The bound the subject's memory model ran under.
        bound: MemoryBound,
        /// The distinct bound-limited model states the check reached.
        states: u64,
    },
    /// The check closed, but at `states` distinct reached model states an
    /// `activate` step stayed unexpanded at `bound` (ADR-027 AE-4): the
    /// reached set is closed only under the instance bound, so it is never
    /// an acceptance.
    InstanceBoundReached {
        /// The instance bound the subject ran under.
        bound: InstanceBound,
        /// The distinct instance-limited model states the check reached.
        states: u64,
    },
}

/// FR-338's certificate request: the model subject, re-admitted by the
/// checker itself, and the item.
pub struct CertificateRequest<'a> {
    /// The model subject (FR-125).
    pub subject: ModelSubject<'a>,
    /// The item.
    pub item: ModelCheckItem<'a>,
}

/// Why [`check_closure`] checked nothing: the request, not the certificate,
/// is at fault.
#[derive(Debug, thiserror::Error)]
pub enum ClosureRefusal {
    /// FR-106's admission of the subject's `ModelSystem` failed.
    #[error("the subject's model system did not admit: {0:?}")]
    Admission(Box<AdmissionFailure>),
    /// A root of the subject is unbounded, or its extent is unclassified.
    #[error(transparent)]
    NotSimulated(Box<NotSimulated>),
    /// The item has no canonical translation.
    #[error(transparent)]
    Item(#[from] ItemRefusal),
    /// The item has no instance (FR-125), so there is nothing to certify.
    #[error("the item has no instance")]
    NoInstance,
    /// The subject has no initial state (ADR-016 EX-10), so no behaviour is
    /// examined and there is nothing to certify.
    #[error("the subject has no initial state")]
    NoInitialState,
    /// A state identity or an automaton key has no canonical encoding.
    #[error("a state or automaton key has no canonical encoding")]
    Encoding,
    /// The subject's protocol system does not build (FR-205).
    #[error(transparent)]
    Protocol(ProtocolRefusal),
    /// A fairness constraint of the item names an operation the subject does
    /// not admit.
    #[error(transparent)]
    Fairness(ProtocolFairnessError),
    /// A refinement whose abstract side is a protocol clause: no abstract
    /// protocol step is decided yet (FR-148)
    /// (`unsupported_construct/not-yet-implemented`).
    #[error("unsupported_construct/not-yet-implemented: a refinement over an abstract protocol subject is not decided yet")]
    ProtocolAbstractSide,
    /// An invariant of the subject broke.
    #[error("the closure check broke invariant {0:?}")]
    Fault(InternalFault),
}

impl From<EncodingRefusal> for ClosureRefusal {
    fn from(_: EncodingRefusal) -> Self {
        Self::Encoding
    }
}

/// FR-338: check `certificate` for `request`'s item, under `limits` and the
/// subject's own evaluation meter and candidate bound, polling `cancel` at
/// every charge.
///
/// # Errors
///
/// [`ClosureRefusal`] when the subject does not admit, a root is unbounded,
/// the item has no translation or no instance, or a state cannot be
/// encoded. A refusal is never an acceptance.
pub fn check_closure(
    request: &CertificateRequest<'_>,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    let simulation = request.subject.simulation.clone();
    let system = admit(simulation)?;
    check_system(
        &system,
        &system,
        Bounds::default(),
        &request.subject,
        &request.item,
        certificate,
        limits,
        cancel,
    )
}

/// FR-338's certificate request over a protocol subject (FR-215, ADR-027
/// TS-6): the protocol subject, re-admitted by the checker itself, and the
/// item.
pub struct ProtocolCertificateRequest<'a> {
    /// The protocol subject (FR-205).
    pub subject: ProtocolSubject<'a>,
    /// The item.
    pub item: ModelCheckItem<'a>,
    /// The `max_store_buffer` the run used; `None` for the default 4
    /// (FR-225).
    pub max_store_buffer: Option<u64>,
    /// The `max_live_instances` the run used; `None` for the default 3
    /// (ADR-027 AE-4).
    pub max_live_instances: Option<u64>,
}

/// FR-338 over a protocol subject: [`check_closure`]'s check, with the
/// successors and terminal classes recomputed by the subject's own
/// protocol system under its resolved memory models (FR-219).
///
/// # Errors
///
/// [`ClosureRefusal`] as [`check_closure`] returns it, and
/// [`ClosureRefusal::Protocol`] when the protocol system does not build.
pub fn check_protocol_closure(
    request: &ProtocolCertificateRequest<'_>,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    let model = admit(request.subject.model.simulation.clone())?;
    let protocol = request
        .subject
        .model
        .simulation
        .package
        .graph()
        .protocol(&request.subject.protocol)
        .ok_or_else(|| {
            ClosureRefusal::Protocol(ProtocolRefusal::UnknownProtocol {
                protocol: request.subject.protocol.clone(),
            })
        })?;
    check_protocol_bounds(&model, protocol, BOUND_POSITIONS)
        .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
    let subject = &request.subject;
    let item = &request.item;
    let live_instances = live_instance_bound(request.max_live_instances);
    let instance = activates_on_each(protocol).then_some(InstanceBound {
        value: live_instances,
    });
    match SubjectMemory::of(&subject.memory, request.max_store_buffer) {
        SubjectMemory::Sc => {
            let system = ProtocolSystem::new(model, subject, Sc)
                .map_err(ClosureRefusal::Protocol)?
                .with_max_live_instances(live_instances);
            check_system(
                &system,
                system.model(),
                Bounds {
                    memory: None,
                    instance,
                },
                &subject.model,
                item,
                certificate,
                limits,
                cancel,
            )
        }
        SubjectMemory::Tso(tso) => {
            let bound = MemoryBound {
                member: MemoryBoundMember::MaxStoreBuffer,
                value: tso.max_store_buffer(),
            };
            let system = ProtocolSystem::new(model, subject, tso)
                .map_err(ClosureRefusal::Protocol)?
                .with_max_live_instances(live_instances);
            check_system(
                &system,
                system.model(),
                Bounds {
                    memory: Some(bound),
                    instance,
                },
                &subject.model,
                item,
                certificate,
                limits,
                cancel,
            )
        }
        SubjectMemory::Ra(ra) => {
            let bound = MemoryBound {
                member: MemoryBoundMember::MaxMessages,
                value: ra.max_messages(),
            };
            let system = ProtocolSystem::new(model, subject, ra)
                .map_err(ClosureRefusal::Protocol)?
                .with_max_live_instances(live_instances);
            check_system(
                &system,
                system.model(),
                Bounds {
                    memory: Some(bound),
                    instance,
                },
                &subject.model,
                item,
                certificate,
                limits,
                cancel,
            )
        }
    }
}

/// The method bounds a checked subject runs under (ADR-025 MB-2, ADR-027
/// AE-4); `None` for a bound the subject has none of.
#[derive(Clone, Copy, Default)]
struct Bounds {
    memory: Option<MemoryBound>,
    instance: Option<InstanceBound>,
}

/// [`check_closure`] over `system`, whose model is `model` and whose memory
/// and instance bounds are `bounds`.
#[allow(clippy::too_many_arguments)]
fn check_system<S: SubjectSystem>(
    system: &S,
    model: &ModelSystem<'_>,
    bounds: Bounds,
    subject: &ModelSubject<'_>,
    item: &ModelCheckItem<'_>,
    certificate: &ClosureCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    let meter = subject.simulation.meter;
    let budgets = Budgets::of(&subject.simulation);
    let instances = match item.instances(subject, model, cancel) {
        Ok(instances) => instances,
        Err(ItemRefusal::ClauseRoot(ClauseRootRefusal::Cancelled(cause))) => {
            return Ok(TemporalCertificateCheck::Stopped(
                IncompleteCause::Cancelled { source: cause },
            ));
        }
        Err(ItemRefusal::Unbounded(refusal)) => {
            return Err(ClosureRefusal::NotSimulated(Box::new(refusal)))
        }
        Err(refusal) => return Err(refusal.into()),
    };
    if instances.is_empty() {
        return Err(ClosureRefusal::NoInstance);
    }
    if system.initial().is_empty() {
        return Err(ClosureRefusal::NoInitialState);
    }

    // Admission reads at most one entry per product state the state budget
    // admits for each instance, and polls the handle at every entry.
    let entries = limits.limits.max_states.saturating_mul(instances.len());
    let mut members = BTreeSet::new();
    let mut models = BTreeSet::new();
    let mut refused: Option<ProductStateRef> = None;
    for (read, state) in certificate.states.iter().enumerate() {
        if let Err(halt) = poll(cancel) {
            return finish(Err(halt));
        }
        if read >= entries {
            return finish(Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(limits.limits.max_states),
            )));
        }
        let admitted = state.model_state.domain() == DigestDomain::SimulationStateKeyV1
            && instances.iter().any(|instance| {
                instance
                    .translation
                    .admits(state.automaton_state.as_bytes())
            });
        if !admitted {
            let order = |state: &ProductStateRef| {
                (
                    *state.model_state.as_bytes(),
                    state.automaton_state.as_bytes().to_vec(),
                )
            };
            if refused
                .as_ref()
                .is_none_or(|least: &ProductStateRef| order(state) < order(least))
            {
                refused = Some(state.clone());
            }
            continue;
        }
        members.insert(state.clone());
        models.insert(state.model_state);
    }
    // FR-338: one refused key rejects the whole certificate, at the least
    // refused state by model-state digest octets then key octets, so the
    // locus does not depend on the order of `states`.
    if let Some(least) = refused {
        return Ok(rejected(CertificateRule::ShapeMismatch, least));
    }

    let mut check = Check {
        system,
        item,
        limits,
        meter,
        budgets,
        cancel,
        members: &members,
        models: &models,
        reached: 0,
        edges: 0,
        keys: BTreeSet::new(),
        limited: BTreeSet::new(),
        instance_limited: BTreeSet::new(),
    };
    let run = check.run(&instances);
    // ADR-025 MB-4: a reached set closed only because the memory bound cut
    // steps is never accepted, so the item never settles `proved` over it.
    if run.is_ok() && !check.limited.is_empty() {
        return match bounds.memory {
            Some(bound) => Ok(TemporalCertificateCheck::MemoryBoundReached {
                bound,
                states: widen(check.limited.len()),
            }),
            None => Err(fault("memory-bound-cut-without-bound")),
        };
    }
    // ADR-027 AE-4, after the memory bound's: likewise for the instance
    // bound.
    if run.is_ok() && !check.instance_limited.is_empty() {
        return match bounds.instance {
            Some(bound) => Ok(TemporalCertificateCheck::InstanceBoundReached {
                bound,
                states: widen(check.instance_limited.len()),
            }),
            None => Err(fault("instance-bound-cut-without-bound")),
        };
    }
    finish(run)
}

fn rejected(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at),
    })
}

/// A reached product state waiting to be expanded.
struct Pending<S: SubjectSystem> {
    state: S::State,
    at: ProductStateRef,
    expansion: Option<SubjectExpansion<S>>,
}

/// An initial state of the subject: its key bytes, its position in the
/// subject's list, its digest and the state.
type Initial<S> = (Vec<u8>, usize, DigestRecord, <S as TransitionSystem>::State);

/// A reached product state of a bounded instance waiting to be expanded.
struct BoundedPending<S: SubjectSystem> {
    state: S::State,
    at: ProductStateRef,
    automaton: BoundedState,
    /// The counted distance from position 0, held at one past the horizon
    /// once it is passed.
    distance: u64,
}

/// A product state of a bounded instance, as the run tells two apart.
type BoundedKey = (DigestRecord, BoundedState, u64);

/// A successor of an expansion with its state-key digest.
type Keyed<S> = (
    <S as TransitionSystem>::TransitionId,
    DigestRecord,
    <S as TransitionSystem>::State,
);

/// The steps that do not count between the product states of a bounded
/// instance that hold an `eventually` debt open (FR-338 "Protocol
/// non-progress acceptance"), with the model state of each end.
struct ZeroEdges<S: SubjectSystem> {
    /// The transitions each such state enables, from the expansion the run
    /// already charged.
    enabled: BTreeMap<BoundedKey, Vec<S::TransitionId>>,
    edges: Vec<(BoundedKey, BoundedKey, S::TransitionId)>,
}

/// What a bounded instance's run keeps: its translation, its `holds` atom's
/// binding and the product states it has reached.
struct BoundedRun<'i> {
    translation: &'i BoundedTranslation,
    letters: &'i dyn ModelLetters,
    binding: &'i qsl_eval::model_subject::ModelInstance,
    visited: BTreeSet<BoundedKey>,
    /// The atom at each model state read so far; `None` is undefined.
    readings: BTreeMap<DigestRecord, Option<bool>>,
}

/// One expansion of a subject's state, and what a method bound left of it
/// unexpanded.
type SubjectExpansion<S> = (
    Expansion<<S as TransitionSystem>::TransitionId, <S as TransitionSystem>::State, ModelFinding>,
    Cut,
);

struct Check<'c, S: SubjectSystem> {
    system: &'c S,
    item: &'c ModelCheckItem<'c>,
    limits: &'c ModelCheckLimits,
    meter: ScalarLimits,
    budgets: Budgets,
    cancel: &'c Cancel,
    members: &'c BTreeSet<ProductStateRef>,
    models: &'c BTreeSet<DigestRecord>,
    reached: usize,
    edges: usize,
    keys: BTreeSet<AutomatonStateKey>,
    /// The reached model states whose expansion cut a step at the memory
    /// bound (ADR-025 MB-3).
    limited: BTreeSet<DigestRecord>,
    /// The reached model states whose expansion left an `activate` step
    /// unexpanded at the instance bound (ADR-027 AE-4).
    instance_limited: BTreeSet<DigestRecord>,
}

impl<S: SubjectSystem> Check<'_, S> {
    fn reject(rule: CertificateRule, at: ProductStateRef) -> Halt {
        Halt::Check(rejected(rule, at))
    }

    fn poll(&self) -> Step<()> {
        poll(self.cancel)
    }

    fn run(&mut self, instances: &[Instance]) -> Step<()> {
        self.poll()?;
        let mut initial = Vec::new();
        for (index, state) in self.system.initial().into_iter().enumerate() {
            let (key, digest) = self.system.state_key(&state)?;
            initial.push((key, index, digest, state));
        }
        initial.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        initial.dedup_by(|later, earlier| later.0 == earlier.0);
        for instance in instances {
            match &instance.translation {
                Translation::Safety(translation) => {
                    self.run_safety(instance, translation, &initial)?;
                }
                Translation::Bounded(translation) => {
                    self.run_bounded(instance, translation, &initial)?;
                }
            }
        }
        Ok(())
    }

    /// The closure of a safety item's instance: every reached model state
    /// stays under the carried root.
    fn run_safety(
        &mut self,
        instance: &Instance,
        translation: &SafetyTranslation,
        initial: &[Initial<S>],
    ) -> Step<()> {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        for (_, _, digest, state) in initial {
            let before = None;
            if let Some(pending) = self.reach(
                instance,
                translation,
                state.clone(),
                *digest,
                before,
                CertificateRule::InitialMissing,
                &mut visited,
            )? {
                queue.push_back(pending);
            }
        }
        while let Some(pending) = queue.pop_front() {
            self.poll()?;
            let (expansion, _) = match pending.expansion {
                Some(expansion) => expansion,
                None => self.expand(&pending.state, &pending.at)?,
            };
            for (_, digest, state) in self.ordered(expansion.successors)? {
                self.poll()?;
                self.charge_edge()?;
                if let Some(next) = self.reach(
                    instance,
                    translation,
                    state,
                    digest,
                    Some(SafetyState::Carried),
                    CertificateRule::SuccessorMissing,
                    &mut visited,
                )? {
                    queue.push_back(next);
                }
            }
        }
        Ok(())
    }

    /// The closure of a bounded instance (TP-2): every product state of a
    /// model state, its bounded monitor state and, up to one past the
    /// horizon, its counted distance, from the initial states.
    fn run_bounded(
        &mut self,
        instance: &Instance,
        translation: &BoundedTranslation,
        initial: &[Initial<S>],
    ) -> Step<()> {
        let ModelCheckItem::Clause { letters, .. } = self.item else {
            return Err(fault("clause-item-translation").into());
        };
        let binding = instance
            .binding
            .as_ref()
            .ok_or_else(|| fault("clause-instance-binding"))?;
        let mut run = BoundedRun {
            translation,
            letters: *letters,
            binding,
            visited: BTreeSet::new(),
            readings: BTreeMap::new(),
        };
        let mut queue = VecDeque::new();
        let mut zero = ZeroEdges::<S> {
            enabled: BTreeMap::new(),
            edges: Vec::new(),
        };
        for (_, _, digest, state) in initial {
            let (pending, _, _) = self.reach_bounded(
                &mut run,
                state.clone(),
                *digest,
                None,
                CertificateRule::InitialMissing,
            )?;
            queue.extend(pending);
        }
        // Only an `eventually` debt can stay open on a loop of steps that
        // do not count: a surviving `always` obligation discharges at its
        // closed boundary and fails on no letter the loop reads twice.
        let owes = translation.kind() == BoundedKind::Eventually;
        while let Some(pending) = queue.pop_front() {
            self.poll()?;
            let (expansion, cut) = self.expand(&pending.state, &pending.at)?;
            if owes && matches!(pending.automaton, BoundedState::Open { .. }) {
                let key = (pending.at.model_state, pending.automaton, pending.distance);
                let enabled = expansion
                    .successors
                    .iter()
                    .map(|(transition, _)| transition.clone())
                    .collect();
                zero.enabled.insert(key, enabled);
            }
            if expansion.successors.is_empty() {
                // A state whose every step the memory or instance bound cut
                // is a boundary state, never closed (ADR-025 MB-3, ADR-027
                // AE-4); any other terminal state closes the monitor by the
                // bounded profile's closed-boundary rule (QSpec FR-091).
                if !(cut.memory || cut.instances)
                    && translation.terminal(pending.automaton).is_rejecting()
                {
                    return Err(Self::reject(CertificateRule::BadState, pending.at));
                }
                continue;
            }
            for (transition, digest, state) in self.ordered(expansion.successors)? {
                self.poll()?;
                self.charge_edge()?;
                let counted = self.system.counts_step(&transition);
                let holds_open =
                    owes && !counted && matches!(pending.automaton, BoundedState::Open { .. });
                let (next, after, distance) = self.reach_bounded(
                    &mut run,
                    state,
                    digest,
                    Some((pending.automaton, pending.distance, counted)),
                    CertificateRule::SuccessorMissing,
                )?;
                queue.extend(next);
                if holds_open && matches!(after, BoundedState::Open { .. }) {
                    let from = (pending.at.model_state, pending.automaton, pending.distance);
                    zero.edges
                        .push((from, (digest, after, distance), transition));
                }
            }
        }
        if let Some(at) = self.zero_progress(&run, zero)? {
            return Err(Self::reject(CertificateRule::WitnessFails, at));
        }
        Ok(())
    }

    /// FR-338 "Protocol non-progress acceptance": a cycle of steps that do
    /// not count among the product states that hold an `eventually` debt
    /// open, fair under the subject's scheduler constraints and the item's
    /// own (ADR-019), is a behaviour that never meets the interval, so the
    /// closure is no proof. The product state of the least state of the
    /// first such component, by product state, is where the check fails.
    fn zero_progress(
        &mut self,
        run: &BoundedRun<'_>,
        zero: ZeroEdges<S>,
    ) -> Step<Option<ProductStateRef>> {
        if zero.edges.is_empty() {
            return Ok(None);
        }
        let ModelCheckItem::Clause { clause, .. } = self.item else {
            return Err(fault("clause-item-translation").into());
        };
        let mut graph = ZeroProgress::<S>::new();
        let mut vertex_of = BTreeMap::new();
        let mut keys = Vec::with_capacity(zero.enabled.len());
        for (key, enabled) in &zero.enabled {
            vertex_of.insert(*key, graph.vertex(self.system, enabled)?);
            keys.push(*key);
        }
        for (from, to, transition) in &zero.edges {
            let (Some(from), Some(to)) = (vertex_of.get(from), vertex_of.get(to)) else {
                return Err(fault("an-edge-joins-two-states").into());
            };
            graph.edge(self.system, *from, *to, transition)?;
        }
        let found = match graph.fair_loop(self.system, clause.fairness(), self.cancel) {
            Ok(found) => found,
            Err(ZeroProgressError::Cancelled) => {
                self.poll()?;
                return Err(fault("a-cancelled-run-has-a-cause").into());
            }
            Err(ZeroProgressError::Encoding(refusal)) => return Err(refusal.into()),
            Err(ZeroProgressError::Fairness(error)) => {
                return Err(ClosureRefusal::Fairness(error).into())
            }
            Err(ZeroProgressError::Graph) => {
                return Err(fault("the-zero-progress-graph-is-built-from-states").into())
            }
        };
        let Some(found) = found else {
            return Ok(None);
        };
        // FR-338: the first fair component by product-state order, which is
        // the least product state of any fair component.
        let mut least: Option<ProductStateRef> = None;
        for vertex in &found.fair_vertices {
            let key = keys
                .get(vertex.index())
                .ok_or_else(|| fault("a-vertex-is-a-state"))?;
            let at = self.bounded_ref(run, key)?;
            if least.as_ref().is_none_or(|held| at < *held) {
                least = Some(at);
            }
        }
        Ok(least)
    }

    /// The certificate's reference to the bounded product state `key`.
    fn bounded_ref(&mut self, run: &BoundedRun<'_>, key: &BoundedKey) -> Step<ProductStateRef> {
        let automaton = run
            .translation
            .key(key.1)
            .map_err(|_| ClosureRefusal::Encoding)?;
        self.product_of(key.0, automaton)
    }

    /// Reach `state`, whose digest is `digest`, from the monitor state and
    /// counted distance `before` (`None` at position zero) by a step that
    /// counts or not: read its letter when the claim reads its distance,
    /// reject a bad state, then require it in the certificate under
    /// `missing`. The state is pending unless this instance reached it
    /// before; the monitor state and counted distance it was read to come
    /// back either way.
    fn reach_bounded(
        &mut self,
        run: &mut BoundedRun<'_>,
        state: S::State,
        digest: DigestRecord,
        before: Option<(BoundedState, u64, bool)>,
        missing: CertificateRule,
    ) -> Step<(Option<BoundedPending<S>>, BoundedState, u64)> {
        let horizon = run.translation.horizon();
        let distance = match before {
            None => 0,
            Some((_, distance, counted)) => distance
                .saturating_add(u64::from(counted))
                .min(horizon.saturating_add(1)),
        };
        // A position past the horizon is never read (ADR-018 UE-1), and a
        // step that counts out of the window closes it before its letter.
        let holds = if distance <= horizon {
            let reading = match run.readings.get(&digest) {
                Some(reading) => *reading,
                None => {
                    let reading = self.atom(
                        run.letters,
                        run.translation.atom(),
                        run.binding,
                        &self.system.atom_state(&state),
                    )?;
                    run.readings.insert(digest, reading);
                    reading
                }
            };
            match reading {
                Some(holds) => holds,
                None => {
                    let key = run
                        .translation
                        .key(BoundedState::Reject)
                        .map_err(|_| ClosureRefusal::Encoding)?;
                    let at = self.product_of(digest, key)?;
                    return Err(Self::reject(CertificateRule::BadState, at));
                }
            }
        } else {
            false
        };
        let after = match before {
            None => run.translation.start(holds),
            Some((monitor, _, counted)) => run.translation.step(monitor, counted, holds),
        };
        let key = run
            .translation
            .key(after)
            .map_err(|_| ClosureRefusal::Encoding)?;
        let at = self.product_of(digest, key)?;
        if after.is_rejecting() {
            return Err(Self::reject(CertificateRule::BadState, at));
        }
        if run.visited.contains(&(digest, after, distance)) {
            return Ok((None, after, distance));
        }
        if self.reached >= self.limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        self.reached += 1;
        run.visited.insert((digest, after, distance));
        if !self.members.contains(&at) {
            return Err(Self::reject(missing, at));
        }
        let pending = BoundedPending {
            state,
            at,
            automaton: after,
            distance,
        };
        Ok((Some(pending), after, distance))
    }

    /// The successors of an expansion in FR-101's canonical order, each
    /// with its state-key digest.
    fn ordered(&self, successors: Vec<(S::TransitionId, S::State)>) -> Step<Vec<Keyed<S>>> {
        let mut keyed = Vec::with_capacity(successors.len());
        for (transition, state) in successors {
            let order = self.system.transition_key(&transition)?;
            let (key, digest) = self.system.state_key(&state)?;
            keyed.push((order, key, digest, transition, state));
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        Ok(keyed
            .into_iter()
            .map(|(_, _, digest, transition, state)| (transition, digest, state))
            .collect())
    }

    /// Charge one newly materialized edge against `max_transitions`.
    fn charge_edge(&mut self) -> Step<()> {
        if self.edges >= self.limits.limits.max_transitions {
            return Err(stopped(
                ModelCheckLimit::MaxTransitions,
                widen(self.limits.limits.max_transitions),
            ));
        }
        self.edges += 1;
        Ok(())
    }

    /// Reach `state`, whose digest is `digest`, from the automaton state
    /// `before` (`None` at position zero): read its letter, reject a bad
    /// state, then require it in the certificate under `missing`. `None`
    /// when this instance reached it before.
    #[allow(clippy::too_many_arguments)]
    fn reach(
        &mut self,
        instance: &Instance,
        translation: &SafetyTranslation,
        state: S::State,
        digest: DigestRecord,
        before: Option<SafetyState>,
        missing: CertificateRule,
        visited: &mut BTreeSet<DigestRecord>,
    ) -> Step<Option<Pending<S>>> {
        if visited.contains(&digest) {
            return Ok(None);
        }
        let (after, expansion) = match translation.item() {
            SafetyItem::ReachableInvariant { atom } => {
                let ModelCheckItem::Clause { letters, .. } = self.item else {
                    return Err(fault("clause-item-translation").into());
                };
                let bound = instance
                    .binding
                    .as_ref()
                    .ok_or_else(|| fault("clause-instance-binding"))?;
                match self.atom(*letters, atom, bound, &self.system.atom_state(&state))? {
                    Some(holds) => (translation.read(before, holds), None),
                    None => {
                        let at = self.product(translation, digest, SafetyState::Rejected)?;
                        return Err(Self::reject(CertificateRule::BadState, at));
                    }
                }
            }
            SafetyItem::DeadlockFreedom => {
                if !self.models.contains(&digest) {
                    let at = self.product(translation, digest, SafetyState::Carried)?;
                    return Err(Self::reject(missing, at));
                }
                let carried = self.product(translation, digest, SafetyState::Carried)?;
                let (expansion, cut) = self.expand(&state, &carried)?;
                // ADR-025 MB-3: a state whose only enabled steps were cut at
                // the memory bound is a boundary state, never a deadlock; the
                // check then settles `MemoryBoundReached`.
                let boundary = expansion.successors.is_empty() && self.limited.contains(&digest);
                let class = if boundary {
                    TerminalClass::NotTerminal
                } else {
                    self.system
                        .terminal_class(&state, &expansion, cut)
                        .map_err(|stop| self.expansion_stop(stop))?
                };
                let holds = match class {
                    TerminalClass::NotTerminal | TerminalClass::Intended => true,
                    TerminalClass::Deadlocked => false,
                    TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                        let at = self.product(translation, digest, SafetyState::Rejected)?;
                        return Err(Self::reject(CertificateRule::BadState, at));
                    }
                };
                (translation.read(before, holds), Some((expansion, cut)))
            }
        };
        let at = self.product(translation, digest, after)?;
        if after.is_rejecting() {
            return Err(Self::reject(CertificateRule::BadState, at));
        }
        if self.reached >= self.limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        self.reached += 1;
        visited.insert(digest);
        if !self.members.contains(&at) {
            return Err(Self::reject(missing, at));
        }
        Ok(Some(Pending {
            state,
            at,
            expansion,
        }))
    }

    /// Expand the reached product state `at`: a contract conjunction the
    /// expansion could not determine makes it bad (ADR-018 UE-6, V-6). A
    /// step the memory model cut at its bound records `at`'s model state as
    /// bound-limited (ADR-025 MB-3), and one the instance bound left an
    /// `activate` step of unexpanded as instance-limited (ADR-027 AE-4).
    fn expand(&mut self, state: &S::State, at: &ProductStateRef) -> Step<SubjectExpansion<S>> {
        self.poll()?;
        let (expansion, cut) = self
            .system
            .successors_bounded(state)
            .map_err(|stop| self.expansion_stop(stop))?;
        if cut.memory {
            self.limited.insert(at.model_state);
        }
        if cut.instances {
            self.instance_limited.insert(at.model_state);
        }
        if expansion.findings.iter().any(|finding| {
            matches!(
                finding,
                ModelFinding::ContractUndetermined { .. }
                    | ModelFinding::ProtocolExpressionUndetermined { .. }
            )
        }) {
            return Err(Self::reject(CertificateRule::BadState, at.clone()));
        }
        Ok((expansion, cut))
    }

    /// The product state of `digest` and `state` under `instance`'s
    /// context, charging a newly materialized automaton key.
    fn product(
        &mut self,
        translation: &SafetyTranslation,
        digest: DigestRecord,
        state: SafetyState,
    ) -> Step<ProductStateRef> {
        let key = translation
            .key(state)
            .map_err(|_| ClosureRefusal::Encoding)?;
        self.product_of(digest, key)
    }

    /// The product state of `digest` and the automaton key `key`, charging
    /// a newly materialized key.
    fn product_of(
        &mut self,
        digest: DigestRecord,
        key: AutomatonStateKey,
    ) -> Step<ProductStateRef> {
        if !self.keys.contains(&key) {
            let count = widen(self.keys.len());
            if count >= self.limits.max_automaton_states {
                return Err(stopped(
                    ModelCheckLimit::MaxAutomatonStates,
                    self.limits.max_automaton_states,
                ));
            }
            self.keys.insert(key.clone());
        }
        Ok(ProductStateRef {
            model_state: digest,
            automaton_state: key,
        })
    }

    /// The `holds` atom `atom` for the bound `instance` at `state`: `None`
    /// when it is undefined.
    fn atom(
        &self,
        letters: &dyn ModelLetters,
        atom: qsl_semantics::check::FormulaNodeId,
        instance: &qsl_eval::model_subject::ModelInstance,
        state: &ModelSimState,
    ) -> Step<Option<bool>> {
        self.poll()?;
        let mut meter = Meter::new(self.meter);
        match letters.atom(atom, instance, state, &mut meter) {
            Ok(AtomReading::True) => Ok(Some(true)),
            Ok(AtomReading::False) => Ok(Some(false)),
            Ok(AtomReading::Undefined(_)) => Ok(None),
            Err(_) => Err(stopped(
                ModelCheckLimit::EvaluationMeter,
                self.meter.work_units,
            )),
        }
    }

    fn expansion_stop(&self, stop: ExpansionStop) -> Halt {
        expansion_halt(&stop, self.budgets, Checker::Closure)
    }
}

fn fault(invariant: &'static str) -> ClosureRefusal {
    Checker::Closure.fault(invariant)
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use ix_trace_rs::trace;
    use qsl_eval::automaton::{SafetyState, SafetyTranslation};

    use super::*;

    /// A product-state locus is written as its model-state digest record
    /// and its automaton key's octets in unpadded base64url, which decode
    /// back to the exact key.
    #[trace("TC-525")]
    #[test]
    fn a_product_state_locus_writes_its_digest_and_key_octets() {
        let key = SafetyTranslation::deadlock_freedom()
            .and_then(|translation| translation.key(SafetyState::Carried))
            .expect("the key encodes");
        let at = CertificateLocus::ProductState(ProductStateRef {
            model_state: DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [1; 32]),
            automaton_state: key.clone(),
        });
        let written = serde_json::to_value(&at).expect("serializes");
        assert_eq!(
            written["model_state"],
            serde_json::json!({
                "domain": "quire.simulation.state-key/v1",
                "digest": "01".repeat(32),
            })
        );
        let octets = written["automaton_state"].as_str().expect("a string");
        assert!(!octets.contains(['=', '+', '/']), "{octets}");
        assert_eq!(
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(octets)
                .expect("base64url"),
            key.as_bytes()
        );
    }
}
