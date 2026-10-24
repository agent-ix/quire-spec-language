// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-220 "Sharing and classification" and "Access shape" at EN-1 (ADR-025
//! MA-1, MA-2, MA-4): which locations a weak `parallel` shares, what each
//! instantiated attempt in it is, and the access shapes the pre-check
//! settles `WeakAccessShape`.
//!
//! EN-1 instantiates each attempt's read and write footprints per admitted
//! FR-120 application, over the receivers and argument vectors whose
//! selected effective preconditions hold, with the clauses its `contracts`
//! select (ADR-025 MA-1; ADR-027 FT-1). A location is shared in an outermost weak
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
use quire_exact::{Value, ValueType};
use quire_semantic_value::model_transition::ModelTransition;

use super::program::Program;
use super::ProtocolRefusal;
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::footprint::Location;
use crate::simulation::model::encode::{reference_enc, Enc};
use crate::simulation::model::FootprintError;
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
    /// An instantiated access class does not admit its selected ordering.
    OrderingMismatch {
        /// The instantiated access class.
        access: AccessClass,
        /// The ordering, `None` for an RMW with no ordering.
        ordering: Option<AccessOrdering>,
    },
}

/// One entry in the pre-check's V-8 `WeakAccessShape.failures` list
/// (ADR-025 MA-4, MV-1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeakAccessFailure {
    /// The node's static declaration path.
    pub site: String,
    /// The attempt instance's model transition; none for a reader or join
    /// policy.
    pub transition: Option<ModelTransition>,
    /// The shape that failed.
    pub shape: AccessShape,
    wire: Enc,
}

/// The pre-check's V-8 cause `WeakAccessShape{failures}` (ADR-025 MA-4,
/// MV-1). The headline fields retain the first failure for callers that need
/// a concise diagnostic; `failures` carries every failure at the state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WeakAccessShape {
    /// The node it is found at, by its static declaration path: the attempt,
    /// or the `parallel`.
    pub site: String,
    /// The attempt instance's transition identity; none for a node failure.
    pub transition: Option<ModelTransition>,
    /// The shape.
    pub shape: AccessShape,
    /// Every failure, in deterministic pre-check order.
    pub failures: Vec<WeakAccessFailure>,
    /// The shape in QSpec FR-439's typed form.
    wire: Enc,
}

impl WeakAccessShape {
    pub(crate) fn join_policy(site: String) -> Self {
        Self::single(
            site,
            None,
            AccessShape::JoinPolicy,
            Enc::tagged("join-policy", Vec::new()),
        )
    }

    fn ordering_mismatch(
        site: String,
        transition: ModelTransition,
        access: AccessClass,
        ordering: Option<AccessOrdering>,
    ) -> Self {
        let access_text = match access {
            AccessClass::Load => "load",
            AccessClass::Store => "store",
            AccessClass::Rmw => "rmw",
            AccessClass::Local => "local",
        };
        let ordering_text = ordering.map_or(Enc::Null, |ordering| {
            Enc::text(match ordering {
                AccessOrdering::Relaxed => "relaxed",
                AccessOrdering::Acquire => "acquire",
                AccessOrdering::Release => "release",
                AccessOrdering::AcqRel => "acq_rel",
                AccessOrdering::SeqCst => "seq_cst",
            })
        });
        Self::single(
            site,
            Some(transition),
            AccessShape::OrderingMismatch { access, ordering },
            Enc::tagged(
                "ordering-mismatch",
                vec![
                    ("access", Enc::text(access_text)),
                    ("ordering", ordering_text),
                ],
            ),
        )
    }

    fn single(
        site: String,
        transition: Option<ModelTransition>,
        shape: AccessShape,
        wire: Enc,
    ) -> Self {
        let failure = WeakAccessFailure {
            site: site.clone(),
            transition: transition.clone(),
            shape: shape.clone(),
            wire: wire.clone(),
        };
        Self {
            site,
            transition,
            shape,
            failures: vec![failure],
            wire,
        }
    }

