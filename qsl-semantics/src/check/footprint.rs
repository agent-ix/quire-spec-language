// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155 "Derivation": the symbolic read, write and enabling footprints of a
//! model operation, derived from its checked `pre` and `post` clause bodies
//! and its frame with the receiver and the parameters left symbolic.
//!
//! The locations and their derivation are QSpec FR-383's. A location here
//! names an object by its role in the operation ([`FootprintObject`]); the
//! engine instantiates the roles per transition identity.

use std::collections::BTreeSet;

use quire_exact::{EffectiveId, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::location::Location;

use crate::check::observation::Observations;
use crate::check::{CheckedGraph, CheckedNode, CheckedStateClause, NodeKind};
use crate::model::clause_observation::Observation;
use crate::model::key::DeclarationKey;
use crate::model::operation::OperationDeclaration;
use crate::model::state::StateModel;

/// The object a symbolic location is about: the operation's receiver or the
/// argument bound to one of its parameters.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FootprintObject {
    /// The operation's receiver (`self`).
    Receiver,
    /// The argument bound to the named reference parameter.
    Param(String),
}

/// One symbolic location (QSpec FR-383 "Locations").
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SymbolicLocation {
    /// The field of one object.
    Field {
        /// The object.
        object: FootprintObject,
        /// The field's declaration key.
        field: DeclarationKey,
    },
    /// The field on any object.
    AnyField {
        /// The field's declaration key.
        field: DeclarationKey,
    },
    /// Whether one object is a member of a population.
    Membership {
        /// The population's declaration key.
        population: DeclarationKey,
        /// The object.
        object: FootprintObject,
    },
    /// Whether any object is a member of a population.
    AnyMembership {
        /// The population's declaration key.
        population: DeclarationKey,
    },
}

impl SymbolicLocation {
    /// Whether this location, as a write, fixes the value of `read` for the
    /// step: the step chooses what it writes, so a post-state read inside its
    /// own write footprint is not an input to it.
    fn covers(&self, read: &Self) -> bool {
        match (self, read) {
            (Self::AnyField { field: written }, Self::Field { field, .. })
            | (Self::AnyField { field: written }, Self::AnyField { field }) => written == field,
            (
                Self::AnyMembership {
                    population: written,
                },
                Self::Membership { population, .. },
            ) => written == population,
            _ => self == read,
        }
    }
}

/// The symbolic footprints of one operation (FR-155 "Outputs").
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SymbolicFootprint {
    /// `R`: what the operation's enabledness and post-state read.
    pub reads: BTreeSet<SymbolicLocation>,
    /// `W`: what the operation's frame lets it write.
    pub writes: BTreeSet<SymbolicLocation>,
    /// What enabledness reads: all of `R` for a model transition.
    pub enabling: BTreeSet<SymbolicLocation>,
}

/// Why an operation has no derivable footprint.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FootprintRefusal {
    /// A clause body holds a form whose reads QSpec FR-383 does not state.
    #[error("clause `{clause}` holds a {form} whose reads have no footprint rule")]
    UnsupportedForm {
        /// The clause's name.
        clause: String,
        /// The form.
        form: &'static str,
    },
    /// The package names a type or a field the selected model does not
    /// declare.
    #[error("the {what} has no declaration in the selected model")]
    Unresolved {
        /// What was named.
        what: &'static str,
    },
}

/// Derive the symbolic footprints of `operation`, declared by `declaring` and
/// applied to a receiver of the object type `receiver_type`, which is
/// `declaring` or a type conforming to it: the receiver's membership and the
/// objects a deletion of it affects follow the receiver's own type. `arguments`
/// gives the object type of each reference parameter's bound argument; a
/// parameter with no entry has no argument, and contributes no location.
///
/// # Errors
///
/// [`FootprintRefusal::UnsupportedForm`] when a clause of the operation reads
/// through a form QSpec FR-383 gives no rule for, and
/// [`FootprintRefusal::Unresolved`] when the model does not declare a type
/// the derivation needs.
pub fn derive_footprint(
    graph: &CheckedGraph,
    model: &StateModel,
    receiver_type: EffectiveId,
    declaring: EffectiveId,
    operation: &OperationDeclaration,
    arguments: &[(String, EffectiveId)],
) -> Result<SymbolicFootprint, FootprintRefusal> {
    derive_from(
        graph.scope().types(),
        graph.state_clauses(),
        model,
        receiver_type,
        declaring,
        operation,
        arguments,
    )
}

