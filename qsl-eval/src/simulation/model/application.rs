// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 TS-3: FR-120's one application function, as a protocol attempt
//! calls it. An attempt applies its operation to every receiver of its
//! receiver type and every argument vector, under the `pre` and `post`
//! clauses its `contracts` select, through the same `frames::apply`
//! [`ModelSystem`]'s own expansion calls; there is no second contract
//! evaluator.

use qsl_semantics::model::observation::Observation;
use qsl_semantics::model::state::ModelState;
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Integer, NodeKey, ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use super::walk::{rows_of, sorted_contract_findings, stop, walk_of, Applied, Budget, Row, Walk};
use super::{contracts, frames, Application, ModelFinding, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::ExpansionStop;

/// The operation an attempt applies (FR-114): the type that declares it, its
/// name, and the type of the attempt's receiver.
#[derive(Clone, Debug)]
pub(crate) struct OperationRef<'o> {
    /// The declaring type.
    pub(crate) declaring: EffectiveId,
    /// The operation's name.
    pub(crate) name: &'o str,
    /// The receiver type the attempt names.
    pub(crate) receiver: EffectiveId,
}

/// The successors and contract findings of one attempt's applications.
#[derive(Clone)]
pub(crate) struct Applications {
    /// Every successor, receivers in ascending (population, key) order.
    pub(crate) successors: Vec<Applied>,
    /// The contract findings, in FR-120's sibling order.
    pub(crate) findings: Vec<ModelFinding>,
}

/// ADR-027 PD-4: how the first application of an attempt, in canonical
/// order, reads its selected preconditions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FirstApplication {
    /// The state holds no receiver of the attempt's type, or its operation
    /// has no argument vector.
    None,
    /// The first selected precondition, in clause order, that is not true.
    Precondition(String),
    /// Every selected precondition holds for the application with this
    /// FR-120 transition identity.
    Enabled(ModelTransition),
}

