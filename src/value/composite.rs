// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 records, tuples and finite recursive values over producer-assigned
//! declaration keys.
//!
//! A [`TypeEnvironment`] admits one checked package's closed set of record,
//! tuple and model object-type declarations. It refuses a containment graph
//! whose unnamed-edge or non-escaping-edge subgraph has a cycle, naming that
//! cycle, a `Reference<T>` whose target is not a model object type, and a set,
//! bag or ordered set whose element type bears IEEE values at any depth.
//! Construction checks every field, position and state against its
//! declaration and refuses at the originating component. Values are immutable
//! and cache their `occ`; sharing a contained value (`Arc`) never creates
//! object identity.
//!
//! `quire_exact::value` (QSL-131) is a redesigned cut of this module, not a
//! duplicate of it: per its own module doc, `ValueType::Enum` carries an
//! inline `EnumShape` where this module's carries a `NodeKey` lookup,
//! `ValueType::Reference` carries an `EffectiveId` where this module's
//! carries a `NodeKey`, and `TypeEnvironment`/`ObjectTypeDeclaration` (the
//! declaration registry) stay out of the kernel by design (ADR-013 O-15).
//! `Presence` has no such divergence -- it names only `Required`/`Optional`
//! and touches neither `Value` nor `ValueType` -- so it is `quire_exact`'s
//! type here. Every other item below (`ValueType`, `Value`, `OptionValue`,
//! `FieldValue`, `FieldDeclaration`, `CompositeValue` and the construction
//! functions over them) stays local because it is parameterized over this
//! module's own `ValueType`/`Value`, not `quire_exact`'s: remaining work,
//! Linear QSL-131.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::collection::{CollectionType, CollectionValue};
use super::decimal::DecimalType;
use super::enumeration::EnumValue;
use super::node::NodeKey;
use super::outcome::{Outcome, Refusal, Stop};
use super::quantity::{Quantity, QuantityUnit};
use super::reference::ObjectReference;
use super::text::Text;
use quire_exact::Decimal;
use quire_exact::IeeeWidth;
use quire_exact::Rational;
use quire_exact::TextType;
use quire_exact::{
    Charge, ChargePoint, CollectionKind, IeeeValue, Integer, IntegerInterval, LimitKind, Meter,
    PopulationId, Presence, RationalDomain,
};
use quire_exact::{IllTyped, IllTypedCause};

/// A declared complete-V1 value type. Two types are the same type exactly when
/// they are equal, collection bounds included.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueType {
    /// `Boolean`.
    Boolean,
    /// The unbounded mathematical `Integer`.
    Integer,
    /// A bounded `Int[lo, hi]`.
    Int(IntegerInterval),
    /// A `Rational[n1, n2; d1, d2]` domain.
    Rational(RationalDomain),
    /// An FR-140 `Decimal[lo, hi; smin, smax; mode]`.
    Decimal(DecimalType),
    /// An FR-148 `Float32` or `Float64`.
    Float(IeeeWidth),
    /// An FR-142 quantity in exactly this unit.
    Quantity(QuantityUnit),
    /// An FR-141 `Text[min, max; profile]`.
    Text(TextType),
    /// A member of the enum declaration with this node key.
    Enum(NodeKey),
    /// `Option<T>`: `none` or a present `T`.
    Option(Box<ValueType>),
    /// The record or tuple declaration with this node key.
    Composite(NodeKey),
    /// A bounded collection type `K<T>[min, max]`.
    Collection(Box<CollectionType>),
    /// `Reference<T>` to an object of the model object type with this key.
    Reference(NodeKey),
    /// FR-153's `Population<T>[N]` parameter type: `N` is the declared
    /// maximum an admitted
    /// [`PopulationBinding`](crate::model::population::PopulationBinding)
    /// must carry (never `T` itself, which
    /// `allInstances<T>(p)`/`lookup<T>(p, r)` name separately at each call,
    /// per FR-153's own table).
    Population(u64),
}

