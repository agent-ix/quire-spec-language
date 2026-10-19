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

use crate::check::{CheckedGraph, CheckedNode, CheckedStateClause, NodeKind, Observation};
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
/// objects a deletion of it affects follow the receiver's own type.
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
) -> Result<SymbolicFootprint, FootprintRefusal> {
    let derivation = Derivation {
        graph,
        model,
        receiver_type,
        declaring,
        operation,
        populations: populations(model)?,
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
fn parameter_target(value_type: &ValueType) -> Option<EffectiveId> {
    match value_type {
        ValueType::Reference(target) => Some(*target),
        ValueType::Option(payload) => parameter_target(payload),
        _ => None,
    }
}

struct Derivation<'a> {
    graph: &'a CheckedGraph,
    model: &'a StateModel,
    receiver_type: EffectiveId,
    declaring: EffectiveId,
    operation: &'a OperationDeclaration,
    populations: Vec<PopulationTypes>,
}

impl Derivation<'_> {
    fn conforms(&self, sub: EffectiveId, sup: EffectiveId) -> bool {
        self.graph.scope().types().conforms(sub, sup)
    }

    /// The populations whose member type `object` conforms to.
    fn populations_holding(&self, object: EffectiveId) -> impl Iterator<Item = &DeclarationKey> {
        self.populations
            .iter()
            .filter(move |population| {
                population
                    .members
                    .iter()
                    .any(|member| self.conforms(object, *member))
            })
            .map(|population| &population.key)
    }

    /// The populations with a member type that conforms to `supertype`.
    fn populations_of_subtypes(
        &self,
        supertype: EffectiveId,
    ) -> impl Iterator<Item = &DeclarationKey> {
        self.populations
            .iter()
            .filter(move |population| {
                population
                    .members
                    .iter()
                    .any(|member| self.conforms(*member, supertype))
            })
            .map(|population| &population.key)
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
            let target = self
                .operation
                .parameters()
                .iter()
                .find(|(parameter, _)| parameter == name)
                .and_then(|(_, value_type)| parameter_target(value_type))
                .ok_or(FootprintRefusal::Unresolved {
                    what: "deleted parameter",
                })?;
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
        for (name, value_type) in self.operation.parameters() {
            if let Some(target) = parameter_target(value_type) {
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
                    .any(|deleted| self.conforms(*deleted, *target))
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
        for clause in self.graph.state_clauses() {
            let Some(owner) = clause.operation() else {
                continue;
            };
            if owner.declaring != self.declaring
                || owner.declaration.name() != self.operation.name()
            {
                continue;
            }
            for (location, observation) in self.clause_reads(clause)? {
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
            for population in self.populations_relating(*target) {
                reads.insert(SymbolicLocation::AnyMembership {
                    population: population.clone(),
                });
            }
        }
    }

    fn type_of(&self, key: &DeclarationKey) -> Result<EffectiveId, FootprintRefusal> {
        self.model
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
        for declaration in self.graph.scope().types().object_types() {
            let identity = declaration.key();
            if holder.is_some_and(|holder| !self.conforms(identity, holder)) {
                continue;
            }
            let Some(type_key) = self.model.type_key(identity) else {
                continue;
            };
            let fields = self
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

    /// Every location `clause` reads, with the instant it reads it at.
    fn clause_reads(
        &self,
        clause: &CheckedStateClause,
    ) -> Result<Vec<(SymbolicLocation, Observation)>, FootprintRefusal> {
        let mut reads = Vec::new();
        let mut pending = vec![clause.body()];
        while let Some(node) = pending.pop() {
            self.node_reads(clause, node, &mut reads)?;
            pending.extend(node.kind().children().into_iter().map(|id| node.at(id)));
        }
        Ok(reads)
    }

    fn node_reads(
        &self,
        clause: &CheckedStateClause,
        node: CheckedNode<'_>,
        reads: &mut Vec<(SymbolicLocation, Observation)>,
    ) -> Result<(), FootprintRefusal> {
        let observation = clause
            .observations
            .of_read(node.location())
            .unwrap_or_else(|| clause.observation());
        let unsupported = |form| FootprintRefusal::UnsupportedForm {
            clause: clause.name().to_owned(),
            form,
        };
        match node.kind() {
            NodeKind::Attribute {
                reference, field, ..
            } => {
                let field = self.field_key(field.owner, &field.name)?;
                let location = match self.holder(clause, node.at(*reference)) {
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
            NodeKind::Local(slot) if clause.binds_result() && *slot == 1 => {
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
    fn holder(&self, clause: &CheckedStateClause, mut node: CheckedNode<'_>) -> Holder {
        loop {
            match node.kind() {
                NodeKind::Value(operand) | NodeKind::Coerce(operand, _) => {
                    node = node.at(*operand);
                }
                NodeKind::Local(0) => return Holder::Receiver,
                NodeKind::Local(slot) => {
                    let first = 1 + usize::from(clause.binds_result());
                    return match clause.parameters().get(*slot) {
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

/// The object a field read is taken from.
enum Holder {
    Receiver,
    Parameter(String),
    Other,
}
