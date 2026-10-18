// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-304, FR-305: the S3 check of an authored model-to-implementation
//! abstraction relation (ADR-017 §3 AR-1 to AR-3), in the `check` core.
//!
//! A relation is a set of bindings, at most one per key: an object type's
//! [`ObjectBinding`], a population's [`PopulationBinding`] and an
//! operation's [`FrameBinding`]. [`CheckedGraph::check_abstraction`] checks
//! the authored bindings against the admitted domain packages the unit
//! selects and produces the one [`CheckedAbstractionRelation`], or every
//! refusal; the relation's constructor is private to this module, so the
//! checker is its only writer.
//!
//! A relation may leave elements unbound: the unbound refusal belongs to the
//! item that references the element (FR-307), not to the relation.
//!
//! The Rust spellings are lexical keys that compare by bytes. This module
//! checks no Rust syntax (FR-304-AC-6); the S2 declaration form
//! (`qsl_forms::AbstractionDeclarationForm`) carries each spelling as
//! written.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::{Code, InternalFault};
use quire_exact::{Identifier, NodeKey};
use quire_semantic_value::location::Location;

use super::lowering::{
    generated_location, object_type_node, population_node, relation_node, text_scalar_node,
    SemanticNode,
};
use super::node_key::{AbstractionTerm, FrameEntry, NodeKeyRefusal, ObjectEntry, PopulationEntry};
use super::CheckedGraph;
use crate::model::key::DeclarationKey;
use crate::model::operation::OperationLookup;

/// A Rust path: a lexical key (QSpec FR-450 `rust-path`).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RustPath(String);

/// A Rust field: an identifier or a tuple-field index (QSpec FR-450
/// `rust-field`).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RustField(String);

/// A Rust parameter identifier (QSpec FR-450 `rust-name`).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RustParameter(String);

macro_rules! lexical_key {
    ($($name:ident),*) => {$(
        impl $name {
            /// The key holding `spelling`, compared by bytes.
            pub fn new(spelling: impl Into<String>) -> Self {
                Self(spelling.into())
            }

            /// The spelling.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    )*};
}
lexical_key!(RustPath, RustField, RustParameter);

/// A frame binding's receiver (QSpec FR-450 `rust-receiver`): the function's
/// method receiver `self`, or one parameter that carries the context object.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RustReceiver {
    /// The bound function's method receiver, `self`.
    SelfValue,
    /// The named function parameter.
    Parameter(RustParameter),
}

/// An object type's binding (ADR-017 AR-2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectBinding {
    /// The Rust type representing the object.
    pub rust_type: RustPath,
    /// Each bound field's Rust field.
    pub fields: BTreeMap<Identifier, RustField>,
}

/// A population's binding (ADR-017 AR-2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PopulationBinding {
    /// The path from the implementation state root to the collection.
    pub collection: RustPath,
}

/// An operation frame's binding (ADR-017 AR-2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FrameBinding {
    /// The implementation function.
    pub function: RustPath,
    /// The function input that carries the operation's context object.
    pub receiver: RustReceiver,
    /// Each operation parameter's Rust parameter.
    pub parameters: BTreeMap<Identifier, RustParameter>,
}

/// An operation's key: the declaring object type and the operation name
/// (ADR-017 AR-2).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OperationKey {
    /// The object type that declares the operation.
    pub declaring: DeclarationKey,
    /// The operation's identifier.
    pub operation: Identifier,
}

/// The key of one binding. Keys order by element kind (object type,
/// population, operation), then by the key itself.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AbstractionKey {
    /// An object type.
    Object(DeclarationKey),
    /// A population.
    Population(DeclarationKey),
    /// An operation frame.
    Frame(OperationKey),
}

impl AbstractionKey {
    /// The identity of the domain package that owns the element.
    pub fn package(&self) -> &str {
        match self {
            Self::Object(key) | Self::Population(key) => &key.package,
            Self::Frame(key) => &key.declaring.package,
        }
    }
}

/// One authored binding: a key and the value of the kind it keys.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum AuthoredBinding {
    /// An object type and its binding.
    Object {
        /// The object type's declaration key.
        key: DeclarationKey,
        /// The binding.
        value: ObjectBinding,
    },
    /// A population and its binding.
    Population {
        /// The population's declaration key.
        key: DeclarationKey,
        /// The binding.
        value: PopulationBinding,
    },
    /// An operation frame and its binding.
    Frame {
        /// The operation key.
        key: OperationKey,
        /// The binding.
        value: FrameBinding,
    },
}