impl ValueType {
    /// `K<element>[bound]`.
    pub fn collection(collection_type: CollectionType) -> Self {
        Self::Collection(Box::new(collection_type))
    }

    /// `Option<payload>`.
    pub fn option(payload: Self) -> Self {
        Self::Option(Box::new(payload))
    }

    /// Whether `value` is a member of this declared type. Composite, option
    /// and collection values carry their declared type, which must be this
    /// type; their contents were admitted at construction.
    pub fn admits(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Boolean, Value::Boolean(_)) | (Self::Integer, Value::Integer(_)) => true,
            (Self::Int(interval), Value::Integer(integer)) => interval.contains(integer),
            (Self::Rational(domain), Value::Rational(rational)) => domain.contains(rational),
            (Self::Decimal(declared), Value::Decimal(decimal)) => declared.contains(decimal),
            (Self::Float(width), Value::Float(float)) => float.width() == *width,
            (Self::Quantity(unit), Value::Quantity(quantity)) => quantity.unit() == unit,
            (Self::Text(declared), Value::Text(text)) => text.text_type() == declared,
            (Self::Enum(declaration), Value::Enum(member)) => member.declaration() == *declaration,
            (Self::Option(payload), Value::Option(option)) => option.payload_type() == &**payload,
            (Self::Composite(declaration), Value::Composite(composite)) => {
                composite.declaration() == *declaration
            }
            (Self::Collection(declared), Value::Collection(collection)) => {
                collection.collection_type() == &**declared
            }
            (Self::Reference(object_type), Value::Reference(reference)) => {
                reference.object_type() == *object_type
            }
            // FR-089: `Value::Population` carries only the opaque
            // `PopulationId` a `model` admission minted, never the
            // `PopulationBinding` itself, so this structural check cannot
            // compare `maximum` against a resolved binding's own declared
            // maximum -- that comparison needs the recorded
            // `PopulationId` -> `PopulationBinding` correspondence, which
            // only `ObjectEnvironment` has access to. Presence is admitted
            // here (mirroring FR-153's own restriction of `Population<T>[N]`
            // to a bare parameter type: no nested context ever reaches this
            // arm); the declared-maximum comparison itself happens in two
            // places instead: argument admission's `validate`
            // (`value/expression/mod.rs`) checks it against every
            // `Population<T>[N]` parameter at call time (FR-089-AC-4/AC-5), and
            // `Machine::resolve_population` (`value/expression/evaluate.rs`)
            // checks it again at the `allInstances`/`lookup` sites that
            // consume the identity (FR-089-AC-3/AC-4/AC-5), refusing a
            // mismatch in either place instead.
            (Self::Population(_), Value::Population(_)) => true,
            (
                Self::Boolean
                | Self::Integer
                | Self::Int(_)
                | Self::Rational(_)
                | Self::Decimal(_)
                | Self::Float(_)
                | Self::Quantity(_)
                | Self::Text(_)
                | Self::Enum(_)
                | Self::Option(_)
                | Self::Composite(_)
                | Self::Collection(_)
                | Self::Reference(_)
                | Self::Population(_),
                _,
            ) => false,
        }
    }
}

/// A completed complete-V1 value. It deliberately has no structural
/// `PartialEq`: equality is the FR-149 relation of
/// [`CheckedEquality::evaluate`](super::CheckedEquality::evaluate).
#[derive(Clone, Debug)]
pub enum Value {
    /// A Boolean.
    Boolean(bool),
    /// A mathematical integer of `Integer` or `Int[..]`.
    Integer(Integer),
    /// An exact reduced rational.
    Rational(Rational),
    /// An exact decimal.
    Decimal(Decimal),
    /// An IEEE bit pattern.
    Float(IeeeValue),
    /// A quantity in its unit.
    Quantity(Quantity),
    /// A text value of its declared type.
    Text(Text),
    /// An enum member.
    Enum(EnumValue),
    /// An option value.
    Option(Arc<OptionValue>),
    /// A record or tuple value.
    Composite(Arc<CompositeValue>),
    /// A collection value.
    Collection(Arc<CollectionValue>),
    /// A terminal object reference.
    Reference(ObjectReference),
    /// FR-089: the opaque `PopulationId` a `model` admission
    /// (`admit_binding`/`admit_invocation`) minted for its admitted
    /// `PopulationBinding` -- never the binding itself, which stays a
    /// `model` type. The evaluator resolves this identity to its binding by
    /// lookup in `model`'s recorded correspondence
    /// (`ObjectEnvironment::resolve_population`), never by decoding the
    /// identity's own bytes.
    Population(PopulationId),
}

