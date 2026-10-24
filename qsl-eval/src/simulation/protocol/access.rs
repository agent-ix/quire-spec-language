// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-220 "Sharing and classification" and "Access shape" at EN-1 (ADR-025
//! MA-1, MA-2, MA-4): which locations a weak `parallel` shares, what each
//! instantiated attempt in it is, and the access shapes the pre-check
//! settles `WeakAccessShape`.
//!
//! EN-1 instantiates each attempt's read and write footprints per FR-120
//! transition identity, over every receiver of its receiver type and every
//! argument vector (FR-155 "State-free"), with the clauses its `contracts`
//! select (ADR-027 FT-1). A location is shared in an outermost weak
//! `parallel` when two attempts in different branches of one `parallel` of
//! its subtree touch it and one of them writes it, a write meeting what the
//! other touches (FR-155's `affects`); every other location an attempt
//! touches is branch-local. An instance whose shared footprint is one field
//! location is a load (read only), a store (written with no pre-state
//! read) or a read-modify-write (both); one with none is local; any other
//! is not one access (MA-4).
//!
//! An attempt's read footprint holds its pre-state reads and the post-state
//! reads outside its own writes (FR-155 "Derivation"), so a write whose
//! postcondition only fixes the new value is a store.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::MemoryModelName;
use qsl_semantics::check::AccessOrdering;
use qsl_semantics::check::{AccessClass, CheckedProtocol, ProtocolNodeId};
use quire_semantic_value::model_transition::ModelTransition;

use super::program::Program;
use super::ProtocolRefusal;
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::footprint::Location;
use crate::simulation::model::encode::{reference_enc, Enc};
use crate::simulation::model::{Cell, ModelSimState, ModelSystem, OperationRef};

/// What one instantiated attempt is (ADR-025 MA-2, MA-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstanceAccess {
    /// One access of its class; the shared location for a load, store or
    /// read-modify-write, none for a local attempt.
    Access {
        /// Its class.
        class: AccessClass,
        /// Its one shared location.
        location: Option<Location>,
    },
    /// Not one access: its shared footprint holds two or more locations, an
    /// any-object location or a population membership.
    Shape {
        /// The shared locations.
        locations: Vec<Location>,
    },
}

/// One instantiated attempt: its FR-120 transition identity, what it is,
/// and the branch-local locations it touches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttemptInstance {
    /// The application's transition identity.
    pub transition: ModelTransition,
    /// What it is.
    pub access: InstanceAccess,
    /// The locations it touches that are not shared.
    pub local: Vec<Location>,
}

/// One attempt of a weak `parallel` (FR-220 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeakAttempt {
    /// The attempt node's static declaration path.
    pub node: String,
    /// Its outermost weak `parallel`'s static declaration path.
    pub parallel: String,
    /// Its `ordering`, when written.
    pub ordering: Option<AccessOrdering>,
    /// Its instances, in ascending transition identity.
    pub instances: Vec<AttemptInstance>,
}

impl WeakAttempt {
    /// What the instance with `transition` is.
    pub fn instance(&self, transition: &ModelTransition) -> Option<&InstanceAccess> {
        self.instances
            .binary_search_by(|instance| instance.transition.cmp(transition))
            .ok()
            .and_then(|at| self.instances.get(at))
            .map(|instance| &instance.access)
    }
}

/// A pre-check's `WeakAccessShape` shape (QSpec FR-439).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccessShape {
    /// `multi-location`: the shared locations of one attempt instance.
    MultiLocation {
        /// The locations.
        locations: Vec<Location>,
    },
    /// `join-policy`: a weak `parallel` whose join is not `all` with no
    /// `outstanding` clause.
    JoinPolicy,
}

/// The pre-check's V-8 cause `WeakAccessShape{site, transition, shape}`
/// (ADR-025 MA-4, MV-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeakAccessShape {
    /// The node it is found at, by its static declaration path: the attempt,
    /// or the `parallel`.
    pub site: String,
    /// The attempt instance's transition identity; none for a join policy.
    pub transition: Option<ModelTransition>,
    /// The shape.
    pub shape: AccessShape,
    /// The shape in QSpec FR-439's typed form.
    wire: Enc,
}

impl WeakAccessShape {
    pub(crate) fn join_policy(site: String) -> Self {
        Self {
            site,
            transition: None,
            shape: AccessShape::JoinPolicy,
            wire: Enc::tagged("join-policy", Vec::new()),
        }
    }

    /// The shape's canonical text: `{"type":"multi-location","locations":
    /// [...]}` or `{"type":"join-policy"}` (QSpec FR-439).
    ///
    /// # Errors
    ///
    /// [`quire_canonical::Error`] when the text exceeds the identity limits.
    pub fn shape_text(&self) -> Result<String, quire_canonical::Error> {
        self.wire
            .canonical_text(quire_canonical::Limits::new(u64::MAX))
    }
}

/// EN-1's sharing and classification of every weak `parallel` of one
/// protocol subject.
#[derive(Clone, Debug, Default)]
pub struct WeakAccesses {
    attempts: BTreeMap<ProtocolNodeId, WeakAttempt>,
    shared: BTreeSet<Location>,
    cells: BTreeMap<Location, Cell>,
}

