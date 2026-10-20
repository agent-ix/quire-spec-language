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

use qsl_forms::{AccessOrdering, JoinPolicy, ProtocolDeclarationForm};
use qsl_foundation::MemoryModelName;
use qsl_foundation::Span;

use super::footprint::{FootprintRefusal, SymbolicFootprint, SymbolicLocation};
use super::protocol_clause::{BoundAttempt, CheckedControlTree, CheckedProtocol, ProtocolNodeId};
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

impl CheckedParallelMemory {
    /// Whether the source default is the sequentially consistent model
    /// (ADR-027 SE-2).
    pub fn is_sequentially_consistent(&self) -> bool {
        self.memory == MemoryModelName::Sc
    }
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
    /// Its access kind, when the footprint alone fixes it: `Local` for an
    /// attempt that touches no data place. Otherwise `None`: which places are
    /// shared across branches (ADR-025 MA-1) is decided by EN-1.
    pub class: Option<AccessClass>,
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

/// Each control's parent in the tree.
fn parent_map(tree: &CheckedControlTree) -> BTreeMap<ProtocolNodeId, ProtocolNodeId> {
    let mut parent = BTreeMap::new();
    for control in &tree.controls {
        for child in &control.children {
            parent.insert(*child, control.node);
        }
    }
    parent
}

/// The ancestors of `node`, innermost first, each with the child of it that
/// leads to `node`.
fn ancestors(
    parent: &BTreeMap<ProtocolNodeId, ProtocolNodeId>,
    node: ProtocolNodeId,
) -> Vec<(ProtocolNodeId, ProtocolNodeId)> {
    let mut found = Vec::new();
    let mut below = node;
    while let Some(&above) = parent.get(&below) {
        found.push((above, below));
        below = above;
    }
    found
}

/// Record each `parallel`'s source default and each `fence`'s ordering, and
/// refuse a nested default that differs from the outermost `parallel`'s and
/// a weak default on a join other than `all` with no `outstanding` clause
/// (FR-219 "Source").
pub(crate) fn check_models(
    protocol: &ProtocolDeclarationForm,
    tree: &CheckedControlTree,
) -> Result<CheckedMemory, Vec<(Span, CheckRefusal)>> {
    let parent = parent_map(tree);
    let parallels: BTreeMap<ProtocolNodeId, &qsl_forms::ParallelForm> = protocol
        .parallels
        .iter()
        .map(|form| (ProtocolNodeId(form.declaration), form))
        .collect();
    let outermost = |node: ProtocolNodeId| {
        ancestors(&parent, node)
            .into_iter()
            .map(|(above, _)| above)
            .rfind(|above| parallels.contains_key(above))
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
        if let (Some(declared), Some(outer_form)) = (
            declared,
            outermost(node).and_then(|outer| parallels.get(&outer)),
        ) {
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

/// What S3 reads of an attempt's footprint: whether it reads or writes any
/// data place (a field, or a population membership other than the
/// existence of the receiver and the arguments), and the places no single
/// access can be.
#[derive(Default)]
struct Accesses {
    reads: bool,
    writes: bool,
    /// Written any-object fields and written population memberships
    /// (ADR-025 MA-4).
    wide_writes: BTreeSet<String>,
}

impl Accesses {
    /// The existence of the receiver and of each argument, which every
    /// operation reads, is not a data access.
    fn of(footprint: &SymbolicFootprint) -> Self {
        let mut accesses = Self {
            reads: footprint
                .reads
                .iter()
                .any(|location| !matches!(location, SymbolicLocation::Membership { .. })),
            writes: !footprint.writes.is_empty(),
            ..Self::default()
        };
        for location in &footprint.writes {
            match location {
                SymbolicLocation::Field { .. } => {}
                SymbolicLocation::AnyField { field } => {
                    accesses.wide_writes.insert(field.node.clone());
                }
                SymbolicLocation::Membership { population, .. }
                | SymbolicLocation::AnyMembership { population } => {
                    accesses
                        .wide_writes
                        .insert(format!("membership of {}", population.node));
                }
            }
        }
        accesses
    }

    /// The access kinds the footprint leaves possible. Which places are
    /// shared across branches (ADR-025 MA-1) is decided on the instantiated
    /// footprints, so any place read may be the one shared location of a
    /// load, any place written the one of a store, and a read and a written
    /// place together an RMW; with every place branch-local the attempt is
    /// local.
    fn possible(&self) -> Vec<AccessClass> {
        let mut possible = vec![AccessClass::Local];
        if self.reads {
            possible.push(AccessClass::Load);
        }
        if self.writes {
            possible.push(AccessClass::Store);
        }
        if self.reads && self.writes {
            possible.push(AccessClass::Rmw);
        }
        possible
    }
}

/// Whether an access of `class` takes `ordering` (ADR-025 MA-3).
fn takes(class: AccessClass, ordering: AccessOrdering) -> bool {
    match class {
        AccessClass::Load => matches!(
            ordering,
            AccessOrdering::Relaxed | AccessOrdering::Acquire | AccessOrdering::SeqCst
        ),
        AccessClass::Store => matches!(
            ordering,
            AccessOrdering::Relaxed | AccessOrdering::Release | AccessOrdering::SeqCst
        ),
        AccessClass::Rmw => true,
        AccessClass::Local => false,
    }
}

/// Check what an attempt's footprint alone decides about its access
/// (FR-220, ADR-025 MA-2 to MA-4).
///
/// Whether a place is shared across the branches of a `parallel` (MA-1) is
/// decided on the instantiated footprints, by EN-1; it cannot be told from
/// the symbolic footprint, which names no object. So S3 decides only what
/// holds whichever places turn out shared:
///
/// - An `ordering` refuses when no access kind the footprint leaves possible
///   takes it: any ordering on an attempt that touches no data place, a
///   `release` or `acq_rel` on an attempt that writes nothing, an `acquire`
///   or `acq_rel` on one that reads nothing.
/// - Under a weak default, a written any-object field or population
///   membership is not one hardware or C11 access, whatever else is shared
///   (MA-4).
/// - Deferred to EN-1 and the MA-4 pre-check: which kind an attempt is when
///   the footprint leaves several possible (so an RMW with no ordering is not
///   refused here), and every shape that depends on which places are shared,
///   such as two fields of the receiver.
///
/// `footprint_of` gives an attempt's symbolic footprint; an attempt whose
/// footprint has no derivation refuses `unsupported_construct`/
/// `not-yet-implemented`.
pub(crate) fn classify(
    protocol: &ProtocolDeclarationForm,
    checked: &CheckedProtocol,
    bound: &[BoundAttempt],
    footprint_of: impl Fn(&BoundAttempt) -> Result<SymbolicFootprint, FootprintRefusal>,
) -> Result<Vec<CheckedAccess>, Vec<(Span, CheckRefusal)>> {
    let parent = parent_map(&checked.controls);
    let model_of: BTreeMap<ProtocolNodeId, MemoryModelName> = checked
        .memory
        .parallels
        .iter()
        .map(|parallel| (parallel.node, parallel.memory))
        .collect();

    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    let mut accesses = Vec::new();
    for form in &protocol.attempts {
        let node = ProtocolNodeId(form.declaration);
        let Some(name) = protocol
            .declarations
            .get(form.declaration)
            .map(|declaration| &declaration.name)
        else {
            continue;
        };
        let ordering = form.ordering.as_ref();
        let weak = ancestors(&parent, node)
            .into_iter()
            .filter_map(|(above, _)| model_of.get(&above).copied())
            .find(|model| model.is_weak());
        let inside = ancestors(&parent, node)
            .into_iter()
            .any(|(above, _)| model_of.contains_key(&above));
        if !inside {
            // MA-1 shares nothing outside a `parallel`, so by MA-3 the
            // attempt is local and takes no ordering.
            if let Some(ordering) = ordering {
                refusals.push((
                    ordering.span,
                    refusal(ProtocolAnchorCause::AccessOrdering {
                        attempt: name.name.clone(),
                        class: AccessClass::Local,
                        ordering: Some(ordering.value),
                        span: ordering.span,
                    }),
                ));
            }
            continue;
        }
        let Some(binding) = bound
            .iter()
            .find(|binding| binding.declaration == form.declaration)
        else {
            continue;
        };
        let found = match footprint_of(binding) {
            Ok(footprint) => Accesses::of(&footprint),
            Err(_) => {
                refusals.push((
                    form.operation.span,
                    refusal(ProtocolAnchorCause::Unimplemented {
                        name: protocol.name.name.clone(),
                        construct: "attempt footprint",
                        span: form.operation.span,
                    }),
                ));
                continue;
            }
        };
        let possible = found.possible();

        if let (Some(model), false) = (weak, found.wide_writes.is_empty()) {
            refusals.push((
                name.span,
                refusal(ProtocolAnchorCause::AccessShape {
                    attempt: name.name.clone(),
                    model,
                    locations: found.wide_writes.iter().cloned().collect(),
                    span: name.span,
                }),
            ));
        }

        if let Some(ordering) = ordering {
            if !possible.iter().any(|&class| takes(class, ordering.value)) {
                // The kind the footprint leaves: local, or the one other.
                let class = possible
                    .iter()
                    .copied()
                    .find(|&class| class != AccessClass::Local)
                    .unwrap_or(AccessClass::Local);
                refusals.push((
                    ordering.span,
                    refusal(ProtocolAnchorCause::AccessOrdering {
                        attempt: name.name.clone(),
                        class,
                        ordering: Some(ordering.value),
                        span: ordering.span,
                    }),
                ));
            }
        }
        accesses.push(CheckedAccess {
            node,
            class: (possible == [AccessClass::Local]).then_some(AccessClass::Local),
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