impl Value {
    /// `occ(v)` of `quire.value.accounting/v1`: one for the value itself plus
    /// every nested occurrence, with `absent` and `null` slots counting zero
    /// and a bag occurrence counted once per multiplicity.
    pub fn occ(&self) -> Integer {
        match self {
            Self::Option(option) => option.occ.clone(),
            Self::Composite(composite) => composite.occ.clone(),
            Self::Collection(collection) => collection.occ().clone(),
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::Rational(_)
            | Self::Decimal(_)
            | Self::Float(_)
            | Self::Quantity(_)
            | Self::Text(_)
            | Self::Enum(_)
            | Self::Reference(_)
            | Self::Population(_) => Integer::one(),
        }
    }
}

/// `none` or a present value of one declared payload type.
#[derive(Clone, Debug)]
pub struct OptionValue {
    payload_type: ValueType,
    payload: Option<Value>,
    occ: Integer,
}

impl OptionValue {
    /// `none` of `Option<payload_type>`.
    pub fn none(payload_type: ValueType) -> Value {
        Value::Option(Arc::new(Self {
            payload_type,
            payload: None,
            occ: Integer::one(),
        }))
    }

    /// A present `payload` of `Option<payload_type>`.
    pub fn present(payload_type: ValueType, payload: Value) -> Result<Value, ConstructionRefusal> {
        if !payload_type.admits(&payload) {
            return Err(ConstructionRefusal {
                component: Component::Payload,
                cause: ConstructionCause::TypeMismatch,
            });
        }
        let occ = Integer::one().add(&payload.occ());
        Ok(Value::Option(Arc::new(Self {
            payload_type,
            payload: Some(payload),
            occ,
        })))
    }

    /// Materialize an already-admitted `payload` as `Option<payload_type>`,
    /// checking no structural `admits()` match. FR-153's `lookup<T>(p, r)
    /// absent empty` (`crate::value::model_query::evaluate_lookup`) calls
    /// this for a present result `found`, whose `object_type` is `r`'s own
    /// runtime most-specific type `F` (FR-143's own identity triple), not the
    /// queried `T`. Soundness does not rest on
    /// `crate::model::population::lookup`'s `type_conforms` call proving `F`
    /// conforms to `T` directly -- it checks `r`'s *declared* static type `S`
    /// against `T`, never `F` against `T`. It rests on chaining two
    /// invariants: admission guarantees `F` conforms to `S`, never `F == S`
    /// -- `S` is `r`'s declared static type, and `r` may itself be the
    /// result of an earlier upcast lookup, so `S` can already be a proper
    /// supertype of `F`. Example: `lookup<A>(p, lookup<A>(p, rb) absent
    /// refused) absent empty` has `S = A` (the outer call's declared static
    /// type) and `F = B` (`rb`'s own most-specific type, preserved through
    /// the inner call by this same soundness chain). `lookup`'s
    /// `type_conforms(S, T)` call then gives `S` conforms to `T`; so `F`
    /// conforms to `T` transitively (`F` conforms to `S` conforms to `T`).
    /// [`Self::present`]'s own `admits()` call cannot verify that chain
    /// itself -- it is exact object-type equality with no model-conformance
    /// knowledge, so it cannot tell a genuine upcast (`F` a proper subtype of
    /// `T`) from a real type mismatch -- which is why this bypasses it.
    /// Mirrors [`crate::value::collection::from_admitted`]'s same role for
    /// `allInstances`.
    pub(crate) fn from_admitted(payload_type: ValueType, payload: Option<Value>) -> Value {
        let occ = match &payload {
            Some(payload) => Integer::one().add(&payload.occ()),
            None => Integer::one(),
        };
        Value::Option(Arc::new(Self {
            payload_type,
            payload,
            occ,
        }))
    }