/// [`derive_footprint`] over the type environment and checked state clauses
/// of a package that is still being checked.
pub(crate) fn derive_from(
    types: &TypeEnvironment,
    clauses: &[CheckedStateClause],
    model: &StateModel,
    receiver_type: EffectiveId,
    declaring: EffectiveId,
    operation: &OperationDeclaration,
    arguments: &[(String, EffectiveId)],
) -> Result<SymbolicFootprint, FootprintRefusal> {
    let derivation = Derivation {
        reader: Reader::new(types, model)?,
        clauses,
        receiver_type,
        declaring,
        operation,
        arguments,
    };
    derivation.run()
}

/// A population and the identities of its member types.
struct PopulationTypes {
    key: DeclarationKey,
    members: Vec<EffectiveId>,
}

fn populations(model: &StateModel) -> Result<Vec<PopulationTypes>, FootprintRefusal> {
    model
        .populations()
        .into_iter()
        .map(|population| {
            let members = population
                .member_types
                .iter()
                .map(|member| {
                    model
                        .type_identity(member)
                        .ok_or(FootprintRefusal::Unresolved {
                            what: "population member type",
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(PopulationTypes {
                key: population.key,
                members,
            })
        })
        .collect()
}

/// The object types a value of `value_type` can refer to, through options and
/// collections. A reference held inside a record or tuple (`Composite`) is not
/// followed, so a delete-blocking read of such a field is not derived.
fn referenced_types(value_type: &ValueType, found: &mut Vec<EffectiveId>) {
    match value_type {
        ValueType::Reference(target) => found.push(*target),
        ValueType::Option(payload) => referenced_types(payload, found),
        ValueType::Collection(collection) => referenced_types(collection.element(), found),
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Composite(_)
        | ValueType::Population(_) => {}
    }
}

/// The object type a reference or optional-reference parameter binds.
pub(crate) fn parameter_target(value_type: &ValueType) -> Option<EffectiveId> {
    match value_type {
        ValueType::Reference(target) => Some(*target),
        ValueType::Option(payload) => parameter_target(payload),
        _ => None,
    }
}

/// FR-156 "Atom footprints": the locations the `holds` atom whose checked
/// condition is `condition` reads, with the `over` parameter (slot 0)
/// symbolic as [`FootprintObject::Receiver`]; the engine replaces it with
/// each instance's object.
///
/// # Errors
///
/// [`FootprintRefusal::UnsupportedForm`] when the condition reads through a
/// form QSpec FR-383 gives no rule for, and [`FootprintRefusal::Unresolved`]
/// when the model does not declare a type the derivation needs.
pub fn derive_atom_reads(
    graph: &CheckedGraph,
    model: &StateModel,
    over: &(String, ValueType),
    condition: CheckedNode<'_>,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(graph.scope().types(), model)?;
    let scope = BodyScope {
        name: "holds",
        binds_result: false,
        parameters: std::slice::from_ref(over),
        observations: None,
        observation: Observation::Current,
    };
    Ok(reader
        .body_reads(&scope, condition)?
        .into_iter()
        .map(|(location, _)| location)
        .collect())
}

/// FR-156 "Visibility": the locations the state constraint `clause` reads.
/// The constraint holds for every object of its context type, so the
/// context's `self` reads are reads on any object, and the membership of
/// every population that can hold such an object is read.
///
/// # Errors
///
/// As [`derive_atom_reads`].
pub fn derive_constraint_reads(
    graph: &CheckedGraph,
    model: &StateModel,
    clause: &CheckedStateClause,
) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
    let reader = Reader::new(graph.scope().types(), model)?;
    let mut reads = BTreeSet::new();
    for (location, _) in reader.body_reads(&BodyScope::of_clause(clause), clause.body())? {
        reads.insert(match location {
            SymbolicLocation::Field {
                object: FootprintObject::Receiver,
                field,
            } => SymbolicLocation::AnyField { field },
            other => other,
        });
    }
    for population in &reader.populations {
        if population
            .members
            .iter()
            .any(|member| reader.conforms(*member, clause.context()))
        {
            reads.insert(SymbolicLocation::AnyMembership {
                population: population.key.clone(),
            });
        }
    }
    Ok(reads)
}

/// What a body read derivation consults of the clause around the body: its
/// name for a refusal, the slots its parameters occupy, and the instant each
/// model read observes.
struct BodyScope<'c> {
    name: &'c str,
    binds_result: bool,
    parameters: &'c [(String, ValueType)],
    observations: Option<&'c Observations>,
    observation: Observation,
}

impl<'c> BodyScope<'c> {
    fn of_clause(clause: &'c CheckedStateClause) -> Self {
        Self {
            name: clause.name(),
            binds_result: clause.binds_result(),
            parameters: clause.parameters(),
            observations: Some(&clause.observations),
            observation: clause.observation(),
        }
    }

    fn observation_of(&self, location: &Location) -> Observation {
        self.observations
            .and_then(|observations| observations.of_read(location))
            .unwrap_or(self.observation)
    }
}

/// The type environment, the model and the populations a read derivation
/// resolves types against.
struct Reader<'a> {
    types: &'a TypeEnvironment,
    model: &'a StateModel,
    populations: Vec<PopulationTypes>,
}

impl<'a> Reader<'a> {
    fn new(types: &'a TypeEnvironment, model: &'a StateModel) -> Result<Self, FootprintRefusal> {
        Ok(Self {
            types,
            model,
            populations: populations(model)?,
        })
    }

    fn conforms(&self, sub: EffectiveId, sup: EffectiveId) -> bool {
        self.types.conforms(sub, sup)
    }

    /// The populations that can hold an object of `target` or of a subtype
    /// of it.
    fn populations_relating(&self, target: EffectiveId) -> impl Iterator<Item = &DeclarationKey> {
        self.populations
            .iter()
            .filter(move |population| {
                population
                    .members
                    .iter()
                    .any(|member| self.conforms(target, *member) || self.conforms(*member, target))
            })
            .map(|population| &population.key)
    }

    fn field_key(
        &self,
        owner: EffectiveId,
        name: &str,
    ) -> Result<DeclarationKey, FootprintRefusal> {
        let owner = self
            .model
            .type_key(owner)
            .ok_or(FootprintRefusal::Unresolved {
                what: "field owner",
            })?;
        Ok(DeclarationKey {
            package: owner.package.clone(),
            node: format!("{}/{name}", owner.node),
        })
    }

    /// Every location the body at `root` reads, with the instant it reads
    /// it at.
    fn body_reads(
        &self,
        scope: &BodyScope<'_>,
        root: CheckedNode<'_>,
    ) -> Result<Vec<(SymbolicLocation, Observation)>, FootprintRefusal> {
        let mut reads = Vec::new();
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            self.node_reads(scope, node, &mut reads)?;
            pending.extend(node.kind().children().into_iter().map(|id| node.at(id)));
        }
        Ok(reads)
    }

    fn node_reads(
        &self,
        scope: &BodyScope<'_>,
        node: CheckedNode<'_>,
        reads: &mut Vec<(SymbolicLocation, Observation)>,
    ) -> Result<(), FootprintRefusal> {
        let observation = scope.observation_of(node.location());
        let unsupported = |form| FootprintRefusal::UnsupportedForm {
            clause: scope.name.to_owned(),
            form,
        };
        match node.kind() {
            NodeKind::Attribute {
                reference, field, ..
            } => {
                let field = self.field_key(field.owner, &field.name)?;
                let location = match self.holder(scope, node.at(*reference)) {
                    Holder::Receiver => SymbolicLocation::Field {
                        object: FootprintObject::Receiver,
                        field,
                    },
                    Holder::Parameter(name) => SymbolicLocation::Field {
                        object: FootprintObject::Param(name),
                        field,
                    },
                    Holder::Other => SymbolicLocation::AnyField { field },
                };
                reads.push((location, observation));
            }
            NodeKind::Reaches { edge, .. } => {
                reads.push((
                    SymbolicLocation::AnyField {
                        field: self.field_key(edge.owner, &edge.name)?,
                    },
                    observation,
                ));
            }
            // A clause that names one population (`M::P`, FR-391) reads that
            // population's membership alone.
            NodeKind::AllInstances { population } | NodeKind::Lookup { population, .. }
                if let NodeKind::NamedPopulation { population, .. } =
                    node.at(*population).kind() =>
            {
                reads.push((
                    SymbolicLocation::AnyMembership {
                        population: population.clone(),
                    },
                    observation,
                ));
            }
            // The population operand is a population-typed parameter, which
            // names no population, so every population related to the result
            // type is read.
            NodeKind::AllInstances { .. } | NodeKind::Lookup { .. } => {
                let mut targets = Vec::new();
                referenced_types(node.value_type(), &mut targets);
                for target in targets {
                    for population in self.populations_relating(target) {
                        reads.push((
                            SymbolicLocation::AnyMembership {
                                population: population.clone(),
                            },
                            observation,
                        ));
                    }
                }
            }
            NodeKind::Local(slot) if scope.binds_result && *slot == 1 => {
                let mut targets = Vec::new();
                referenced_types(node.value_type(), &mut targets);
                for target in targets {
                    for population in self.populations_relating(target) {
                        reads.push((
                            SymbolicLocation::AnyMembership {
                                population: population.clone(),
                            },
                            Observation::Post,
                        ));
                    }
                }
            }
            NodeKind::Navigate { .. } => return Err(unsupported("relationship navigation")),
            NodeKind::ReachesEnd { .. } => return Err(unsupported("relationship reachability")),
            NodeKind::Call { .. } | NodeKind::ImportedCall { .. } => {
                return Err(unsupported("function call"))
            }
            NodeKind::Dispatch { .. } => return Err(unsupported("dispatched call")),
            NodeKind::ProtocolReference { .. } => return Err(unsupported("protocol reference")),
            _ => {}
        }
        Ok(())
    }

    /// Which object a field read is taken from.
    fn holder(&self, scope: &BodyScope<'_>, mut node: CheckedNode<'_>) -> Holder {
        loop {
            match node.kind() {
                NodeKind::Value(operand) | NodeKind::Coerce(operand, _) => {
                    node = node.at(*operand);
                }
                NodeKind::Local(0) => return Holder::Receiver,
                NodeKind::Local(slot) => {
                    let first = 1 + usize::from(scope.binds_result);
                    return match scope.parameters.get(*slot) {
                        Some((name, value_type))
                            if *slot >= first && parameter_target(value_type).is_some() =>
                        {
                            Holder::Parameter(name.clone())
                        }
                        _ => Holder::Other,
                    };
                }
                _ => return Holder::Other,
            }
        }
    }
}