impl AuthoredBinding {
    /// The binding's key.
    pub fn key(&self) -> AbstractionKey {
        match self {
            Self::Object { key, .. } => AbstractionKey::Object(key.clone()),
            Self::Population { key, .. } => AbstractionKey::Population(key.clone()),
            Self::Frame { key, .. } => AbstractionKey::Frame(key.clone()),
        }
    }
}

/// The `state`/`frame` and `state`/`operation_anchor` nodes of an operation
/// a clause or attempt of the unit names (FR-305).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameNodes {
    frame: NodeKey,
    anchor: NodeKey,
}

impl FrameNodes {
    /// The operation's `state`/`frame` node key.
    pub fn frame(&self) -> NodeKey {
        self.frame
    }

    /// The operation's `state`/`operation_anchor` node key.
    pub fn anchor(&self) -> NodeKey {
        self.anchor
    }
}

/// A checked frame binding: the authored binding and, when the unit has the
/// operation's frame, its nodes (FR-305).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedFrameBinding {
    binding: FrameBinding,
    frame_nodes: Option<FrameNodes>,
}

impl CheckedFrameBinding {
    /// The authored binding.
    pub fn binding(&self) -> &FrameBinding {
        &self.binding
    }

    /// The operation's frame and anchor nodes: `Some` when a clause or
    /// attempt of the unit names the operation, `None` otherwise.
    pub fn frame_nodes(&self) -> Option<FrameNodes> {
        self.frame_nodes
    }
}

/// A checked binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedBinding {
    /// An object type's binding.
    Object(ObjectBinding),
    /// A population's binding.
    Population(PopulationBinding),
    /// An operation frame's binding.
    Frame(CheckedFrameBinding),
}

/// The checked abstraction relation: at most one binding per key.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckedAbstractionRelation {
    bindings: BTreeMap<AbstractionKey, CheckedBinding>,
    /// The location of each `abstraction` declaration the bindings were read
    /// from, in source order; empty for a relation bound without source.
    sites: Vec<Location>,
}

impl CheckedAbstractionRelation {
    /// This relation read from the declarations located at `sites`: the
    /// emitter writes one `declaration` occurrence of the relation node at
    /// each (QSpec FR-451).
    #[must_use]
    pub fn with_sites(mut self, sites: Vec<Location>) -> Self {
        self.sites = sites;
        self
    }

    /// The location of each declaration the bindings were read from.
    pub fn sites(&self) -> &[Location] {
        &self.sites
    }

    /// The binding of `key`, if bound.
    pub fn get(&self, key: &AbstractionKey) -> Option<&CheckedBinding> {
        self.bindings.get(key)
    }

    /// Every binding, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&AbstractionKey, &CheckedBinding)> {
        self.bindings.iter()
    }

    /// How many bindings there are.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether nothing is bound.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// The entry of a binding that is not admissible (ADR-017 AR-3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MalformedEntry {
    /// A frame binding maps a parameter to the receiver's spelling.
    ReceiverParameter {
        /// The operation parameter.
        parameter: Identifier,
        /// The Rust parameter, which is also the receiver.
        rust: RustParameter,
    },
    /// The key names a declaration that is not an object type.
    NotAnObjectType,
    /// The key names a declaration that is not a population.
    NotAPopulation,
    /// An object binding names a field the type does not declare.
    UnknownField(Identifier),
    /// A frame binding names a parameter the operation does not declare.
    UnknownParameter(Identifier),
    /// A frame binding omits a parameter the operation declares.
    OmittedParameter(Identifier),
    /// A frame binding maps two parameters to one Rust parameter.
    SharedRustParameter {
        /// The first operation parameter, in identifier order.
        first: Identifier,
        /// The second operation parameter.
        second: Identifier,
        /// The Rust parameter both map to.
        rust: RustParameter,
    },
}