    /// The declared payload type.
    pub fn payload_type(&self) -> &ValueType {
        &self.payload_type
    }

    /// The present payload, or `None` for `none`.
    pub fn payload(&self) -> Option<&Value> {
        self.payload.as_ref()
    }
}

/// The state of one field slot.
#[derive(Clone, Debug)]
pub enum FieldValue {
    /// A present value.
    Present(Value),
    /// `absent`: the `?` field was omitted.
    Absent,
    /// Explicit `null`, distinct from `absent`.
    Null,
}

/// A declaration-owned named field; its identity is (declaration key, name).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDeclaration {
    name: String,
    value_type: ValueType,
    presence: Presence,
}

impl FieldDeclaration {
    /// The field `name: value_type` or `name: value_type?`.
    pub fn new(name: impl Into<String>, value_type: ValueType, presence: Presence) -> Self {
        Self {
            name: name.into(),
            value_type,
            presence,
        }
    }

    /// The field identifier.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared value type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// Whether the field was declared with `?`.
    pub fn presence(&self) -> Presence {
        self.presence
    }
}

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

    /// Every admitted object type in key order (QSL-180 K5:
    /// `check::type_form::resolve_named_type`'s own name search, alongside
    /// [`Self::composites`]/enum members, so a `TypeForm` naming a declared
    /// object type resolves to `ValueType::Reference` the same way one
    /// naming a record, tuple or enum resolves to `Composite`/`Enum`).
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
                        &field.value_type,
                        true,
                        field.presence == Presence::Optional,
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
            let expression = supplied.remove(field.name.as_str());
            let component = || Component::Field(field.name.clone());
            match (&expression, field.presence) {
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
                    match admitted(&field.value_type, expression(meter)) {
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

/// A deferred expression: it runs only when construction reaches it.
pub type Deferred<'a> = Box<dyn FnOnce(&mut Meter) -> Outcome<Value> + 'a>;

/// A record field in a record value expression. Omitting a `?` field
/// constructs `absent`.
pub enum FieldExpression<'a> {
    /// `f: e`.
    Evaluate(Deferred<'a>),
    /// `f: null`.
    Null,
}

impl std::fmt::Debug for FieldExpression<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Evaluate(_) => formatter.write_str("Evaluate(..)"),
            Self::Null => formatter.write_str("Null"),
        }
    }
}

/// The completed value of a deferred expression, which a checked program
/// guarantees is a member of `value_type`.
fn admitted(value_type: &ValueType, outcome: Outcome<Value>) -> Result<Value, Stop> {
    let value = outcome.into_stop()?;
    if value_type.admits(&value) {
        Ok(value)
    } else {
        Err(Stop::Refused(Refusal::CheckedInvariant))
    }
}

/// Charge `composite.result-retain` with `occ(result)`, then expose it.
pub(crate) fn retain_composite(value: Value, meter: &mut Meter) -> Result<Value, Stop> {
    let occ = value.occ();
    meter.charge(
        Charge::new(ChargePoint::CompositeResultRetain)
            .exact_size(LimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ),
    )?;
    Ok(value)
}

/// Where a construction refusal originates.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Component {
    /// The composite value as a whole (its declaration or arity).
    Value,
    /// A named record field or object attribute.
    Field(String),
    /// A zero-based tuple position.
    Position(usize),
    /// A zero-based collection occurrence in source order.
    Element(usize),
    /// An option payload.
    Payload,
}