struct Derivation<'a> {
    reader: Reader<'a>,
    clauses: &'a [CheckedStateClause],
    receiver_type: EffectiveId,
    declaring: EffectiveId,
    operation: &'a OperationDeclaration,
    arguments: &'a [(String, EffectiveId)],
}

impl Derivation<'_> {
    /// The populations whose member type `object` conforms to.
    fn populations_holding(&self, object: EffectiveId) -> impl Iterator<Item = &DeclarationKey> {
        self.reader
            .populations
            .iter()
            .filter(move |population| {
                population
                    .members
                    .iter()
                    .any(|member| self.reader.conforms(object, *member))
            })
            .map(|population| &population.key)
    }

    /// The populations with a member type that conforms to `supertype`.
    fn populations_of_subtypes(
        &self,
        supertype: EffectiveId,
    ) -> impl Iterator<Item = &DeclarationKey> {
        self.reader
            .populations
            .iter()
            .filter(move |population| {
                population
                    .members
                    .iter()
                    .any(|member| self.reader.conforms(*member, supertype))
            })
            .map(|population| &population.key)
    }

    /// The object type of the argument bound to the parameter `name`.
    fn argument_type(&self, name: &str) -> Option<EffectiveId> {
        self.arguments
            .iter()
            .find(|(parameter, _)| parameter == name)
            .map(|(_, object_type)| *object_type)
    }

    fn run(&self) -> Result<SymbolicFootprint, FootprintRefusal> {
        let effect = self.operation.effect();
        let mut writes = BTreeSet::new();
        let mut reads = BTreeSet::new();

        for field in &effect.receiver_modifies {
            writes.insert(SymbolicLocation::Field {
                object: FootprintObject::Receiver,
                field: field.clone(),
            });
        }
        for field in &effect.modifies {
            writes.insert(SymbolicLocation::AnyField {
                field: field.clone(),
            });
        }
        let mut deleted_types: Vec<EffectiveId> = Vec::new();
        if effect.deletes_receiver {
            deleted_types.push(self.receiver_type);
            for population in self.populations_holding(self.receiver_type) {
                writes.insert(SymbolicLocation::Membership {
                    population: population.clone(),
                    object: FootprintObject::Receiver,
                });
            }
        }
        for name in &effect.deletes_arguments {
            let Some(target) = self.argument_type(name) else {
                continue;
            };
            deleted_types.push(target);
            for population in self.populations_holding(target) {
                writes.insert(SymbolicLocation::Membership {
                    population: population.clone(),
                    object: FootprintObject::Param(name.clone()),
                });
            }
        }
        for deleted in &effect.deletes {
            let deleted = self.type_of(deleted)?;
            deleted_types.push(deleted);
            for population in self.populations_of_subtypes(deleted) {
                writes.insert(SymbolicLocation::AnyMembership {
                    population: population.clone(),
                });
            }
        }
        let mut created_types: Vec<EffectiveId> = Vec::new();
        for created in &effect.creates {
            let created = self.type_of(created)?;
            created_types.push(created);
            for population in self.populations_of_subtypes(created) {
                writes.insert(SymbolicLocation::AnyMembership {
                    population: population.clone(),
                });
            }
        }

        // The receiver's and each reference argument's membership.
        for population in self.populations_holding(self.receiver_type) {
            reads.insert(SymbolicLocation::Membership {
                population: population.clone(),
                object: FootprintObject::Receiver,
            });
        }
        for (name, _) in self.operation.parameters() {
            if let Some(target) = self.argument_type(name) {
                for population in self.populations_holding(target) {
                    reads.insert(SymbolicLocation::Membership {
                        population: population.clone(),
                        object: FootprintObject::Param(name.clone()),
                    });
                }
            }
        }

        // Every field that could name a deleted object blocks its deletion.
        for (field, targets) in self.reference_fields(None)? {
            if targets.iter().any(|target| {
                deleted_types
                    .iter()
                    .any(|deleted| self.reader.conforms(*deleted, *target))
            }) {
                reads.insert(SymbolicLocation::AnyField { field });
            }
        }

        // A creation reads the membership of the populations it adds to and
        // of the populations its reference fields name; so does a write to a
        // reference-valued field.
        for created in &created_types {
            for population in self.populations_of_subtypes(*created) {
                reads.insert(SymbolicLocation::AnyMembership {
                    population: population.clone(),
                });
            }
            for (_, targets) in self.reference_fields(Some(*created))? {
                self.read_target_memberships(&targets, &mut reads);
            }
        }
        let written: BTreeSet<&DeclarationKey> = effect
            .modifies
            .iter()
            .chain(&effect.receiver_modifies)
            .collect();
        for (field, targets) in self.reference_fields(None)? {
            if written.contains(&field) {
                self.read_target_memberships(&targets, &mut reads);
            }
        }

        // The clause bodies.
        for clause in self.clauses {
            let Some(owner) = clause.operation() else {
                continue;
            };
            if owner.declaring != self.declaring
                || owner.declaration.name() != self.operation.name()
            {
                continue;
            }
            for (location, observation) in self
                .reader
                .body_reads(&BodyScope::of_clause(clause), clause.body())?
            {
                // A post-state read inside the write footprint reads a value
                // the step itself chooses.
                let chosen = observation == Observation::Post
                    && writes.iter().any(|written| written.covers(&location));
                if !chosen {
                    reads.insert(location);
                }
            }
        }

        Ok(SymbolicFootprint {
            enabling: reads.clone(),
            reads,
            writes,
        })
    }

    fn read_target_memberships(
        &self,
        targets: &[EffectiveId],
        reads: &mut BTreeSet<SymbolicLocation>,
    ) {
        for target in targets {
            for population in self.reader.populations_relating(*target) {
                reads.insert(SymbolicLocation::AnyMembership {
                    population: population.clone(),
                });
            }
        }
    }

    fn type_of(&self, key: &DeclarationKey) -> Result<EffectiveId, FootprintRefusal> {
        self.reader
            .model
            .type_identity(key)
            .ok_or(FootprintRefusal::Unresolved { what: "frame type" })
    }

    /// Every reference-valued field of the object types `holder` conforms to,
    /// or of every object type when `holder` is `None`, with the object types
    /// each can refer to.
    fn reference_fields(
        &self,
        holder: Option<EffectiveId>,
    ) -> Result<Vec<(DeclarationKey, Vec<EffectiveId>)>, FootprintRefusal> {
        let mut found = Vec::new();
        for declaration in self.reader.types.object_types() {
            let identity = declaration.key();
            if holder.is_some_and(|holder| !self.reader.conforms(identity, holder)) {
                continue;
            }
            let Some(type_key) = self.reader.model.type_key(identity) else {
                continue;
            };
            let fields = self
                .reader
                .model
                .fields(type_key)
                .map_err(|_| FootprintRefusal::Unresolved {
                    what: "object type fields",
                })?
                .unwrap_or_default();
            for field in fields {
                let mut targets = Vec::new();
                referenced_types(&field.value_type, &mut targets);
                if !targets.is_empty() {
                    found.push((field.key, targets));
                }
            }
        }
        found.sort();
        found.dedup();
        Ok(found)
    }
}

/// The object a field read is taken from.
enum Holder {
    Receiver,
    Parameter(String),
    Other,
}
