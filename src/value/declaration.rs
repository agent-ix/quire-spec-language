// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-143 declared record, tuple and model object-type registry, and
//! the FR-149 checked equality layer over it.
//!
//! It owns the registry (`TypeEnvironment`, `ObjectTypeDeclaration`,
//! `CompositeDeclaration`, `CompositeShape`, `InvalidDeclaration`,
//! `RecursionEdges`, `DeclarationCause`) and the check-level equality layer
//! (`EqualityOperator`, `EqualityOperand`, `EqualitySchedule`,
//! `CheckedEquality`, `TypeEnvironment::check_equality`,
//! `admits_equality_conversion`, `operand_value`). None of these are kernel
//! types (ADR-011 §6.1: "`TypeEnvironment` and `ObjectTypeDeclaration` ...
//! are not kernel types and stay in layer 3"), so this is a layer-3
//! `semantic_value` module, over this crate's own `Value`/`ValueType`
//! (`value::composite`'s module doc).
//!
//! The equality layer lives here because it is parameterized over a
//! `TypeEnvironment` and a checked `ValueType`. The occurrence-pair walk it
//! schedules (`plan_pairs`/`planned_equality`/`plan_equality`) takes only a
//! pair of completed `Value`s and is owned by `value::equality`.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::{
    compare_shifted, power_of_ten_bits, sbits, sdigits, Charge, ChargePoint, ComparisonOperator,
    Decimal, DecimalOperation, IllTyped, IllTypedCause, Integer, LimitKind, Meter, Presence,
    Rational,
};

use super::composite::{
    composite, fill_slots, match_names, refuse, retain_composite, Component, ConstructionCause,
    ConstructionRefusal, Deferred, FieldDeclaration, FieldExpression, FieldValue, Value, ValueType,
};
use super::decimal::{evaluate_decimal, DecimalType};
use super::enumeration::compare_enum;
use super::equality::planned_equality;
use super::node::NodeKey;
use super::outcome::{Outcome, Refusal, Stop};
use super::quantity::{
    compare_quantity, convert_quantity, ConvertedValue, Quantity, QuantityTarget,
};
use super::text::compare_text;
use quire_exact::CollectionKind;

/// The shape of a composite declaration. Complete V1 has no variant or sum
/// declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositeShape {
    /// A record with fields in declaration order.
    Record(Vec<FieldDeclaration>),
    /// A tuple of exactly these position types.
    Tuple(Vec<ValueType>),
}

/// A record or tuple declaration with its producer-assigned I04 node key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositeDeclaration {
    key: NodeKey,
    name: String,
    shape: CompositeShape,
}

impl CompositeDeclaration {
    /// The declaration `name` with node key `key`.
    pub fn new(key: NodeKey, name: impl Into<String>, shape: CompositeShape) -> Self {
        Self {
            key,
            name: name.into(),
            shape,
        }
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declared name, used only to name refusals.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared shape.
    pub fn shape(&self) -> &CompositeShape {
        &self.shape
    }
}

/// A model object type exported by a bound model, with its attributes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectTypeDeclaration {
    key: NodeKey,
    name: String,
    attributes: Vec<FieldDeclaration>,
    /// Every directly declared supertype (FR-151/FR-152/FR-153 generalization,
    /// #204 round 1 H1), empty unless [`Self::with_supertypes`] sets it.
    supertypes: Vec<NodeKey>,
}

impl ObjectTypeDeclaration {
    /// The object type `name` with declaration identity `key`, declaring no
    /// supertype. See [`Self::with_supertypes`] to declare one.
    pub fn new(key: NodeKey, name: impl Into<String>, attributes: Vec<FieldDeclaration>) -> Self {
        Self {
            key,
            name: name.into(),
            attributes,
            supertypes: Vec::new(),
        }
    }

    /// Declares this object type's own direct supertypes: every key
    /// [`TypeEnvironment::new`] admits here must itself name a declared
    /// object type, and the whole supertypes graph must be acyclic. Consumes
    /// and returns `self` so every existing [`Self::new`] call site is
    /// unaffected.
    #[must_use]
    pub fn with_supertypes(mut self, supertypes: Vec<NodeKey>) -> Self {
        self.supertypes = supertypes;
        self
    }

    /// The object-type declaration identity.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The attributes in declaration order.
    pub fn attributes(&self) -> &[FieldDeclaration] {
        &self.attributes
    }

    /// Every directly declared supertype, in declaration order.
    pub fn supertypes(&self) -> &[NodeKey] {
        &self.supertypes
    }
}

/// Why a declaration set is not admitted.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("declaration {declaration} refused: {cause:?}")]
pub struct InvalidDeclaration {
    /// The name of the declaration where the refusal originates.
    pub declaration: String,
    /// The typed cause.
    pub cause: DeclarationCause,
}

impl InvalidDeclaration {
    /// The `refused { code }` spelling.
    pub fn code(&self) -> &'static str {
        match self.cause {
            DeclarationCause::DuplicateKey
            | DeclarationCause::DuplicateMember(_)
            | DeclarationCause::UnknownDeclaration(_) => "invalid_semantic_graph",
            DeclarationCause::Type(_)
            | DeclarationCause::Recursion { .. }
            | DeclarationCause::GeneralizationCycle { .. } => IllTyped::CODE,
        }
    }
}