/// Why S3 refused an authored binding (ADR-017 AR-3).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AbstractionRefusal {
    /// The key resolves to no admitted declaration, or an operation key
    /// names a type that does not itself declare the operation.
    MissingDeclaration {
        /// The binding's key.
        key: AbstractionKey,
        /// The identity of the domain package that owns the key.
        package: String,
    },
    /// Two bindings for one key, equal or not.
    ConflictingBinding {
        /// The key.
        key: AbstractionKey,
        /// The earlier binding.
        first: Box<AuthoredBinding>,
        /// The later binding.
        second: Box<AuthoredBinding>,
    },
    /// A binding with a defective entry.
    MalformedDeclaration {
        /// The binding's key.
        key: AbstractionKey,
        /// The defective entry.
        entry: MalformedEntry,
    },
    /// An admitted operation declares a parameter whose name is not an
    /// identifier: a broken check invariant.
    Internal(InternalFault),
}

impl AbstractionRefusal {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingDeclaration { .. } => Code::MissingDeclaration,
            Self::ConflictingBinding { .. } | Self::MalformedDeclaration { .. } => {
                Code::InvalidModelBinding
            }
            Self::Internal(_) => Code::RuntimeInvariant,
        }
    }

    /// The catalog cause.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::MissingDeclaration { .. } => "missing-name",
            Self::ConflictingBinding { .. } => "conflicting-binding",
            Self::MalformedDeclaration { .. } => "malformed-declaration",
            Self::Internal(_) => "established-invariant-broken",
        }
    }
}

