// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Applications and transition identities": the expansion of one
//! model state. It evaluates the state's invariants, then every application
//! of every object's operations, collecting the successors and the findings.

use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::Observation;
use quire_exact::{EffectiveId, ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use super::footprints::Enforcement;
use super::{
    contracts, encode, frames, identity_limits, state_document, Application, ClauseInfo,
    ModelFinding, ModelSimState, ModelSystem, BROKEN, EXHAUSTED,
};
use crate::simulation::expansion::{Expansion, ExpansionLimit, ExpansionStop};
use crate::simulation::key;

/// A stop with `cause`.
pub(super) const fn stop(cause: qsl_foundation::CatalogCode) -> ExpansionStop {
    ExpansionStop { cause, limit: None }
}

/// `resource_exhausted`/`insufficient-next-charge` naming the budget `limit`
/// that ran out.
pub(super) const fn exhausted(limit: ExpansionLimit) -> ExpansionStop {
    ExpansionStop {
        cause: EXHAUSTED,
        limit: Some(limit),
    }
}

/// The candidates one expansion has generated, against its bound
/// (FR-120 "Post-states", ADR-016 G-4).
pub(crate) struct Budget {
    used: u64,
    max: u64,
}

impl Budget {
    /// A budget of `max` candidates, none used.
    pub(super) const fn new(max: u64) -> Self {
        Self { used: 0, max }
    }

    /// Count one candidate or (candidate, result) pair.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` when the count would
    /// exceed the bound.
    pub(super) fn charge(&mut self) -> Result<(), ExpansionStop> {
        self.used = self.used.saturating_add(1);
        if self.used > self.max {
            Err(exhausted(ExpansionLimit::MaxCandidates))
        } else {
            Ok(())
        }
    }
}

/// One object of the expanded state.
pub(super) struct Row {
    pub(super) population: DeclarationKey,
    pub(super) key: String,
    pub(super) reference: ObjectReference,
}

impl Row {
    /// The object's most-specific type.
    pub(super) fn type_id(&self) -> EffectiveId {
        self.reference.object_type()
    }
}

/// What an expansion reads: the system, the expanded state, its observation
/// and its objects in ascending (population, key) order.
pub(super) struct Walk<'s, 'a> {
    pub(super) system: &'s ModelSystem<'a>,
    pub(super) state: &'s ModelSimState,
    pub(super) observation: Observation,
    pub(super) rows: Vec<Row>,
    /// The footprints each step's clauses are held to, when the expansion
    /// enforces them.
    pub(super) enforcement: Option<&'s Enforcement<'s>>,
}

impl Walk<'_, '_> {
    /// The references of every object of the state, in row order.
    pub(super) fn references(&self) -> Vec<ObjectReference> {
        self.rows.iter().map(|row| row.reference.clone()).collect()
    }

    /// The `pre` and `post` clauses of `application` at `row` that
    /// `select` admits by their checked identity: every applicable clause
    /// for a model step, the clauses an attempt's `contracts` select for a
    /// protocol attempt (ADR-027 ST-1).
    pub(super) fn selected<'c>(
        &'c self,
        row: &Row,
        application: &Application,
        select: impl Fn(quire_exact::NodeKey) -> bool,
    ) -> Selected<'c> {
        let clauses = &self.system.clauses;
        let pick = |group: &'c [ClauseInfo]| {
            self.applicable(group, row, application.declaring, &application.name)
                .into_iter()
                .filter(|clause| select(clause.identity))
                .collect()
        };
        Selected {
            preconditions: pick(&clauses.preconditions),
            postconditions: pick(&clauses.postconditions),
        }
    }

    /// The clauses in `group` that apply to `row` as the receiver of the
    /// operation `name` declared by `declaring`: the clause resolves to that
    /// operation and the receiver's type conforms to the clause's context
    /// (FR-120 "Contract clauses").
    pub(super) fn applicable<'c>(
        &self,
        group: &'c [ClauseInfo],
        row: &Row,
        declaring: EffectiveId,
        name: &str,
    ) -> Vec<&'c ClauseInfo> {
        group
            .iter()
            .filter(|clause| {
                clause
                    .operation
                    .as_ref()
                    .is_some_and(|(owner, operation)| *owner == declaring && operation == name)
                    && self.system.types.conforms(row.type_id(), clause.context)
            })
            .collect()
    }
}

