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
        super::encode::Enc::Shared(self.semantic_enc_shared(state))
    }

    /// The cached typed semantic key shared by unchanged clones of `state`.
    /// The cache is scoped to this system's typing context.
    pub(crate) fn semantic_enc_shared(
        &self,
        state: &ModelSimState,
    ) -> std::sync::Arc<super::encode::Enc> {
        let context = std::sync::Arc::downgrade(&self.semantic_context);
        let mut cache = state
            .semantic_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(cached) = cache.as_ref() {
            if std::sync::Weak::ptr_eq(&cached.context, &context) {
                return std::sync::Arc::clone(&cached.semantic);
            }
        }
        let semantic = std::sync::Arc::new(super::encode::semantic_enc(
            &self.model,
            &state.state,
            self.typing(),
        ));
        *cache = Some(super::SemanticCacheEntry {
            context,
            semantic: std::sync::Arc::clone(&semantic),
            canonical_model_key: None,
        });
        semantic
    }

    /// The canonical bytes of a model-state key, shared by unchanged clones
    /// and by the applied-at memo. The state-local cache keeps one byte vector
    /// while all memo entries refer to it through `Arc`.
    pub(crate) fn canonical_model_key_bytes(
        &self,
        state: &ModelSimState,
    ) -> Result<std::sync::Arc<Vec<u8>>, crate::simulation::EncodingRefusal> {
        let identity = super::identity_limits();
        let context = std::sync::Arc::downgrade(&self.semantic_context);
        let mut cache = state
            .semantic_cache
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(cached) = cache.as_ref() {
            if std::sync::Weak::ptr_eq(&cached.context, &context) {
                if let Some(bytes) = &cached.canonical_model_key {
                    return Ok(std::sync::Arc::clone(bytes));
                }
            }
        }
        let semantic = cache
            .as_ref()
            .filter(|cached| std::sync::Weak::ptr_eq(&cached.context, &context))
            .map(|cached| std::sync::Arc::clone(&cached.semantic))
            .unwrap_or_else(|| {
                std::sync::Arc::new(super::encode::semantic_enc(
                    &self.model,
                    &state.state,
                    self.typing(),
                ))
            });
        let key = super::encode::model_key_with_semantic(super::encode::Enc::Shared(
            std::sync::Arc::clone(&semantic),
        ));
        let bytes = std::sync::Arc::new(
            quire_canonical::to_vec(&key, quire_canonical::Limits::new(identity.input_bytes))
                .map_err(|error| crate::simulation::EncodingRefusal::of(&error, identity))?,
        );
        *cache = Some(super::SemanticCacheEntry {
            context,
            semantic,
            canonical_model_key: Some(std::sync::Arc::clone(&bytes)),
        });
        Ok(bytes)
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

/// What one attempt's application of an observed state came to: the memo key
/// is the observed state's key and anchor, the attempt (its operation and the
/// clauses it selects are the attempt's alone) and the transition identities
/// the caller checks.
type AppliedAt = (
    std::sync::Arc<Vec<u8>>,
    String,
    String,
    Option<BTreeSet<ModelTransition>>,
    bool,
);

/// A memoized application and the candidates it charged.
enum AppliedKind {
    Applications(Applications),
    Enabled(Vec<ModelTransition>),
}

/// The applications of one run's attempts. An application is a function of
/// the observed state, the attempt and the checked identities (each call
/// draws its own budget), so a state seen again gives the same result, and
/// the candidates it charged are charged again. The system owns the memo, so
/// it lives and dies with the run.
#[derive(Default)]
pub(crate) struct AppliedMemo {
    applied: Mutex<BTreeMap<AppliedAt, (AppliedKind, u64)>>,
}

impl ModelSystem<'_> {
    /// [`Self::apply_operation_matching`] for the attempt `attempt`, whose
    /// `operation` and `select` it names, remembered per observed state.
    ///
    /// # Errors
    ///
    /// As [`Self::apply_operation_matching`].
    pub(crate) fn apply_attempt(
        &self,
        attempt: &str,
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Applications, ExpansionStop> {
        let at = self.applied_at(attempt, observed, selected_transitions, false);
        if let Some(at) = &at {
            if let Some((AppliedKind::Applications(found), charged)) = self.lock_applied().get(at) {
                budget.charge_many(usize::try_from(*charged).unwrap_or(usize::MAX))?;
                return Ok(found.clone());
            }
        }
        let before = budget.used();
        let applications = self.apply_operation_matching(
            observed,
            operation,
            select,
            selected_transitions,
            budget,
        )?;
        if let Some(at) = at {
            self.lock_applied().insert(
                at,
                (
                    AppliedKind::Applications(applications.clone()),
                    budget.used() - before,
                ),
            );
        }
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
        observed: &ModelSimState,
        operation: &OperationRef<'_>,
        select: &dyn Fn(NodeKey) -> bool,
        selected_transitions: Option<&BTreeSet<ModelTransition>>,
        budget: &mut Budget,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let at = self.applied_at(attempt, observed, selected_transitions, true);
        if let Some(at) = &at {
            if let Some((AppliedKind::Enabled(found), charged)) = self.lock_applied().get(at) {
                budget.charge_many(usize::try_from(*charged).unwrap_or(usize::MAX))?;
                return Ok(found.clone());
            }
        }
        let before = budget.used();
        let enabled = self.enabled_transitions_matching(
            observed,
            operation,
            select,
            selected_transitions,
            budget,
        )?;
        if let Some(at) = at {
            self.lock_applied().insert(
                at,
                (
                    AppliedKind::Enabled(enabled.clone()),
                    budget.used() - before,
                ),
            );
        }
        Ok(enabled)
    }

    fn lock_applied(&self) -> std::sync::MutexGuard<'_, BTreeMap<AppliedAt, (AppliedKind, u64)>> {
        self.applied
            .applied
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The memo key of `attempt` at `observed`; `None` when the state has no
    /// key, which is then not remembered.
    fn applied_at(
        &self,
        attempt: &str,
        observed: &ModelSimState,
        selected_transitions: Option<&BTreeSet<ModelTransition>>,
        enabled_only: bool,
    ) -> Option<AppliedAt> {
        let bytes = self.canonical_model_key_bytes(observed).ok()?;
        Some((
            bytes,
            format!("{:?}", observed.anchor),
            attempt.to_owned(),
            selected_transitions.cloned(),
            enabled_only,
        ))
    }
}
