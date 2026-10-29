// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 (ADR-020 RE-2): the per-step simulation records a refinement
//! declaration's `requirements` hook writes beside its refinement record.
//!
//! A refinement with no hidden field, no history field and no `any` row
//! writes one [`StepObligation::Initial`] record and one
//! [`StepObligation::Step`] record per step row, each listing as hypotheses
//! the invariant clauses whose context is an object type of the concrete
//! model. With a hidden field, a history field or an `any` row it writes
//! none: an `any` row has no per-step obligation, so the other rows' records
//! would not discharge the safety half, and a hidden field or a history field
//! makes `map` no function of the concrete step.
//!
//! A record's members are the refinement node, the obligation and the
//! hypotheses. Each record is written into `CheckedGraph::requirements()` as
//! an `operation-contract` record keyed by its occurrence (ADR-013 O-07):
//! the refinement node, role `generated`, and ordinal 0 for `Initial` or row
//! position + 1 for a `Step`. The hypotheses are sorted ascending by node
//! key and deduplicated, so the order of the invariant clauses in the source
//! does not change a record.
//!
//! A row is named by its zero-based position in the refinement node's
//! `steps` array, which QSpec FR-375 sorts; the records come out in that
//! order, so reordering rows in the source changes nothing.
//!
//! A record's extent is FR-146 "Claim extent"'s: a `Step` record ranges over
//! its concrete operation's receiver, result and parameter types (keyed
//! under the operation's anchor node, as a frame record keys them under its
//! frame node), each `_` argument's abstract parameter domain, the context
//! population, each population the operation's contract clauses walk and
//! each population a `reaches` in the state mapping walks; an `Initial`
//! record over the population covering each concrete object type the state
//! mapping maps and, since its obligation evaluates the mapping, each
//! population a `reaches` in the state mapping walks.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::StateClauseKind;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{EffectiveId, NodeKey, Origin, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::location::Location;
use quire_semantic_value::step_simulation::{StepObligation, StepRowRef, StepSimulationRecord};

use super::claims::{wire, RequirementRecord};
use super::family::OccurrenceRole;
use super::references::ModelReferences;
use super::refinement::RowRoots;
use super::refusal::{CheckCause, CheckRefusal, KeyFault};
use super::state_clause::{classify_refusal, record, PopulationDomain};
use super::{CheckedRefinement, CheckedStateClause};

/// One per-step simulation record with its ADR-013 O-07 occurrence key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepSimulationEntry {
    occurrence: OccurrenceKey,
    record: StepSimulationRecord,
}

impl StepSimulationEntry {
    fn new(refinement: NodeKey, ordinal: u64, record: StepSimulationRecord) -> Self {
        let origin = Origin::new(OccurrenceRole::Generated.kernel_role(), ordinal);
        Self {
            occurrence: OccurrenceKey::new(wire(refinement), origin),
            record,
        }
    }

    /// The occurrence key: the refinement node, role `generated`, and
    /// ordinal 0 for the `Initial` record or the row's position + 1.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The QSV record.
    pub fn record(&self) -> &StepSimulationRecord {
        &self.record
    }
}

/// What S3 reads to give the records their extents.
pub(crate) struct ExtentInputs<'a> {
    /// The refinement's V2 node.
    pub(crate) node: NodeKey,
    /// For each `steps` entry in V2 order, its source row's position among
    /// [`CheckedRefinement::steps`].
    pub(crate) step_sources: &'a [usize],
    /// The `Boolean` type node every record is result-bounded at.
    pub(crate) boolean: NodeKey,
    /// The populations each concrete operation's contract clauses range
    /// over, by the operation's declaring type and name.
    pub(crate) contracts: &'a BTreeMap<(EffectiveId, String), BTreeSet<PopulationDomain>>,
    /// The `model`/`object_type` node of a population's member type.
    pub(crate) object_node: &'a dyn Fn(EffectiveId) -> Option<NodeKey>,
    pub(crate) types: &'a TypeEnvironment,
    pub(crate) position_limit: u64,
    /// The refinement declaration's location, where a fault is reported.
    pub(crate) location: &'a Location,
}

/// The positions [`CheckedRefinement::step_row_ref`] reads: for each source
/// row, its position in the V2 `steps` order `step_sources` gives.
pub(crate) fn row_positions(
    step_sources: &[usize],
    location: &Location,
) -> Result<Vec<StepRowRef>, CheckRefusal> {
    let mut positions = vec![None; step_sources.len()];
    for (position, source) in step_sources.iter().enumerate() {
        let position = u32::try_from(position).map_err(|_| fault(location))?;
        *positions.get_mut(*source).ok_or_else(|| fault(location))? =
            Some(StepRowRef::new(position));
    }
    positions
        .into_iter()
        .map(|position| position.ok_or_else(|| fault(location)))
        .collect()
}

