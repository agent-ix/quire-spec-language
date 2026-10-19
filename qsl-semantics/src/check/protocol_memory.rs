// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-219, FR-220: the S3 memory facets of a protocol's `parallel` controls,
//! attempts and `fence` controls.
//!
//! A `parallel`'s `memory` clause records its source default (`sc` when
//! absent). A nested `parallel` declares the default of its outermost
//! `parallel` or none, and a `parallel` declaring a weak default (`tso` or
//! `ra`) joins `all` with no `outstanding` clause. A `fence` records its
//! ordering. The request's selection, the resolved model and the obligation
//! identity belong to the model checker, not to S3 (FR-219 "Request
//! selection", "Identity").

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AccessOrdering, AttemptForm, JoinPolicy, MemoryModelName, ProtocolDeclarationForm};
use qsl_foundation::Span;

use super::footprint::{FootprintRefusal, SymbolicFootprint, SymbolicLocation};
use super::protocol_clause::{BoundAttempt, CheckedControlTree, CheckedProtocol, ProtocolNodeId};
use crate::model::key::DeclarationKey;
use super::refusal::{CheckCause, CheckRefusal, ProtocolAnchorCause};
use quire_semantic_value::location::{Location, Origin};

/// One checked `parallel` control's source default (FR-219 "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedParallelMemory {
    /// The `parallel` control.
    pub node: ProtocolNodeId,
    /// Its `memory` clause's model, `Sc` when none is written.
    pub memory: MemoryModelName,
}

/// One checked `fence` control (FR-220 "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedFence {
    /// The `fence` control.
    pub node: ProtocolNodeId,
    /// Its ordering: `acquire`, `release`, `acq_rel` or `seq_cst`.
    pub ordering: AccessOrdering,
}

/// What an attempt does to the shared locations of its `parallel` (FR-220,
/// ADR-025 MA-2).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AccessClass {
    /// Reads one shared location only.
    Load,
    /// Writes one shared location with no pre-state read of it.
    Store,
    /// Reads one shared location in the pre-state and writes it.
    Rmw,
    /// Touches no shared location.
    Local,
}

/// One checked attempt inside a `parallel` (FR-220 "Outputs").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedAccess {
    /// The `attempt` control.
    pub node: ProtocolNodeId,
    /// Its access kind over the shared locations of the enclosing
    /// `parallel` controls.
    pub class: AccessClass,
    /// Its `ordering`, when written.
    pub ordering: Option<AccessOrdering>,
}

impl CheckedAccess {
    /// Whether the access carries an ordering (ADR-025 MA-2).
    pub fn is_atomic(&self) -> bool {
        self.ordering.is_some()
    }
}

/// The memory facets of one checked protocol.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckedMemory {
    /// Every `parallel`, in declaration order.
    pub parallels: Vec<CheckedParallelMemory>,
    /// Every `fence`, in declaration order.
    pub fences: Vec<CheckedFence>,
    /// Every attempt inside a `parallel`, in declaration order.
    pub accesses: Vec<CheckedAccess>,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// Record each `parallel`'s source default and each `fence`'s ordering, and
/// refuse a nested default that differs from the outermost `parallel`'s and
/// a weak default on a join other than `all` with no `outstanding` clause
/// (FR-219 "Source").
pub(crate) fn check_models(
    protocol: &ProtocolDeclarationForm,
    tree: &CheckedControlTree,
) -> Result<CheckedMemory, Vec<(Span, CheckRefusal)>> {
    let mut parent: BTreeMap<ProtocolNodeId, ProtocolNodeId> = BTreeMap::new();
    for control in &tree.controls {
        for child in &control.children {
            parent.insert(*child, control.node);
        }
    }
    let parallels: BTreeMap<ProtocolNodeId, &qsl_forms::ParallelForm> = protocol
        .parallels
        .iter()
        .map(|form| (ProtocolNodeId(form.declaration), form))
        .collect();
    let outermost = |node: ProtocolNodeId| {
        let mut found = None;
        let mut current = parent.get(&node).copied();
        while let Some(ancestor) = current {
            if parallels.contains_key(&ancestor) {
                found = Some(ancestor);
            }
            current = parent.get(&ancestor).copied();
        }
        found
    };

    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    let mut memory = CheckedMemory::default();
    for form in &protocol.parallels {
        let node = ProtocolNodeId(form.declaration);
        let declared = form.memory.as_ref();
        let model = declared.map_or(MemoryModelName::Sc, |declared| declared.value);
        memory.parallels.push(CheckedParallelMemory {
            node,
            memory: model,
        });
        if let (Some(declared), Some(outer)) = (declared, outermost(node)) {
            let outer_form = parallels[&outer];
            let outer_model = outer_form
                .memory
                .as_ref()
                .map_or(MemoryModelName::Sc, |outer| outer.value);
            if declared.value != outer_model {
                refusals.push((
                    declared.span,
                    refusal(ProtocolAnchorCause::MemoryConflict {
                        outermost: outer_form.name.name.clone(),
                        outermost_model: outer_model,
                        nested: form.name.name.clone(),
                        nested_model: declared.value,
                        span: declared.span,
                    }),
                ));
            }
        }
        if declared.is_some_and(|declared| declared.value.is_weak()) {
            let policy = match (&form.join.value, &form.outstanding) {
                (JoinPolicy::All, None) => None,
                (JoinPolicy::All, Some(_)) => Some("all outstanding"),
                (JoinPolicy::Any, _) => Some("any"),
                (JoinPolicy::Quorum(_), _) => Some("quorum"),
                (JoinPolicy::Predicate(_), _) => Some("predicate"),
            };
            if let Some(policy) = policy {
                refusals.push((
                    form.join.span,
                    refusal(ProtocolAnchorCause::WeakJoin {
                        parallel: form.name.name.clone(),
                        model,
                        policy: policy.to_owned(),
                        span: form.join.span,
                    }),
                ));
            }
        }
    }
    memory.fences = protocol
        .fences
        .iter()
        .map(|fence| CheckedFence {
            node: ProtocolNodeId(fence.declaration),
            ordering: fence.ordering.value,
        })
        .collect();
    if refusals.is_empty() {
        Ok(memory)
    } else {
        Err(refusals)
    }
}