impl WeakAccesses {
    /// Every attempt of a weak `parallel`, by its node's static declaration
    /// path.
    pub fn attempts(&self) -> impl Iterator<Item = &WeakAttempt> {
        self.attempts.values()
    }

    /// The attempt at the static declaration path `node`.
    pub fn attempt(&self, node: &str) -> Option<&WeakAttempt> {
        self.attempts.values().find(|attempt| attempt.node == node)
    }

    pub(crate) fn of(&self, node: ProtocolNodeId) -> Option<&WeakAttempt> {
        self.attempts.get(&node)
    }

    /// Every shared location, of every weak `parallel`.
    pub fn shared(&self) -> &BTreeSet<Location> {
        &self.shared
    }

    /// The cell a touched field location names.
    pub fn cell(&self, location: &Location) -> Option<&Cell> {
        self.cells.get(location)
    }

    /// The first instance, attempts in ascending node and instances in
    /// ascending transition identity, that is not one access.
    pub(crate) fn first_shape(&self) -> Option<WeakAccessShape> {
        self.attempts.values().find_map(|attempt| {
            attempt.instances.iter().find_map(|instance| {
                let InstanceAccess::Shape { locations } = &instance.access else {
                    return None;
                };
                Some(WeakAccessShape {
                    site: attempt.node.clone(),
                    transition: Some(instance.transition.clone()),
                    shape: AccessShape::MultiLocation {
                        locations: locations.clone(),
                    },
                    wire: Enc::tagged(
                        "multi-location",
                        vec![(
                            "locations",
                            Enc::Array(
                                locations
                                    .iter()
                                    .map(|location| location_enc(location, self.cell(location)))
                                    .collect(),
                            ),
                        )],
                    ),
                })
            })
        })
    }
}

/// QSpec FR-435's typed form of a location: `{"type":"location","object":
/// <reference>,"field":"<name>"}`, `{"type":"location","object_type":
/// "<type>","field":"<name>"}` for any object, `{"type":"location",
/// "population":"<population>"}` for a membership.
pub(crate) fn location_enc(location: &Location, cell: Option<&Cell>) -> Enc {
    let field_name = |field: &qsl_semantics::model::key::DeclarationKey| {
        cell.map_or_else(
            || {
                field
                    .node
                    .rsplit_once('/')
                    .map_or(field.node.clone(), |(_, name)| name.to_owned())
            },
            |cell| cell.name.clone(),
        )
    };
    match location {
        Location::Field { object, field } => Enc::tagged(
            "location",
            vec![
                ("object", reference_enc(object)),
                ("field", Enc::text(field_name(field))),
            ],
        ),
        Location::AnyField { field } => Enc::tagged(
            "location",
            vec![
                (
                    "object_type",
                    Enc::text(
                        field
                            .node
                            .rsplit_once('/')
                            .map_or(field.node.clone(), |(owner, _)| owner.to_owned()),
                    ),
                ),
                ("field", Enc::text(field_name(field))),
            ],
        ),
        Location::Membership { population, .. } | Location::AnyMembership { population } => {
            Enc::tagged(
                "location",
                vec![("population", Enc::text(population.node.clone()))],
            )
        }
        Location::Protocol(_) => Enc::Unencodable("a protocol location"),
    }
}

/// One touch of a location by an attempt instance.
struct Touch {
    location: Location,
    write: bool,
    attempt: ProtocolNodeId,
}

/// One instance's footprint, model locations only.
struct Footprinted {
    transition: ModelTransition,
    reads: BTreeSet<Location>,
    writes: BTreeSet<Location>,
}

