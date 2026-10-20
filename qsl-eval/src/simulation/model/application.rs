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

use super::walk::{rows_of, sorted_contract_findings, walk_of, Applied, Budget, Row, Walk};
use super::{contracts, frames, Application, ModelFinding, ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;

/// The operation an attempt applies (FR-114): the type that declares it, its
/// name, and the type of the attempt's receiver.
#[derive(Clone, Copy, Debug)]
pub(crate) struct OperationRef<'o> {
    /// The declaring type.
    pub(crate) declaring: EffectiveId,
    /// The operation's name.
    pub(crate) name: &'o str,
    /// The receiver type the attempt names.
    pub(crate) receiver: EffectiveId,
}

/// The successors and contract findings of one attempt's applications.
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

    /// The evaluation meter budget each clause evaluation gets afresh.
    pub(crate) fn meter(&self) -> quire_exact::ScalarLimits {
        self.meter
    }

    /// The `semantic` member of `state`'s key (FR-120 "State, state key and
    /// anchor"), the member a protocol state's key shares (ADR-027 PS-8).
    pub(crate) fn semantic_enc(&self, state: &ModelSimState) -> super::encode::Enc {
        super::encode::semantic_enc(&self.model, &state.state)
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
        let Some(domain) = super::domain::Domain::of(value_type, &references, self.types) else {
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
    fn application_at(&self, row: &Row, operation: &OperationRef<'_>) -> Option<&Application> {
        if !self.types.conforms(row.type_id(), operation.receiver) {
            return None;
        }
        self.applications.get(&row.type_id())?.iter().find(|application| {
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
                &mut contract,
                &mut successors,
            )?;
        }
        Ok(Applications {
            successors,
            findings: sorted_contract_findings(contract),
        })
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