/// Why a construction is `refused { code: ill_typed }`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConstructionCause {
    /// The declaration is not a record or tuple of the environment, or has
    /// the other shape.
    UnknownDeclaration,
    /// A required field is omitted.
    MissingField,
    /// A supplied field is not declared.
    UndeclaredField,
    /// A field is supplied twice.
    DuplicateField,
    /// `null` is supplied for a required field.
    NullForRequiredField,
    /// A tuple call has another argument count than its declared arity.
    WrongArity {
        /// Declared arity.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// A value that is not a member of the declared type.
    TypeMismatch,
}

/// A typed construction refusal at its originating component.
#[derive(Clone, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("construction refused at {component:?}: {cause:?}")]
pub struct ConstructionRefusal {
    /// The originating component.
    pub component: Component,
    /// The typed cause.
    pub cause: ConstructionCause,
}

impl ConstructionRefusal {
    /// The `refused { code }` spelling.
    pub const CODE: &'static str = IllTyped::CODE;
}

fn refuse<T>(component: Component, cause: ConstructionCause) -> Result<T, ConstructionRefusal> {
    Err(ConstructionRefusal { component, cause })
}

fn duplicate_name(fields: &[FieldDeclaration]) -> Option<String> {
    let mut seen = BTreeSet::new();
    fields
        .iter()
        .find(|field| !seen.insert(field.name.as_str()))
        .map(|field| field.name.clone())
}

/// Index supplied entries by declared name, refusing an undeclared or
/// repeated name.
fn match_names<'n, T>(
    declared: &[FieldDeclaration],
    supplied: Vec<(&'n str, T)>,
) -> Result<BTreeMap<&'n str, T>, ConstructionRefusal> {
    let mut by_name = BTreeMap::new();
    for (name, entry) in supplied {
        let component = || Component::Field(name.to_owned());
        if !declared.iter().any(|field| field.name == name) {
            return refuse(component(), ConstructionCause::UndeclaredField);
        }
        if by_name.insert(name, entry).is_some() {
            return refuse(component(), ConstructionCause::DuplicateField);
        }
    }
    Ok(by_name)
}

/// Declaration-ordered slots of a record or object from supplied fields.
pub(crate) fn fill_slots(
    declared: &[FieldDeclaration],
    supplied: Vec<(&str, FieldValue)>,
) -> Result<Box<[FieldValue]>, ConstructionRefusal> {
    let mut by_name = match_names(declared, supplied)?;
    let mut slots = Vec::with_capacity(declared.len());
    for field in declared {
        let component = || Component::Field(field.name.clone());
        let slot = by_name
            .remove(field.name.as_str())
            .unwrap_or(FieldValue::Absent);
        match (&slot, field.presence) {
            (FieldValue::Absent, Presence::Required) => {
                return refuse(component(), ConstructionCause::MissingField)
            }
            (FieldValue::Null, Presence::Required) => {
                return refuse(component(), ConstructionCause::NullForRequiredField)
            }
            (FieldValue::Present(value), _) if !field.value_type.admits(value) => {
                return refuse(component(), ConstructionCause::TypeMismatch)
            }
            (FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null, _) => {}
        }
        slots.push(slot);
    }
    Ok(slots.into_boxed_slice())
}

/// `1 + occ` of every present slot.
pub(crate) fn slots_occ(slots: &[FieldValue]) -> Integer {
    slots.iter().fold(Integer::one(), |occ, slot| match slot {
        FieldValue::Present(value) => occ.add(&value.occ()),
        FieldValue::Absent | FieldValue::Null => occ,
    })
}

fn composite(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    let occ = slots_occ(&slots);
    Value::Composite(Arc::new(CompositeValue {
        declaration,
        slots,
        occ,
    }))
}

/// A record or tuple value.
#[derive(Clone, Debug)]
pub struct CompositeValue {
    declaration: NodeKey,
    slots: Box<[FieldValue]>,
    occ: Integer,
}

impl CompositeValue {
    /// The declaration node key.
    pub fn declaration(&self) -> NodeKey {
        self.declaration
    }

    /// Field or position states in declaration order.
    pub fn slots(&self) -> &[FieldValue] {
        &self.slots
    }
}
