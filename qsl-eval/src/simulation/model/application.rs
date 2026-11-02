// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 TS-3: FR-120's one application function, as a protocol attempt
//! calls it. An attempt applies its operation to every receiver of its
//! receiver type and every argument vector, under the `pre` and `post`
//! clauses its `contracts` select, through the same `frames::apply`
//! [`ModelSystem`]'s own expansion calls; there is no second contract
//! evaluator.

use qsl_semantics::model::observation::Observation;
use quire_exact::{EffectiveId, NodeKey};
use quire_semantic_value::model_transition::ModelTransition;

use std::borrow::Cow;
use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

use super::walk::{rows_of, sorted_contract_findings, walk_of, Applied, Budget, Row, Walk};
use super::{contracts, frames, Application, ModelFinding, ModelSimState, ModelSystem};
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
        let mut contract = Vec::new();
        let mut successors = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, operation) else {
                continue;
            };
            let clauses = walk.selected(row, application, select);
            frames::apply(
                &walk,
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
                },
            )?;
        }
        Ok(Applications {
            successors,
            findings: sorted_contract_findings(contract),
        })
    }

    /// FR-220 MA-1: the enabled transition identities at `observed`, checking
    /// only candidate identities in `selected_transitions` when supplied.
    /// Candidate enumeration and budget charges still cover the full domain.
    pub(crate) fn enabled_transitions_matching(
        &self,
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&std::collections::BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let walk = walk_of(self, observed, None)?;
        let mut contract = Vec::new();
        let mut successors = Vec::new();
        let mut transitions = Vec::new();
        for row in &walk.rows {
            let Some(application) = self.application_at(row, operation) else {
                continue;
            };
            let clauses = walk.selected(row, application, select);
            frames::apply(
                &walk,
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
    observation: OnceCell<Observation>,
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
    Applications(Applications),
    Enabled(Vec<ModelTransition>),
}

/// The memoized applications of one run, by observed state identity, then by
/// attempt.
type AppliedByObservation = BTreeMap<(Vec<u8>, String), BTreeMap<AppliedAt, (AppliedKind, u64)>>;

/// The applications of one run's attempts. An application is a function of
/// the observed state, the attempt and the checked identities (each call
/// draws its own budget), so a state seen again gives the same result, and
/// the candidates it charged are charged again. The system owns the memo, so
/// it lives and dies with the run.
#[derive(Default)]
pub(crate) struct AppliedMemo {
    applied: Mutex<AppliedByObservation>,
}

impl ModelSystem<'_> {
    /// `state` as an observation the application memo identifies by its
    /// model key bytes and anchor.
    pub(crate) fn observed<'s>(&self, state: Cow<'s, ModelSimState>) -> Observed<'s> {
        let key = super::encode::model_key(&self.model, &state.state, self.typing());
        let identity = quire_canonical::to_vec(
            &key,
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

    /// [`Self::observation_of`] `observed`, built once per observation.
    ///
    /// # Errors
    ///
    /// As [`Self::observation_of`].
    pub(crate) fn observation_at<'o>(
        &self,
        observed: &'o Observed<'_>,
    ) -> Result<&'o Observation, ExpansionStop> {
        if let Some(observation) = observed.observation.get() {
            return Ok(observation);
        }
        let observation = self.observation_of(observed.state())?;
        Ok(observed.observation.get_or_init(|| observation))
    }

    /// [`Self::apply_operation_matching`] for the attempt `attempt`, whose
    /// `operation` and `select` it names, remembered per observed state.
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
    ) -> Result<Applications, ExpansionStop> {
        let at = (attempt.to_owned(), selected_transitions.cloned(), false);
        if let Some((AppliedKind::Applications(found), charged)) =
            self.remembered(&observed.identity, &at)
        {
            budget.charge_many(usize::try_from(charged).unwrap_or(usize::MAX))?;
            return Ok(found);
        }
        let before = budget.used();
        let applications = self.apply_operation_matching(
            observed.state(),
            operation,
            select,
            selected_transitions,
            budget,
        )?;
        self.remember(
            &observed.identity,
            at,
            AppliedKind::Applications(applications.clone()),
            budget.used() - before,
        );
        Ok(applications)
    }

    /// [`Self::enabled_transitions_matching`] for the attempt `attempt`,
    /// remembered per observed state.
    ///
    /// # Errors
    ///
    /// As [`Self::enabled_transitions_matching`].
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
        let enabled = self.enabled_transitions_matching(
            observed.state(),
            operation,
            select,
            selected_transitions,
            budget,
        )?;
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
