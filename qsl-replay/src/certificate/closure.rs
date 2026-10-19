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
//! Both items EN-1 decides here, TP-1 and deadlock freedom, have no future
//! interval, so FR-338's non-progress acceptance check does not apply: they
//! keep pure bad-prefix closure.

use std::collections::{BTreeSet, VecDeque};

use qsl_eval::automaton::{AutomatonStateKey, SafetyItem, SafetyState};
use qsl_eval::model_subject::{ModelLetters, ModelSubject};
use qsl_eval::simulation::model::{
    check_model_bounds, ModelFinding, ModelSimState, ModelSystem, TerminalClass,
};
use qsl_eval::simulation::{
    EncodingRefusal, Expansion, ExpansionLimit, ExpansionStop, NotSimulated, TransitionSystem,
};
use qsl_eval::temporal::AtomReading;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::InternalFault;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{Cancel, Meter, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;
use serde::{ser::SerializeStruct, Serialize, Serializer};

use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::item::{Instance, ItemRefusal, ModelCheckItem};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};
use crate::proof_result::IncompleteCause;

/// The type positions the requires-bound pre-check may visit, as EN-1's.
const BOUND_POSITIONS: u64 = 1 << 20;

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
        object.serialize_field(
            "model_state",
            &DigestWire {
                domain: self.model_state.domain().as_str(),
                digest: self.model_state.hex(),
            },
        )?;
        let octets = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(self.automaton_state.as_bytes());
        object.serialize_field("automaton_state", &octets)?;
        object.end()
    }
}

#[derive(Serialize)]
struct DigestWire {
    domain: &'static str,
    digest: String,
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
    let instances = request.item.instances()?;
    if instances.is_empty() {
        return Err(ClosureRefusal::NoInstance);
    }
    let simulation = request.subject.simulation.clone();
    let meter = simulation.meter;
    let max_candidates = simulation.expansion.max_candidates;
    let system = ModelSystem::new(simulation)
        .map_err(|failure| ClosureRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS)
        .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
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
        if cancel.poll() {
            if let Some(source) = cancel.cause() {
                return Ok(TemporalCertificateCheck::Stopped(
                    IncompleteCause::Cancelled { source },
                ));
            }
        }
        if read >= entries {
            return Ok(TemporalCertificateCheck::Stopped(
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxStates,
                    value: widen(limits.limits.max_states),
                },
            ));
        }
        let admitted = state.model_state.domain() == DigestDomain::SimulationStateKeyV1
            && instances.iter().any(|instance| {
                instance
                    .translation
                    .admit(state.automaton_state.as_bytes())
                    .is_ok()
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
        system: &system,
        item: &request.item,
        limits,
        meter,
        max_candidates,
        cancel,
        members: &members,
        models: &models,
        reached: 0,
        edges: 0,
        keys: BTreeSet::new(),
    };
    match check.run(&instances) {
        Ok(()) => Ok(TemporalCertificateCheck::Accepted),
        Err(Halt::Check(result)) => Ok(result),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

fn rejected(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at),
    })
}

/// A finished check: a rejection or a stop, or a refusal.
enum Halt {
    Check(TemporalCertificateCheck),
    Refusal(ClosureRefusal),
}

impl From<ClosureRefusal> for Halt {
    fn from(refusal: ClosureRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<EncodingRefusal> for Halt {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(refusal.into())
    }
}

type Step<T> = Result<T, Halt>;

/// A reached product state waiting to be expanded.
struct Pending {
    state: ModelSimState,
    at: ProductStateRef,
    expansion: Option<Expansion<ModelTransition, ModelSimState, ModelFinding>>,
}

struct Check<'c, 'a> {
    system: &'c ModelSystem<'a>,
    item: &'c ModelCheckItem<'c>,
    limits: &'c ModelCheckLimits,
    meter: ScalarLimits,
    max_candidates: u64,
    cancel: &'c Cancel,
    members: &'c BTreeSet<ProductStateRef>,
    models: &'c BTreeSet<DigestRecord>,
    reached: usize,
    edges: usize,
    keys: BTreeSet<AutomatonStateKey>,
}

impl Check<'_, '_> {
    fn stopped(limit: ModelCheckLimit, value: u64) -> Halt {
        Halt::Check(TemporalCertificateCheck::Stopped(
            IncompleteCause::LimitReached { limit, value },
        ))
    }

    fn reject(rule: CertificateRule, at: ProductStateRef) -> Halt {
        Halt::Check(rejected(rule, at))
    }