/// What the location sharing and classification rules read of a footprint
/// location: the field or population it is about, with every object of it
/// taken as one, since which objects a receiver or argument names is known
/// only when the engine instantiates the footprint (ADR-025 MA-1, MA-2).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Place {
    /// A field, of one object or of any object.
    Field(DeclarationKey),
    /// A population's membership, of one object or of any object.
    Membership(DeclarationKey),
}

impl Place {
    fn text(&self) -> String {
        match self {
            Self::Field(field) => field.node.clone(),
            Self::Membership(population) => format!("membership of {}", population.node),
        }
    }
}

/// One attempt's accesses over [`Place`]s: what it reads in the pre-state,
/// what it writes, and which of its places name more than one object or a
/// population membership.
#[derive(Default)]
struct Accesses {
    reads: BTreeSet<Place>,
    writes: BTreeSet<Place>,
    /// Places a single hardware or C11 access cannot be: any-object
    /// locations and population memberships (ADR-025 MA-4).
    wide: BTreeSet<Place>,
}

impl Accesses {
    /// The accesses of `footprint`. The membership of the receiver or of an
    /// argument that every operation reads is the object's existence, not a
    /// data access.
    fn of(footprint: &SymbolicFootprint) -> Self {
        let mut accesses = Self::default();
        for location in &footprint.reads {
            match location {
                SymbolicLocation::Field { field, .. } => {
                    accesses.reads.insert(Place::Field(field.clone()));
                }
                SymbolicLocation::AnyField { field } => {
                    accesses.reads.insert(Place::Field(field.clone()));
                    accesses.wide.insert(Place::Field(field.clone()));
                }
                SymbolicLocation::AnyMembership { population } => {
                    accesses.reads.insert(Place::Membership(population.clone()));
                    accesses.wide.insert(Place::Membership(population.clone()));
                }
                SymbolicLocation::Membership { .. } => {}
            }
        }
        for location in &footprint.writes {
            match location {
                SymbolicLocation::Field { field, .. } => {
                    accesses.writes.insert(Place::Field(field.clone()));
                }
                SymbolicLocation::AnyField { field } => {
                    accesses.writes.insert(Place::Field(field.clone()));
                    accesses.wide.insert(Place::Field(field.clone()));
                }
                SymbolicLocation::Membership { population, .. }
                | SymbolicLocation::AnyMembership { population } => {
                    accesses.writes.insert(Place::Membership(population.clone()));
                    accesses.wide.insert(Place::Membership(population.clone()));
                }
            }
        }
        accesses
    }

    fn places(&self) -> impl Iterator<Item = &Place> {
        self.reads.union(&self.writes)
    }
}