/// EN-1 (MA-1, MA-2): instantiate and classify every attempt of every
/// `parallel` `governing` resolves weak, over the identities of `initial`.
///
/// # Errors
///
/// [`ProtocolRefusal`] when an application has no footprint or the
/// identities cannot be listed.
pub(crate) fn classify(
    model: &ModelSystem<'_>,
    protocol: &CheckedProtocol,
    program: &Program<'_>,
    governing: &BTreeMap<ProtocolNodeId, MemoryModelName>,
    initial: Option<&ModelSimState>,
) -> Result<WeakAccesses, ProtocolRefusal> {
    let Some(initial) = initial else {
        return Ok(WeakAccesses::default());
    };
    let unlisted = |_: ExpansionStop| ProtocolRefusal::fault("attempt-identities-list");
    // Each attempt's chain: its outermost weak `parallel` and every
    // (`parallel`, branch label) above it, innermost first.
    let mut chains: BTreeMap<ProtocolNodeId, (ProtocolNodeId, Vec<(ProtocolNodeId, String)>)> =
        BTreeMap::new();
    for node in program.attempts.keys() {
        let mut chain = Vec::new();
        let mut outermost = None;
        let mut below = *node;
        while let Some(above) = program.parent(below)? {
            if governing.contains_key(&above) {
                let label = program.control(below)?.name.clone();
                chain.push((above, label));
                outermost = Some(above);
            }
            below = above;
        }
        if let Some(outermost) =
            outermost.filter(|parallel| governing.get(parallel).is_some_and(|m| m.is_weak()))
        {
            chains.insert(*node, (outermost, chain));
        }
    }

    let mut footprints: BTreeMap<ProtocolNodeId, Vec<Footprinted>> = BTreeMap::new();
    for node in chains.keys() {
        let attempt = program
            .attempts
            .get(node)
            .ok_or_else(|| ProtocolRefusal::fault("an-attempt-node-is-an-attempt"))?;
        let operation = OperationRef {
            declaring: attempt.declaring,
            name: &attempt.operation,
            receiver: attempt.receiver,
        };
        let mut instances = Vec::new();
        for transition in model
            .attempt_identities(initial, &operation)
            .map_err(unlisted)?
        {
            let footprint = model
                .footprint_selecting(&transition, Some(&attempt.contracts))
                .map_err(|_| ProtocolRefusal::Unsupported(super::Unsupported::AccessFootprint))?;
            let model_only = |locations: BTreeSet<Location>| -> BTreeSet<Location> {
                locations
                    .into_iter()
                    .filter(|location| !matches!(location, Location::Protocol(_)))
                    .collect()
            };
            instances.push(Footprinted {
                transition,
                reads: model_only(footprint.reads),
                writes: model_only(footprint.writes),
            });
        }
        footprints.insert(*node, instances);
    }

    // MA-1: two touches in different branches of one `parallel` conflict
    // when one writes a location that meets the other's.
    let mut touches: Vec<Touch> = Vec::new();
    for (node, instances) in &footprints {
        for instance in instances {
            for location in &instance.reads {
                touches.push(Touch {
                    location: location.clone(),
                    write: instance.writes.contains(location),
                    attempt: *node,
                });
            }
            for location in instance.writes.difference(&instance.reads) {
                touches.push(Touch {
                    location: location.clone(),
                    write: true,
                    attempt: *node,
                });
            }
        }
    }
    let apart = |left: ProtocolNodeId, right: ProtocolNodeId| {
        let (Some((_, left)), Some((_, right))) = (chains.get(&left), chains.get(&right)) else {
            return false;
        };
        left.iter().any(|(parallel, label)| {
            right
                .iter()
                .any(|(other, other_label)| other == parallel && other_label != label)
        })
    };
    let mut shared = BTreeSet::new();
    for (index, first) in touches.iter().enumerate() {
        for second in touches.iter().skip(index + 1) {
            if !apart(first.attempt, second.attempt) {
                continue;
            }
            let conflict = (first.write && first.location.affects(&second.location))
                || (second.write && second.location.affects(&first.location));
            if conflict {
                shared.insert(first.location.clone());
                shared.insert(second.location.clone());
            }
        }
    }

    let mut cells = BTreeMap::new();
    for touch in &touches {
        if let std::collections::btree_map::Entry::Vacant(slot) =
            cells.entry(touch.location.clone())
        {
            if let Some(cell) = model.cell_of(initial, &touch.location).map_err(unlisted)? {
                slot.insert(cell);
            }
        }
    }

    let ordering_of: BTreeMap<ProtocolNodeId, Option<AccessOrdering>> = protocol
        .memory
        .accesses
        .iter()
        .map(|access| (access.node, access.ordering))
        .collect();
    let mut attempts = BTreeMap::new();
    for (node, instances) in footprints {
        let Some((parallel, _)) = chains.get(&node) else {
            continue;
        };
        let classified = instances
            .into_iter()
            .map(|instance| classify_instance(instance, &shared))
            .collect();
        attempts.insert(
            node,
            WeakAttempt {
                node: program.text(node)?.to_owned(),
                parallel: program.text(*parallel)?.to_owned(),
                ordering: ordering_of.get(&node).copied().flatten(),
                instances: classified,
            },
        );
    }
    Ok(WeakAccesses {
        attempts,
        shared,
        cells,
    })
}

/// MA-2 and MA-4 over one instance's footprint.
fn classify_instance(instance: Footprinted, shared: &BTreeSet<Location>) -> AttemptInstance {
    let touched: BTreeSet<&Location> = instance.reads.union(&instance.writes).collect();
    let (on, local): (Vec<&Location>, Vec<&Location>) = touched
        .into_iter()
        .partition(|location| shared.contains(*location));
    let access = match on.as_slice() {
        [] => InstanceAccess::Access {
            class: AccessClass::Local,
            location: None,
        },
        [location @ Location::Field { .. }] => {
            let read = instance.reads.contains(*location);
            let write = instance.writes.contains(*location);
            let class = match (read, write) {
                (true, true) => AccessClass::Rmw,
                (false, true) => AccessClass::Store,
                (true, false) | (false, false) => AccessClass::Load,
            };
            InstanceAccess::Access {
                class,
                location: Some((*location).clone()),
            }
        }
        _ => InstanceAccess::Shape {
            locations: on.into_iter().cloned().collect(),
        },
    };
    AttemptInstance {
        transition: instance.transition,
        access,
        local: local.into_iter().cloned().collect(),
    }
}