/// The `pre` and `post` clauses one application evaluates, each kind in
/// ascending name order.
pub(super) struct Selected<'c> {
    pub(super) preconditions: Vec<&'c ClauseInfo>,
    pub(super) postconditions: Vec<&'c ClauseInfo>,
}

/// A contract finding with the keys its position among its siblings follows:
/// ascending transition-identity bytes, then candidate state-key bytes (none
/// first), then clause name (FR-120 "Invariants").
pub(super) struct ContractFinding {
    pub(super) transition: Vec<u8>,
    pub(super) candidate: Option<Vec<u8>>,
    pub(super) clause: String,
    pub(super) finding: ModelFinding,
}

/// One successor of one application (FR-120 "Applications and transition
/// identities"): its transition identity, the state it reaches, and the
/// receiver, argument vector and result the application chose.
pub(crate) struct Applied {
    /// The FR-120 transition identity.
    pub(crate) transition: ModelTransition,
    /// The post-state.
    pub(crate) state: ModelSimState,
    /// The receiver.
    pub(crate) receiver: ObjectReference,
    /// The arguments, in parameter order.
    pub(crate) arguments: Vec<Value>,
    /// The result; `None` for an operation with no result.
    pub(crate) result: Option<Value>,
}

/// The objects of `state`, in ascending (population, key) order.
pub(super) fn rows_of(
    system: &ModelSystem<'_>,
    state: &qsl_semantics::model::state::ModelState,
) -> Result<Vec<Row>, ExpansionStop> {
    let mut rows = Vec::new();
    for (population, objects) in &state.populations {
        for (object_key, object) in objects {
            let reference = system
                .model
                .object_reference(&object.type_identity, object_key)
                .map_err(|_| stop(BROKEN))?;
            rows.push(Row {
                population: population.clone(),
                key: object_key.clone(),
                reference,
            });
        }
    }
    Ok(rows)
}

/// What an expansion of `state` reads: its synthesized observation and its
/// objects (FR-120 "Frame and observation seam"), and the footprints its
/// clauses are held to when the expansion enforces them.
pub(super) fn walk_of<'s, 'a>(
    system: &'s ModelSystem<'a>,
    state: &'s ModelSimState,
    enforcement: Option<&'s Enforcement<'s>>,
) -> Result<Walk<'s, 'a>, ExpansionStop> {
    let identity = identity_limits();
    let state_key = encode::model_key(&system.model, &state.state);
    let (_, digest) = key::state_key(&state_key, identity).map_err(|_| stop(BROKEN))?;
    let observation = system
        .model
        .observation(&state.state, state_document(&digest))
        .map_err(|_| stop(BROKEN))?;
    Ok(Walk {
        system,
        state,
        observation,
        rows: rows_of(system, &state.state)?,
        enforcement,
    })
}

/// Expand `state`: its findings and every successor (FR-120).
pub(super) fn expand_state(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    enforcement: Option<&Enforcement<'_>>,
) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
    let walk = walk_of(system, state, enforcement)?;
    let mut budget = system.budget();

    let mut findings = contracts::invariants(&walk)?;
    let mut contract = Vec::new();
    let mut successors = Vec::new();
    for row in &walk.rows {
        let Some(applications) = system.applications.get(&row.type_id()) else {
            continue;
        };
        for application in applications {
            let clauses = walk.selected(row, application, |_| true);
            frames::apply(
                &walk,
                row,
                application,
                &clauses,
                &mut budget,
                &mut contract,
                &mut successors,
            )?;
        }
    }
    findings.extend(sorted_contract_findings(contract));
    Ok(Expansion {
        successors: successors
            .into_iter()
            .map(|applied| (applied.transition, applied.state))
            .collect(),
        findings,
    })
}

/// Contract findings in the order their siblings follow (FR-120
/// "Invariants"): ascending transition-identity bytes, then candidate
/// state-key bytes (none first), then clause name.
pub(super) fn sorted_contract_findings(mut contract: Vec<ContractFinding>) -> Vec<ModelFinding> {
    contract.sort_by(|left: &ContractFinding, right: &ContractFinding| {
        left.transition
            .cmp(&right.transition)
            .then_with(|| left.candidate.cmp(&right.candidate))
            .then_with(|| left.clause.cmp(&right.clause))
    });
    contract.into_iter().map(|entry| entry.finding).collect()
}
