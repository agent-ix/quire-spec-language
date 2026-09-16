// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 records, tuples and finite recursive values over declaration-owned
//! identities.
//!
//! A [`TypeEnvironment`] admits a closed set of composite declarations and
//! refuses recursion that does not cross a named field. Construction checks
//! every field, position and state against its declaration and refuses at the
//! originating component with a typed [`ConstructionRefusal`]. Values are
//! immutable; sharing a contained value (`Arc`) never creates object identity.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::collection::{CardinalityViolation, CollectionKind, CollectionValue};
use super::decimal::{Decimal, DecimalType};
use super::enumeration::EnumValue;
use super::integer::Integer;
use super::node::NodeKey;
use super::outcome::Outcome;
use super::quantity::{Quantity, QuantityUnit};
use super::rational::Rational;
use super::reference::ObjectReference;
use super::text::{Text, TextType};

/// A declared complete-V1 value type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueType {
    /// `Boolean`.
    Boolean,
    /// The unbounded mathematical `Integer`.
    Integer,
    /// The unbounded exact `Rational`.
    Rational,
    /// An FR-140 `Decimal[lo, hi; smin, smax; mode]`.
    Decimal(DecimalType),
    /// An FR-142 quantity in exactly this unit.
    Quantity(QuantityUnit),
    /// An FR-141 `Text[min, max; profile]`.
    Text(TextType),
    /// A member of the enum declaration with this node key.
    Enum(NodeKey),
    /// `Option<T>`: `none` or `present(T)`.
    Option(Box<ValueType>),
    /// A record, tuple or recursive variant declaration.
    Composite(NodeKey),
    /// A collection kind over one element type.
    Collection(CollectionKind, Box<ValueType>),
    /// `Reference<T>` to an object whose state has declaration `T`.
    Reference(NodeKey),
}

impl ValueType {
    /// Whether `value` is a member of this declared type.
    pub fn admits(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Boolean, Value::Boolean(_))
            | (Self::Integer, Value::Integer(_))
            | (Self::Rational, Value::Rational(_)) => true,
            (Self::Decimal(declared), Value::Decimal(decimal)) => declared.contains(decimal),
            (Self::Quantity(unit), Value::Quantity(quantity)) => quantity.unit() == unit,
            (Self::Text(declared), Value::Text(text)) => text.text_type() == declared,
            (Self::Enum(declaration), Value::Enum(member)) => member.declaration() == *declaration,
            (Self::Option(payload), Value::Option(option)) => option.payload_type() == &**payload,
            (Self::Composite(declaration), Value::Composite(composite)) => {
                composite.declaration() == *declaration
            }
            (Self::Collection(kind, element), Value::Collection(collection)) => {
                collection.kind() == *kind && collection.element_type() == &**element
            }
            (Self::Reference(object_type), Value::Reference(reference)) => {
                reference.object_type() == *object_type
            }
            (
                Self::Boolean
                | Self::Integer
                | Self::Rational
                | Self::Decimal(_)
                | Self::Quantity(_)
                | Self::Text(_)
                | Self::Enum(_)
                | Self::Option(_)
                | Self::Composite(_)
                | Self::Collection(..)
                | Self::Reference(_),
                _,
            ) => false,
        }
    }

    /// Composite declarations this type names, split by whether reaching them
    /// is containment (`true`) or only an object reference (`false`).
    fn declarations(&self, found: &mut Vec<(NodeKey, bool)>) {
        let mut pending = vec![self];
        while let Some(value_type) = pending.pop() {
            match value_type {
                Self::Composite(key) => found.push((*key, true)),
                Self::Reference(key) => found.push((*key, false)),
                Self::Option(payload) | Self::Collection(_, payload) => pending.push(payload),
                Self::Boolean
                | Self::Integer
                | Self::Rational
                | Self::Decimal(_)
                | Self::Quantity(_)
                | Self::Text(_)
                | Self::Enum(_) => {}
            }
        }
    }
}

/// A completed complete-V1 value. It deliberately has no structural
/// `PartialEq`: equality is the FR-149 relation of
/// [`evaluate_equality`](super::evaluate_equality).
#[derive(Clone, Debug)]
pub enum Value {
    /// A Boolean.
    Boolean(bool),
    /// A mathematical integer.
    Integer(Integer),
    /// An exact rational.
    Rational(Rational),
    /// An exact decimal.
    Decimal(Decimal),
    /// A quantity in its unit.
    Quantity(Quantity),
    /// A text value of its declared type.
    Text(Text),
    /// An enum member.
    Enum(EnumValue),
    /// An option value.
    Option(Arc<OptionValue>),
    /// A record, tuple or variant value.
    Composite(Arc<CompositeValue>),
    /// A collection value.
    Collection(Arc<CollectionValue>),
    /// A terminal object reference.
    Reference(ObjectReference),
}

