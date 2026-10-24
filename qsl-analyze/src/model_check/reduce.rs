// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 EI-1 (FR-151, FR-159): the reductions a model-check request
//! selects, the normative preservation table, and the pre-check that admits
//! a selection before the first expansion.
//!
//! The pre-check runs after FR-126's root classification, in FR-159's
//! order: (a) look up each selected reduction in the table, symmetry, then
//! partial-order, then constraint, and settle the first "no" as
//! `ReductionNotPreserving`; (b) admit the symmetry declarations (form, then
//! closure of the initial states under the group's generators); (c) settle
//! V-8 for the first selected reduction the system does not offer; (d)
//! resolve the constraint. No state is expanded before it passes.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::{
    EncodingRefusal, Permutation, SymmetryClass, SymmetryGroup, TransitionSystem,
};
use qsl_replay::{
    ModelCheckItem, ReductionCause, ReductionKind, SymmetryBrokenCause, SymmetryRefusal,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_semantic_value::fairness::PropertyForm;

/// One symmetry declaration of a request (FR-151): classes of
/// interchangeable keys of `population`'s universe. A key in no class, or in
/// a class of one, is fixed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymmetryDeclaration {
    /// The population.
    pub population: DeclarationKey,
    /// The classes, each a list of keys of the population's universe.
    pub classes: Vec<Vec<String>>,
}

/// The reductions a request selects (FR-159 Inputs). The default selects
/// none.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Reductions {
    /// The symmetry declarations; empty selects no symmetry.
    pub symmetry: Vec<SymmetryDeclaration>,
    /// Whether partial-order reduction is selected.
    pub partial_order: bool,
    /// The state constraint's clause name, when one is selected.
    pub constraint: Option<String>,
}

impl Reductions {
    /// Whether the request selects no reduction.
    pub fn is_empty(&self) -> bool {
        self.symmetry.is_empty() && !self.partial_order && self.constraint.is_none()
    }

    /// The selected reductions, in the order symmetry, partial-order,
    /// constraint.
    pub fn selected(&self) -> Vec<ReductionKind> {
        let mut selected = Vec::new();
        if !self.symmetry.is_empty() {
            selected.push(ReductionKind::Symmetry);
        }
        if self.partial_order {
            selected.push(ReductionKind::PartialOrder);
        }
        if self.constraint.is_some() {
            selected.push(ReductionKind::StateConstraint);
        }
        selected
    }
}

/// A row of the preservation table (ADR-021 PT-2): an item's form, with
/// whether its formula has an interval operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableForm {
    /// TP-1, a reachable-state invariant.
    ReachableInvariant,
    /// TP-2, bounded MLTL.
    BoundedMltl,
    /// TP-3, infinite-trace safety.
    Safety {
        /// Whether the formula has an interval operator.
        interval: bool,
    },
    /// TP-4, liveness, under any fairness set.
    Liveness {
        /// Whether the formula has an interval operator.
        interval: bool,
    },
    /// The deadlock-freedom item.
    DeadlockFreedom,
    /// A refinement item's safety half (ADR-020).
    RefinementSafety,
    /// A refinement item's liveness half (ADR-020).
    RefinementLiveness,
}

impl TableForm {
    /// The row of a model-check item; `None` for a clause with no property
    /// form.
    pub fn of(item: &ModelCheckItem<'_>) -> Option<Self> {
        let ModelCheckItem::Clause { clause, .. } = item else {
            return Some(Self::DeadlockFreedom);
        };
        let interval = clause
            .operators()
            .iter()
            .any(|operator| operator.interval.is_some());
        Some(match clause.property_form()? {
            PropertyForm::ReachableInvariant => Self::ReachableInvariant,
            PropertyForm::BoundedMltl => Self::BoundedMltl,
            PropertyForm::Safety => Self::Safety { interval },
            PropertyForm::Liveness => Self::Liveness { interval },
        })
    }
}