    fn poll(&self) -> Step<()> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(Halt::Check(TemporalCertificateCheck::Stopped(
                    IncompleteCause::Cancelled { source },
                )));
            }
        }
        Ok(())
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
            let mut visited = BTreeSet::new();
            let mut queue = VecDeque::new();
            for (_, _, digest, state) in &initial {
                let before = None;
                if let Some(pending) = self.reach(
                    instance,
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
                let expansion = match pending.expansion {
                    Some(expansion) => expansion,
                    None => self.expand(&pending.state, &pending.at)?,
                };
                let mut successors = Vec::with_capacity(expansion.successors.len());
                for (transition, state) in expansion.successors {
                    let order = self.system.transition_key(&transition)?;
                    let (key, digest) = self.system.state_key(&state)?;
                    successors.push((order, key, digest, state));
                }
                successors.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
                for (_, _, digest, state) in successors {
                    self.poll()?;
                    if self.edges >= self.limits.limits.max_transitions {
                        return Err(Self::stopped(
                            ModelCheckLimit::MaxTransitions,
                            widen(self.limits.limits.max_transitions),
                        ));
                    }
                    self.edges += 1;
                    if let Some(next) = self.reach(
                        instance,
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
        }
        Ok(())
    }

    /// Reach `state`, whose digest is `digest`, from the automaton state
    /// `before` (`None` at position zero): read its letter, reject a bad
    /// state, then require it in the certificate under `missing`. `None`
    /// when this instance reached it before.
    fn reach(
        &mut self,
        instance: &Instance,
        state: ModelSimState,
        digest: DigestRecord,
        before: Option<SafetyState>,
        missing: CertificateRule,
        visited: &mut BTreeSet<DigestRecord>,
    ) -> Step<Option<Pending>> {
        if visited.contains(&digest) {
            return Ok(None);
        }
        let translation = &instance.translation;
        let (after, expansion) = match translation.item() {
            SafetyItem::ReachableInvariant { atom } => {
                let ModelCheckItem::Clause { letters, .. } = self.item else {
                    return Err(fault("clause-item-translation").into());
                };
                let object = instance
                    .binding
                    .as_ref()
                    .ok_or_else(|| fault("clause-instance-binding"))?;
                match self.atom(*letters, atom, object, &state)? {
                    Some(holds) => (translation.read(before, holds), None),
                    None => {
                        let at = self.product(instance, digest, SafetyState::Rejected)?;
                        return Err(Self::reject(CertificateRule::BadState, at));
                    }
                }
            }
            SafetyItem::DeadlockFreedom => {
                if !self.models.contains(&digest) {
                    let at = self.product(instance, digest, SafetyState::Carried)?;
                    return Err(Self::reject(missing, at));
                }
                let carried = self.product(instance, digest, SafetyState::Carried)?;
                let expansion = self.expand(&state, &carried)?;
                let class = self
                    .system
                    .terminal_class(&state, !expansion.successors.is_empty())
                    .map_err(|stop| self.expansion_stop(stop))?;
                let holds = match class {
                    TerminalClass::NotTerminal | TerminalClass::Intended => true,
                    TerminalClass::Deadlocked => false,
                    TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                        let at = self.product(instance, digest, SafetyState::Rejected)?;
                        return Err(Self::reject(CertificateRule::BadState, at));
                    }
                };
                (translation.read(before, holds), Some(expansion))
            }
        };
        let at = self.product(instance, digest, after)?;
        if after.is_rejecting() {
            return Err(Self::reject(CertificateRule::BadState, at));
        }
        if self.reached >= self.limits.limits.max_states {
            return Err(Self::stopped(
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
    /// expansion could not determine makes it bad (ADR-018 UE-6, V-6).
    fn expand(
        &self,
        state: &ModelSimState,
        at: &ProductStateRef,
    ) -> Step<Expansion<ModelTransition, ModelSimState, ModelFinding>> {
        self.poll()?;
        let expansion = self
            .system
            .successors(state)
            .map_err(|stop| self.expansion_stop(stop))?;
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Err(Self::reject(CertificateRule::BadState, at.clone()));
        }
        Ok(expansion)
    }

    /// The product state of `digest` and `state` under `instance`'s
    /// context, charging a newly materialized automaton key.
    fn product(
        &mut self,
        instance: &Instance,
        digest: DigestRecord,
        state: SafetyState,
    ) -> Step<ProductStateRef> {
        let key = instance
            .translation
            .key(state)
            .map_err(|_| ClosureRefusal::Encoding)?;
        if !self.keys.contains(&key) {
            let count = widen(self.keys.len());
            if count >= self.limits.max_automaton_states {
                return Err(Self::stopped(
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

    /// The `holds` atom `atom` for `object` at `state`: `None` when it is
    /// undefined.
    fn atom(
        &self,
        letters: &dyn ModelLetters,
        atom: qsl_semantics::check::FormulaNodeId,
        object: &quire_exact::ObjectReference,
        state: &ModelSimState,
    ) -> Step<Option<bool>> {
        self.poll()?;
        let mut meter = Meter::new(self.meter);
        match letters.atom(atom, object, state, &mut meter) {
            Ok(AtomReading::True) => Ok(Some(true)),
            Ok(AtomReading::False) => Ok(Some(false)),
            Ok(AtomReading::Undefined(_)) => Ok(None),
            Err(_) => Err(Self::stopped(
                ModelCheckLimit::EvaluationMeter,
                self.meter.work_units,
            )),
        }
    }

    fn expansion_stop(&self, stop: ExpansionStop) -> Halt {
        match stop.limit {
            Some(ExpansionLimit::EvaluationMeter) => {
                Self::stopped(ModelCheckLimit::EvaluationMeter, self.meter.work_units)
            }
            Some(ExpansionLimit::MaxCandidates) => {
                Self::stopped(ModelCheckLimit::MaxCandidates, self.max_candidates)
            }
            None if stop.is_internal_failure() => fault("model-expansion-invariant").into(),
            None => Halt::Check(TemporalCertificateCheck::Stopped(
                IncompleteCause::ResourceExhausted,
            )),
        }
    }
}

fn fault(invariant: &'static str) -> ClosureRefusal {
    ClosureRefusal::Fault(InternalFault::new("closure-check", invariant))
}

fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
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