/// `none` or `present(v)` of one declared payload type.
#[derive(Clone, Debug)]
pub struct OptionValue {
    payload_type: ValueType,
    payload: Option<Value>,
}

impl OptionValue {
    /// `none` of `Option<payload_type>`.
    pub fn none(payload_type: ValueType) -> Value {
        Value::Option(Arc::new(Self {
            payload_type,
            payload: None,
        }))
    }

    /// `present(payload)` of `Option<payload_type>`.
    pub fn present(payload_type: ValueType, payload: Value) -> Result<Value, ConstructionRefusal> {
        if !payload_type.admits(&payload) {
            return Err(ConstructionRefusal {
                component: Component::Payload,
                cause: ConstructionCause::TypeMismatch,
            });
        }
        Ok(Value::Option(Arc::new(Self {
            payload_type,
            payload: Some(payload),
        })))
    }

    /// The declared payload type.
    pub fn payload_type(&self) -> &ValueType {
        &self.payload_type
    }

    /// The payload of `present`, or `None` for `none`.
    pub fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

/// Which states a declared record or constructor field admits besides a
/// present value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Presence {
    /// Only a present value.
    Required,
    /// A present value or explicit absence.
    Optional,
    /// A present value or explicit `null`.
    Nullable,
    /// A present value, explicit absence or explicit `null`.
    OptionalNullable,
}

impl Presence {
    fn admits_absent(self) -> bool {
        matches!(self, Self::Optional | Self::OptionalNullable)
    }

    fn admits_null(self) -> bool {
        matches!(self, Self::Nullable | Self::OptionalNullable)
    }
}

/// The state of one field slot: a value, explicit absence or explicit `null`.
#[derive(Clone, Debug)]
pub enum FieldValue {
    /// A present value.
    Present(Value),
    /// Explicit absence.
    Absent,
    /// Explicit `null`, distinct from absence.
    Null,
}

/// A declaration-owned named field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDeclaration {
    key: NodeKey,
    value_type: ValueType,
    presence: Presence,
}

impl FieldDeclaration {
    /// A field with declaration-owned identity `key`.
    pub fn new(key: NodeKey, value_type: ValueType, presence: Presence) -> Self {
        Self {
            key,
            value_type,
            presence,
        }
    }

    /// The field identity.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declared value type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// The admitted states.
    pub fn presence(&self) -> Presence {
        self.presence
    }
}

/// One named constructor of a recursive variant declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstructorDeclaration {
    key: NodeKey,
    fields: Vec<FieldDeclaration>,
}

impl ConstructorDeclaration {
    /// A constructor with identity `key` and its fields in declaration order.
    pub fn new(key: NodeKey, fields: Vec<FieldDeclaration>) -> Self {
        Self { key, fields }
    }

    /// The constructor identity.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// Its fields in declaration order.
    pub fn fields(&self) -> &[FieldDeclaration] {
        &self.fields
    }
}

/// The shape of a composite declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositeShape {
    /// A named record with fields in declaration order.
    Record(Vec<FieldDeclaration>),
    /// A positional tuple of exactly these position types.
    Tuple(Vec<ValueType>),
    /// A (possibly recursive) variant of named constructors.
    Variant(Vec<ConstructorDeclaration>),
}

/// A composite declaration with its I04 semantic-node key.
// SPEC-GAP(119-5): the pinned specification defines node preimages for enum,
// dimension and unit declarations but none for record, tuple or variant
// declarations, fields or constructors, so their keys are supplied opaquely
// and never recomputed here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositeDeclaration {
    key: NodeKey,
    shape: CompositeShape,
}

impl CompositeDeclaration {
    /// A declaration with node key `key`.
    pub fn new(key: NodeKey, shape: CompositeShape) -> Self {
        Self { key, shape }
    }

    /// The declaration node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declared shape.
    pub fn shape(&self) -> &CompositeShape {
        &self.shape
    }

    /// Every type position, with whether it is a named field.
    fn member_types(&self) -> Vec<(&ValueType, bool)> {
        match &self.shape {
            CompositeShape::Record(fields) => fields
                .iter()
                .map(|field| (&field.value_type, true))
                .collect(),
            CompositeShape::Tuple(positions) => positions.iter().map(|ty| (ty, false)).collect(),
            CompositeShape::Variant(constructors) => constructors
                .iter()
                .flat_map(|constructor| &constructor.fields)
                .map(|field| (&field.value_type, true))
                .collect(),
        }
    }
}