/// ADR-021 PT-2: whether `reduction` preserves the verdict of an item of
/// `form`. Symmetry and a state constraint read "yes" for every form;
/// partial-order reduction reads "yes" for TP-1, for TP-3 and TP-4 with no
/// interval operator under any fairness set, and for the deadlock-freedom
/// item, and "no" for TP-2, any formula with an interval operator, and both
/// refinement halves.
pub fn preserves(reduction: ReductionKind, form: TableForm) -> bool {
    match reduction {
        ReductionKind::Symmetry | ReductionKind::StateConstraint => match form {
            TableForm::ReachableInvariant
            | TableForm::BoundedMltl
            | TableForm::Safety { .. }
            | TableForm::Liveness { .. }
            | TableForm::DeadlockFreedom
            | TableForm::RefinementSafety
            | TableForm::RefinementLiveness => true,
        },
        ReductionKind::PartialOrder => match form {
            TableForm::ReachableInvariant
            | TableForm::DeadlockFreedom
            | TableForm::Safety { interval: false }
            | TableForm::Liveness { interval: false } => true,
            TableForm::BoundedMltl
            | TableForm::Safety { interval: true }
            | TableForm::Liveness { interval: true }
            | TableForm::RefinementSafety
            | TableForm::RefinementLiveness => false,
        },
    }
}

/// FR-159 (a): the first selected reduction, in the order symmetry,
/// partial-order, constraint, whose row reads "no" for `form`.
pub fn not_preserving(reductions: &Reductions, form: TableForm) -> Option<ReductionKind> {
    reductions
        .selected()
        .into_iter()
        .find(|reduction| !preserves(*reduction, form))
}

/// FR-159 (c): the first selected reduction, in the order symmetry,
/// partial-order, constraint, that `system` does not offer.
pub fn unoffered(reductions: &Reductions, system: &impl TransitionSystem) -> Option<ReductionKind> {
    let offered = system.offered_reductions();
    reductions
        .selected()
        .into_iter()
        .find(|reduction| match reduction {
            ReductionKind::Symmetry => !offered.symmetry,
            ReductionKind::PartialOrder => !offered.partial_order,
            ReductionKind::StateConstraint => !offered.constraint,
        })
}

/// FR-151 "Form" and "Group": check each declaration in request order and
/// form the run's group, each class of two or more keys in ascending key
/// order.
///
/// # Errors
///
/// The first declaration's [`SymmetryRefusal`].
pub fn admit(
    system: &ModelSystem<'_>,
    declarations: &[SymmetryDeclaration],
) -> Result<SymmetryGroup, SymmetryRefusal> {
    let mut classes = Vec::new();
    // A key is in at most one class of its population across every
    // declaration: two classes sharing a key would make the group's classes
    // overlap, and the canonicaliser's map no bijection.
    let mut seen: BTreeMap<&DeclarationKey, BTreeSet<&String>> = BTreeMap::new();
    for (index, declaration) in declarations.iter().enumerate() {
        let population = &declaration.population;
        if system.is_symmetric(population) != Some(true) {
            return Err(SymmetryRefusal::NotSymmetric {
                declaration: index,
                population: population.clone(),
            });
        }
        let universe = system
            .universe(population)
            .ok_or_else(|| SymmetryRefusal::NoUniverse {
                declaration: index,
                population: population.clone(),
            })?;
        let seen = seen.entry(population).or_default();
        for class in &declaration.classes {
            for key in class {
                if !universe.contains(key) {
                    return Err(SymmetryRefusal::UnknownKey {
                        declaration: index,
                        key: key.clone(),
                    });
                }
                if !seen.insert(key) {
                    return Err(SymmetryRefusal::RepeatedKey {
                        declaration: index,
                        key: key.clone(),
                    });
                }
            }
        }
        for class in &declaration.classes {
            if class.len() >= 2 {
                let mut keys = class.clone();
                keys.sort();
                classes.push(SymmetryClass {
                    population: population.clone(),
                    keys,
                });
            }
        }
    }
    Ok(SymmetryGroup { classes })
}

/// FR-151 "Generators": for each class of keys `k1 < … < km`, in class
/// order, the transposition `(k1 k2)` and, when `m >= 3`, the cycle
/// `(k1 k2 … km)`.
pub fn generators(group: &SymmetryGroup) -> Vec<(DeclarationKey, Permutation)> {
    let mut generators = Vec::new();
    for class in &group.classes {
        let [first, second, ..] = class.keys.as_slice() else {
            continue;
        };
        generators.push((
            class.population.clone(),
            Permutation::of(
                &class.population,
                [
                    (first.clone(), second.clone()),
                    (second.clone(), first.clone()),
                ],
            ),
        ));
        if class.keys.len() >= 3 {
            let cycle = class
                .keys
                .iter()
                .zip(class.keys.iter().cycle().skip(1))
                .map(|(from, to)| (from.clone(), to.clone()));
            generators.push((
                class.population.clone(),
                Permutation::of(&class.population, cycle),
            ));
        }
    }
    generators
}