impl<'a> ModelSystem<'a> {
    /// The package's type environment.
    pub(crate) fn types(&self) -> &'a quire_semantic_value::declaration::TypeEnvironment {
        self.types
    }

    /// The domain package identity that declares the object type `type_id`;
    /// `None` when no selected model declares it.
    pub(crate) fn declaring_package(&self, type_id: EffectiveId) -> Option<&str> {
        self.model.type_key(type_id).map(|key| key.package.as_str())
    }

    /// The `semantic` member of `state`'s key (FR-120 "State, state key and
    /// anchor"), the member a protocol state's key shares (ADR-027 PS-8).
    pub(crate) fn semantic_enc(&self, state: &ModelSimState) -> super::encode::Enc {
        super::encode::semantic_enc(&self.model, &state.state, self.typing())
    }

    /// Every value of `value_type` over the objects of `state`, in FR-120's
    /// domain order, each counted against `budget`; `None` when the type
    /// has no finite domain here (FR-120 "Bounds").
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` when the budget runs
    /// out.
    pub(crate) fn values_of(
        &self,
        value_type: &quire_exact::ValueType,
        state: &ModelSimState,
        budget: &mut Budget,
    ) -> Result<Option<Vec<quire_exact::Value>>, ExpansionStop> {
        let references: Vec<_> = rows_of(self, &state.state)?
            .into_iter()
            .map(|row| row.reference)
            .collect();
        if let quire_exact::ValueType::Composite(_) | quire_exact::ValueType::Enum(_) = value_type {
            return super::domain::composite_values(
                value_type,
                &references,
                self.types,
                &self.record_orders,
                || budget.charge(),
            );
        }
        let Some(domain) =
            super::domain::Domain::of(value_type, &references, self.types, &self.record_orders)
        else {
            return Ok(None);
        };
        let mut values = Vec::new();
        let mut index = 0u128;
        while let Some(value) = domain.nth(index) {
            budget.charge()?;
            values.push(value);
            index = index.saturating_add(1);
        }
        Ok(Some(values))
    }

    /// Whether [`Self::values_of`] enumerates `value_type`: a type FR-120
    /// enumerates, or an enum, record, tuple or union of finite fields.
    pub(crate) fn enumerates(&self, value_type: &quire_exact::ValueType) -> bool {
        super::domain::enumerable_in(value_type, self.types)
    }

    /// The child-index paths of the leaves of `value_type` that
    /// [`Self::enumerates`] refuses.
    pub(crate) fn unenumerable_paths(&self, value_type: &quire_exact::ValueType) -> Vec<Vec<u32>> {
        super::domain::unenumerable_paths(value_type, self.types)
    }

    /// The candidate budget of one expansion (FR-120 "Post-states").
    pub(crate) fn budget(&self) -> Budget {
        Budget::new(self.limits.max_candidates)
    }

    /// FR-120's synthesized observation of `state` (FR-120 "Frame and
    /// observation seam"), the observation a protocol slot reads.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when the state
    /// cannot be observed.
    pub(crate) fn observation_of(
        &self,
        state: &ModelSimState,
    ) -> Result<Observation, ExpansionStop> {
        Ok(walk_of(self, state, None)?.observation)
    }

    /// FR-120 "Invariants": the invariant findings of `state`.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an evaluation stops.
    pub(crate) fn invariant_findings(
        &self,
        state: &ModelSimState,
    ) -> Result<Vec<ModelFinding>, ExpansionStop> {
        contracts::invariants(&walk_of(self, state, None)?)
    }

    /// The application `operation` names at `row`, by its declaring type
    /// and name in the row's effective view.
    pub(super) fn application_at(
        &self,
        row: &Row,
        operation: &OperationRef<'_>,
    ) -> Option<&Application> {
        if !self
            .types
            .conforms(row.type_id(), operation.receiver.clone())
        {
            return None;
        }
        self.applications
            .get(&row.type_id())?
            .iter()
            .find(|application| {
                application.declaring == operation.declaring && application.name == operation.name
            })
    }

    /// ADR-027 ST-1 and TS-3: every application of `operation` at the
    /// thread observation `observed`, over the `pre` and `post` clauses
    /// whose checked identity `select` admits, counted against `budget`.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the successor set cannot be known.
    pub(crate) fn apply_operation(
        &self,
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        budget: &mut Budget,
    ) -> Result<Applications, ExpansionStop> {
        self.apply_operation_matching(observed, operation, select, None, budget)
    }

    /// As [`Self::apply_operation`], checking only candidate identities in
    /// `selected_transitions` when supplied. The full argument domain is
    /// still enumerated and charged against `budget`.
    pub(crate) fn apply_operation_matching(
        &self,
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&std::collections::BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Applications, ExpansionStop> {
        let walk = walk_of(self, observed, None)?;
        self.apply_walk(&walk, operation, select, selected_transitions, budget, None)
    }

    /// [`Self::apply_operation_matching`] over the walk `walk`, recording in
    /// `charged`, when given, the candidates each enabled argument vector
    /// charged beyond its own count, by transition identity.
    fn apply_walk(
        &self,
        walk: &Walk<'_, '_>,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&std::collections::BTreeSet<ModelTransition>>,
        budget: &mut Budget,
        mut charged: Option<&mut Vec<(ModelTransition, u64)>>,
    ) -> Result<Applications, ExpansionStop> {
        let mut contract = Vec::new();
        let mut successors = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, operation) else {
                continue;
            };
            let clauses = walk.selected(row, application, select);
            frames::apply(
                walk,
                row,
                application,
                &clauses,
                budget,
                frames::Outputs {
                    contract: &mut contract,
                    successors: &mut successors,
                    enabled: None,
                    enabled_transitions: None,
                    selected_transitions,
                    charged: charged.as_deref_mut(),
                },
            )?;
        }
        Ok(Applications {
            successors,
            findings: sorted_contract_findings(contract),
        })
    }

    /// FR-220 MA-1: the enabled transition identities over the walk `walk`,
    /// checking only candidate identities in `selected_transitions` when
    /// supplied. Candidate enumeration and budget charges still cover the
    /// full domain.
    fn enabled_walk(
        &self,
        walk: &Walk<'_, '_>,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&std::collections::BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let mut contract = Vec::new();
        let mut successors = Vec::new();
        let mut transitions = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, operation) else {
                continue;
            };
            let clauses = walk.selected(row, application, select);
            frames::apply(
                walk,
                row,
                application,
                &clauses,
                budget,
                frames::Outputs {
                    contract: &mut contract,
                    successors: &mut successors,
                    enabled: None,
                    enabled_transitions: Some(&mut transitions),
                    selected_transitions,
                    charged: None,
                },
            )?;
        }
        Ok(transitions)
    }

    /// ADR-027 PD-4: how the first application of `operation` at
    /// `observed`, in canonical order (receiver, then argument vector),
    /// reads the selected preconditions.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when an evaluation stops.
    pub(crate) fn first_application(
        &self,
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<FirstApplication, ExpansionStop> {
        let walk: Walk<'_, '_> = walk_of(self, observed, None)?;
        for row in rows_of(self, &observed.state)? {
            let Some(application) = self.application_at(&row, operation) else {
                continue;
            };
            let clauses = walk.selected(&row, application, select);
            return Ok(
                match frames::first_precondition(&walk, &row, application, &clauses)? {
                    None => FirstApplication::None,
                    Some((_, Some(clause))) => FirstApplication::Precondition(clause),
                    Some((transition, None)) => FirstApplication::Enabled(transition),
                },
            );
        }
        Ok(FirstApplication::None)
    }
}

/// An observed state's identity in the application memo: its key bytes and
/// anchor. `None` when the state has no key, which is then not remembered.
type ObservedIdentity = Option<(Vec<u8>, String)>;

/// A thread's observation of a model state with its memo identity, encoded
/// once however many attempts are applied to it.
pub(crate) struct Observed<'s> {
    state: Cow<'s, ModelSimState>,
    identity: ObservedIdentity,
    /// FR-120's synthesized observation of the state, once a body reads it.
    observation: OnceCell<Arc<Observation>>,
}

/// A field value as a departure from a base state names it.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Held {
    Absent,
    Boolean(bool),
    Integer(Integer),
    String(String),
    Reference(ObjectReference),
}

impl Held {
    /// `value` as a departure names it; `None` for a value of another form.
    fn of(value: Option<&Value>) -> Option<Self> {
        Some(match value {
            None => Self::Absent,
            Some(Value::Boolean(held)) => Self::Boolean(*held),
            Some(Value::Integer(held)) => Self::Integer(held.clone()),
            Some(Value::String(held)) => Self::String(held.clone()),
            Some(Value::Reference(held)) => Self::Reference(held.clone()),
            Some(_) => return None,
        })
    }
}

/// Whether two field values are the same value: equal Booleans, integers,
/// strings or references, or one shared option, record, tuple, union or
/// collection. Two values this cannot tell apart may still be equal.
fn identical(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Boolean(left), Value::Boolean(right)) => left == right,
        (Value::Integer(left), Value::Integer(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Reference(left), Value::Reference(right)) => left == right,
        (Value::Option(left), Value::Option(right)) => Arc::ptr_eq(left, right),
        (Value::Composite(left), Value::Composite(right)) => Arc::ptr_eq(left, right),
        (Value::Union(left), Value::Union(right)) => Arc::ptr_eq(left, right),
        (Value::Collection(left), Value::Collection(right)) => Arc::ptr_eq(left, right),
        _ => false,
    }
}

/// How a model state departs from a base state with the same objects and
/// extents: each (population, object, field) whose value is not
/// [`identical`] to the base's, with the value the state holds there. Two
/// states with one departure from one base are the same state.
type Departure = Vec<(DeclarationKey, String, DeclarationKey, Held)>;

/// The departure of `state` from `base`; `None` when they differ in their
/// populations, objects, object types or extents, or in a field value
/// [`Held`] cannot name.
fn departure(base: &ModelState, state: &ModelState) -> Option<Departure> {
    if base.populations.len() != state.populations.len() || base.extents != state.extents {
        return None;
    }
    let mut departed = Vec::new();
    for ((population, before), (other, after)) in base.populations.iter().zip(&state.populations) {
        if population != other || before.len() != after.len() {
            return None;
        }
        for ((key, before), (other, after)) in before.iter().zip(after) {
            if key != other || before.type_identity != after.type_identity {
                return None;
            }
            let fields: BTreeSet<&DeclarationKey> =
                before.fields.keys().chain(after.fields.keys()).collect();
            for field in fields {
                let held = after.fields.get(field);
                match (before.fields.get(field), held) {
                    (Some(left), Some(right)) if identical(left, right) => {}
                    _ => departed.push((
                        population.clone(),
                        key.clone(),
                        field.clone(),
                        Held::of(held)?,
                    )),
                }
            }
        }
    }
    Some(departed)
}

/// The memo identities of the observations one expansion makes of the
/// model state `base`, by anchor and departure from `base`, so a state
/// observed again (by another thread, another attempt or another message of
/// the same value) is not encoded again.
pub(crate) struct ObservedIdentities<'b> {
    base: &'b ModelState,
    known: RefCell<BTreeMap<(String, Departure), ObservedIdentity>>,
}

impl<'b> ObservedIdentities<'b> {
    /// No identity yet of an observation of `base`.
    pub(crate) fn new(base: &'b ModelSimState) -> Self {
        Self {
            base: &base.state,
            known: RefCell::new(BTreeMap::new()),
        }
    }
}

impl Observed<'_> {
    /// The observed model state.
    pub(crate) fn state(&self) -> &ModelSimState {
        &self.state
    }
}

/// What one attempt's application of an observed state came to, beside the
/// observed state's identity: the attempt (its operation and the clauses it
/// selects are the attempt's alone), the transition identities the caller
/// checks, and whether only enabled identities were asked for.
type AppliedAt = (String, Option<BTreeSet<ModelTransition>>, bool);

/// A memoized application and the candidates it charged.
#[derive(Clone)]
enum AppliedKind {
    /// The applications, with the candidates each enabled argument vector
    /// charged beyond its own count, by transition identity.
    Applications(Arc<Applications>, Arc<[(ModelTransition, u64)]>),
    Enabled(Vec<ModelTransition>),
}

/// `all`, the applications of every argument vector, restricted to those
/// whose transition identity `selected` holds: what the application checking
/// only `selected` finds, since an unselected vector adds no successor and
/// no finding (FR-220 MA-1).
///
/// # Errors
///
/// `runtime_invariant`/`established-invariant-broken` for a contract
/// finding that names no transition.
fn restricted(
    all: &Applications,
    selected: &BTreeSet<ModelTransition>,
) -> Result<Applications, ExpansionStop> {
    let mut findings = Vec::new();
    for finding in &all.findings {
        let ModelFinding::ContractUndetermined { transition, .. } = finding else {
            return Err(stop(BROKEN));
        };
        if selected.contains(transition) {
            findings.push(finding.clone());
        }
    }
    Ok(Applications {
        successors: all
            .successors
            .iter()
            .filter(|applied| selected.contains(&applied.transition))
            .cloned()
            .collect(),
        findings,
    })
}

/// The memoized applications of one run, by observed state identity, then by
/// attempt.
type AppliedByObservation = BTreeMap<(Vec<u8>, String), BTreeMap<AppliedAt, (AppliedKind, u64)>>;

/// The applications of one run's attempts. An application is a function of
/// the observed state, the attempt and the checked identities (each call
/// draws its own budget), so a state seen again gives the same result, and
/// the candidates it charged are charged again. So is a state's synthesized
/// observation. The system owns the memo, so it lives and dies with the run.
#[derive(Default)]
pub(crate) struct AppliedMemo {
    applied: Mutex<AppliedByObservation>,
    observations: Mutex<ObservationsByIdentity>,
}

/// The synthesized observations of one run, by observed state identity.
type ObservationsByIdentity = BTreeMap<(Vec<u8>, String), Arc<Observation>>;

impl<'a> ModelSystem<'a> {
    /// `state` as an observation the application memo identifies by its
    /// model key bytes and anchor.
    pub(crate) fn observed<'s>(&self, state: Cow<'s, ModelSimState>) -> Observed<'s> {
        let key = super::encode::model_key(&self.model, &state.state, self.typing());
        let identity = self
            .canonical()
            .to_vec(
                &key.0,
                quire_canonical::Limits::new(super::identity_limits().input_bytes),
            )
            .ok()
            .map(|bytes| (bytes, format!("{:?}", state.anchor)));
        Observed {
            state,
            identity,
            observation: OnceCell::new(),
        }
    }

    /// The walk of `observed`, over the observation it holds or the memo
    /// remembers of its identity, or else a newly synthesized one.
    ///
    /// # Errors
    ///
    /// As [`walk_of`].
    fn walk_observed<'s>(
        &'s self,
        observed: &'s Observed<'_>,
    ) -> Result<Walk<'s, 'a>, ExpansionStop> {
        match self.remembered_observation(observed) {
            Some(observation) => Ok(Walk {
                system: self,
                state: observed.state(),
                observation: Observation::clone(observation),
                rows: rows_of(self, &observed.state().state)?,
                enforcement: None,
                post_presence: None,
            }),
            None => walk_of(self, observed.state(), None),
        }
    }

    /// The observation `observed` holds, or else the one the memo remembers
    /// of its identity, which it then holds.
    fn remembered_observation<'o>(
        &self,
        observed: &'o Observed<'_>,
    ) -> Option<&'o Arc<Observation>> {
        if let Some(observation) = observed.observation.get() {
            return Some(observation);
        }
        let identity = observed.identity.as_ref()?;
        let remembered = self.lock_observations().get(identity).cloned()?;
        Some(observed.observation.get_or_init(|| remembered))
    }

    /// Hold `observation`, synthesized of `observed`'s state, as its
    /// observation unless it holds one, remembering it by its identity.
    fn keep_observation<'o>(
        &self,
        observed: &'o Observed<'_>,
        observation: Observation,
    ) -> &'o Arc<Observation> {
        observed.observation.get_or_init(|| {
            let observation = Arc::new(observation);
            if let Some(identity) = &observed.identity {
                self.lock_observations()
                    .insert(identity.clone(), Arc::clone(&observation));
            }
            observation
        })
    }

    /// [`Self::observed`], reusing the identity of a state `identities`
    /// already holds.
    pub(crate) fn observed_from<'s>(
        &self,
        state: Cow<'s, ModelSimState>,
        identities: &ObservedIdentities<'_>,
    ) -> Observed<'s> {
        let Some(departure) = departure(identities.base, &state.state) else {
            return self.observed(state);
        };
        let key = (format!("{:?}", state.anchor), departure);
        if let Some(identity) = identities.known.borrow().get(&key) {
            return Observed {
                state,
                identity: identity.clone(),
                observation: OnceCell::new(),
            };
        }
        let observed = self.observed(state);
        identities
            .known
            .borrow_mut()
            .insert(key, observed.identity.clone());
        observed
    }

    /// [`Self::observation_of`] `observed`, built once per observation.
    ///
    /// # Errors
    ///
    /// As [`Self::observation_of`].
    pub(crate) fn observation_at<'o>(
        &self,
        observed: &'o Observed<'_>,
    ) -> Result<&'o Observation, ExpansionStop> {
        if let Some(observation) = self.remembered_observation(observed) {
            return Ok(observation);
        }
        let observation = self.observation_of(observed.state())?;
        Ok(self.keep_observation(observed, observation))
    }

    /// [`Self::apply_operation_matching`] for the attempt `attempt`, whose
    /// `operation` and `select` it names, remembered per observed state.
    /// Checking only `selected_transitions` of a state whose every
    /// application is remembered restricts those applications, charging
    /// what the selected argument vectors charged.
    ///
    /// # Errors
    ///
    /// As [`Self::apply_operation_matching`].
    pub(crate) fn apply_attempt(
        &self,
        attempt: &str,
        observed: &Observed<'_>,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Arc<Applications>, ExpansionStop> {
        let at = (attempt.to_owned(), selected_transitions.cloned(), false);
        if let Some((AppliedKind::Applications(found, _), charged)) =
            self.remembered(&observed.identity, &at)
        {
            budget.charge_many(usize::try_from(charged).unwrap_or(usize::MAX))?;
            return Ok(found);
        }
        if let Some(selected) = selected_transitions {
            let every = (attempt.to_owned(), None, false);
            if let Some((AppliedKind::Applications(all, by_transition), charged)) =
                self.remembered(&observed.identity, &every)
            {
                let unselected: u64 = by_transition
                    .iter()
                    .filter(|(transition, _)| !selected.contains(transition))
                    .map(|(_, charged)| charged)
                    .sum();
                let charged = charged
                    .checked_sub(unselected)
                    .ok_or_else(|| stop(BROKEN))?;
                budget.charge_many(usize::try_from(charged).unwrap_or(usize::MAX))?;
                return Ok(Arc::new(restricted(&all, selected)?));
            }
        }
        let before = budget.used();
        let mut by_transition = Vec::new();
        let walk = self.walk_observed(observed)?;
        let applications = self.apply_walk(
            &walk,
            operation,
            select,
            selected_transitions,
            budget,
            Some(&mut by_transition),
        );
        self.keep_observation(observed, walk.observation);
        let applications = Arc::new(applications?);
        self.remember(
            &observed.identity,
            at,
            AppliedKind::Applications(Arc::clone(&applications), by_transition.into()),
            budget.used() - before,
        );
        Ok(applications)
    }

    /// FR-220 MA-1: the enabled transition identities of the attempt `attempt`,
    /// remembered per observed state.
    ///
    /// # Errors
    ///
    /// As [`Self::apply_operation_matching`].
    pub(crate) fn enabled_attempt(
        &self,
        attempt: &str,
        observed: &Observed<'_>,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let at = (attempt.to_owned(), selected_transitions.cloned(), true);
        if let Some((AppliedKind::Enabled(found), charged)) =
            self.remembered(&observed.identity, &at)
        {
            budget.charge_many(usize::try_from(charged).unwrap_or(usize::MAX))?;
            return Ok(found);
        }
        let before = budget.used();
        let walk = self.walk_observed(observed)?;
        let enabled = self.enabled_walk(&walk, operation, select, selected_transitions, budget);
        self.keep_observation(observed, walk.observation);
        let enabled = enabled?;
        self.remember(
            &observed.identity,
            at,
            AppliedKind::Enabled(enabled.clone()),
            budget.used() - before,
        );
        Ok(enabled)
    }

    fn lock_applied(&self) -> std::sync::MutexGuard<'_, AppliedByObservation> {
        self.applied
            .applied
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn lock_observations(&self) -> std::sync::MutexGuard<'_, ObservationsByIdentity> {
        self.applied
            .observations
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The remembered application `at` of the observed state `identity`.
    fn remembered(
        &self,
        identity: &ObservedIdentity,
        at: &AppliedAt,
    ) -> Option<(AppliedKind, u64)> {
        let identity = identity.as_ref()?;
        let applied = self.lock_applied();
        let (kind, charged) = applied.get(identity)?.get(at)?;
        Some((kind.clone(), *charged))
    }

    /// Remember the application `at` of the observed state `identity`.
    fn remember(
        &self,
        identity: &ObservedIdentity,
        at: AppliedAt,
        kind: AppliedKind,
        charged: u64,
    ) {
        let Some(identity) = identity else {
            return;
        };
        let mut applied = self.lock_applied();
        if let Some(entries) = applied.get_mut(identity) {
            entries.insert(at, (kind, charged));
        } else {
            applied.insert(identity.clone(), BTreeMap::from([(at, (kind, charged))]));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::collections::BTreeSet;

    use ix_trace_rs::trace;
    use quire_semantic_value::model_transition::ModelTransition;
    use serde_json::json;

    use super::super::walk::{rows_of, Budget};
    use super::{Applications, OperationRef};
    use crate::simulation::expansion::ExpansionLimit;
    use crate::simulation::sampled::world::{initial, Model, Operation, World};

    /// One cell `c` whose `v: Int[0, 3]` is 0, and `set(n: Int[0, 3])`,
    /// whose postcondition `Put` makes `v` the argument.
    fn cells() -> Model {
        const PACKAGE: &str = "test/cells";
        let val = format!("ix://{PACKAGE}/Val");
        Model {
            package: PACKAGE,
            object: "Cell",
            population: "cells",
            key: "c",
            integers: vec![("Val", 0, 3)],
            fields: vec![("v", val.clone())],
            operations: vec![Operation {
                name: "set",
                parameters: vec![("n", val)],
                modifies: vec!["v"],
            }],
            body: "post Put using v on M::Cell::set { self.v = n }\n".to_owned(),
            initial: json!({"v": {"integer": "0"}}),
        }
    }

    /// The applications' transitions, arguments and post-states, and their
    /// findings.
    fn view(applications: &Applications) -> (Vec<String>, String) {
        let successors = applications
            .successors
            .iter()
            .map(|applied| {
                format!(
                    "{:?} {:?} {:?}",
                    applied.transition, applied.arguments, applied.state
                )
            })
            .collect();
        (successors, format!("{:?}", applications.findings))
    }

    /// FR-120's candidate budget: an attempt that checks only some argument
    /// vectors of a state whose every application the run remembers
    /// restricts the remembered application, and charges what applying only
    /// those vectors afresh charges. It completes under exactly that many
    /// candidates, with the fresh application's successors and findings, and
    /// stops naming `MaxCandidates` under one fewer.
    #[trace("TC-472", "FR-120-AC-21")]
    #[test]
    fn a_restricted_remembered_application_charges_what_a_fresh_one_does() {
        let world = World::new(&cells());
        let system = world.system();
        let state = initial(&system);
        let [row] = <[_; 1]>::try_from(rows_of(&system, &state.state).expect("rows"))
            .unwrap_or_else(|_| panic!("one cell"));
        let operation = OperationRef {
            declaring: row.type_id(),
            name: "set",
            receiver: row.type_id(),
        };
        let select = |_| true;

        let mut every = Budget::new(u64::MAX);
        let all = system
            .apply_operation_matching(&state, &operation, &select, None, &mut every)
            .expect("applies");
        let transitions: Vec<ModelTransition> = all
            .successors
            .iter()
            .map(|applied| applied.transition.clone())
            .collect();
        assert_eq!(transitions.len(), 4, "one successor per argument");
        let selected: BTreeSet<ModelTransition> = transitions[1..3].iter().cloned().collect();

        let mut alone = Budget::new(u64::MAX);
        let fresh = system
            .apply_operation_matching(&state, &operation, &select, Some(&selected), &mut alone)
            .expect("applies");
        let charge = alone.used();
        assert!(charge < every.used(), "the unselected vectors charge");

        let observed = system.observed(Cow::Borrowed(&state));
        system
            .apply_attempt(
                "a",
                &observed,
                &operation,
                &select,
                None,
                &mut Budget::new(u64::MAX),
            )
            .expect("applies");

        let mut exact = Budget::new(charge);
        let restricted = system
            .apply_attempt(
                "a",
                &observed,
                &operation,
                &select,
                Some(&selected),
                &mut exact,
            )
            .expect("the selected vectors' charge fits");
        assert_eq!(exact.used(), charge);
        assert_eq!(view(&restricted), view(&fresh));

        let stopped = system.apply_attempt(
            "a",
            &observed,
            &operation,
            &select,
            Some(&selected),
            &mut Budget::new(charge - 1),
        );
        assert_eq!(
            stopped.err().map(|stop| stop.limit),
            Some(Some(ExpansionLimit::MaxCandidates))
        );
    }
}