/// The per-step simulation records of `refinement`, each with its
/// `operation-contract` requirement record; empty when the refinement
/// writes none (it has no [`CheckedRefinement::simulation`] roots). The
/// hypotheses are the invariants among `clauses` whose context is a
/// concrete-model object type.
pub(crate) fn simulation_records(
    refinement: &CheckedRefinement,
    clauses: &[CheckedStateClause],
    inputs: &ExtentInputs<'_>,
) -> Result<Vec<(StepSimulationEntry, RequirementRecord)>, CheckRefusal> {
    let Some(roots) = &refinement.simulation else {
        return Ok(Vec::new());
    };
    let location = inputs.location;
    let hypotheses: Vec<NodeKey> = clauses
        .iter()
        .filter(|clause| {
            clause.kind() == StateClauseKind::Invariant
                && refinement
                    .concrete_object_types()
                    .contains(&clause.context())
        })
        .map(CheckedStateClause::identity)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let keyed = refinement
        .row_keys
        .as_ref()
        .ok_or_else(|| fault(location))?;
    let mut written = Vec::with_capacity(inputs.step_sources.len() + 1);
    // The `Initial` obligation evaluates the state mapping, so it ranges
    // over the populations a `reaches` there walks as well.
    let mut initial_populations = roots.initial.clone();
    initial_populations.extend(roots.mapping.iter().copied());
    let initial = extent_record(None, &initial_populations, None, inputs)?;
    written.push((
        StepSimulationEntry::new(
            inputs.node,
            0,
            StepSimulationRecord::new(inputs.node, StepObligation::Initial, hypotheses.clone()),
        ),
        initial,
    ));
    for (position, source) in inputs.step_sources.iter().enumerate() {
        let row = refinement
            .steps()
            .get(*source)
            .ok_or_else(|| fault(location))?;
        let Some(RowRoots {
            operation,
            types,
            context: Ok(context),
            ..
        }) = &row.roots
        else {
            return Err(fault(location));
        };
        let anchor = keyed
            .steps
            .get(*source)
            .and_then(|keyed| keyed.source_node)
            .ok_or_else(|| fault(location))?;
        let mut populations: BTreeSet<PopulationDomain> = roots.mapping.clone();
        populations.extend(*context);
        populations.extend(inputs.contracts.get(operation).into_iter().flatten());
        let record = extent_record(
            Some((anchor, types)),
            &populations,
            Some(operation.clone()),
            inputs,
        )?;
        let position = u32::try_from(position).map_err(|_| fault(location))?;
        written.push((
            StepSimulationEntry::new(
                inputs.node,
                u64::from(position) + 1,
                StepSimulationRecord::new(
                    inputs.node,
                    StepObligation::Step {
                        row: StepRowRef::new(position),
                    },
                    hypotheses.clone(),
                ),
            ),
            record,
        ));
    }
    Ok(written)
}

/// One record's `operation-contract` requirement record: its type roots
/// keyed under `anchor`, and `populations`, an authored bound making one
/// add no domain (FR-380).
fn extent_record(
    roots: Option<(NodeKey, &Vec<ValueType>)>,
    populations: &BTreeSet<PopulationDomain>,
    operation: Option<(EffectiveId, String)>,
    inputs: &ExtentInputs<'_>,
) -> Result<RequirementRecord, CheckRefusal> {
    let location = inputs.location;
    let populations: Vec<(PopulationDomain, NodeKey)> = populations
        .iter()
        .map(|domain| (inputs.object_node)(domain.object).map(|object| (*domain, object)))
        .collect::<Option<_>>()
        .ok_or_else(|| fault(location))?;
    let (anchor, types) = match roots {
        Some((anchor, types)) => (anchor, types.iter().map(|value| (anchor, value)).collect()),
        None => (inputs.node, Vec::new()),
    };
    let references = ModelReferences::new(
        BTreeSet::new(),
        populations.iter().map(|(domain, _)| *domain),
        operation,
    );
    record(
        &types,
        Some(anchor),
        &populations,
        inputs.boolean,
        inputs.types,
        inputs.position_limit,
        references,
    )
    .map_err(|failure| classify_refusal(failure, location))
}

/// A record S3 cannot key.
fn fault(location: &Location) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::InternalFault(Box::new(KeyFault::UnkeyableRequirements)),
    }
}