/// Which recursion-rule subgraph has a cycle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RecursionEdges {
    /// A cycle of edges that start at tuple positions.
    Unnamed,
    /// A cycle of edges that pass no `?`, `Option` or minimum-zero collection.
    NonEscaping,
}

/// The typed cause of an [`InvalidDeclaration`].
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DeclarationCause {
    /// Two declarations share one node key.
    DuplicateKey,
    /// Two fields or attributes of one declaration share this name.
    DuplicateMember(String),
    /// A type names a key that is no declaration of the package.
    UnknownDeclaration(NodeKey),
    /// A member type is ill-typed: a `Reference<T>` target that is not a model
    /// object type (`type-mismatch`), or an IEEE-bearing set, bag or ordered-set
    /// element type (`operator-ineligible`).
    Type(IllTypedCause),
    /// The recursion rule refuses this cycle of declaration names, whose first
    /// and last entries are the same declaration.
    Recursion {
        /// The offending subgraph.
        edges: RecursionEdges,
        /// The declaration names along the cycle.
        cycle: Vec<String>,
    },
    /// An object type's own declared `supertypes` (H1, #204 round 1) form a
    /// cycle: a separate graph from [`Self::Recursion`]'s field-containment
    /// one -- generalization, not containment -- so it earns its own
    /// variant rather than reusing [`RecursionEdges`].
    GeneralizationCycle {
        /// The object-type names along the cycle, first and last equal.
        cycle: Vec<String>,
    },
}

/// One checked package's closed, admitted record, tuple and object-type
/// declarations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeEnvironment {
    composites: BTreeMap<NodeKey, CompositeDeclaration>,
    object_types: BTreeMap<NodeKey, ObjectTypeDeclaration>,
    /// Every object type's own proper ancestor set (H1, #204 round 1):
    /// transitive, not just direct, `supertypes`. Precomputed once in
    /// [`TypeEnvironment::new`], after the supertypes graph is known
    /// acyclic, so [`Self::conforms`] is a plain set lookup.
    ancestors: BTreeMap<NodeKey, BTreeSet<NodeKey>>,
}

/// One containment edge of the recursion rule.
#[derive(Clone, Copy)]
struct Edge {
    target: NodeKey,
    named: bool,
    escapes: bool,
}

impl TypeEnvironment {
    /// Admit `composites` and `object_types` as one closed environment.
    pub fn new(
        composites: impl IntoIterator<Item = CompositeDeclaration>,
        object_types: impl IntoIterator<Item = ObjectTypeDeclaration>,
    ) -> Result<Self, InvalidDeclaration> {
        let mut environment = Self::default();
        for declaration in composites {
            let refuse = |cause| InvalidDeclaration {
                declaration: declaration.name.clone(),
                cause,
            };
            if let CompositeShape::Record(fields) = &declaration.shape {
                if let Some(name) = duplicate_name(fields) {
                    return Err(refuse(DeclarationCause::DuplicateMember(name)));
                }
            }
            if environment.object_types.contains_key(&declaration.key)
                || environment.composites.contains_key(&declaration.key)
            {
                return Err(refuse(DeclarationCause::DuplicateKey));
            }
            environment.composites.insert(declaration.key, declaration);
        }
        for declaration in object_types {
            let refuse = |cause| InvalidDeclaration {
                declaration: declaration.name.clone(),
                cause,
            };
            if let Some(name) = duplicate_name(&declaration.attributes) {
                return Err(refuse(DeclarationCause::DuplicateMember(name)));
            }
            if environment.object_types.contains_key(&declaration.key)
                || environment.composites.contains_key(&declaration.key)
            {
                return Err(refuse(DeclarationCause::DuplicateKey));
            }
            environment
                .object_types
                .insert(declaration.key, declaration);
        }
        environment.check_member_types()?;
        environment.check_recursion(RecursionEdges::Unnamed)?;
        environment.check_recursion(RecursionEdges::NonEscaping)?;
        environment.check_supertypes()?;
        environment.ancestors = environment.compute_ancestors();
        Ok(environment)
    }

    /// The admitted record or tuple declaration with this key.
    pub fn composite(&self, key: NodeKey) -> Option<&CompositeDeclaration> {
        self.composites.get(&key)
    }

    /// Every admitted record and tuple declaration in key order.
    pub(crate) fn composites(&self) -> impl Iterator<Item = &CompositeDeclaration> {
        self.composites.values()
    }

    /// The admitted object type with this key.
    pub fn object_type(&self, key: NodeKey) -> Option<&ObjectTypeDeclaration> {
        self.object_types.get(&key)
    }

    /// Every admitted object type in key order, for resolving a type name
    /// to `ValueType::Reference` alongside [`Self::composites`].
    pub(crate) fn object_types(&self) -> impl Iterator<Item = &ObjectTypeDeclaration> {
        self.object_types.values()
    }

