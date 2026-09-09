// SPDX-License-Identifier: AGPL-3.0-only
//! FR-007: validate exact finite inputs before admitting an execution context.

mod api;
mod budget;
mod frames;
mod inventory;
mod values;

use super::{ObjectIdentity, QualifiedName, Snapshot};
use crate::checking::{Catalog, CheckedPackage};
use crate::Code;
use budget::{Budget, Result, Stage};
use quire_contract_ir as ir;
use std::collections::{BTreeMap, BTreeSet};

pub use api::{
    ExecutionSelection, ObservationSelection, RuntimeInput, RuntimeLocation, RuntimePathSegment,
    RuntimeReference, ValidatedContext, ValidationLimits, ValidationReport, ValidationStatus,
    ValidationUsage,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct PopulationKey {
    model: ir::RequirementRef,
    record: ir::SymbolName,
    universe: ir::SymbolName,
}

impl PopulationKey {
    fn of(identity: &ObjectIdentity) -> Self {
        Self {
            model: identity.model.clone(),
            record: identity.record.clone(),
            universe: identity.universe.clone(),
        }
    }
    fn path(&self) -> Vec<RuntimePathSegment> {
        vec![
            RuntimePathSegment::Model(self.model.clone()),
            RuntimePathSegment::Population {
                record: self.record.clone(),
                universe: self.universe.clone(),
            },
        ]
    }
}

#[derive(Debug)]
struct PopulationIndex {
    position: usize,
    // A duplicate key retains presence but cannot select one object's fields.
    objects: BTreeMap<String, Option<usize>>,
    complete: bool,
    unambiguous: bool,
}

type PopulationIndexes = BTreeMap<usize, BTreeMap<PopulationKey, PopulationIndex>>;
type StateIndexes = BTreeMap<usize, BTreeMap<QualifiedName, Option<super::ValueId>>>;
type BoundModels = BTreeMap<usize, BTreeSet<ir::RequirementRef>>;

#[derive(Clone, Copy, Debug, Default)]
struct Selected {
    current: Option<usize>,
    pre: Option<usize>,
    post: Option<usize>,
    invocation: Option<usize>,
}

impl Selected {
    fn snapshot(self, observation: ir::StateObservation) -> Option<usize> {
        match observation {
            ir::StateObservation::Current => self.current,
            ir::StateObservation::Pre => self.pre,
            ir::StateObservation::Post => self.post,
        }
    }
    fn snapshots(self) -> impl Iterator<Item = (ir::StateObservation, usize)> {
        [
            (ir::StateObservation::Current, self.current),
            (ir::StateObservation::Pre, self.pre),
            (ir::StateObservation::Post, self.post),
        ]
        .into_iter()
        .filter_map(|(o, i)| i.map(|i| (o, i)))
    }
}

#[derive(Clone, Copy)]
enum Address {
    Snapshot(usize),
    Invocation(usize),
}

struct Validator<'input, 'model, F> {
    checked: &'input CheckedPackage<'model>,
    input: &'input RuntimeInput,
    selection: &'input ExecutionSelection,
    budget: Budget<'input, F>,
    catalogs: &'input [Catalog<'model>],
    inventory: BTreeMap<(String, String), Vec<Address>>,
    selected: Selected,
    clause: Option<usize>,
    indexes: PopulationIndexes,
    states: StateIndexes,
    bound_models: BoundModels,
    required_states: BTreeSet<(ir::StateObservation, QualifiedName)>,
}

impl<'input, 'model, F: FnMut() -> bool> Validator<'input, 'model, F> {
    fn catalog(&self, owner: &ir::RequirementRef) -> Option<&'input Catalog<'model>> {
        self.catalogs
            .iter()
            .find(|catalog| catalog.model.environment().owner() == owner)
    }

    fn at_snapshot(&mut self, index: usize, observation: ir::StateObservation) {
        self.budget.location.artifact =
            RuntimeReference::Snapshot(self.input.snapshots[index].reference());
        self.budget.location.observation = Some(observation);
        self.budget.location.path.clear();
    }