impl CheckedGraph {
    /// FR-304, FR-305: checks `bindings`, in source order, against the
    /// admitted domain packages. Every refused binding is reported, in
    /// source order, and a refusal yields no relation.
    ///
    /// # Errors
    ///
    /// Every [`AbstractionRefusal`] of the bindings.
    pub fn check_abstraction(
        &self,
        bindings: &[AuthoredBinding],
    ) -> Result<CheckedAbstractionRelation, Vec<AbstractionRefusal>> {
        let mut accepted: BTreeMap<AbstractionKey, &AuthoredBinding> = BTreeMap::new();
        let mut refusals = Vec::new();
        for binding in bindings {
            if let Err(refusal) = self.admit_binding(binding) {
                refusals.push(*refusal);
                continue;
            }
            match accepted.entry(binding.key()) {
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(binding);
                }
                std::collections::btree_map::Entry::Occupied(first) => {
                    refusals.push(AbstractionRefusal::ConflictingBinding {
                        key: first.key().clone(),
                        first: Box::new((*first.get()).clone()),
                        second: Box::new(binding.clone()),
                    });
                }
            }
        }
        if !refusals.is_empty() {
            return Err(refusals);
        }
        let bindings = accepted
            .into_iter()
            .map(|(key, binding)| {
                let checked = match binding {
                    AuthoredBinding::Object { value, .. } => CheckedBinding::Object(value.clone()),
                    AuthoredBinding::Population { value, .. } => {
                        CheckedBinding::Population(value.clone())
                    }
                    AuthoredBinding::Frame { key, value } => {
                        CheckedBinding::Frame(CheckedFrameBinding {
                            binding: value.clone(),
                            frame_nodes: self.frame_nodes(key),
                        })
                    }
                };
                (key, checked)
            })
            .collect();
        Ok(CheckedAbstractionRelation {
            bindings,
            sites: Vec::new(),
        })
    }

    /// Whether `binding` is admissible on its own: its key resolves and its
    /// maps are well formed.
    fn admit_binding(&self, binding: &AuthoredBinding) -> Result<(), Box<AbstractionRefusal>> {
        let missing = |key: AbstractionKey| {
            Box::new(AbstractionRefusal::MissingDeclaration {
                package: key.package().to_owned(),
                key,
            })
        };
        // QSpec FR-450 rule 2: a key that names a declaration of another
        // kind is malformed, not missing.
        let unresolved = |key: &DeclarationKey, wrong: MalformedEntry| {
            if self.declaration_keys.contains(key) {
                Box::new(AbstractionRefusal::MalformedDeclaration {
                    key: binding.key(),
                    entry: wrong,
                })
            } else {
                missing(binding.key())
            }
        };
        match binding {
            AuthoredBinding::Object { key, value } => {
                let context = self
                    .object_types
                    .get(key)
                    .ok_or_else(|| unresolved(key, MalformedEntry::NotAnObjectType))?;
                if !self.is_object_type_node(key) {
                    return Err(Box::new(AbstractionRefusal::MalformedDeclaration {
                        key: binding.key(),
                        entry: MalformedEntry::NotAnObjectType,
                    }));
                }
                for field in value.fields.keys() {
                    if self
                        .scope
                        .types()
                        .attribute(*context, field.as_str())
                        .is_none()
                    {
                        return Err(Box::new(AbstractionRefusal::MalformedDeclaration {
                            key: binding.key(),
                            entry: MalformedEntry::UnknownField(field.clone()),
                        }));
                    }
                }
                Ok(())
            }
            AuthoredBinding::Population { key, .. } => {
                if self.population_keys.contains_key(key) {
                    Ok(())
                } else {
                    Err(unresolved(key, MalformedEntry::NotAPopulation))
                }
            }
            AuthoredBinding::Frame { key, value } => {
                let context = self
                    .object_types
                    .get(&key.declaring)
                    .ok_or_else(|| unresolved(&key.declaring, MalformedEntry::NotAnObjectType))?;
                if !self.is_object_type_node(&key.declaring) {
                    return Err(Box::new(AbstractionRefusal::MalformedDeclaration {
                        key: binding.key(),
                        entry: MalformedEntry::NotAnObjectType,
                    }));
                }
                let OperationLookup::Declared {
                    declaring,
                    operation,
                } = self.scope.operations().resolve(
                    self.scope.types(),
                    *context,
                    key.operation.as_str(),
                )
                else {
                    return Err(missing(binding.key()));
                };
                if declaring != *context {
                    return Err(missing(binding.key()));
                }
                let malformed = |entry| {
                    Box::new(AbstractionRefusal::MalformedDeclaration {
                        key: binding.key(),
                        entry,
                    })
                };
                let declared: BTreeSet<&str> = operation
                    .parameters()
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect();
                if let Some(unknown) = value
                    .parameters
                    .keys()
                    .find(|name| !declared.contains(name.as_str()))
                {
                    return Err(malformed(MalformedEntry::UnknownParameter(unknown.clone())));
                }
                if let Some((omitted, _)) = operation
                    .parameters()
                    .iter()
                    .find(|(name, _)| !value.parameters.keys().any(|bound| bound.as_str() == name))
                {
                    return Err(malformed(MalformedEntry::OmittedParameter(
                        Identifier::new(omitted).map_err(|_| {
                            Box::new(AbstractionRefusal::Internal(InternalFault::new(
                                "check.abstraction",
                                "operation-parameter-is-an-identifier",
                            )))
                        })?,
                    )));
                }
                if let RustReceiver::Parameter(receiver) = &value.receiver {
                    if let Some((parameter, _)) =
                        value.parameters.iter().find(|(_, rust)| *rust == receiver)
                    {
                        return Err(malformed(MalformedEntry::ReceiverParameter {
                            parameter: parameter.clone(),
                            rust: receiver.clone(),
                        }));
                    }
                }
                let mut seen: BTreeMap<&RustParameter, &Identifier> = BTreeMap::new();
                for (parameter, rust) in &value.parameters {
                    if let Some(first) = seen.insert(rust, parameter) {
                        return Err(malformed(MalformedEntry::SharedRustParameter {
                            first: first.clone(),
                            second: parameter.clone(),
                            rust: rust.clone(),
                        }));
                    }
                }
                Ok(())
            }
        }
    }

    /// Whether the node of `declaration` is a `model`/`object_type` node, the
    /// only target kind QSpec FR-451 admits: a systems interface is an object
    /// type record whose node is another form.
    fn is_object_type_node(&self, declaration: &DeclarationKey) -> bool {
        let Some(node) = self.object_nodes.get(declaration) else {
            return false;
        };
        matches!(
            object_type_node(declaration, self.identity),
            Ok(Some(plain)) if plain.key() == *node
        )
    }

    /// FR-305: the nodes of the frame of the operation `key` names, when a
    /// clause or attempt of the unit names it.
    fn frame_nodes(&self, key: &OperationKey) -> Option<FrameNodes> {
        let context = *self.object_types.get(&key.declaring)?;
        self.operation_frames
            .iter()
            .find(|frame| {
                frame.operation.declaring == context
                    && frame.operation.declaration.name() == key.operation.as_str()
            })
            .map(|frame| FrameNodes {
                frame: frame.frame,
                anchor: frame.anchor,
            })
    }

    /// ADR-017 AR-4 "Referenced elements": the keys of the model elements
    /// the requirement record at `item` references, or `None` when `item`
    /// names no record.
    ///
    /// # Errors
    ///
    /// An internal fault when a reference names a declaration the package
    /// does not hold.
    pub fn item_elements(
        &self,
        item: &OccurrenceKey,
    ) -> Option<Result<BTreeSet<AbstractionKey>, InternalFault>> {
        let references = self.requirements.get(item)?.references();
        let fault = |invariant| InternalFault::new("check.item-elements", invariant);
        let object = |id| {
            self.object_types
                .iter()
                .find_map(|(key, identity)| (*identity == id).then(|| key.clone()))
                .ok_or_else(|| fault("referenced-object-is-admitted"))
        };
        let mut elements = BTreeSet::new();
        let resolved = (|| -> Result<(), InternalFault> {
            for id in &references.objects {
                elements.insert(AbstractionKey::Object(object(*id)?));
            }
            for domain in &references.populations {
                let key = self
                    .population_keys
                    .iter()
                    .find_map(|(key, population)| (population == domain).then(|| key.clone()))
                    .ok_or_else(|| fault("referenced-population-is-admitted"))?;
                elements.insert(AbstractionKey::Population(key));
            }
            if let Some((declaring, name)) = &references.operation {
                elements.insert(AbstractionKey::Frame(OperationKey {
                    declaring: object(*declaring)?,
                    operation: Identifier::new(name)
                        .map_err(|_| fault("referenced-operation-name-is-an-identifier"))?,
                }));
            }
            Ok(())
        })();
        Some(resolved.map(|()| elements))
    }
}