/// FR-151 "Closed initial states": each initial state in subject order and
/// each generator in class order; the first whose image is no initial
/// state's key is `SymmetryBroken`.
///
/// # Errors
///
/// [`ClosureRefusal`] when a state key has no canonical encoding or a
/// permuted set has no canonical-key order.
pub fn closure(
    system: &ModelSystem<'_>,
    group: &SymmetryGroup,
) -> Result<Option<ReductionCause>, ClosureRefusal> {
    let initial = system.initial();
    let mut keys = BTreeSet::new();
    for state in &initial {
        keys.insert(system.state_key(state).map_err(ClosureRefusal::Encoding)?.0);
    }
    let generators = generators(group);
    for (index, state) in initial.iter().enumerate() {
        for (population, generator) in &generators {
            let image = system
                .permute(state, generator)
                .map_err(|_| ClosureRefusal::Unordered)?;
            if !keys.contains(
                &system
                    .state_key(&image)
                    .map_err(ClosureRefusal::Encoding)?
                    .0,
            ) {
                return Ok(Some(ReductionCause::SymmetryBroken {
                    population: population.clone(),
                    cause: SymmetryBrokenCause::InitialStatesNotClosed {
                        initial: u64::try_from(index).unwrap_or(u64::MAX),
                        generator: generator.clone(),
                    },
                }));
            }
        }
    }
    Ok(None)
}

/// Why [`closure`] could not decide the initial states' closure.
#[derive(Debug, thiserror::Error)]
pub enum ClosureRefusal {
    /// A state key has no canonical encoding.
    #[error(transparent)]
    Encoding(EncodingRefusal),
    /// A permuted set or bag has no canonical-key order.
    #[error("a permuted collection has no canonical-key order")]
    Unordered,
}

/// One instance the run checks for its orbit (FR-151 "Instances"): the
/// bound key, every key it stands for, and the group it runs under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstanceGroup {
    /// The bound instance; `None` for an item with no `over` binding.
    pub representative: Option<ModelInstance>,
    /// Every key the instance stands for, ascending; empty with no binding.
    pub orbit: Vec<String>,
    /// The stabiliser of the bound key in the run's group, or the run's
    /// group with no binding.
    pub group: SymmetryGroup,
}

/// FR-151 "Instances": partition `bindings` into orbits of `group` (each
/// class is one orbit; each other key its own), one instance per orbit,
/// bound to the orbit's least key and run under its stabiliser. With no
/// binding (the deadlock-freedom item) the one instance runs under `group`.
pub fn instance_groups(
    bindings: Option<&[ModelInstance]>,
    group: &SymmetryGroup,
) -> Vec<InstanceGroup> {
    let Some(bindings) = bindings else {
        return vec![InstanceGroup {
            representative: None,
            orbit: Vec::new(),
            group: group.clone(),
        }];
    };
    let mut covered: BTreeSet<(&DeclarationKey, &str)> = BTreeSet::new();
    let mut groups = Vec::new();
    for binding in bindings {
        if covered.contains(&(&binding.population, binding.key.as_str())) {
            continue;
        }
        let orbit = group
            .class_of(&binding.population, &binding.key)
            .map_or_else(|| vec![binding.key.clone()], |class| class.keys.clone());
        let Some(least) = orbit.first().cloned() else {
            continue;
        };
        for key in &orbit {
            if let Some(member) = bindings
                .iter()
                .find(|other| other.population == binding.population && other.key == *key)
            {
                covered.insert((&member.population, member.key.as_str()));
            }
        }
        groups.push(InstanceGroup {
            group: group.stabiliser(&binding.population, &least),
            representative: Some(ModelInstance {
                population: binding.population.clone(),
                key: least,
            }),
            orbit,
        });
    }
    groups
}