    pub(crate) fn from_failures(
        failures: Vec<WeakAccessFailure>,
    ) -> Result<Option<Self>, ProtocolRefusal> {
        let mut keyed = failures
            .into_iter()
            .map(|failure| {
                let transition_bytes = failure
                    .transition
                    .as_ref()
                    .map(|transition| {
                        transition
                            .canonical_bytes(quire_canonical::Limits::new(u64::MAX))
                            .map_err(|_| ProtocolRefusal::fault("a-weak-access-transition-encodes"))
                    })
                    .transpose()?
                    .unwrap_or_default();
                Ok((
                    failure.transition.is_none(),
                    transition_bytes,
                    failure.site.clone(),
                    failure,
                ))
            })
            .collect::<Result<Vec<_>, ProtocolRefusal>>()?;
        keyed.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        let (failures, transition_texts): (Vec<_>, Vec<_>) = keyed
            .into_iter()
            .map(|(_, bytes, _, failure)| {
                let transition_text =
                    if bytes.is_empty() {
                        None
                    } else {
                        Some(String::from_utf8(bytes).map_err(|_| {
                            ProtocolRefusal::fault("a-weak-access-transition-is-utf8")
                        })?)
                    };
                Ok((failure, transition_text))
            })
            .collect::<Result<Vec<_>, ProtocolRefusal>>()?
            .into_iter()
            .unzip();
        let Some(first) = failures.first().cloned() else {
            return Ok(None);
        };
        let wire = Enc::Array(
            failures
                .iter()
                .zip(transition_texts)
                .map(|(failure, transition_text)| {
                    Enc::Object(vec![
                        ("site", Enc::text(failure.site.clone())),
                        ("transition", transition_text.map_or(Enc::Null, Enc::text)),
                        ("shape", failure.wire.clone()),
                    ])
                })
                .collect(),
        );
        Ok(Some(Self {
            site: first.site.clone(),
            transition: first.transition.clone(),
            shape: first.shape.clone(),
            failures,
            wire,
        }))
    }

    /// Every `{site, transition, shape}` failure as canonical JSON.
    ///
    /// # Errors
    ///
    /// [`quire_canonical::Error`] when the text exceeds the identity limits.
    pub fn failures_text(&self) -> Result<String, quire_canonical::Error> {
        self.wire
            .canonical_text(quire_canonical::Limits::new(u64::MAX))
    }

    /// The first failure's shape as canonical JSON.
    pub fn shape_text(&self) -> Result<String, quire_canonical::Error> {
        self.failures
            .first()
            .ok_or_else(|| quire_canonical::Error::Serialize("no weak-access failures".into()))?
            .shape_text()
    }
}

impl WeakAccessFailure {
    /// The shape's canonical text.
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
    /// Each `event`, `effect`, `commit`, `send` or `receive` node of a weak
    /// `parallel` whose constraint reads a shared location: its static
    /// declaration path and the shared locations it reads.
    readers: BTreeMap<ProtocolNodeId, (String, Vec<Location>)>,
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

    /// The shared locations the constraint of the node at the static
    /// declaration path `node` reads; empty for a node that is no reader or
    /// reads none (ADR-025 MA-1).
    pub fn node_reads(&self, node: &str) -> &[Location] {
        self.readers
            .values()
            .find(|(path, _)| path == node)
            .map_or(&[], |(_, locations)| locations.as_slice())
    }

    /// Every shared location, of every weak `parallel`.
    pub fn shared(&self) -> &BTreeSet<Location> {
        &self.shared
    }

    /// The cell a touched field location names.
    pub fn cell(&self, location: &Location) -> Option<&Cell> {
        self.cells.get(location)
    }