/// The nodes the emitter writes for a checked relation (QSpec FR-451,
/// FR-306): the one `correspondence`/`abstraction_relation` node, and the
/// `model`/`object_type`, `relation`/`population` and `scalar_type`/`text`
/// nodes it names that the checked graph does not hold.
#[derive(Clone, Debug)]
pub struct AbstractionNodes {
    relation: SemanticNode,
    support: Vec<SemanticNode>,
    sites: Vec<Location>,
}

impl AbstractionNodes {
    /// The relation node.
    pub fn relation(&self) -> &SemanticNode {
        &self.relation
    }

    /// The nodes the relation names that the checked graph does not hold,
    /// ascending by key.
    pub fn support(&self) -> &[SemanticNode] {
        &self.support
    }

    /// The location of each of the relation node's `declaration`
    /// occurrences, in order: one per declaration the relation was read from,
    /// or the package root for a relation bound without source. Each
    /// supporting node has one `generated` occurrence at the first.
    pub fn sites(&self) -> &[Location] {
        &self.sites
    }
}

/// Why a checked relation has no nodes.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AbstractionNodeRefusal {
    /// The key names no `model`/`object_type` node of an admitted domain
    /// package: the declaration is not an object type, or its node is a
    /// systems interface, which QSpec FR-451 does not admit as a target.
    #[error("{key:?} is no `model`/`object_type` node of an admitted domain package")]
    NotAnObjectTypeNode {
        /// The declaration.
        key: DeclarationKey,
    },
    /// A Rust path has an empty segment, so it has no array of segments.
    #[error("the Rust path {spelling:?} has an empty segment")]
    EmptyRustSegment {
        /// The path as authored.
        spelling: String,
    },
    /// A node has no key.
    #[error(transparent)]
    Key(#[from] NodeKeyRefusal),
    /// A binding's kind disagrees with its key's: a broken check invariant.
    #[error("a check invariant is broken: {0:?}")]
    Internal(Box<InternalFault>),
}

/// `path` as QSpec FR-451's `RustPath`: its `::`-separated segments.
fn rust_path(path: &RustPath) -> Result<Vec<String>, AbstractionNodeRefusal> {
    let segments: Vec<String> = path.as_str().split("::").map(str::to_owned).collect();
    if segments.iter().any(String::is_empty) {
        return Err(AbstractionNodeRefusal::EmptyRustSegment {
            spelling: path.as_str().to_owned(),
        });
    }
    Ok(segments)
}

/// `receiver` as QSpec FR-451's `RustReceiver`: `"self"` or the parameter.
fn receiver_spelling(receiver: &RustReceiver) -> String {
    match receiver {
        RustReceiver::SelfValue => "self".to_owned(),
        RustReceiver::Parameter(parameter) => parameter.as_str().to_owned(),
    }
}

/// The nodes a relation names that the checked graph does not hold, as they
/// are found.
#[derive(Default)]
struct Support {
    nodes: BTreeMap<NodeKey, SemanticNode>,
    text: Option<NodeKey>,
}