    /// Whether `sub` conforms to `sup` (H1, #204 round 1): reflexive
    /// (`sub == sup` always conforms), or `sup` is a proper ancestor of
    /// `sub` in the admitted supertypes graph. `false` for either key
    /// outside this environment's own admitted object types, never a panic.
    pub fn conforms(&self, sub: NodeKey, sup: NodeKey) -> bool {
        sub == sup
            || self
                .ancestors
                .get(&sub)
                .is_some_and(|ancestors| ancestors.contains(&sup))
    }

    /// Check a type named outside a declaration (a parameter or result type):
    /// every named declaration exists, every `Reference<T>` names a model
    /// object type, and no set, bag or ordered set has an IEEE-bearing element
    /// type.
    pub fn check_type(&self, value_type: &ValueType) -> Result<(), IllTyped> {
        match self.type_refusal(value_type) {
            None => Ok(()),
            Some(DeclarationCause::Type(cause)) => Err(IllTyped { cause }),
            Some(_) => Err(IllTyped {
                cause: IllTypedCause::TypeMismatch,
            }),
        }
    }

    /// Whether `value_type` contains `Float32` or `Float64` at any depth,
    /// through record and tuple declarations included.
    pub fn contains_ieee(&self, value_type: &ValueType) -> bool {
        let mut visited = BTreeSet::new();
        let mut pending = vec![value_type];
        while let Some(value_type) = pending.pop() {
            match value_type {
                ValueType::Float(_) => return true,
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Composite(key) => {
                    if !visited.insert(*key) {
                        continue;
                    }
                    match self
                        .composites
                        .get(key)
                        .map(|declaration| &declaration.shape)
                    {
                        Some(CompositeShape::Record(fields)) => {
                            pending.extend(fields.iter().map(FieldDeclaration::value_type));
                        }
                        Some(CompositeShape::Tuple(positions)) => pending.extend(positions),
                        None => {}
                    }
                }
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Quantity(_)
                | ValueType::Text(_)
                | ValueType::Enum(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => {}
            }
        }
        false
    }

    /// The first refusal of one type's named declarations and element types.
    fn type_refusal(&self, value_type: &ValueType) -> Option<DeclarationCause> {
        let mut pending = vec![value_type];
        while let Some(value_type) = pending.pop() {
            match value_type {
                ValueType::Composite(key) if !self.composites.contains_key(key) => {
                    return Some(DeclarationCause::UnknownDeclaration(*key));
                }
                ValueType::Reference(key) if !self.object_types.contains_key(key) => {
                    return Some(if self.composites.contains_key(key) {
                        DeclarationCause::Type(IllTypedCause::TypeMismatch)
                    } else {
                        DeclarationCause::UnknownDeclaration(*key)
                    });
                }
                // FR-153 names a population binding only as the direct
                // operand of `allInstances`/`lookup` (checked by
                // `Typer::all_instances`/`Typer::lookup` themselves, which
                // never route the population expression's own type through
                // this walk) and as a bare parameter type (bypassed by
                // `bind_parameters`, the one caller allowed to name it).
                // Every other named-type context -- an equality operand or
                // `Convert` target (`TypeEnvironment::check_equality`'s own
                // explicit refusal covers the former), an `Option` payload, a
                // collection element, or a record/tuple/object-type member --
                // refuses it here, never admitting a binding into a context
                // FR-153 never gives it Outputs for.
                ValueType::Population(_) => {
                    return Some(DeclarationCause::Type(IllTypedCause::OperatorIneligible));
                }
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => {
                    if collection.kind() != CollectionKind::Sequence
                        && self.contains_ieee(collection.element())
                    {
                        return Some(DeclarationCause::Type(IllTypedCause::OperatorIneligible));
                    }
                    pending.push(collection.element());
                }
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Float(_)
                | ValueType::Quantity(_)
                | ValueType::Text(_)
                | ValueType::Enum(_)
                | ValueType::Composite(_)
                | ValueType::Reference(_) => {}
            }
        }
        None
    }

    fn check_member_types(&self) -> Result<(), InvalidDeclaration> {
        let composites = self.composites.values().map(|declaration| {
            let types: Vec<&ValueType> = match &declaration.shape {
                CompositeShape::Record(fields) => {
                    fields.iter().map(FieldDeclaration::value_type).collect()
                }
                CompositeShape::Tuple(positions) => positions.iter().collect(),
            };
            (&declaration.name, types)
        });
        let object_types = self.object_types.values().map(|declaration| {
            let types = declaration
                .attributes
                .iter()
                .map(FieldDeclaration::value_type)
                .collect();
            (&declaration.name, types)
        });
        for (name, types) in composites.chain(object_types) {
            if let Some(cause) = types.into_iter().find_map(|ty| self.type_refusal(ty)) {
                return Err(InvalidDeclaration {
                    declaration: name.clone(),
                    cause,
                });
            }
        }
        Ok(())
    }

    /// The recursion-rule edges leaving one declaration.
    fn edges(declaration: &CompositeDeclaration) -> Vec<Edge> {
        let members: Vec<(&ValueType, bool, bool)> = match &declaration.shape {
            CompositeShape::Record(fields) => fields
                .iter()
                .map(|field| {
                    (
                        field.value_type(),
                        true,
                        field.presence() == Presence::Optional,
                    )
                })
                .collect(),
            CompositeShape::Tuple(positions) => {
                positions.iter().map(|ty| (ty, false, false)).collect()
            }
        };
        let mut edges = Vec::new();
        for (value_type, named, escapes) in members {
            let mut pending = vec![(value_type, escapes)];
            while let Some((value_type, escapes)) = pending.pop() {
                match value_type {
                    ValueType::Composite(target) => edges.push(Edge {
                        target: *target,
                        named,
                        escapes,
                    }),
                    ValueType::Option(payload) => pending.push((payload, true)),
                    ValueType::Collection(collection) => pending.push((
                        collection.element(),
                        escapes || collection.bound().minimum() == 0,
                    )),
                    ValueType::Boolean
                    | ValueType::Integer
                    | ValueType::Int(_)
                    | ValueType::Rational(_)
                    | ValueType::Decimal(_)
                    | ValueType::Float(_)
                    | ValueType::Quantity(_)
                    | ValueType::Text(_)
                    | ValueType::Enum(_)
                    | ValueType::Reference(_)
                    | ValueType::Population(_) => {}
                }
            }
        }
        edges
    }

    /// Refuse the first cycle, in declaration-key order, of one recursion-rule
    /// subgraph.
    fn check_recursion(&self, subgraph: RecursionEdges) -> Result<(), InvalidDeclaration> {
        let graph: BTreeMap<NodeKey, Vec<NodeKey>> = self
            .composites
            .values()
            .map(|declaration| {
                let targets = Self::edges(declaration)
                    .into_iter()
                    .filter(|edge| match subgraph {
                        RecursionEdges::Unnamed => !edge.named,
                        RecursionEdges::NonEscaping => !edge.escapes,
                    })
                    .map(|edge| edge.target)
                    .collect();
                (declaration.key, targets)
            })
            .collect();
        let name = |key: &NodeKey| {
            self.composites
                .get(key)
                .map_or_else(String::new, |declaration| declaration.name.clone())
        };
        let mut finished = BTreeSet::new();
        for root in graph.keys() {
            if finished.contains(root) {
                continue;
            }
            let mut path: Vec<(NodeKey, usize)> = vec![(*root, 0)];
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                let Some(target) = graph.get(&node).and_then(|targets| targets.get(*next)) else {
                    finished.insert(node);
                    path.pop();
                    continue;
                };
                *next += 1;
                if let Some(start) = path.iter().position(|(on_path, _)| on_path == target) {
                    let mut cycle: Vec<String> =
                        path.iter().skip(start).map(|(key, _)| name(key)).collect();
                    cycle.push(name(target));
                    return Err(InvalidDeclaration {
                        declaration: name(target),
                        cause: DeclarationCause::Recursion {
                            edges: subgraph,
                            cycle,
                        },
                    });
                }
                if !finished.contains(target) {
                    path.push((*target, 0));
                }
            }
        }
        Ok(())
    }

