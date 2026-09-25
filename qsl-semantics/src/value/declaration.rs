// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-143 declared record, tuple and model object-type registry, and
//! the FR-149 checked equality layer over it: QSL's `semantic_value`
//! registry module (ADR-011 §6.2).
//!
//! It owns the registry (`TypeEnvironment`, `ObjectTypeDeclaration`,
//! `CompositeDeclaration`, `CompositeShape`, `InvalidDeclaration`,
//! `RecursionEdges`, `DeclarationCause`) and the check-level equality layer
//! (`EqualityOperator`, `EqualityOperand`, `EqualitySchedule`,
//! `CheckedEquality`, `TypeEnvironment::check_equality`,
//! `admits_equality_conversion`, `operand_value`). The environment also
//! carries the package's quantity [`UnitTable`], since a `ValueType::Quantity`
//! names its unit only by id. None of these are kernel
//! types (ADR-011 §6.1: "`TypeEnvironment` and `ObjectTypeDeclaration` ...
//! are not kernel types and stay in layer 3"), so this is a layer-3
//! `semantic_value` module, over `quire_exact`'s own `Value`/`ValueType`.
//!
//! The equality layer lives here because it is parameterized over a
//! `TypeEnvironment` and a checked `ValueType`. The occurrence-pair walk it
//! schedules is [`quire_exact::planned_equality`]/[`quire_exact::plan_equality`],
//! called directly by `CheckedEquality::run`: the kernel's own `plan_pairs` is `pub(crate)`
//! there, and this layer never needs the lower-level pair count that
//! `value::expression::evaluate`'s `Machine` gets straight from
//! `quire_exact::member_equal` (QSL-131 V5b).
//!
//! [`FieldDeclaration`], [`Component`], [`ConstructionCause`] and
//! [`ConstructionRefusal`] (QSL-131 V5b, moved here from the now-deleted
//! `value::composite`) are QSL's own, non-kernel types (ADR-011 §6.1; ADR-013
//! O-15): a field is identified here by its declared *name* (a `str`-keyed
//! lookup, `match_names`/`fill_slots`), not by the kernel's opaque
//! `MemberId` (`quire_exact::FieldDeclaration`'s own key) -- adopting
//! `MemberId` here needs the ADR-013 O-06 member-identity resolution that
//! `check`'s `CheckedGraph` (ADR-013 T-1, S-3) has not yet landed
//! (`value::member`'s own doc comment: "No production caller constructs a
//! `Member` yet"). `ConstructionCause` also carries `UnknownDeclaration`,
//! which the kernel's own `ConstructionCause` has no need of: the kernel's
//! `record`/`tuple` take their declared shape directly, with no registry
//! lookup to fail, while this module's `TypeEnvironment::record`/`tuple`/
//! `evaluate_record`/`evaluate_tuple` look a `NodeKey` up in the registry
//! first and must report that lookup's own failure. Because of this,
//! `TypeEnvironment` cannot call the kernel's checked `record`/`tuple`
//! constructors either (they need the kernel's `MemberId`-keyed
//! `FieldDeclaration`); it does its own name-keyed checking exactly as
//! before, through `fill_slots`/`match_names`, and then calls the
//! kernel's trusted, unchecked
//! [`from_admitted_slots`] to materialize
//! the result -- mirroring `quire_exact::OptionValue::from_admitted`'s
//! identical bypass role, which `value::expression::evaluate` already calls
//! directly.
//!
//! FR-089-AC-6 (QSL-131 V5): kernel `ValueType::admits` refuses every
//! `(ValueType::Population, Value::Population)` pair outright -- the
//! declared-maximum comparison (FR-089-AC-5) is the QSL layer's own check.
//! `Population` is FR-153's own restriction: it is never nested inside a
//! record field, tuple position, option payload or collection element (every
//! such context is refused earlier, at declaration admission, by
//! `TypeEnvironment::type_refusal`), so none of this module's `admits()`
//! calls (`fill_slots`'s field check) ever receive a `Population` pair; only
//! `value::expression::validate`'s top-level parameter-admission loop needs
//! the FR-089-AC-5 compensation, since `Population<T>[N]` is reachable there
//! directly as a bare parameter type.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use qsl_foundation::diagnostic::{LimitExceeded, LimitKind as StageLimitKind, StageFailure};

use quire_exact::{
    compare_shifted, power_of_ten_bits, sbits, sdigits, Charge, ChargePoint, ComparisonOperator,
    Decimal, DecimalOperation, DecimalType, IllTyped, IllTypedCause, Integer, LimitKind, Meter,
    Outcome, Presence, Quantity, Rational, Refusal,
};
use quire_exact::{from_admitted_slots, retain_composite, Deferred, FieldValue, Value, ValueType};

use super::enumeration::{compare_enum, EnumMemberIndex};
use super::quantity::{
    compare_quantity, convert_quantity, ConvertedValue, QuantityTarget, UnitScope, UnitTable,
};
use super::stop::{outcome_from_stop, outcome_into_stop, Stop};
use quire_exact::CollectionKind;
use quire_exact::EffectiveId;
use quire_exact::EnumShape;
use quire_exact::NodeKey;
use quire_exact::{compare_text, evaluate_decimal};
use std::fmt;

/// One object-type field's identity: the object type that declares it and
/// its declared name (FR-151 field redefinition names its target this way).
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FieldRef {
    /// The declaring object type's effective identity.
    pub owner: EffectiveId,
    /// The field's declared name.
    pub name: String,
}

impl FieldRef {
    /// The field `name` declared by the object type `owner`.
    pub fn new(owner: EffectiveId, name: impl Into<String>) -> Self {
        Self {
            owner,
            name: name.into(),
        }
    }
}

/// A declaration-owned named field; its identity is (declaration key, name).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldDeclaration {
    name: String,
    value_type: ValueType,
    presence: Presence,
    /// The inherited field this one redefines (FR-151
    /// `quire.model.normalize.redefine/v1`), if any. Only an object-type
    /// field may carry one.
    redefines: Option<FieldRef>,
}

impl AsRef<FieldDeclaration> for FieldDeclaration {
    fn as_ref(&self) -> &FieldDeclaration {
        self
    }
}

impl FieldDeclaration {
    /// The field `name: value_type` or `name: value_type?`.
    pub fn new(name: impl Into<String>, value_type: ValueType, presence: Presence) -> Self {
        Self {
            name: name.into(),
            value_type,
            presence,
            redefines: None,
        }
    }

    /// This field, redefining the inherited field `target` (FR-151): the
    /// producer copies the domain package's own `redefines` link, which
    /// phase-4 normalization has already checked. In every object type that
    /// inherits both, `target` is hidden and this field takes its one slot.
    #[must_use]
    pub fn with_redefines(mut self, target: FieldRef) -> Self {
        self.redefines = Some(target);
        self
    }

    /// The inherited field this one redefines, if any.
    pub fn redefines(&self) -> Option<&FieldRef> {
        self.redefines.as_ref()
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

/// A record field in a record value expression. Omitting a `?` field
/// constructs `absent`.
pub enum FieldExpression<'a> {
    /// `f: e`.
    Evaluate(Deferred<'a>),
    /// `f: null`.
    Null,
}

impl fmt::Debug for FieldExpression<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evaluate(_) => formatter.write_str("Evaluate(..)"),
            Self::Null => formatter.write_str("Null"),
        }
    }
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

/// Refuse construction of `component` with `cause`.
fn refuse<T>(component: Component, cause: ConstructionCause) -> Result<T, ConstructionRefusal> {
    Err(ConstructionRefusal { component, cause })
}