impl CheckedGraph {
    /// QSpec FR-451, FR-306: the nodes the emitter writes for `relation`. The
    /// relation node's key is the digest of its body, so every binding is
    /// part of it, and its `dependencies` are the unique digest-ascending set
    /// of every target.
    ///
    /// # Errors
    ///
    /// An [`AbstractionNodeRefusal`] when a binding names no object type node
    /// or a Rust path has no segments.
    pub fn abstraction_nodes(
        &self,
        relation: &CheckedAbstractionRelation,
    ) -> Result<AbstractionNodes, AbstractionNodeRefusal> {
        let mut support = Support::default();
        let mut objects = Vec::new();
        let mut populations = Vec::new();
        let mut frames = Vec::new();
        for (key, binding) in relation.iter() {
            match (key, binding) {
                (AbstractionKey::Object(declaration), CheckedBinding::Object(value)) => {
                    objects.push(ObjectEntry::new(
                        self.object_target(declaration, &mut support)?,
                        rust_path(&value.rust_type)?,
                        value
                            .fields
                            .iter()
                            .map(|(field, rust)| {
                                (field.as_str().to_owned(), rust.as_str().to_owned())
                            })
                            .collect(),
                    ));
                }
                (AbstractionKey::Population(declaration), CheckedBinding::Population(value)) => {
                    populations.push(PopulationEntry::new(
                        self.population_target(declaration, &mut support)?,
                        rust_path(&value.collection)?,
                    ));
                }
                (AbstractionKey::Frame(operation), CheckedBinding::Frame(checked)) => {
                    let value = checked.binding();
                    frames.push(FrameEntry::new(
                        self.object_target(&operation.declaring, &mut support)?,
                        operation.operation.as_str().to_owned(),
                        rust_path(&value.function)?,
                        receiver_spelling(&value.receiver),
                        value
                            .parameters
                            .iter()
                            .map(|(parameter, rust)| {
                                (parameter.as_str().to_owned(), rust.as_str().to_owned())
                            })
                            .collect(),
                    ));
                }
                (
                    AbstractionKey::Object(_)
                    | AbstractionKey::Population(_)
                    | AbstractionKey::Frame(_),
                    CheckedBinding::Object(_)
                    | CheckedBinding::Population(_)
                    | CheckedBinding::Frame(_),
                ) => {
                    return Err(AbstractionNodeRefusal::Internal(Box::new(
                        InternalFault::new("check.abstraction", "binding-kind-matches-key-kind"),
                    )));
                }
            }
        }
        let node = relation_node(
            AbstractionTerm::new(objects, populations, frames),
            self.identity,
        )?;
        Ok(AbstractionNodes {
            relation: node,
            support: support.nodes.into_values().collect(),
            sites: match relation.sites() {
                [] => vec![generated_location()],
                declared => declared.to_vec(),
            },
        })
    }

    /// The key of the `model`/`object_type` node of `declaration`, adding the
    /// node to `support` when the checked graph does not hold it.
    fn object_target(
        &self,
        declaration: &DeclarationKey,
        support: &mut Support,
    ) -> Result<NodeKey, AbstractionNodeRefusal> {
        let refused = || AbstractionNodeRefusal::NotAnObjectTypeNode {
            key: declaration.clone(),
        };
        let key = *self.object_nodes.get(declaration).ok_or_else(refused)?;
        if self.semantic_graph.node(key).is_some() || support.nodes.contains_key(&key) {
            return Ok(key);
        }
        // A domain record whose node is not an `object_type` (a systems
        // interface) keys to another node than this content does.
        let node = object_type_node(declaration, self.identity)?
            .filter(|node| node.key() == key)
            .ok_or_else(refused)?;
        support.nodes.insert(key, node);
        Ok(key)
    }

    /// The key of the `relation`/`population` node of `declaration`, adding
    /// it and the text scalar node it is typed at to `support`.
    fn population_target(
        &self,
        declaration: &DeclarationKey,
        support: &mut Support,
    ) -> Result<NodeKey, AbstractionNodeRefusal> {
        let text = match support.text {
            Some(text) => text,
            None => {
                let node = text_scalar_node(self.identity)?;
                let text = node.key();
                if self.semantic_graph.node(text).is_none() {
                    support.nodes.insert(text, node);
                }
                support.text = Some(text);
                text
            }
        };
        let node = population_node(declaration, text, self.identity)?;
        let key = node.key();
        support.nodes.insert(key, node);
        Ok(key)
    }
}