/// Classify every attempt inside a `parallel` over the places its enclosing
/// `parallel` controls share, and refuse an ordering its kind does not take
/// and, under a weak default, an access that is not one location (FR-220,
/// ADR-025 MA-1 to MA-4).
///
/// A place is shared in a `parallel` when attempts in two or more of its
/// branches access it, at least one writes it and at least one reads it: a
/// place nothing reads is a register of its thread, whichever objects the
/// threads write it on. `footprint_of` gives an attempt's symbolic footprint;
/// an attempt whose footprint has no derivation refuses
/// `unsupported_construct`/`not-yet-implemented`.
pub(crate) fn classify(
    protocol: &ProtocolDeclarationForm,
    checked: &CheckedProtocol,
    bound: &[BoundAttempt],
    footprint_of: impl Fn(&BoundAttempt) -> Result<SymbolicFootprint, FootprintRefusal>,
) -> Result<Vec<CheckedAccess>, Vec<(Span, CheckRefusal)>> {
    let tree = &checked.controls;
    let mut parent: BTreeMap<ProtocolNodeId, ProtocolNodeId> = BTreeMap::new();
    for control in &tree.controls {
        for child in &control.children {
            parent.insert(*child, control.node);
        }
    }
    let model_of: BTreeMap<ProtocolNodeId, MemoryModelName> = checked
        .memory
        .parallels
        .iter()
        .map(|parallel| (parallel.node, parallel.memory))
        .collect();

    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    // Each attempt inside a `parallel`, with its accesses and the branches
    // that hold it: (parallel, branch) from the innermost out.
    let mut attempts: Vec<(&AttemptForm, Accesses, Vec<(ProtocolNodeId, ProtocolNodeId)>)> =
        Vec::new();
    for form in &protocol.attempts {
        let node = ProtocolNodeId(form.declaration);
        let mut holders = Vec::new();
        let mut below = node;
        let mut current = parent.get(&node).copied();
        while let Some(ancestor) = current {
            if model_of.contains_key(&ancestor) {
                holders.push((ancestor, below));
            }
            below = ancestor;
            current = parent.get(&ancestor).copied();
        }
        if holders.is_empty() {
            continue;
        }
        let Some(binding) = bound
            .iter()
            .find(|binding| binding.declaration == form.declaration)
        else {
            continue;
        };
        match footprint_of(binding) {
            Ok(footprint) => attempts.push((form, Accesses::of(&footprint), holders)),
            Err(_) => refusals.push((
                form.operation.span,
                refusal(ProtocolAnchorCause::Unimplemented {
                    name: protocol.name.name.clone(),
                    construct: "attempt footprint",
                    span: form.operation.span,
                }),
            )),
        }
    }

    // Per `parallel` and place: the branches that read it and that write it.
    type Touch = (BTreeSet<ProtocolNodeId>, BTreeSet<ProtocolNodeId>);
    let mut touched: BTreeMap<(ProtocolNodeId, Place), Touch> = BTreeMap::new();
    for (_, accesses, holders) in &attempts {
        for &(parallel, branch) in holders {
            for place in accesses.places() {
                let entry = touched.entry((parallel, place.clone())).or_default();
                if accesses.reads.contains(place) {
                    entry.0.insert(branch);
                }
                if accesses.writes.contains(place) {
                    entry.1.insert(branch);
                }
            }
        }
    }
    let shared = |parallel: ProtocolNodeId, place: &Place| {
        touched.get(&(parallel, place.clone())).is_some_and(|(readers, writers)| {
            !readers.is_empty()
                && !writers.is_empty()
                && readers.union(writers).count() >= 2
        })
    };

    let mut accesses = Vec::with_capacity(attempts.len());
    for (form, found, holders) in &attempts {
        let name = &protocol.declarations[form.declaration].name;
        let shared_places: BTreeSet<&Place> = found
            .places()
            .filter(|place| holders.iter().any(|&(parallel, _)| shared(parallel, place)))
            .collect();
        let reads = shared_places.iter().any(|place| found.reads.contains(*place));
        let writes = shared_places.iter().any(|place| found.writes.contains(*place));
        let class = match (reads, writes) {
            (false, false) => AccessClass::Local,
            (true, false) => AccessClass::Load,
            (false, true) => AccessClass::Store,
            (true, true) => AccessClass::Rmw,
        };

        // MA-4: under a weak default, one access is one location.
        let weak = holders
            .iter()
            .filter_map(|(parallel, _)| model_of.get(parallel))
            .copied()
            .find(|model| model.is_weak());
        if let Some(model) = weak {
            let wide = shared_places.iter().any(|place| found.wide.contains(*place));
            if shared_places.len() >= 2 || wide {
                refusals.push((
                    name.span,
                    refusal(ProtocolAnchorCause::AccessShape {
                        attempt: name.name.clone(),
                        model,
                        locations: shared_places.iter().map(|place| place.text()).collect(),
                        span: name.span,
                    }),
                ));
            }
        }

        // MA-3: the orderings each kind takes.
        let ordering = form.ordering.as_ref();
        let admitted = match (class, ordering.map(|ordering| ordering.value)) {
            (_, None) => class != AccessClass::Rmw,
            (AccessClass::Load, Some(o)) => matches!(
                o,
                AccessOrdering::Relaxed | AccessOrdering::Acquire | AccessOrdering::SeqCst
            ),
            (AccessClass::Store, Some(o)) => matches!(
                o,
                AccessOrdering::Relaxed | AccessOrdering::Release | AccessOrdering::SeqCst
            ),
            (AccessClass::Rmw, Some(_)) => true,
            (AccessClass::Local, Some(_)) => false,
        };
        if !admitted {
            let span = ordering.map_or(name.span, |ordering| ordering.span);
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::AccessOrdering {
                    attempt: name.name.clone(),
                    class,
                    ordering: ordering.map(|ordering| ordering.value),
                    span,
                }),
            ));
        }
        accesses.push(CheckedAccess {
            node: ProtocolNodeId(form.declaration),
            class,
            ordering: ordering.map(|ordering| ordering.value),
        });
    }
    if refusals.is_empty() {
        Ok(accesses)
    } else {
        refusals.sort_by_key(|(span, _)| span.start);
        Err(refusals)
    }
}

#[cfg(test)]
mod tests;