    /// Every object type's own declared `supertypes`, keyed by its own key
    /// (H1, #204 round 1): every entry must itself name an admitted object
    /// type, and the whole graph must be acyclic -- refuses the first cycle
    /// found, in declaration-key order, exactly as [`Self::check_recursion`]
    /// does for the separate field-containment graph.
    fn check_supertypes(&self) -> Result<(), InvalidDeclaration> {
        let name = |key: &NodeKey| {
            self.object_types
                .get(key)
                .map_or_else(String::new, |declaration| declaration.name.clone())
        };
        for declaration in self.object_types.values() {
            for supertype in &declaration.supertypes {
                if !self.object_types.contains_key(supertype) {
                    return Err(InvalidDeclaration {
                        declaration: declaration.name.clone(),
                        cause: DeclarationCause::UnknownDeclaration(*supertype),
                    });
                }
            }
        }
        let mut finished: BTreeSet<NodeKey> = BTreeSet::new();
        for root in self.object_types.keys() {
            if finished.contains(root) {
                continue;
            }
            let mut path: Vec<(NodeKey, usize)> = vec![(*root, 0)];
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                let Some(target) = self
                    .object_types
                    .get(&node)
                    .and_then(|declaration| declaration.supertypes.get(*next))
                else {
                    finished.insert(node);
                    path.pop();
                    continue;
                };
                *next += 1;
                if let Some(start) = path.iter().position(|(on_path, _)| on_path == target) {
                    let mut cycle: Vec<String> =
                        path.iter().skip(start).map(|(key, _)| name(key)).collect();
                    cycle.push(name(target));
                    return Err(InvalidDeclaration {
                        declaration: name(target),
                        cause: DeclarationCause::GeneralizationCycle { cycle },
                    });
                }
                if !finished.contains(target) {
                    path.push((*target, 0));
                }
            }
        }
        Ok(())
    }

    /// Every object type's own proper ancestor set, transitively closed over
    /// the admitted (already known acyclic, see [`Self::check_supertypes`])
    /// supertypes graph. An explicit stack, never native recursion, mirroring
    /// [`Self::check_recursion`]'s own bounded walk: a node's ancestors are
    /// folded in only after every direct supertype's own ancestors are
    /// already known (a post-order finish), so each node is visited once and
    /// the walk is bounded by the object-type count, not call-stack depth.
    fn compute_ancestors(&self) -> BTreeMap<NodeKey, BTreeSet<NodeKey>> {
        let mut ancestors: BTreeMap<NodeKey, BTreeSet<NodeKey>> = BTreeMap::new();
        let mut finished: BTreeSet<NodeKey> = BTreeSet::new();
        for root in self.object_types.keys() {
            if finished.contains(root) {
                continue;
            }
            let mut path: Vec<(NodeKey, usize)> = vec![(*root, 0)];
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                let Some(target) = self
                    .object_types
                    .get(&node)
                    .and_then(|declaration| declaration.supertypes.get(*next))
                else {
                    let mut own_ancestors = BTreeSet::new();
                    if let Some(declaration) = self.object_types.get(&node) {
                        for supertype in &declaration.supertypes {
                            own_ancestors.insert(*supertype);
                            if let Some(further) = ancestors.get(supertype) {
                                own_ancestors.extend(further.iter().copied());
                            }
                        }
                    }
                    ancestors.insert(node, own_ancestors);
                    finished.insert(node);
                    path.pop();
                    continue;
                };
                *next += 1;
                if !finished.contains(target) {
                    path.push((*target, 0));
                }
            }
        }
        ancestors
    }

    /// Construct a record from its supplied fields. An omitted `?` field is
    /// `absent`.
    pub fn record(
        &self,
        declaration: NodeKey,
        fields: Vec<(&str, FieldValue)>,
    ) -> Result<Value, ConstructionRefusal> {
        let Some(CompositeShape::Record(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        let slots = fill_slots(declared, fields)?;
        Ok(composite(declaration, slots))
    }

    /// Construct a tuple of exactly its declared arity.
    pub fn tuple(
        &self,
        declaration: NodeKey,
        positions: Vec<Value>,
    ) -> Result<Value, ConstructionRefusal> {
        let declared = self.tuple_positions(declaration, positions.len())?;
        if let Some(position) = declared
            .iter()
            .zip(&positions)
            .position(|(value_type, value)| !value_type.admits(value))
        {
            return refuse(
                Component::Position(position),
                ConstructionCause::TypeMismatch,
            );
        }
        let slots = positions.into_iter().map(FieldValue::Present).collect();
        Ok(composite(declaration, slots))
    }

    /// Evaluate a record value expression. Every construction refusal is
    /// decided before any field expression runs. Field expressions then run in
    /// declaration order, whatever the source order; the first one that does
    /// not complete becomes the outcome and no later one runs. A completed
    /// record charges `composite.result-retain`.
    pub fn evaluate_record(
        &self,
        declaration: NodeKey,
        fields: Vec<(&str, FieldExpression<'_>)>,
        meter: &mut Meter,
    ) -> Result<Outcome<Value>, ConstructionRefusal> {
        let Some(CompositeShape::Record(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        let mut supplied = match_names(declared, fields)?;
        let mut plan = Vec::with_capacity(declared.len());
        for field in declared {
            let expression = supplied.remove(field.name());
            let component = || Component::Field(field.name().to_owned());
            match (&expression, field.presence()) {
                (None, Presence::Required) => {
                    return refuse(component(), ConstructionCause::MissingField)
                }
                (Some(FieldExpression::Null), Presence::Required) => {
                    return refuse(component(), ConstructionCause::NullForRequiredField)
                }
                (None | Some(FieldExpression::Null | FieldExpression::Evaluate(_)), _) => {}
            }
            plan.push((field, expression));
        }
        let mut slots = Vec::with_capacity(plan.len());
        for (field, expression) in plan {
            let slot = match expression {
                None => FieldValue::Absent,
                Some(FieldExpression::Null) => FieldValue::Null,
                Some(FieldExpression::Evaluate(expression)) => {
                    match admitted(field.value_type(), expression(meter)) {
                        Ok(value) => FieldValue::Present(value),
                        Err(stop) => return Ok(Outcome::from_stop(Err(stop))),
                    }
                }
            };
            slots.push(slot);
        }
        Ok(Outcome::from_stop(retain_composite(
            composite(declaration, slots.into_boxed_slice()),
            meter,
        )))
    }

    /// Evaluate a tuple call `T(e, ...)`: the arity is checked first, then the
    /// arguments run in position order under the first-stopped rule, then a
    /// completed tuple charges `composite.result-retain`.
    pub fn evaluate_tuple(
        &self,
        declaration: NodeKey,
        positions: Vec<Deferred<'_>>,
        meter: &mut Meter,
    ) -> Result<Outcome<Value>, ConstructionRefusal> {
        let declared = self.tuple_positions(declaration, positions.len())?;
        let mut slots = Vec::with_capacity(declared.len());
        for (value_type, expression) in declared.iter().zip(positions) {
            match admitted(value_type, expression(meter)) {
                Ok(value) => slots.push(FieldValue::Present(value)),
                Err(stop) => return Ok(Outcome::from_stop(Err(stop))),
            }
        }
        Ok(Outcome::from_stop(retain_composite(
            composite(declaration, slots.into_boxed_slice()),
            meter,
        )))
    }

    fn tuple_positions(
        &self,
        declaration: NodeKey,
        supplied: usize,
    ) -> Result<&[ValueType], ConstructionRefusal> {
        let Some(CompositeShape::Tuple(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        if declared.len() != supplied {
            return refuse(
                Component::Value,
                ConstructionCause::WrongArity {
                    declared: declared.len(),
                    supplied,
                },
            );
        }
        Ok(declared)
    }

    fn shape(&self, declaration: NodeKey) -> Option<&CompositeShape> {
        self.composites.get(&declaration).map(|d| &d.shape)
    }
}

fn duplicate_name(fields: &[FieldDeclaration]) -> Option<String> {
    let mut seen = BTreeSet::new();
    fields
        .iter()
        .find(|field| !seen.insert(field.name()))
        .map(|field| field.name().to_owned())
}

/// The completed value of a deferred expression, which a checked program
/// guarantees is a member of `value_type`. Its only callers are
/// `evaluate_record`/`evaluate_tuple` below.
fn admitted(value_type: &ValueType, outcome: Outcome<Value>) -> Result<Value, Stop> {
    let value = outcome.into_stop()?;
    if value_type.admits(&value) {
        Ok(value)
    } else {
        Err(Stop::Refused(Refusal::CheckedInvariant))
    }
}

/// The grammar's equality operators.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EqualityOperator {
    /// `=`.
    Equal,
    /// `!=`: the same schedule, retaining the negated Boolean.
    NotEqual,
}

impl EqualityOperator {
    fn comparison(self) -> ComparisonOperator {
        match self {
            Self::Equal => ComparisonOperator::Equal,
            Self::NotEqual => ComparisonOperator::NotEqual,
        }
    }
}

/// The static type of one equality operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityOperand {
    source: ValueType,
    target: Option<ValueType>,
}

impl EqualityOperand {
    /// An operand `e` of static type `source`.
    pub fn typed(source: ValueType) -> Self {
        Self {
            source,
            target: None,
        }
    }

    /// An operand `convert<target>(e)` for `e` of static type `source`.
    pub fn converted(source: ValueType, target: ValueType) -> Self {
        Self {
            source,
            target: Some(target),
        }
    }

    /// The comparison type.
    fn comparison_type(&self) -> &ValueType {
        self.target.as_ref().unwrap_or(&self.source)
    }
}

/// The schedule FR-149 selects from the common type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EqualitySchedule {
    /// A top-level text pair: the FR-141 text schedule.
    Text,
    /// A top-level enumeration pair: `enum.*`.
    Enum,
    /// A top-level quantity pair: the FR-142 comparison schedule.
    Quantity,
    /// Every other common type: the occurrence-pair plan.
    Plan,
}

/// A type-checked equality expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedEquality {
    operator: EqualityOperator,
    left: EqualityOperand,
    right: EqualityOperand,
    schedule: EqualitySchedule,
}

impl TypeEnvironment {
    /// Type-check `left op right`. Every refusal is made before any charge.
    pub fn check_equality(
        &self,
        operator: EqualityOperator,
        left: EqualityOperand,
        right: EqualityOperand,
    ) -> Result<CheckedEquality, IllTyped> {
        let ill_typed = |cause| Err(IllTyped { cause });
        for operand in [&left, &right] {
            if let Some(target) = &operand.target {
                if !admits_equality_conversion(&operand.source, target) {
                    return ill_typed(IllTypedCause::TypeMismatch);
                }
            }
        }
        let (left_type, right_type) = (left.comparison_type(), right.comparison_type());
        if self.contains_ieee(left_type) || self.contains_ieee(right_type) {
            return ill_typed(IllTypedCause::OperatorIneligible);
        }
        let schedule = match (left_type, right_type) {
            (ValueType::Text(l), ValueType::Text(r)) if l.profile() != r.profile() => {
                return ill_typed(IllTypedCause::DistinctTextProfiles)
            }
            (ValueType::Text(_), ValueType::Text(_)) => EqualitySchedule::Text,
            (ValueType::Enum(l), ValueType::Enum(r)) if l != r => {
                return ill_typed(IllTypedCause::DistinctEnumDeclarations)
            }
            (ValueType::Enum(_), ValueType::Enum(_)) => EqualitySchedule::Enum,
            (ValueType::Quantity(l), ValueType::Quantity(r)) if !l.has_dimension_of(r) => {
                return ill_typed(IllTypedCause::IncompatibleDimensions)
            }
            (ValueType::Quantity(l), ValueType::Quantity(r)) if l != r => {
                return ill_typed(IllTypedCause::DistinctUnits)
            }
            (ValueType::Quantity(_), ValueType::Quantity(_)) => EqualitySchedule::Quantity,
            // FR-153 names a population binding only as the direct operand of
            // `allInstances`/`lookup`, never as an equality operand: refuse it
            // here rather than falling into the `l == r` plan schedule below,
            // which would otherwise accept `p == p` and only fail at
            // evaluation (`plan_pairs`'s own checked-invariant catch-all).
            (ValueType::Population(_), _) | (_, ValueType::Population(_)) => {
                return ill_typed(IllTypedCause::OperatorIneligible)
            }
            (l, r) if l == r => EqualitySchedule::Plan,
            _ => return ill_typed(IllTypedCause::TypeMismatch),
        };
        Ok(CheckedEquality {
            operator,
            left,
            right,
            schedule,
        })
    }
}

impl CheckedEquality {
    /// The selected schedule.
    pub fn schedule(&self) -> EqualitySchedule {
        self.schedule
    }

    /// Evaluate over the two completed operand values, left conversion first.
    /// Neither source value is changed.
    pub fn evaluate(&self, left: &Value, right: &Value, meter: &mut Meter) -> Outcome<bool> {
        Outcome::from_stop(self.run(left, right, meter))
    }

    fn run(&self, left: &Value, right: &Value, meter: &mut Meter) -> Result<bool, Stop> {
        let left = operand_value(&self.left, left, meter)?;
        let right = operand_value(&self.right, right, meter)?;
        let operator = self.operator.comparison();
        let scheduled = match (self.schedule, &left, &right) {
            (EqualitySchedule::Text, Value::Text(l), Value::Text(r)) => {
                compare_text(operator, l, r, meter)
            }
            (EqualitySchedule::Enum, Value::Enum(l), Value::Enum(r)) => {
                compare_enum(operator, l, r, meter)
            }
            (EqualitySchedule::Quantity, Value::Quantity(l), Value::Quantity(r)) => {
                compare_quantity(operator, l, r, meter)
            }
            (EqualitySchedule::Plan, _, _) => {
                let equal = planned_equality(&left, &right, meter)?;
                return Ok(equal == (self.operator == EqualityOperator::Equal));
            }
            (
                EqualitySchedule::Text | EqualitySchedule::Enum | EqualitySchedule::Quantity,
                _,
                _,
            ) => return Err(invariant()),
        };
        scheduled.map_err(|_| invariant())?.into_stop()
    }
}

fn invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

/// Whether (`source`, `target`) is a row of the closed FR-149
/// equality-conversion table, decided from declared bounds alone.
pub fn admits_equality_conversion(source: &ValueType, target: &ValueType) -> bool {
    match (source, target) {
        (source, target) if source == target => true,
        (ValueType::Int(interval), target) => {
            integer_source_admits(interval.lower(), interval.upper(), target)
        }
        (ValueType::Rational(from), ValueType::Rational(to)) => {
            to.numerator().lower() <= from.numerator().lower()
                && from.numerator().upper() <= to.numerator().upper()
                && to.denominator().lower() <= from.denominator().lower()
                && from.denominator().upper() <= to.denominator().upper()
        }
        (
            ValueType::Rational(from),
            target @ (ValueType::Integer | ValueType::Int(_) | ValueType::Decimal(_)),
        ) if from.denominator().upper() == &Integer::one() => {
            integer_source_admits(from.numerator().lower(), from.numerator().upper(), target)
        }
        (ValueType::Decimal(from), ValueType::Rational(to)) => {
            let zero = Integer::zero();
            to.numerator().lower() <= from.lower().min(&zero)
                && from.upper().max(&zero) <= to.numerator().upper()
                && to.denominator().lower() <= &Integer::one()
                && compare_shifted(
                    &Integer::one(),
                    u64::from(from.max_scale()),
                    to.denominator().upper(),
                )
                .is_le()
        }
        (ValueType::Decimal(from), ValueType::Decimal(to)) => {
            let zero = Integer::zero();
            to.min_scale() <= from.min_scale()
                && from.max_scale() <= to.max_scale()
                && if to.min_scale() == from.min_scale() {
                    to.lower() <= from.lower() && from.upper() <= to.upper()
                } else {
                    to.lower() <= from.lower().min(&zero) && from.upper().max(&zero) <= to.upper()
                }
        }
        (ValueType::Decimal(from), target @ (ValueType::Integer | ValueType::Int(_)))
            if from.max_scale() == 0 =>
        {
            integer_source_admits(from.lower(), from.upper(), target)
        }
        (ValueType::Quantity(from), ValueType::Quantity(to)) => from.converts_to(to),
        _ => false,
    }
}

/// The rows for a source `Int[lo, hi]`.
fn integer_source_admits(lower: &Integer, upper: &Integer, target: &ValueType) -> bool {
    match target {
        ValueType::Integer => true,
        ValueType::Int(to) => to.lower() <= lower && upper <= to.upper(),
        ValueType::Rational(to) => {
            let one = Integer::one();
            to.numerator().lower() <= lower
                && upper <= to.numerator().upper()
                && to.denominator().lower() <= &one
                && &one <= to.denominator().upper()
        }
        ValueType::Decimal(to) => {
            let shift = u64::from(to.min_scale());
            compare_shifted(lower, shift, to.lower()).is_ge()
                && compare_shifted(upper, shift, to.upper()).is_le()
        }
        ValueType::Boolean
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => false,
    }
}

/// The comparison value of one operand after its admitted conversion.
pub(crate) fn operand_value(
    operand: &EqualityOperand,
    value: &Value,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    if !operand.source.admits(value) {
        return Err(invariant());
    }
    let Some(target) = &operand.target else {
        return Ok(value.clone());
    };
    let converted = match (&operand.source, target, value) {
        (source, target, value) if source == target => value.clone(),
        (_, ValueType::Integer | ValueType::Int(_), Value::Integer(_)) => value.clone(),
        (_, ValueType::Rational(_), Value::Integer(integer)) => {
            Value::Rational(Rational::from_integer(integer.clone()))
        }
        (_, ValueType::Rational(_), Value::Rational(_)) => value.clone(),
        (_, ValueType::Integer | ValueType::Int(_), Value::Rational(rational))
            if rational.is_integer() =>
        {
            Value::Integer(rational.numerator().clone())
        }
        (_, ValueType::Decimal(to), Value::Integer(integer)) => {
            integer_to_decimal(integer, to, meter)?
        }
        (_, ValueType::Decimal(to), Value::Rational(rational)) if rational.is_integer() => {
            integer_to_decimal(rational.numerator(), to, meter)?
        }
        (_, ValueType::Rational(_), Value::Decimal(decimal)) => {
            decimal_to_rational(decimal, meter)?
        }
        (_, ValueType::Decimal(to), Value::Decimal(decimal)) => {
            let result =
                evaluate_decimal(DecimalOperation::Round(decimal), to, meter).into_stop()?;
            Value::Decimal(result.value().clone())
        }
        (_, ValueType::Integer | ValueType::Int(_), Value::Decimal(decimal)) => {
            let rational = decimal.normalized().to_rational();
            if !rational.is_integer() {
                return Err(invariant());
            }
            Value::Integer(rational.numerator().clone())
        }
        (_, ValueType::Quantity(unit), Value::Quantity(quantity)) => {
            let conversion = convert_quantity(quantity, unit, &QuantityTarget::Exact, meter)
                .map_err(|_| invariant())?
                .into_stop()?;
            match conversion.value() {
                ConvertedValue::Exact(exact) => {
                    Value::Quantity(Quantity::new(exact.clone(), unit.clone()))
                }
                ConvertedValue::Decimal(_) | ConvertedValue::Integer { .. } => {
                    return Err(invariant())
                }
            }
        }
        _ => return Err(invariant()),
    };
    if target.admits(&converted) {
        Ok(converted)
    } else {
        Err(invariant())
    }
}

/// `Int[..]` or `Rational[..;d,1]` integer `n` into `Decimal[c1,c2;s1,s2]`,
/// retained as `(n × 10^s1, s1)`.
fn integer_to_decimal(
    value: &Integer,
    target: &DecimalType,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let scale = u64::from(target.min_scale());
    meter.charge(
        Charge::new(ChargePoint::DecimalOperands)
            .size(LimitKind::IntegerBits, value.magnitude_bits())
            .size(LimitKind::DecimalDigits, value.decimal_digits())
            .size(LimitKind::ValueOccurrences, 1),
    )?;
    let (bits, digits) = (sbits(value, scale), sdigits(value, scale));
    meter.charge(
        Charge::new(ChargePoint::DecimalScaleExpansion)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, bits.clone())
            .exact_size(LimitKind::DecimalDigits, digits.clone()),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .exact_size(LimitKind::IntegerBits, bits.clone())
            .exact_size(LimitKind::DecimalDigits, digits.clone()),
    )?;
    // Retention upscales `n` at scale 0 by `k = s1`, sized before
    // `n × 10^s1` is materialized.
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, bits)
            .exact_size(LimitKind::DecimalDigits, digits)
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    let coefficient = value.mul(&Integer::power_of_ten(scale));
    Ok(Value::Decimal(Decimal::new(
        coefficient,
        target.min_scale(),
    )))
}