/// Why a declaration set is not admitted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("invalid composite declaration {declaration}: {cause:?}")]
pub struct InvalidDeclaration {
    /// The declaration where the refusal originates.
    pub declaration: NodeKey,
    /// The typed cause.
    pub cause: DeclarationCause,
}

/// The typed cause of an [`InvalidDeclaration`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum DeclarationCause {
    /// Two declarations share one node key.
    DuplicateDeclaration,
    /// Two fields or constructors of one declaration share an identity.
    DuplicateMember(NodeKey),
    /// A variant declares no constructor.
    EmptyVariant,
    /// A member type names a declaration outside the environment.
    UnknownDeclaration(NodeKey),
    /// A containment cycle through this declaration crosses no named field.
    UnnamedRecursion,
}

/// A closed, admitted set of composite declarations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TypeEnvironment {
    declarations: BTreeMap<NodeKey, CompositeDeclaration>,
}

/// Where a construction refusal originates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Component {
    /// The composite value as a whole (its declaration or arity).
    Value,
    /// A named record or constructor field.
    Field(NodeKey),
    /// A zero-based tuple position.
    Position(usize),
    /// A zero-based collection element occurrence.
    Element(usize),
    /// An option payload.
    Payload,
}

/// Why a construction refuses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionCause {
    /// The declaration is not in the environment, or has another shape.
    UnknownDeclaration,
    /// The variant has no constructor with this identity.
    UnknownConstructor,
    /// A declared field has no value or explicit state.
    MissingField,
    /// A supplied field is not declared.
    ExtraField,
    /// A field is supplied more than once.
    DuplicateField,
    /// The tuple has the wrong number of positions.
    WrongArity {
        /// Declared arity.
        declared: usize,
        /// Supplied positions.
        supplied: usize,
    },
    /// Explicit absence where the field does not admit it.
    AbsenceNotAdmitted,
    /// Explicit `null` where the field does not admit it.
    NullNotAdmitted,
    /// A value that is not a member of the declared type.
    TypeMismatch,
    /// A collection's declared cardinality bound is missing or violated.
    Cardinality(CardinalityViolation),
}

/// A typed construction refusal at its originating component.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("construction refused at {component:?}: {cause:?}")]
pub struct ConstructionRefusal {
    /// The originating component.
    pub component: Component,
    /// The typed cause.
    pub cause: ConstructionCause,
}

fn refuse<T>(component: Component, cause: ConstructionCause) -> Result<T, ConstructionRefusal> {
    Err(ConstructionRefusal { component, cause })
}

/// A record field expression: an evaluated value outcome or an explicit state.
#[derive(Clone, Debug)]
pub enum FieldExpression {
    /// The outcome of the field's value expression.
    Evaluated(Outcome<Value>),
    /// Explicit absence.
    Absent,
    /// Explicit `null`.
    Null,
}

/// A record, tuple or variant value.
#[derive(Clone, Debug)]
pub struct CompositeValue {
    declaration: NodeKey,
    constructor: NodeKey,
    slots: Box<[FieldValue]>,
}

impl CompositeValue {
    /// The declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// The constructor identity; a record or tuple uses its declaration key.
    pub fn constructor(&self) -> NodeKey {
        self.constructor
    }

    /// Field or position states in declaration order.
    pub fn slots(&self) -> &[FieldValue] {
        &self.slots
    }
}

impl TypeEnvironment {
    /// Admit `declarations` as one closed environment.
    pub fn new(
        declarations: impl IntoIterator<Item = CompositeDeclaration>,
    ) -> Result<Self, InvalidDeclaration> {
        let mut admitted = BTreeMap::new();
        for declaration in declarations {
            let key = declaration.key;
            check_members(&declaration)?;
            if admitted.insert(key, declaration).is_some() {
                return Err(InvalidDeclaration {
                    declaration: key,
                    cause: DeclarationCause::DuplicateDeclaration,
                });
            }
        }
        let environment = Self {
            declarations: admitted,
        };
        environment.check_references()?;
        environment.check_recursion()?;
        Ok(environment)
    }

    /// The admitted declaration with this key.
    pub fn declaration(&self, key: NodeKey) -> Option<&CompositeDeclaration> {
        self.declarations.get(&key)
    }