    fn at_invocation(&mut self, index: usize) {
        self.budget.location.artifact =
            RuntimeReference::Invocation(self.input.invocations[index].reference());
        self.budget.location.observation = None;
        self.budget.location.path.clear();
    }

    fn run(&mut self) -> Result<()> {
        // Locate the exact authored clause before creating native source loci.
        for (index, clause) in self.checked.clauses().iter().enumerate() {
            self.budget.poll()?;
            if clause.binding().requirement == self.selection.requirement
                && clause.binding().clause == self.selection.clause
            {
                self.clause = Some(index);
                self.budget.span = self.checked.linked().unit().clauses()[index].span;
                break;
            }
        }
        self.inspect_inventory()?;
        let Some(clause) = self.clause else {
            self.budget.location.artifact = match &self.selection.observation {
                ObservationSelection::Current { snapshot, .. } => {
                    RuntimeReference::Snapshot(snapshot.clone())
                }
                ObservationSelection::Invocation { invocation } => {
                    RuntimeReference::Invocation(invocation.clone())
                }
            };
            self.budget.location.observation = None;
            self.budget.location.path.clear();
            self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "requested authored clause is not in the checked package",
            )?;
            return Ok(());
        };
        self.select(clause)?;
        self.index_populations()?;
        self.inspect_values(clause)?;
        self.inspect_invocation(clause)?;
        // Index construction is finished. Move its ownership out for this
        // read-only pass, so diagnostics can mutate without cloning the maps.
        // Restore on every returned outcome; a callback panic unwinds the request.
        let indexes = std::mem::take(&mut self.indexes);
        let result = self.inspect_frames(clause, &indexes);
        self.indexes = indexes;
        result
    }
}

/// Validate exact model/population/invocation correspondence without evaluating a predicate.
///
/// Every returned context has passed value, closure, delta and frame checks.
/// Cancellation returns an incomplete report unless a known invalid defect was
/// already observed. A panic from the caller's poll unwinds normally.
pub fn validate<'checked, 'model>(
    checked: &'checked CheckedPackage<'model>,
    input: RuntimeInput,
    selection: ExecutionSelection,
    limits: ValidationLimits,
    poll: impl FnMut() -> bool,
) -> std::result::Result<ValidatedContext<'checked, 'model>, Box<ValidationReport>> {
    let artifact = match &selection.observation {
        ObservationSelection::Current { snapshot, .. } => {
            RuntimeReference::Snapshot(snapshot.clone())
        }
        ObservationSelection::Invocation { invocation } => {
            RuntimeReference::Invocation(invocation.clone())
        }
    };
    let location = RuntimeLocation {
        artifact,
        observation: None,
        requirement: selection.requirement.clone(),
        clause: selection.clause.clone(),
        path: Vec::new(),
    };
    let mut validator = Validator {
        checked,
        input: &input,
        selection: &selection,
        budget: Budget::new(checked.linked().unit().source(), location, limits, poll),
        catalogs: checked.catalogs(),
        inventory: BTreeMap::new(),
        selected: Selected::default(),
        clause: None,
        indexes: BTreeMap::new(),
        states: BTreeMap::new(),
        bound_models: BTreeMap::new(),
        required_states: BTreeSet::new(),
    };
    let result = validator.run();
    if result.is_err() || validator.budget.failed() {
        return Err(validator.budget.report());
    }
    let Some(clause_index) = validator.clause else {
        // Defensive library invariant; never fabricate a successful context.
        let _ = validator.budget.issue(
            Stage::Binding,
            Code::InvalidModelBinding,
            "validated request lacks an authored clause",
        );
        return Err(validator.budget.report());
    };
    let selected = validator.selected;
    let indexes = validator.indexes;
    let states = validator.states;
    let usage = validator.budget.usage;
    Ok(ValidatedContext {
        checked,
        input,
        selection,
        clause_index,
        selected,
        indexes,
        states,
        usage,
    })
}

fn snapshot_key(snapshot: &Snapshot, index: usize) -> PopulationKey {
    let population = &snapshot.draft().populations[index];
    PopulationKey {
        model: population.model.clone(),
        record: population.record.clone(),
        universe: population.universe.clone(),
    }
}