/// A `Decimal` `(c, s)` into an exact rational.
fn decimal_to_rational(value: &Decimal, meter: &mut Meter) -> Result<Value, Stop> {
    let representation = value.representation();
    let coefficient = representation.coefficient();
    let scale = u64::from(representation.scale());
    meter.charge(
        Charge::new(ChargePoint::DecimalOperands)
            .size(LimitKind::IntegerBits, coefficient.magnitude_bits())
            .size(LimitKind::DecimalDigits, coefficient.decimal_digits())
            .size(LimitKind::ValueOccurrences, 1),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalScaleExpansion)
            .size(LimitKind::ScaleExpansion, scale)
            .exact_size(LimitKind::IntegerBits, power_of_ten_bits(scale)),
    )?;
    meter.charge(
        Charge::new(ChargePoint::DecimalArithmetic)
            .exact_size(
                LimitKind::IntegerBits,
                Integer::from(coefficient.magnitude_bits()).max(power_of_ten_bits(scale)),
            )
            .exact_size(
                LimitKind::DecimalDigits,
                Integer::from(coefficient.decimal_digits())
                    .max(Integer::from(scale).add(&Integer::one())),
            ),
    )?;
    let rational = representation.to_rational();
    let maxparts = rational.max_part_bits();
    meter.charge(
        Charge::new(ChargePoint::DecimalResultRetain)
            .size(LimitKind::IntegerBits, maxparts)
            .size(LimitKind::ValueOccurrences, 1)
            .results(1),
    )?;
    Ok(Value::Rational(rational))
}