    /// Every access or reader shape the pre-check settles at this state.
    pub(crate) fn first_shape(&self) -> Result<Option<WeakAccessShape>, ProtocolRefusal> {
        let mut failures = Vec::new();
        for attempt in self.attempts.values() {
            for instance in &attempt.instances {
                match &instance.access {
                    InstanceAccess::Shape { locations } => failures.push(self.multi_location(
                        attempt.node.clone(),
                        Some(instance.transition.clone()),
                        locations,
                    )),
                    InstanceAccess::Access { class, .. }
                        if ordering_mismatch(*class, attempt.ordering) =>
                    {
                        failures.push(WeakAccessShape::ordering_mismatch(
                            attempt.node.clone(),
                            instance.transition.clone(),
                            *class,
                            attempt.ordering,
                        ));
                    }
                    InstanceAccess::Access { .. } => {}
                }
            }
        }
        for (node, locations) in self.readers.values() {
            if !locations.is_empty() {
                failures.push(self.multi_location(node.clone(), None, locations));
            }
        }
        WeakAccessShape::from_failures(
            failures
                .into_iter()
                .flat_map(|shape| shape.failures)
                .collect(),
        )
    }

    fn multi_location(
        &self,
        site: String,
        transition: Option<ModelTransition>,
        locations: &[Location],
    ) -> WeakAccessShape {
        WeakAccessShape::single(
            site,
            transition,
            AccessShape::MultiLocation {
                locations: locations.to_vec(),
            },
            Enc::tagged(
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
        )
    }
}

fn ordering_mismatch(class: AccessClass, ordering: Option<AccessOrdering>) -> bool {
    match class {
        AccessClass::Load => matches!(
            ordering,
            Some(AccessOrdering::Release | AccessOrdering::AcqRel)
        ),
        AccessClass::Store => matches!(
            ordering,
            Some(AccessOrdering::Acquire | AccessOrdering::AcqRel)
        ),
        AccessClass::Rmw => ordering.is_none(),
        AccessClass::Local => ordering.is_some(),
    }
}

/// QSpec FR-435's typed form of a location: `{"type":"location","object":
/// <reference>,"field":"<name>"}`, `{"type":"location","object_type":
/// "<type>","field":"<name>"}` for any object, `{"type":"location",
/// "population":"<population>"}` for a membership.
pub(crate) fn location_enc(location: &Location, cell: Option<&Cell>) -> Enc {
    let field_name = |field: &quire_checked::key::DeclarationKey| {
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

/// A node's outermost weak `parallel` and the (`parallel`, branch label)
/// pairs above it, innermost first.
type Chain = (ProtocolNodeId, Vec<(ProtocolNodeId, String)>);

/// One touch of a location by an attempt instance or a reader node.
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

/// Which EN-1 footprint set to derive (ADR-025 MA-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClassificationMode {
    /// Guard-agnostic footprints that reserve memory-model backing.
    Potential,
    /// Applications enabled in the supplied model observation.
    Enabled,
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
    over: &Value,
    mode: ClassificationMode,
) -> Result<WeakAccesses, ProtocolRefusal> {
    let Some(initial) = initial else {
        return Ok(WeakAccesses::default());
    };
    let unlisted = |_: ExpansionStop| ProtocolRefusal::fault("attempt-identities-list");
    // Each node's chain: its outermost weak `parallel` and every
    // (`parallel`, branch label) above it, innermost first.
    let chain_of = |node: ProtocolNodeId| -> Result<Option<Chain>, ProtocolRefusal> {
        let mut chain = Vec::new();
        let mut outermost = None;
        let mut below = node;
        while let Some(above) = program.parent(below)? {
            if governing.contains_key(&above) {
                let label = program.control(below)?.name.clone();
                chain.push((above, label));
                outermost = Some(above);
            }
            below = above;
        }
        Ok(outermost
            .filter(|parallel| governing.get(parallel).is_some_and(|m| m.is_weak()))
            .map(|outermost| (outermost, chain)))
    };
    let mut chains: BTreeMap<ProtocolNodeId, Chain> = BTreeMap::new();
    for node in program.attempts.keys() {
        if let Some(chain) = chain_of(*node)? {
            chains.insert(*node, chain);
        }
    }
    // MA-1: an `event`, `effect`, `commit`, `send` or `receive` node of a
    // weak `parallel` whose constraint reads a model location is a reader of
    // it. A `finish` and a node outside every weak `parallel` are not.
    let mut node_reads: BTreeMap<ProtocolNodeId, BTreeSet<Location>> = BTreeMap::new();
    let mut reader_chains: BTreeMap<ProtocolNodeId, Chain> = BTreeMap::new();
    let Value::Reference(over) = over else {
        return Err(ProtocolRefusal::fault("the-over-input-is-a-reference"));
    };
    let parameters: Vec<(String, ValueType)> = protocol
        .slots
        .binders
        .iter()
        .map(|binder| (binder.name.clone(), binder.value_type.clone()))
        .collect();
    let constraints = program
        .events
        .iter()
        .map(|(node, record)| (node, record.constraint))
        .chain(
            program
                .messages
                .iter()
                .map(|(node, message)| (node, message.record.constraint)),
        );
    for (node, constraint) in constraints {
        let (Some(body), Some(chain)) = (constraint, chain_of(*node)?) else {
            continue;
        };
        let path = program.text(*node)?;
        let reads =
            model
                .node_reads(path, &parameters, body, over)
                .map_err(|error| match error {
                    FootprintError::Refused(refusal) => ProtocolRefusal::NodeFootprint {
                        node: path.to_owned(),
                        refusal,
                    },
                    FootprintError::UnknownTransition | FootprintError::UnknownConstraint => {
                        ProtocolRefusal::fault("a-node-constraint-has-a-footprint")
                    }
                })?;
        node_reads.insert(*node, reads);
        reader_chains.insert(*node, chain);
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
        let transitions = match mode {
            ClassificationMode::Potential => model
                .attempt_identities(initial, &operation)
                .map_err(unlisted)?,
            ClassificationMode::Enabled => {
                let selected = |clause| attempt.contracts.contains(&clause);
                model
                    .apply_operation(initial, &operation, &selected, &mut model.budget())
                    .map_err(unlisted)?
                    .successors
                    .into_iter()
                    .map(|applied| applied.transition)
                    .collect()
            }
        };
        let mut instances = BTreeMap::new();
        for transition in transitions {
            let footprint = model
                .footprint_selecting(&transition, Some(&attempt.contracts))
                .map_err(|_| ProtocolRefusal::Unsupported(super::Unsupported::AccessFootprint))?;
            let model_only = |locations: BTreeSet<Location>| -> BTreeSet<Location> {
                locations
                    .into_iter()
                    .filter(|location| !matches!(location, Location::Protocol(_)))
                    .collect()
            };
            instances
                .entry(transition.clone())
                .or_insert_with(|| Footprinted {
                    transition,
                    reads: model_only(footprint.reads),
                    writes: model_only(footprint.writes),
                });
        }
        footprints.insert(*node, instances.into_values().collect());
    }

    // MA-1: two touches in different branches of one `parallel` conflict
    // when one writes a location that meets the other's.
    let mut touches: Vec<Touch> = Vec::new();
    for (node, reads) in &node_reads {
        for location in reads {
            touches.push(Touch {
                location: location.clone(),
                write: false,
                attempt: *node,
            });
        }
    }
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
        let chain = |node: ProtocolNodeId| chains.get(&node).or_else(|| reader_chains.get(&node));
        let (Some((_, left)), Some((_, right))) = (chain(left), chain(right)) else {
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
    let mut readers = BTreeMap::new();
    for (node, reads) in node_reads {
        let shared_reads = reads
            .into_iter()
            .filter(|location| shared.contains(location))
            .collect();
        readers.insert(node, (program.text(node)?.to_owned(), shared_reads));
    }
    Ok(WeakAccesses {
        attempts,
        readers,
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