    fn check_references(&self) -> Result<(), InvalidDeclaration> {
        for declaration in self.declarations.values() {
            let mut found = Vec::new();
            for (value_type, _) in declaration.member_types() {
                value_type.declarations(&mut found);
            }
            if let Some((unknown, _)) = found
                .into_iter()
                .find(|(key, _)| !self.declarations.contains_key(key))
            {
                return Err(InvalidDeclaration {
                    declaration: declaration.key,
                    cause: DeclarationCause::UnknownDeclaration(unknown),
                });
            }
        }
        Ok(())
    }

    /// Refuse a containment cycle that crosses no named field: the subgraph of
    /// unnamed containment edges must be acyclic.
    // SPEC-GAP(119-6): FR-143 allows recursion "when every recursion edge
    // crosses a named constructor field" without defining a named constructor
    // field. A record field and a variant constructor field are named; a tuple
    // position, and an option payload or collection element reached only
    // through positions, are not. Object references are not containment.
    fn check_recursion(&self) -> Result<(), InvalidDeclaration> {
        let unnamed: BTreeMap<NodeKey, Vec<NodeKey>> = self
            .declarations
            .values()
            .map(|declaration| {
                let mut found = Vec::new();
                for (value_type, named) in declaration.member_types() {
                    if !named {
                        value_type.declarations(&mut found);
                    }
                }
                let targets = found
                    .into_iter()
                    .filter_map(|(key, contained)| contained.then_some(key))
                    .collect();
                (declaration.key, targets)
            })
            .collect();
        let mut finished = BTreeSet::new();
        for root in unnamed.keys() {
            if finished.contains(root) {
                continue;
            }
            let mut on_path = BTreeSet::from([*root]);
            let mut stack = vec![(*root, 0_usize)];
            while let Some((node, next)) = stack.last_mut() {
                let node = *node;
                let targets = unnamed.get(&node).map_or(&[][..], Vec::as_slice);
                let Some(target) = targets.get(*next).copied() else {
                    on_path.remove(&node);
                    finished.insert(node);
                    stack.pop();
                    continue;
                };
                *next += 1;
                if on_path.contains(&target) {
                    return Err(InvalidDeclaration {
                        declaration: target,
                        cause: DeclarationCause::UnnamedRecursion,
                    });
                }
                if !finished.contains(&target) {
                    on_path.insert(target);
                    stack.push((target, 0));
                }
            }
        }
        Ok(())
    }