/// Index supplied entries by declared name, refusing an undeclared or
/// repeated name. This module's own `TypeEnvironment::evaluate_record` uses
/// it to match supplied `FieldExpression`s to declared fields the same way
/// [`fill_slots`] matches supplied `FieldValue`s.
fn match_names<'n, F: AsRef<FieldDeclaration>, T>(
    declared: &[F],
    supplied: Vec<(&'n str, T)>,
) -> Result<BTreeMap<&'n str, T>, ConstructionRefusal> {
    let mut by_name = BTreeMap::new();
    for (name, entry) in supplied {
        let component = || Component::Field(name.to_owned());
        if !declared.iter().any(|field| field.as_ref().name == name) {
            return refuse(component(), ConstructionCause::UndeclaredField);
        }
        if by_name.insert(name, entry).is_some() {
            return refuse(component(), ConstructionCause::DuplicateField);
        }
    }
    Ok(by_name)
}

/// Declaration-ordered slots of a record or object from supplied fields. An
/// object's `declared` is its type's effective attribute set
/// ([`TypeEnvironment::attributes`]), so an inherited field has a slot.
pub(crate) fn fill_slots<F: AsRef<FieldDeclaration>>(
    declared: &[F],
    supplied: Vec<(&str, FieldValue)>,
) -> Result<Box<[FieldValue]>, ConstructionRefusal> {
    let mut by_name = match_names(declared, supplied)?;
    let mut slots = Vec::with_capacity(declared.len());
    for field in declared {
        let field = field.as_ref();
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

/// Build a composite value of `declaration` from already name-checked
/// slots, through the kernel's trusted
/// [`from_admitted_slots`](quire_exact::from_admitted_slots) bypass: this
/// module's `TypeEnvironment` construction methods have already checked
/// every slot against its own declared shape, so no second, kernel-side
/// check is needed (and the kernel's own checked `record`/`tuple` are not
/// reachable here, since they take `MemberId`-keyed declarations this
/// module does not have -- see the module doc comment).
fn composite(declaration: NodeKey, slots: Box<[FieldValue]>) -> Value {
    from_admitted_slots(declaration, slots)
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

/// A model object type exported by a bound model, with its attributes. It is
/// keyed by its effective-declaration identity (ADR-013 O-05), the identity a
/// `Reference<T>` value's type component carries (FR-143), which `model`
/// computes over the domain package's effective view -- never a checked
/// node id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectTypeDeclaration {
    key: EffectiveId,
    name: String,
    attributes: Vec<FieldDeclaration>,
    /// Every directly declared supertype (FR-151/FR-152/FR-153 generalization,
    /// #204 round 1 H1), empty unless [`Self::with_supertypes`] sets it.
    supertypes: Vec<EffectiveId>,
}

impl ObjectTypeDeclaration {
    /// The object type `name` with declaration identity `key`, declaring no
    /// supertype. See [`Self::with_supertypes`] to declare one.
    pub fn new(
        key: EffectiveId,
        name: impl Into<String>,
        attributes: Vec<FieldDeclaration>,
    ) -> Self {
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
    pub fn with_supertypes(mut self, supertypes: Vec<EffectiveId>) -> Self {
        self.supertypes = supertypes;
        self
    }

    /// The object type's effective-declaration identity.
    pub fn key(&self) -> EffectiveId {
        self.key
    }

    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The attributes this type itself declares, in declaration order. The
    /// type's full attribute set, inherited fields included, is
    /// [`TypeEnvironment::attributes`].
    pub fn attributes(&self) -> &[FieldDeclaration] {
        &self.attributes
    }

    /// Every directly declared supertype, in declaration order.
    pub fn supertypes(&self) -> &[EffectiveId] {
        &self.supertypes
    }
}

/// One attribute of an object type's effective attribute set: a field the
/// type declares or inherits and that no other field of the set redefines.
/// It is one storage slot of every object of that type.
///
/// A cheap handle: every descendant that inherits the attribute unchanged
/// shares the one allocation its declaring type made, so a deep chain holds
/// each field's declaration once, not once per descendant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveAttribute(Arc<AttributeSlot>);

#[derive(Debug, Eq, PartialEq)]
struct AttributeSlot {
    owner: EffectiveId,
    field: FieldDeclaration,
    /// This field and every field it stands in for: those it redefines,
    /// transitively, and those a more derived redefinition of the same
    /// target hid. Ascending, so [`EffectiveAttribute::stands_for`] is a
    /// binary search.
    lineage: Vec<FieldRef>,
    /// The same fields as `lineage`, as positions in the admission's
    /// [`FieldTable`], ascending.
    members: Vec<usize>,
    /// This field's own position in the admission's [`FieldTable`].
    identity: usize,
}

impl EffectiveAttribute {
    /// The object type that declares this field.
    pub fn owner(&self) -> EffectiveId {
        self.0.owner
    }

    /// The field's declaration.
    pub fn field(&self) -> &FieldDeclaration {
        &self.0.field
    }

    /// This field's own identity.
    pub fn identity(&self) -> FieldRef {
        FieldRef::new(self.0.owner, self.0.field.name())
    }

    /// Whether this slot holds `field`: `field` is this attribute, or a field
    /// it redefines or hides.
    pub fn stands_for(&self, field: &FieldRef) -> bool {
        self.0.lineage.binary_search(field).is_ok()
    }
}

impl AsRef<FieldDeclaration> for EffectiveAttribute {
    fn as_ref(&self) -> &FieldDeclaration {
        &self.0.field
    }
}

/// The NFR-012 default `ancestor_steps` ceiling, which [`TypeEnvironment::new`]
/// admits under. The model's own `ModelNormalizationLimits` and
/// `PopulationAdmissionLimits` defaults read this same value.
pub const DEFAULT_ANCESTOR_STEPS: u64 = 100_000;

/// The NFR-012 default admission `work_units` budget, the same value as the
/// model's own `ModelNormalizationLimits::work_units` and
/// `PopulationAdmissionLimits::work_units` defaults.
pub const DEFAULT_WORK_UNITS: u64 = 16_777_216;

/// The ceilings [`TypeEnvironment::bounded`] admits object types under.
/// Every member is a real limit; zero is never "unlimited".
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TypeEnvironmentLimits {
    /// FR-082's `ancestor_steps`: the most types one conformance walk may
    /// expand, the type it starts from included.
    pub ancestor_steps: u64,
    /// Cumulative work units admission may spend building the ancestor
    /// closure and flattening every object type's attributes. One unit is
    /// one ancestor or attribute copied into a type's set, or one field a
    /// lineage names. Running out is a work-budget stage limit.
    pub work_units: u64,
}

impl Default for TypeEnvironmentLimits {
    fn default() -> Self {
        Self {
            ancestor_steps: DEFAULT_ANCESTOR_STEPS,
            work_units: DEFAULT_WORK_UNITS,
        }
    }
}

/// The admission work meter: charges fail once `limit` units are spent.
struct WorkBudget {
    spent: u64,
    limit: u64,
}

impl WorkBudget {
    fn new(limit: u64) -> Self {
        Self { spent: 0, limit }
    }

    /// Charge `units`, or the work-budget stage limit (FR-082, ADR-014 B-3)
    /// once the budget would be passed, naming the cumulative total the
    /// refused charge would have reached.
    fn charge(&mut self, units: usize) -> Result<(), LimitExceeded> {
        let units = u64::try_from(units).unwrap_or(u64::MAX);
        match self.spent.checked_add(units) {
            Some(spent) if spent <= self.limit => {
                self.spent = spent;
                Ok(())
            }
            _ => Err(LimitExceeded::new(
                StageLimitKind::WorkBudget,
                self.limit,
                u128::from(self.spent) + u128::from(units),
            )),
        }
    }
}

/// Why admitting one declaration stopped: a refusal of the input, or a
/// reached stage limit, which names no declaration.
enum Stopped {
    Refused(DeclarationCause),
    Limit(LimitExceeded),
}

impl From<DeclarationCause> for Stopped {
    fn from(cause: DeclarationCause) -> Self {
        Self::Refused(cause)
    }
}

impl From<LimitExceeded> for Stopped {
    fn from(limit: LimitExceeded) -> Self {
        Self::Limit(limit)
    }
}

impl Stopped {
    /// This stop as the admission's stage failure, a refusal naming
    /// `declaration`.
    fn at(self, declaration: &str) -> StageFailure<InvalidDeclaration> {
        match self {
            Self::Refused(cause) => StageFailure::Refused(InvalidDeclaration {
                declaration: declaration.to_owned(),
                cause,
            }),
            Self::Limit(limit) => StageFailure::Limit(limit),
        }
    }
}

/// Type-environment admission's outcome (FR-082): a refusal of the
/// declarations, or a `TypeEnvironmentLimits` ceiling reached, which is a
/// stage limit (ADR-014 B-3) with no locus, since the object types come
/// from an admitted domain package, not a source unit (FR-096).
pub type Admission<T> = Result<T, StageFailure<InvalidDeclaration>>;

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
            | DeclarationCause::UnknownDeclaration(_)
            | DeclarationCause::UnknownObjectType(_)
            | DeclarationCause::RedefinitionTarget(_)
            | DeclarationCause::RedefinitionConflict(_) => "invalid_semantic_graph",
            DeclarationCause::Type(_)
            | DeclarationCause::Recursion { .. }
            | DeclarationCause::GeneralizationCycle { .. }
            | DeclarationCause::RedefinitionWidens(_) => IllTyped::CODE,
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
    /// Two fields of one record, or two attributes of one object type's
    /// effective attribute set, share this name. An inherited field and a
    /// field of the same name that does not redefine it are two attributes.
    DuplicateMember(String),
    /// A field's `redefines` names no field of a proper ancestor of its
    /// owning object type, or a record field declares a `redefines`.
    RedefinitionTarget(FieldRef),
    /// Two attributes of one object type's effective set both stand for
    /// this field, and neither owner is a proper descendant of the other:
    /// FR-151's conflicting redefinitions of one member reaching a type.
    RedefinitionConflict(FieldRef),
    /// A field that redefines this one widens it: an optional redefiner of
    /// a required field, or a value type admitting a value the redefined
    /// field's type does not.
    RedefinitionWidens(FieldRef),
    /// A type names a key that is no declaration of the package.
    UnknownDeclaration(NodeKey),
    /// A declared supertype names an effective identity that is no admitted
    /// object type of the package.
    UnknownObjectType(EffectiveId),
    /// A member type is ill-typed: a `Reference<T>` target that is not an
    /// admitted model object type (`type-mismatch`), or an IEEE-bearing set,
    /// bag or ordered-set element type (`operator-ineligible`).
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
    object_types: BTreeMap<EffectiveId, ObjectTypeDeclaration>,
    /// Every object type's own proper ancestor set (H1, #204 round 1):
    /// transitive, not just direct, `supertypes`. Precomputed once in
    /// [`TypeEnvironment::new`], after the supertypes graph is known
    /// acyclic, so [`Self::conforms`] is a plain set lookup.
    ancestry: Ancestry,
    /// Every object type's effective attribute set (QSL-57): its own fields
    /// plus every ancestor's, less each field another field of the set
    /// redefines. Computed once in [`TypeEnvironment::bounded`]; an object's
    /// storage slots are exactly this set, so an attribute lookup never
    /// walks ancestors.
    effective: BTreeMap<EffectiveId, Vec<EffectiveAttribute>>,
    /// The units a `ValueType::Quantity` of this package names by id.
    units: UnitTable,
}

/// One containment edge of the recursion rule.
#[derive(Clone, Copy)]
struct Edge {
    target: NodeKey,
    named: bool,
    escapes: bool,
}

impl TypeEnvironment {
    /// Admit `composites` and `object_types` as one closed environment,
    /// under the NFR-012 default ceilings ([`TypeEnvironmentLimits::default`]).
    /// See [`Self::bounded`].
    pub fn new(
        composites: impl IntoIterator<Item = CompositeDeclaration>,
        object_types: impl IntoIterator<Item = ObjectTypeDeclaration>,
    ) -> Admission<Self> {
        Self::bounded(composites, object_types, TypeEnvironmentLimits::default())
    }

    /// Admit `composites` and `object_types` as one closed environment.
    ///
    /// `limits.ancestor_steps` is the FR-082 ceiling the model walks this
    /// package's conformance under at evaluation (the population binding's
    /// own `ancestor_steps`). An object type whose walk would expand more
    /// types than that, itself included, stops admission with a node-count
    /// stage limit (FR-082, ADR-014 B-3). Check time is the stricter
    /// side: every conformance question the checker answers from this
    /// environment is one evaluation completes with the same verdict.
    ///
    /// `limits.work_units` bounds the whole admission's ancestor-closure
    /// and flattening work; running out is a work-budget stage limit.
    pub fn bounded(
        composites: impl IntoIterator<Item = CompositeDeclaration>,
        object_types: impl IntoIterator<Item = ObjectTypeDeclaration>,
        limits: TypeEnvironmentLimits,
    ) -> Admission<Self> {
        let mut environment = Self::default();
        for declaration in composites {
            let refuse = |cause| {
                StageFailure::Refused(InvalidDeclaration {
                    declaration: declaration.name.clone(),
                    cause,
                })
            };
            if let CompositeShape::Record(fields) = &declaration.shape {
                if let Some(name) = duplicate_name(fields) {
                    return Err(refuse(DeclarationCause::DuplicateMember(name)));
                }
                if let Some(target) = fields.iter().find_map(FieldDeclaration::redefines) {
                    return Err(refuse(DeclarationCause::RedefinitionTarget(target.clone())));
                }
            }
            if environment.composites.contains_key(&declaration.key) {
                return Err(refuse(DeclarationCause::DuplicateKey));
            }
            environment.composites.insert(declaration.key, declaration);
        }
        for declaration in object_types {
            let refuse = |cause| {
                StageFailure::Refused(InvalidDeclaration {
                    declaration: declaration.name.clone(),
                    cause,
                })
            };
            if let Some(name) = duplicate_name(&declaration.attributes) {
                return Err(refuse(DeclarationCause::DuplicateMember(name)));
            }
            if environment.object_types.contains_key(&declaration.key) {
                return Err(refuse(DeclarationCause::DuplicateKey));
            }
            environment
                .object_types
                .insert(declaration.key, declaration);
        }
        environment
            .check_member_types()
            .map_err(StageFailure::Refused)?;
        environment
            .check_recursion(RecursionEdges::Unnamed)
            .map_err(StageFailure::Refused)?;
        environment
            .check_recursion(RecursionEdges::NonEscaping)
            .map_err(StageFailure::Refused)?;
        let mut budget = WorkBudget::new(limits.work_units);
        environment.check_supertypes(&mut budget)?;
        environment.ancestry = environment.compute_ancestors(&mut budget)?;
        environment
            .check_ancestor_steps(limits.ancestor_steps)
            .map_err(StageFailure::Limit)?;
        let table = FieldTable::new(&environment.object_types);
        environment
            .check_redefinitions(&table)
            .map_err(StageFailure::Refused)?;
        let effective = environment.compute_effective(&table, &mut budget)?;
        environment.effective = effective;
        Ok(environment)
    }

    /// The object type's effective attribute set, in slot order: its own
    /// fields in declaration order, then each direct supertype's slots in
    /// its own order, the supertypes in declaration order. Each attribute
    /// appears once; a field a more derived attribute of the set stands for
    /// is left out, and that attribute keeps its own place. `None` for a key
    /// that is no admitted object type.
    pub fn attributes(&self, object_type: EffectiveId) -> Option<&[EffectiveAttribute]> {
        self.effective.get(&object_type).map(Vec::as_slice)
    }

    /// The attribute `name` resolves to in the object type's effective set.
    pub fn attribute(&self, object_type: EffectiveId, name: &str) -> Option<&EffectiveAttribute> {
        self.attributes(object_type)?
            .iter()
            .find(|attribute| attribute.field().name() == name)
    }

    /// This environment with `units` as its quantity unit table.
    pub fn with_units(mut self, units: UnitTable) -> Self {
        self.units = units;
        self
    }

    /// The quantity units this package's types name by id.
    pub fn units(&self) -> &UnitTable {
        &self.units
    }

    /// The admitted record or tuple declaration with this key.
    pub fn composite(&self, key: NodeKey) -> Option<&CompositeDeclaration> {
        self.composites.get(&key)
    }

    /// Every admitted record and tuple declaration in key order.
    pub(crate) fn composites(&self) -> impl Iterator<Item = &CompositeDeclaration> {
        self.composites.values()
    }

    /// The admitted object type with this effective identity.
    pub fn object_type(&self, key: EffectiveId) -> Option<&ObjectTypeDeclaration> {
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
    pub fn conforms(&self, sub: EffectiveId, sup: EffectiveId) -> bool {
        sub == sup || self.ancestry.is_ancestor(sub, sup)
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
                // FR-143: `T` in `Reference<T>` must name a model object
                // type; any other target is `type-mismatch`. The key is an
                // effective identity (ADR-013 O-05), so an admitted record or
                // tuple (keyed by `NodeKey`) can never satisfy it either.
                ValueType::Reference(key) if !self.object_types.contains_key(key) => {
                    return Some(DeclarationCause::Type(IllTypedCause::TypeMismatch));
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
    ///
    /// Each type is marked while it is on the walk's path, so a back edge is
    /// found without scanning the path, and `budget` is charged each edge
    /// the walk follows.
    fn check_supertypes(&self, budget: &mut WorkBudget) -> Admission<()> {
        let name = |key: &EffectiveId| {
            self.object_types
                .get(key)
                .map_or_else(String::new, |declaration| declaration.name.clone())
        };
        for declaration in self.object_types.values() {
            for supertype in &declaration.supertypes {
                if !self.object_types.contains_key(supertype) {
                    return Err(StageFailure::Refused(InvalidDeclaration {
                        declaration: declaration.name.clone(),
                        cause: DeclarationCause::UnknownObjectType(*supertype),
                    }));
                }
            }
        }
        let mut finished: BTreeSet<EffectiveId> = BTreeSet::new();
        let mut on_path: BTreeSet<EffectiveId> = BTreeSet::new();
        for root in self.object_types.keys() {
            if finished.contains(root) {
                continue;
            }
            let mut path: Vec<(EffectiveId, usize)> = vec![(*root, 0)];
            on_path.insert(*root);
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                let Some(target) = self
                    .object_types
                    .get(&node)
                    .and_then(|declaration| declaration.supertypes.get(*next))
                else {
                    finished.insert(node);
                    on_path.remove(&node);
                    path.pop();
                    continue;
                };
                *next += 1;
                budget.charge(1).map_err(StageFailure::Limit)?;
                if on_path.contains(target) {
                    // Found once, on the refusal path only.
                    let start = path
                        .iter()
                        .position(|(on_path, _)| on_path == target)
                        .unwrap_or_default();
                    let mut cycle: Vec<String> =
                        path.iter().skip(start).map(|(key, _)| name(key)).collect();
                    cycle.push(name(target));
                    return Err(StageFailure::Refused(InvalidDeclaration {
                        declaration: name(target),
                        cause: DeclarationCause::GeneralizationCycle { cycle },
                    }));
                }
                if !finished.contains(target) {
                    on_path.insert(*target);
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
    ///
    /// Each type's set is a copy of its supertypes' sets, so the whole
    /// closure costs the sum of every type's ancestor count; `budget` is
    /// charged that, a supertype and its ancestors at a time, before the
    /// copy is made.
    fn compute_ancestors(&self, budget: &mut WorkBudget) -> Admission<Ancestry> {
        // Positions are `u32`: a package with more object types than that
        // could never be flattened within any budget either. Flattening
        // charges at least one unit a type, so the count is the least the
        // budget would have to reach.
        if u32::try_from(self.object_types.len()).is_err() {
            let types = u128::try_from(self.object_types.len()).unwrap_or(u128::MAX);
            return Err(StageFailure::Limit(LimitExceeded::new(
                StageLimitKind::WorkBudget,
                budget.limit,
                types.max(u128::from(budget.limit) + 1),
            )));
        }
        let positions: BTreeMap<EffectiveId, u32> = self
            .object_types
            .keys()
            .zip(0_u32..)
            .map(|(key, position)| (*key, position))
            .collect();
        let mut ancestors: Vec<Option<Vec<u32>>> = vec![None; self.object_types.len()];
        let finished = |ancestors: &[Option<Vec<u32>>], key: &EffectiveId| {
            positions
                .get(key)
                .and_then(|position| ancestors.get(*position as usize))
                .is_some_and(Option::is_some)
        };
        for root in self.object_types.values() {
            if finished(&ancestors, &root.key) {
                continue;
            }
            let mut path: Vec<(&ObjectTypeDeclaration, usize)> = vec![(root, 0)];
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                if let Some(target) = node.supertypes.get(*next) {
                    *next += 1;
                    if !finished(&ancestors, target) {
                        if let Some(general) = self.object_types.get(target) {
                            path.push((general, 0));
                        }
                    }
                    continue;
                }
                path.pop();
                let own = Self::own_ancestors(node, &positions, &ancestors, budget)
                    .map_err(StageFailure::Limit)?;
                if let Some(slot) = positions
                    .get(&node.key)
                    .and_then(|position| ancestors.get_mut(*position as usize))
                {
                    *slot = Some(own);
                }
            }
        }
        Ok(Ancestry {
            positions,
            ancestors: ancestors
                .into_iter()
                .map(Option::unwrap_or_default)
                .collect(),
        })
    }

    /// `node`'s proper ancestors, ascending, from its direct supertypes'
    /// finished sets. `budget` is charged each supertype plus its ancestor
    /// count before that set is copied; with several supertypes, the
    /// gathered positions are sorted and deduplicated once, and that sort
    /// is charged too (`k` units a doubling of the `k` gathered).
    fn own_ancestors(
        node: &ObjectTypeDeclaration,
        positions: &BTreeMap<EffectiveId, u32>,
        ancestors: &[Option<Vec<u32>>],
        budget: &mut WorkBudget,
    ) -> Result<Vec<u32>, LimitExceeded> {
        let mut own: Vec<u32> = Vec::new();
        for supertype in &node.supertypes {
            let Some(position) = positions.get(supertype).copied() else {
                continue;
            };
            let further = ancestors
                .get(position as usize)
                .and_then(Option::as_deref)
                .unwrap_or_default();
            budget.charge(further.len().saturating_add(1))?;
            own.push(position);
            own.extend_from_slice(further);
        }
        match node.supertypes.len() {
            // No supertype, or one: its set is already ascending, and only
            // its own position needs placing.
            0 => {}
            1 => {
                own.rotate_left(1);
                if let Some(position) = own.pop() {
                    if let Err(at) = own.binary_search(&position) {
                        own.insert(at, position);
                    }
                }
            }
            _ => {
                let gathered = own.len();
                let doublings = usize::try_from(gathered.max(1).ilog2())
                    .unwrap_or(usize::MAX)
                    .saturating_add(1);
                budget.charge(gathered.saturating_mul(doublings))?;
                own.sort_unstable();
                own.dedup();
            }
        }
        Ok(own)
    }

    /// Stop at the first object type, in key order, whose FR-082 conformance
    /// walk expands more than `limit` types: a node-count stage limit whose
    /// actual counter is the ceiling plus one, the step the walk would stop
    /// at (FR-082). The model's walk from `S`
    /// (`ModelIndex::conforms`) expands `S` and then each distinct ancestor
    /// once, and stops early only when it meets its target, so `1 +` the
    /// ancestor count is the most any walk from `S` expands. Admitting only
    /// types within `limit` makes every walk from an admitted type complete
    /// under the same ceiling at evaluation.
    fn check_ancestor_steps(&self, limit: u64) -> Result<(), LimitExceeded> {
        for declaration in self.object_types.values() {
            let ancestors = self.ancestry.count(declaration.key);
            let expanded = u64::try_from(ancestors)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            if expanded > limit {
                return Err(LimitExceeded::new(
                    StageLimitKind::NodeCount,
                    limit,
                    u128::from(limit) + 1,
                ));
            }
        }
        Ok(())
    }

    /// Refuse the first object-type field, in key then declaration order,
    /// whose `redefines` names no field of a proper ancestor of its owner
    /// (FR-151: the target must be inherited by the owning type).
    fn check_redefinitions(&self, table: &FieldTable<'_>) -> Result<(), InvalidDeclaration> {
        for declaration in self.object_types.values() {
            for target in declaration
                .attributes
                .iter()
                .filter_map(FieldDeclaration::redefines)
            {
                let inherited = self.ancestry.is_ancestor(declaration.key, target.owner)
                    && table.position(target).is_some();
                if !inherited {
                    return Err(InvalidDeclaration {
                        declaration: declaration.name.clone(),
                        cause: DeclarationCause::RedefinitionTarget(target.clone()),
                    });
                }
            }
        }
        Ok(())
    }

    /// The first field `attribute` stands in for that it widens: `deref(r).f`
    /// through that field's owner reads `attribute`'s slot, so its presence
    /// must be required where that field's is, and every value its type
    /// admits that field's type must admit ([`Self::narrows`]). Otherwise
    /// the read would yield a value its checked type does not describe.
    fn widened(&self, attribute: &EffectiveAttribute, table: &FieldTable<'_>) -> Option<FieldRef> {
        let field = attribute.field();
        attribute
            .0
            .members
            .iter()
            .zip(&attribute.0.lineage)
            .find(|(member, _)| {
                table.field(**member).is_some_and(|redefined| {
                    (field.presence == Presence::Optional
                        && redefined.presence == Presence::Required)
                        || !Self::narrows(&field.value_type, &redefined.value_type)
                })
            })
            .map(|(_, hidden)| hidden.clone())
    }

    /// Whether every value `narrower` admits, `wider` admits *and*
    /// evaluation's `ValueType::admits` accepts a value of `narrower` where
    /// `wider` is declared: the same type, or an `Int` interval within
    /// `Integer` or within a wider `Int` interval. A reference field is
    /// redefined only with the identical reference type, since `admits`
    /// matches a reference's object type exactly. Any other pair is refused,
    /// never guessed.
    fn narrows(narrower: &ValueType, wider: &ValueType) -> bool {
        match (narrower, wider) {
            (narrower, wider) if narrower == wider => true,
            (ValueType::Int(_), ValueType::Integer) => true,
            (ValueType::Int(inner), ValueType::Int(outer)) => {
                outer.contains(inner.lower()) && outer.contains(inner.upper())
            }
            _ => false,
        }
    }

    /// Every object type's effective attribute set (QSL-57), flattened once.
    ///
    /// This applies FR-151's phase-4 redefinition result
    /// (`quire.model.normalize.redefine/v1`) to the `redefines` links a
    /// producer copies from the normalized domain package: a redefined field
    /// stays declared but is hidden in every type that also has its
    /// redefiner; when several redefinitions of one field reach a type, only
    /// the one whose owner is a proper descendant of every other stays
    /// exposed, and it stands for the others' slots too. Two exposed
    /// attributes that still stand for one field conflict
    /// ([`DeclarationCause::RedefinitionConflict`]), and two exposed
    /// attributes of one name are two fields, not a redefinition
    /// ([`DeclarationCause::DuplicateMember`]).
    ///
    /// Types are flattened supertypes first (an explicit post-order stack),
    /// each from its own fields and its direct supertypes' finished sets, so
    /// no type re-walks its ancestors and an inherited attribute is shared,
    /// not copied. `budget` is charged every attribute and lineage field a
    /// type's flattening reads.
    fn compute_effective(
        &self,
        table: &FieldTable<'_>,
        budget: &mut WorkBudget,
    ) -> Admission<BTreeMap<EffectiveId, Vec<EffectiveAttribute>>> {
        let mut effective: BTreeMap<EffectiveId, Vec<EffectiveAttribute>> = BTreeMap::new();
        let mut scratch = Scratch::new(table.len(), table.name_count());
        for (root, declaration) in &self.object_types {
            if effective.contains_key(root) {
                continue;
            }
            let mut path: Vec<(&ObjectTypeDeclaration, usize)> = vec![(declaration, 0)];
            while let Some((node, next)) = path.last_mut() {
                let node = *node;
                if let Some(supertype) = node.supertypes.get(*next) {
                    *next += 1;
                    if !effective.contains_key(supertype) {
                        if let Some(general) = self.object_types.get(supertype) {
                            path.push((general, 0));
                        }
                    }
                    continue;
                }
                path.pop();
                let attributes = self
                    .flatten(node, &effective, table, &mut scratch, budget)
                    .map_err(|stopped| stopped.at(&node.name))?;
                effective.insert(node.key, attributes);
            }
        }
        Ok(effective)
    }

    /// One object type's effective attribute set, from its own fields and
    /// its direct supertypes' already flattened sets.
    ///
    /// Every attribute standing for a common field is one redefinition
    /// group (a union-find over the fields their lineages name). A group of
    /// one is kept as it is. A larger group keeps only the attribute whose
    /// owner is a proper descendant of every other member's owner, with the
    /// members' lineages merged into it; a group with no such member is
    /// FR-151's conflict.
    fn flatten(
        &self,
        declaration: &ObjectTypeDeclaration,
        effective: &BTreeMap<EffectiveId, Vec<EffectiveAttribute>>,
        table: &FieldTable<'_>,
        scratch: &mut Scratch,
        budget: &mut WorkBudget,
    ) -> Result<Vec<EffectiveAttribute>, Stopped> {
        scratch.next_type();
        let mut candidates: Vec<EffectiveAttribute> =
            Vec::with_capacity(declaration.attributes.len());
        let mut fresh: Vec<bool> = Vec::with_capacity(declaration.attributes.len());
        for field in &declaration.attributes {
            let attribute = own_attribute(declaration.key, field, table, budget)?;
            scratch.claim_identity(attribute.0.identity, candidates.len());
            candidates.push(attribute);
            fresh.push(true);
        }
        for supertype in &declaration.supertypes {
            let inherited = effective.get(supertype).map_or(&[][..], Vec::as_slice);
            budget.charge(inherited.len())?;
            for attribute in inherited {
                // A diamond passes one field down two paths: keep it once,
                // with both paths' lineages when they differ.
                match scratch.identity(attribute.0.identity) {
                    None => {
                        scratch.claim_identity(attribute.0.identity, candidates.len());
                        candidates.push(attribute.clone());
                        fresh.push(false);
                    }
                    Some(kept) => {
                        let Some(existing) = candidates.get(kept) else {
                            continue;
                        };
                        budget.charge(attribute.0.members.len())?;
                        if Arc::ptr_eq(&existing.0, &attribute.0)
                            || existing.0.members == attribute.0.members
                        {
                            continue;
                        }
                        let merged = merge(existing, [existing, attribute], table, budget)?;
                        if let (Some(slot), Some(flag)) =
                            (candidates.get_mut(kept), fresh.get_mut(kept))
                        {
                            *slot = merged;
                            *flag = true;
                        }
                    }
                }
            }
        }

        let mut groups = UnionFind::new(candidates.len());
        for (index, attribute) in candidates.iter().enumerate() {
            budget.charge(attribute.0.members.len())?;
            for &member in &attribute.0.members {
                match scratch.holder(member) {
                    Some(holder) => groups.union(holder, index, member),
                    None => scratch.hold(member, index),
                }
            }
        }

        let mut members_of: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut roots = Vec::with_capacity(candidates.len());
        for index in 0..candidates.len() {
            let root = groups.find(index);
            roots.push(root);
            if groups.size(root) > 1 {
                members_of.entry(root).or_default().push(index);
            }
        }
        // Each group's surviving position, and its (possibly merged) attribute.
        let mut survivors: BTreeMap<usize, (usize, EffectiveAttribute)> = BTreeMap::new();
        for (root, members) in &members_of {
            budget.charge(members.len())?;
            let owner = |index: usize| candidates.get(index).map(EffectiveAttribute::owner);
            let properly = |sub: usize, sup: usize| match (owner(sub), owner(sup)) {
                (Some(sub), Some(sup)) => sub != sup && self.conforms(sub, sup),
                _ => false,
            };
            let mut best = members.first().copied().unwrap_or(*root);
            for &member in members {
                if properly(member, best) {
                    best = member;
                }
            }
            let conflict = members
                .iter()
                .any(|&member| member != best && !properly(best, member));
            let Some(winner) = candidates.get(best) else {
                continue;
            };
            if conflict {
                // A group of two or more was joined over a common field, so
                // `shared` is always set; the winner's own field is named
                // otherwise, never an invented one.
                let shared = groups
                    .shared(*root)
                    .and_then(|position| table.reference(position))
                    .unwrap_or_else(|| winner.identity());
                return Err(DeclarationCause::RedefinitionConflict(shared).into());
            }
            let merged = merge(
                winner,
                members.iter().filter_map(|&member| candidates.get(member)),
                table,
                budget,
            )?;
            if let Some(flag) = fresh.get_mut(best) {
                *flag |= !Arc::ptr_eq(&merged.0, &winner.0);
            }
            survivors.insert(*root, (best, merged));
        }

        let mut attributes = Vec::with_capacity(candidates.len());
        let mut checked = Vec::with_capacity(candidates.len());
        for (index, attribute) in candidates.into_iter().enumerate() {
            let root = roots.get(index).copied().unwrap_or(index);
            let is_fresh = fresh.get(index).copied().unwrap_or(true);
            match survivors.get(&root) {
                Some((best, merged)) if *best == index => {
                    attributes.push(merged.clone());
                    checked.push(is_fresh);
                }
                Some(_) => {}
                None => {
                    attributes.push(attribute);
                    checked.push(is_fresh);
                }
            }
        }
        // An attribute a supertype already admitted unchanged was checked
        // there; only this type's own and merged attributes can widen.
        if let Some(widened) = attributes
            .iter()
            .zip(&checked)
            .filter(|(_, fresh)| **fresh)
            .find_map(|(attribute, _)| self.widened(attribute, table))
        {
            return Err(DeclarationCause::RedefinitionWidens(widened).into());
        }
        budget.charge(attributes.len())?;
        for attribute in &attributes {
            let taken = table
                .name_of(attribute.0.identity)
                .is_some_and(|name| !scratch.claim_name(name));
            if taken {
                return Err(
                    DeclarationCause::DuplicateMember(attribute.field().name().to_owned()).into(),
                );
            }
        }
        Ok(attributes)
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
                        Err(stop) => return Ok(outcome_from_stop(Err(stop))),
                    }
                }
            };
            slots.push(slot);
        }
        Ok(retain_composite(
            composite(declaration, slots.into_boxed_slice()),
            meter,
        ))
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
                Err(stop) => return Ok(outcome_from_stop(Err(stop))),
            }
        }
        Ok(retain_composite(
            composite(declaration, slots.into_boxed_slice()),
            meter,
        ))
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

/// Every admitted object type's proper ancestors, as positions in key
/// order: four bytes an ancestor, so the closure of a deep chain stays
/// small.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Ancestry {
    positions: BTreeMap<EffectiveId, u32>,
    /// By position: that type's proper ancestors, ascending.
    ancestors: Vec<Vec<u32>>,
}

impl Ancestry {
    fn of(&self, key: EffectiveId) -> Option<&[u32]> {
        let position = self.positions.get(&key)?;
        self.ancestors.get(*position as usize).map(Vec::as_slice)
    }

    /// Whether `ancestor` is a proper ancestor of `key`.
    fn is_ancestor(&self, key: EffectiveId, ancestor: EffectiveId) -> bool {
        match (self.of(key), self.positions.get(&ancestor)) {
            (Some(ancestors), Some(position)) => ancestors.binary_search(position).is_ok(),
            _ => false,
        }
    }

    /// How many proper ancestors `key` has.
    fn count(&self, key: EffectiveId) -> usize {
        self.of(key).map_or(0, <[u32]>::len)
    }
}

/// `field`'s own attribute, declared by `owner`: it stands for itself and
/// every field it redefines, transitively. [`TypeEnvironment::check_redefinitions`]
/// has admitted every link, and each one moves to a proper ancestor of an
/// acyclic graph; `budget` is charged each link all the same.
fn own_attribute(
    owner: EffectiveId,
    field: &FieldDeclaration,
    table: &FieldTable<'_>,
    budget: &mut WorkBudget,
) -> Result<EffectiveAttribute, Stopped> {
    let own = FieldRef::new(owner, field.name());
    let identity = table
        .position(&own)
        .ok_or_else(|| DeclarationCause::RedefinitionTarget(own.clone()))?;
    let mut members = vec![identity];
    let mut next = field.redefines();
    while let Some(target) = next {
        budget.charge(1)?;
        let position = table
            .position(target)
            .ok_or_else(|| DeclarationCause::RedefinitionTarget(target.clone()))?;
        members.push(position);
        next = table.field(position).and_then(FieldDeclaration::redefines);
    }
    members.sort_unstable();
    members.dedup();
    Ok(EffectiveAttribute(Arc::new(AttributeSlot {
        owner,
        field: field.clone(),
        lineage: table.references(&members),
        members,
        identity,
    })))
}

/// `winner` standing for every field any of `group` stands for. `winner`
/// itself, shared, when it already does.
fn merge<'a>(
    winner: &EffectiveAttribute,
    group: impl IntoIterator<Item = &'a EffectiveAttribute>,
    table: &FieldTable<'_>,
    budget: &mut WorkBudget,
) -> Result<EffectiveAttribute, LimitExceeded> {
    let mut members: Vec<usize> = Vec::new();
    for attribute in group {
        budget.charge(attribute.0.members.len())?;
        members.extend_from_slice(&attribute.0.members);
    }
    budget.charge(winner.0.members.len())?;
    members.extend_from_slice(&winner.0.members);
    members.sort_unstable();
    members.dedup();
    if members == winner.0.members {
        return Ok(winner.clone());
    }
    Ok(EffectiveAttribute(Arc::new(AttributeSlot {
        owner: winner.0.owner,
        field: winner.0.field.clone(),
        lineage: table.references(&members),
        members,
        identity: winner.0.identity,
    })))
}

/// Every object-type field of one admission, numbered in [`FieldRef`]
/// order (owner, then name), so a sorted list of positions and the sorted
/// list of the fields they name line up. Names are numbered too, for the
/// duplicate-member check.
struct FieldTable<'e> {
    positions: BTreeMap<(EffectiveId, &'e str), usize>,
    /// By position: the owner, the declaration and the name's number.
    fields: Vec<(EffectiveId, &'e FieldDeclaration, usize)>,
    names: usize,
}

impl<'e> FieldTable<'e> {
    fn new(object_types: &'e BTreeMap<EffectiveId, ObjectTypeDeclaration>) -> Self {
        let mut sorted: BTreeMap<(EffectiveId, &'e str), &'e FieldDeclaration> = BTreeMap::new();
        for declaration in object_types.values() {
            for field in &declaration.attributes {
                sorted.insert((declaration.key, field.name()), field);
            }
        }
        let mut names: BTreeMap<&'e str, usize> = BTreeMap::new();
        let mut positions = BTreeMap::new();
        let mut fields = Vec::with_capacity(sorted.len());
        for (position, ((owner, name), field)) in sorted.into_iter().enumerate() {
            let next = names.len();
            let number = *names.entry(name).or_insert(next);
            positions.insert((owner, name), position);
            fields.push((owner, field, number));
        }
        Self {
            positions,
            fields,
            names: names.len(),
        }
    }

    fn len(&self) -> usize {
        self.fields.len()
    }

    fn name_count(&self) -> usize {
        self.names
    }

    fn position(&self, field: &FieldRef) -> Option<usize> {
        self.positions
            .get(&(field.owner, field.name.as_str()))
            .copied()
    }

    fn field(&self, position: usize) -> Option<&'e FieldDeclaration> {
        self.fields.get(position).map(|(_, field, _)| *field)
    }

    fn name_of(&self, position: usize) -> Option<usize> {
        self.fields.get(position).map(|(_, _, name)| *name)
    }

    fn reference(&self, position: usize) -> Option<FieldRef> {
        self.fields
            .get(position)
            .map(|(owner, field, _)| FieldRef::new(*owner, field.name()))
    }

    /// The fields at `positions`, in the same (ascending) order.
    fn references(&self, positions: &[usize]) -> Vec<FieldRef> {
        positions
            .iter()
            .filter_map(|position| self.reference(*position))
            .collect()
    }
}

/// Per-type marks for [`TypeEnvironment::flatten`], reused across types:
/// a mark counts only when it carries the current type's epoch, so moving
/// to the next type clears every mark at once.
struct Scratch {
    epoch: u64,
    /// By field position: the candidate holding that field as its identity.
    identities: Vec<(u64, usize)>,
    /// By field position: the first candidate whose lineage names it.
    holders: Vec<(u64, usize)>,
    /// By name number: whether a kept attribute already has that name.
    names: Vec<u64>,
}

impl Scratch {
    fn new(fields: usize, names: usize) -> Self {
        Self {
            epoch: 0,
            identities: vec![(0, 0); fields],
            holders: vec![(0, 0); fields],
            names: vec![0; names],
        }
    }

    fn next_type(&mut self) {
        self.epoch = self.epoch.saturating_add(1);
    }

    fn marked(marks: &[(u64, usize)], epoch: u64, position: usize) -> Option<usize> {
        marks
            .get(position)
            .filter(|(mark, _)| *mark == epoch)
            .map(|(_, candidate)| *candidate)
    }

    fn identity(&self, position: usize) -> Option<usize> {
        Self::marked(&self.identities, self.epoch, position)
    }

    fn claim_identity(&mut self, position: usize, candidate: usize) {
        if let Some(mark) = self.identities.get_mut(position) {
            *mark = (self.epoch, candidate);
        }
    }

    fn holder(&self, position: usize) -> Option<usize> {
        Self::marked(&self.holders, self.epoch, position)
    }

    fn hold(&mut self, position: usize, candidate: usize) {
        if let Some(mark) = self.holders.get_mut(position) {
            *mark = (self.epoch, candidate);
        }
    }

    /// Mark `name` taken; `false` when it already was.
    fn claim_name(&mut self, name: usize) -> bool {
        match self.names.get_mut(name) {
            Some(mark) if *mark == self.epoch => false,
            Some(mark) => {
                *mark = self.epoch;
                true
            }
            None => true,
        }
    }
}

/// Redefinition groups: a union-find over one type's candidates, joined
/// whenever two lineages name a common field. Each group remembers the
/// first such field, for the conflict refusal.
struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
    shared: Vec<Option<usize>>,
}

impl UnionFind {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            size: vec![1; len],
            shared: vec![None; len],
        }
    }

    fn find(&mut self, mut node: usize) -> usize {
        loop {
            let parent = self.parent.get(node).copied().unwrap_or(node);
            if parent == node {
                return node;
            }
            let grandparent = self.parent.get(parent).copied().unwrap_or(parent);
            if let Some(slot) = self.parent.get_mut(node) {
                *slot = grandparent;
            }
            node = grandparent;
        }
    }

    /// Join `left`'s and `right`'s groups over the common field `field`.
    fn union(&mut self, left: usize, right: usize, field: usize) {
        let (left, right) = (self.find(left), self.find(right));
        if left == right {
            return;
        }
        let (small, large) = if self.size(left) < self.size(right) {
            (left, right)
        } else {
            (right, left)
        };
        let merged = self.size(small).saturating_add(self.size(large));
        let shared = self.shared(large).or(self.shared(small)).or(Some(field));
        if let Some(slot) = self.parent.get_mut(small) {
            *slot = large;
        }
        if let Some(slot) = self.size.get_mut(large) {
            *slot = merged;
        }
        if let Some(slot) = self.shared.get_mut(large) {
            *slot = shared;
        }
    }

    fn size(&self, root: usize) -> usize {
        self.size.get(root).copied().unwrap_or(1)
    }

    fn shared(&self, root: usize) -> Option<usize> {
        self.shared.get(root).copied().flatten()
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
    let value = outcome_into_stop(outcome)?;
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

    /// The `convert<target>` target, when the operand converts.
    pub(crate) fn target(&self) -> Option<&ValueType> {
        self.target.as_ref()
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
    /// The unit of every top-level quantity type the operands name, resolved
    /// at checking, so evaluation reads no package table.
    units: UnitTable,
    /// ADR-013 T-6 (last sentence): the checked `VariantId -> EnumValue`
    /// index, captured at checking so `Self::evaluate` needs no extra
    /// argument. Empty and never consulted unless `schedule` is
    /// `EqualitySchedule::Enum`. Filtered (SR-511 M2) to only the compared
    /// operands' own `EnumShape` -- never the whole package's enum-member
    /// index -- so a checked package with many sizeable enums does not
    /// retain O(equality nodes x total enum members) in its checked IR.
    enum_members: EnumMemberIndex,
}

impl TypeEnvironment {
    /// Type-check `left op right` against this environment's unit table.
    /// Every refusal is made before any charge. `enum_members` is the
    /// ADR-013 T-6 (last sentence) checked `VariantId -> EnumValue` index:
    /// `TypeEnvironment` itself holds no enum declarations (those are
    /// `check::Scope::enums`', ADR-011 §6.1's own module split), so a
    /// caller checking an `Enum`-scheduled equality supplies it here, once,
    /// rather than [`CheckedEquality::evaluate`] taking it as an extra
    /// argument every caller -- including every non-enum test -- would
    /// otherwise need to thread through. An empty index is correct for any
    /// caller that never checks an enum equality.
    pub fn check_equality(
        &self,
        operator: EqualityOperator,
        left: EqualityOperand,
        right: EqualityOperand,
        enum_members: &EnumMemberIndex,
    ) -> Result<CheckedEquality, IllTyped> {
        self.check_equality_in(
            &UnitScope::new(&self.units),
            operator,
            left,
            right,
            &|shape: &EnumShape| enum_members.filtered(shape.variants()),
        )
    }

    /// [`Self::check_equality`] against one checking stage's units, which
    /// add the compound units its expressions formed. `enum_members` gives
    /// the member index of one compared enum shape: exactly its own
    /// members (SR-511 M2). `check`'s `Scope` answers it from a table built
    /// once per shape (QSL-205), so an equality does not copy its enum.
    pub(crate) fn check_equality_in(
        &self,
        units: &UnitScope<'_>,
        operator: EqualityOperator,
        left: EqualityOperand,
        right: EqualityOperand,
        enum_members: &dyn Fn(&EnumShape) -> EnumMemberIndex,
    ) -> Result<CheckedEquality, IllTyped> {
        let ill_typed = |cause| Err(IllTyped { cause });
        for operand in [&left, &right] {
            if let Some(target) = &operand.target {
                if !admits_equality_conversion(&operand.source, target, units) {
                    return ill_typed(IllTypedCause::TypeMismatch);
                }
            }
        }
        // Every top-level quantity type resolves, or the operands name a unit
        // this package has not admitted (as an unknown enum declaration is a
        // type mismatch at checking).
        let mut resolved = UnitTable::default();
        for value_type in [&left.source, &right.source]
            .into_iter()
            .chain(left.target.iter())
            .chain(right.target.iter())
        {
            if let ValueType::Quantity(id) = value_type {
                let Some(unit) = units.get(*id) else {
                    return ill_typed(IllTypedCause::TypeMismatch);
                };
                resolved.insert(unit.clone());
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
            (ValueType::Quantity(l), ValueType::Quantity(r))
                if resolved
                    .get(*l)
                    .zip(resolved.get(*r))
                    .is_some_and(|(l, r)| !l.has_dimension_of(r)) =>
            {
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
            // FR-153-AC-6 / TC-198 L08 (QSL-57): a `Reference<A>` and a
            // `Reference<B>` denote the same real object when one type
            // conforms to the other, as `lookup<A>(p, rb) = rb` does. The
            // plan compares the two references' full identity whatever
            // their static types, so this admits more pairs and decides
            // none differently.
            (ValueType::Reference(l), ValueType::Reference(r))
                if self.conforms(*l, *r) || self.conforms(*r, *l) =>
            {
                EqualitySchedule::Plan
            }
            (l, r) if l == r => EqualitySchedule::Plan,
            _ => return ill_typed(IllTypedCause::TypeMismatch),
        };
        // SR-511 M2: retain only the compared enum declaration's own
        // members (`left_type`'s `EnumShape`, which schedule selection
        // above already confirmed equals `right_type`'s), not a clone of
        // the whole package's `enum_members` index. Computed before `left`
        // moves into the struct literal below, since `left_type` borrows it.
        let enum_members = match (schedule, left_type) {
            (EqualitySchedule::Enum, ValueType::Enum(shape)) => enum_members(shape),
            _ => EnumMemberIndex::default(),
        };
        Ok(CheckedEquality {
            operator,
            left,
            right,
            schedule,
            units: resolved,
            enum_members,
        })
    }
}

impl CheckedEquality {
    /// The selected schedule.
    pub fn schedule(&self) -> EqualitySchedule {
        self.schedule
    }

    /// Evaluate over the two completed operand values, left conversion first.
    /// Neither source value is changed. For the `Enum` schedule, a bare
    /// kernel `Value::Enum` (ADR-013 O-14/OQ-D) carries only its `VariantId`
    /// and rank, so this resolves each side back to its full declaration/
    /// ordered/case data through the `enum_members` index captured at
    /// checking (ADR-013 T-6, last sentence) before calling [`compare_enum`],
    /// which needs none of the other schedules' declaration or unit data.
    pub fn evaluate(&self, left: &Value, right: &Value, meter: &mut Meter) -> Outcome<bool> {
        outcome_from_stop(self.run(left, right, meter))
    }

    fn run(&self, left: &Value, right: &Value, meter: &mut Meter) -> Result<bool, Stop> {
        let units = UnitScope::new(&self.units);
        let left = operand_value(&self.left, left, &units, meter)?;
        let right = operand_value(&self.right, right, &units, meter)?;
        let operator = self.operator.comparison();
        let scheduled = match (self.schedule, &left, &right) {
            (EqualitySchedule::Text, Value::Text(l), Value::Text(r)) => {
                compare_text(operator, l, r, meter)
            }
            (EqualitySchedule::Enum, Value::Enum(l), Value::Enum(r)) => {
                let (Some(l), Some(r)) = (
                    self.enum_members.resolve(l.variant()),
                    self.enum_members.resolve(r.variant()),
                ) else {
                    return Err(invariant());
                };
                compare_enum(operator, l, r, meter)
            }
            (EqualitySchedule::Quantity, Value::Quantity(l), Value::Quantity(r)) => {
                let (Some(l), Some(r)) = (units.resolve(l), units.resolve(r)) else {
                    return Err(invariant());
                };
                compare_quantity(operator, l, r, meter)
            }
            (EqualitySchedule::Plan, _, _) => {
                let equal = outcome_into_stop(quire_exact::planned_equality(&left, &right, meter))?;
                return Ok(equal == (self.operator == EqualityOperator::Equal));
            }
            (
                EqualitySchedule::Text | EqualitySchedule::Enum | EqualitySchedule::Quantity,
                _,
                _,
            ) => return Err(invariant()),
        };
        outcome_into_stop(scheduled.map_err(|_| invariant())?)
    }
}

fn invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

/// Whether (`source`, `target`) is a row of the closed FR-149
/// equality-conversion table, decided from declared bounds alone and, for a
/// quantity pair, from the units `units` resolves; an unresolved unit admits
/// no conversion.
pub(crate) fn admits_equality_conversion(
    source: &ValueType,
    target: &ValueType,
    units: &UnitScope<'_>,
) -> bool {
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
        (ValueType::Quantity(from), ValueType::Quantity(to)) => units
            .get(*from)
            .zip(units.get(*to))
            .is_some_and(|(from, to)| from.converts_to(to)),
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

/// The comparison value of one operand after its admitted conversion, with
/// quantity units read from `units`.
pub fn operand_value(
    operand: &EqualityOperand,
    value: &Value,
    units: &UnitScope<'_>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    // A `Reference<T>` operand's value carries its object's most specific
    // type, which is `T` or a type conforming to it (`lookup<T>` returns
    // the object as found): `ValueType::admits`'s exact type match would
    // refuse a real upcast, so a reference operand is checked by kind only.
    // The checker already proved the static types related.
    // The checker guarantees the value's type conforms to `T`; the plan then
    // compares only the identity triple, so the kind check is sufficient.
    let admitted = match (&operand.source, value) {
        (ValueType::Reference(_), value) => matches!(value, Value::Reference(_)),
        (source, value) => source.admits(value),
    };
    if !admitted {
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
            let result = outcome_into_stop(evaluate_decimal(
                DecimalOperation::Round(decimal),
                to,
                meter,
            ))?;
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
            let (Some(source), Some(target)) = (units.resolve(quantity), units.get(*unit)) else {
                return Err(invariant());
            };
            let conversion = outcome_into_stop(
                convert_quantity(source, target, &QuantityTarget::Exact, meter)
                    .map_err(|_| invariant())?,
            )?;
            match conversion.value() {
                ConvertedValue::Exact(exact) => {
                    Value::Quantity(Quantity::new(exact.clone(), *unit))
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