    /// Construct a record from every declared field's value or explicit state.
    pub fn record(
        &self,
        declaration: NodeKey,
        fields: Vec<(NodeKey, FieldValue)>,
    ) -> Result<Value, ConstructionRefusal> {
        let Some(CompositeShape::Record(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        let slots = fill_fields(declared, fields)?;
        Ok(composite(declaration, declaration, slots))
    }

    /// Construct a tuple of exactly its declared arity.
    pub fn tuple(
        &self,
        declaration: NodeKey,
        positions: Vec<Value>,
    ) -> Result<Value, ConstructionRefusal> {
        let Some(CompositeShape::Tuple(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        if declared.len() != positions.len() {
            return refuse(
                Component::Value,
                ConstructionCause::WrongArity {
                    declared: declared.len(),
                    supplied: positions.len(),
                },
            );
        }
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
        Ok(composite(declaration, declaration, slots))
    }

    /// Construct one named constructor of a variant declaration.
    pub fn variant(
        &self,
        declaration: NodeKey,
        constructor: NodeKey,
        fields: Vec<(NodeKey, FieldValue)>,
    ) -> Result<Value, ConstructionRefusal> {
        let Some(CompositeShape::Variant(constructors)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        let Some(declared) = constructors.iter().find(|c| c.key == constructor) else {
            return refuse(Component::Value, ConstructionCause::UnknownConstructor);
        };
        let slots = fill_fields(&declared.fields, fields)?;
        Ok(composite(declaration, constructor, slots))
    }

    /// Construct a record whose field values are nested expression outcomes.
    /// The field set is checked first; then a non-completed nested outcome
    /// propagates unchanged and no record exists.
    // SPEC-GAP(119-8): FR-149 propagates "that same disposition" from a nested
    // operand construction without saying which one when several fields stop.
    // The first stopped field in declaration order propagates.
    pub fn evaluate_record(
        &self,
        declaration: NodeKey,
        fields: Vec<(NodeKey, FieldExpression)>,
    ) -> Result<Outcome<Value>, ConstructionRefusal> {
        let Some(CompositeShape::Record(declared)) = self.shape(declaration) else {
            return refuse(Component::Value, ConstructionCause::UnknownDeclaration);
        };
        let mut by_key = match_fields(declared, fields)?;
        let mut values = Vec::with_capacity(declared.len());
        for field in declared {
            let value = match by_key.remove(&field.key) {
                Some(FieldExpression::Evaluated(Outcome::Completed(value))) => {
                    FieldValue::Present(value)
                }
                Some(FieldExpression::Evaluated(Outcome::Undefined(reason))) => {
                    return Ok(Outcome::Undefined(reason))
                }
                Some(FieldExpression::Evaluated(Outcome::Refused(reason))) => {
                    return Ok(Outcome::Refused(reason))
                }
                Some(FieldExpression::Evaluated(Outcome::Incomplete(record))) => {
                    return Ok(Outcome::Incomplete(record))
                }
                Some(FieldExpression::Absent) => FieldValue::Absent,
                Some(FieldExpression::Null) => FieldValue::Null,
                None => {
                    return refuse(Component::Field(field.key), ConstructionCause::MissingField)
                }
            };
            values.push((field.key, value));
        }
        self.record(declaration, values).map(Outcome::Completed)
    }

    fn shape(&self, declaration: NodeKey) -> Option<&CompositeShape> {
        self.declarations.get(&declaration).map(|d| &d.shape)
    }
}

fn duplicate(keys: impl IntoIterator<Item = NodeKey>) -> Option<NodeKey> {
    let mut seen = BTreeSet::new();
    keys.into_iter().find(|key| !seen.insert(*key))
}

fn check_members(declaration: &CompositeDeclaration) -> Result<(), InvalidDeclaration> {
    let invalid = |cause| InvalidDeclaration {
        declaration: declaration.key,
        cause,
    };
    match &declaration.shape {
        CompositeShape::Record(fields) => {
            if let Some(key) = duplicate(fields.iter().map(FieldDeclaration::key)) {
                return Err(invalid(DeclarationCause::DuplicateMember(key)));
            }
        }
        CompositeShape::Tuple(_) => {}
        CompositeShape::Variant(constructors) => {
            if constructors.is_empty() {
                return Err(invalid(DeclarationCause::EmptyVariant));
            }
            if let Some(key) = duplicate(constructors.iter().map(ConstructorDeclaration::key)) {
                return Err(invalid(DeclarationCause::DuplicateMember(key)));
            }
            for constructor in constructors {
                if let Some(key) = duplicate(constructor.fields.iter().map(FieldDeclaration::key)) {
                    return Err(invalid(DeclarationCause::DuplicateMember(key)));
                }
            }
        }
    }
    Ok(())
}

/// Index supplied fields by key, refusing an undeclared or repeated field,
/// then a declared field with no supplied entry.
fn match_fields<T>(
    declared: &[FieldDeclaration],
    fields: Vec<(NodeKey, T)>,
) -> Result<BTreeMap<NodeKey, T>, ConstructionRefusal> {
    let mut by_key = BTreeMap::new();
    for (key, value) in fields {
        if !declared.iter().any(|field| field.key == key) {
            return refuse(Component::Field(key), ConstructionCause::ExtraField);
        }
        if by_key.insert(key, value).is_some() {
            return refuse(Component::Field(key), ConstructionCause::DuplicateField);
        }
    }
    if let Some(missing) = declared
        .iter()
        .find(|field| !by_key.contains_key(&field.key))
    {
        return refuse(
            Component::Field(missing.key),
            ConstructionCause::MissingField,
        );
    }
    Ok(by_key)
}

fn fill_fields(
    declared: &[FieldDeclaration],
    fields: Vec<(NodeKey, FieldValue)>,
) -> Result<Box<[FieldValue]>, ConstructionRefusal> {
    let mut by_key = match_fields(declared, fields)?;
    let mut slots = Vec::with_capacity(declared.len());
    for field in declared {
        let component = Component::Field(field.key);
        let Some(value) = by_key.remove(&field.key) else {
            return refuse(component, ConstructionCause::MissingField);
        };
        match &value {
            FieldValue::Absent if !field.presence.admits_absent() => {
                return refuse(component, ConstructionCause::AbsenceNotAdmitted)
            }
            FieldValue::Null if !field.presence.admits_null() => {
                return refuse(component, ConstructionCause::NullNotAdmitted)
            }
            FieldValue::Present(value) if !field.value_type.admits(value) => {
                return refuse(component, ConstructionCause::TypeMismatch)
            }
            FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null => {}
        }
        slots.push(value);
    }
    Ok(slots.into_boxed_slice())
}

fn composite(declaration: NodeKey, constructor: NodeKey, slots: Box<[FieldValue]>) -> Value {
    Value::Composite(Arc::new(CompositeValue {
        declaration,
        constructor,
        slots,
    }))
}
