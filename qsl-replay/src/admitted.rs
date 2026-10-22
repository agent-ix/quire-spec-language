// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facade's admitted-context entry (FR-394, FR-396, FR-398): a compiled
//! unit's emitted package together with the admitted enum, record, tuple,
//! union and unit tables its check retained.
//!
//! ADR-011 T12, FR-087 and Runtime AD-004 forbid a consumer to build a node
//! key. A consumer that needs the key of an enum declaration or case, of a
//! record, tuple or union declaration and a union's member ids, or the kernel
//! id and conversion relation of a declared unit, compiles through
//! [`compile_admitted`] and looks it up in the
//! [`AdmittedContext`] it returns, by the wire node id the emitted package
//! carries. The context holds the keys the checker minted and nothing else,
//! so a lookup never creates a key; it finds one or refuses.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::{Code, SourceIdentity};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckedGraph, CheckedTypeNode, CompositeForm, NominalNode, SemanticGraph,
};
use qsl_semantics::library::PackageId;
use quire_exact::{Identifier, Integer, NodeKey, UnionMember, UnitId, ValueType, VariantId};
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape};
use quire_semantic_value::quantity::{QuantityUnit, UnitTable};
use quire_semantic_value::unit::{NominalUnitForm, NominalUnitNode};

use crate::bounds::ReplayLimits;
use crate::compile::{recompile_unit, CompiledPackage};
use crate::execute::{LimitAboveReader, ReplayRefusal};
use crate::request::StageLimits;
use crate::spine::{CompileRefusal, DependencyInput};

/// One case of an admitted enum: its identifier and the key the checker
/// minted for its member node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEnumCase {
    case: Identifier,
    key: NodeKey,
}

impl AdmittedEnumCase {
    /// The case identifier.
    pub fn case(&self) -> &Identifier {
        &self.case
    }

    /// The admitted key of the case's member node.
    pub fn key(&self) -> NodeKey {
        self.key
    }
}

/// One admitted enum declaration: its qualified name, ordering and key, and
/// every one of its cases, used or not, in FR-141 canonical order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedEnum {
    key: NodeKey,
    qualified_name: Vec<Identifier>,
    ordered: bool,
    cases: Vec<AdmittedEnumCase>,
}

impl AdmittedEnum {
    /// The admitted key of the declaration node.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declaration's qualified name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// Whether the enum is ordered.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// Every case, in canonical order.
    pub fn cases(&self) -> &[AdmittedEnumCase] {
        &self.cases
    }

    /// The case named `case`.
    pub fn case(&self, case: &Identifier) -> Option<&AdmittedEnumCase> {
        self.cases.iter().find(|admitted| admitted.case == *case)
    }
}

/// Which composite an [`AdmittedComposite`] declaration is.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmittedCompositeForm {
    /// A record: named fields.
    Record,
    /// A tuple: positions.
    Tuple,
    /// A union: members, each with a payload.
    Union,
}

/// One member of an admitted union: its identifier, the `VariantId` the kernel
/// union value of it carries, and its payload position types.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedUnionMember {
    binding: UnionMember,
    payload: Vec<ValueType>,
}

impl AdmittedUnionMember {
    /// The member identifier.
    pub fn identifier(&self) -> &Identifier {
        self.binding.identifier()
    }

    /// The member's `VariantId`: the one the check bound to the member
    /// (QSpec FR-441), which a `UnionMember` of a replayed union value of it
    /// carries and a QSV union declaration names the member by.
    pub fn variant(&self) -> VariantId {
        self.binding.variant()
    }

    /// The sealed binding the check minted for the member: its declaration
    /// is the union's [`AdmittedComposite::node`], and it is what
    /// `TypeEnvironment::seal_union_verified(key, node, bindings)` takes and
    /// what `UnionValue::from_admitted` carries. A consumer clones these in
    /// declared order and builds no key.
    pub fn member(&self) -> &UnionMember {
        &self.binding
    }

    /// The member's payload position types in declared order; empty for a
    /// member without payload. A record, tuple or union position is
    /// `ValueType::Composite` of that declaration's kernel key.
    pub fn payload(&self) -> &[ValueType] {
        &self.payload
    }
}

/// One admitted record, tuple or union declaration: the id of its `composite_type`
/// node, which a lookup takes, and its kernel key, the key the package's
/// checked `TypeEnvironment` gives the declaration and which every
/// `ValueType::Composite` of it and `CompositeDeclaration` of it carries;
/// with its qualified name and its form, and for a union its members in
/// declared order.
///
/// For a record or tuple the replayed values carry `key()` too. A union
/// differs: its types use `key()`, but the checked environment settles it
/// under `node()` (`union_key(key()) == node()`), and every `UnionMember` of
/// its values and every seal of it use `node()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedComposite {
    node: NodeKey,
    key: NodeKey,
    qualified_name: Vec<Identifier>,
    shape: AdmittedShape,
}

/// The form of an [`AdmittedComposite`], carrying a union's members so they
/// exist exactly for a union.
#[derive(Clone, Debug, Eq, PartialEq)]
enum AdmittedShape {
    Record,
    Tuple,
    Union(Vec<AdmittedUnionMember>),
}

impl AdmittedComposite {
    /// The id of the declaration's `composite_type` node: the wire node id
    /// [`AdmittedContext::composite_declaration`] finds it by. For a union it
    /// is also the settled key: the `final_key` of `seal_union_verified`, the
    /// declaration of every [`AdmittedUnionMember::member`] and the id a
    /// replay witness names the union by.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The declaration's kernel key: the key `CompositeDeclaration::new` and
    /// `ValueType::Composite` take for it, equal to the one QSL's checked
    /// `TypeEnvironment` and replay use. For a union this is the handle of its
    /// types (the `handle` of `seal_union_verified`); its values and members
    /// carry [`Self::node`].
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The declaration's qualified name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// Whether the declaration is a record, a tuple or a union.
    pub fn form(&self) -> AdmittedCompositeForm {
        match self.shape {
            AdmittedShape::Record => AdmittedCompositeForm::Record,
            AdmittedShape::Tuple => AdmittedCompositeForm::Tuple,
            AdmittedShape::Union(_) => AdmittedCompositeForm::Union,
        }
    }

    /// A union's members in declared order, each with its sealed binding,
    /// `VariantId` and payload; `None` for a record or tuple. A consumer
    /// seals the union with `seal_union_verified(key(), node(), bindings)` over
    /// these bindings. A replay witness names a union by [`Self::node`] and a
    /// member by its identifier.
    pub fn union_members(&self) -> Option<&[AdmittedUnionMember]> {
        match &self.shape {
            AdmittedShape::Union(members) => Some(members),
            AdmittedShape::Record | AdmittedShape::Tuple => None,
        }
    }
}

/// One admitted dimension: the id of its `dimension` node and its qualified
/// name. Dimensions have no kernel id; a unit names its dimension by this
/// node id. The table holds the dimensions an admitted unit names, and the
/// base dimensions those are derived from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedDimension {
    node: NodeKey,
    qualified_name: Vec<Identifier>,
    terms: Vec<(NodeKey, Integer)>,
}

impl AdmittedDimension {
    /// The id of the dimension's node: the wire node id
    /// [`AdmittedContext::dimension_declaration`] finds it by and
    /// [`AdmittedUnit::dimension`] names it by.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The declaration's qualified name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// Whether the dimension is a base dimension (`dimension Length;`)
    /// rather than one derived from base dimensions.
    pub fn is_base(&self) -> bool {
        self.terms.is_empty()
    }

    /// The base dimensions a derived dimension is made of, each as its node
    /// id and nonzero exponent, ascending by node id; empty for a base
    /// dimension.
    pub fn terms(&self) -> &[(NodeKey, Integer)] {
        &self.terms
    }
}

/// One admitted declared unit: the id of its `unit` node, which a lookup
/// takes, its kernel [`UnitId`], the id every `ValueType::Quantity` of it,
/// every quantity value of it and QSL's own checked `Quantity<u>` carry;
/// with its qualified name, its dimension node and its [`QuantityUnit`].
///
/// The node id and the kernel id are different values. The kernel id is
/// `UnitId::declared` of the node id, a digest under the
/// `quire.checked-semantic-node/v1` label, so the node id's bytes are not the
/// kernel id's bytes: a consumer reads this id and never derives one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedUnit {
    node: NodeKey,
    id: UnitId,
    qualified_name: Vec<Identifier>,
    dimension: NodeKey,
    unit: QuantityUnit,
}

impl AdmittedUnit {
    /// The id of the unit's node: the wire node id
    /// [`AdmittedContext::unit_declaration`] finds it by.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The unit's kernel id: the id `ValueType::Quantity` and a kernel
    /// `Quantity` take for it, equal to the one QSL's checked type
    /// environment and replay use.
    pub fn id(&self) -> UnitId {
        self.id
    }

    /// The declaration's qualified name.
    pub fn qualified_name(&self) -> &[Identifier] {
        &self.qualified_name
    }

    /// The node id of the unit's dimension ([`AdmittedDimension::node`]).
    pub fn dimension(&self) -> NodeKey {
        self.dimension
    }

    /// The unit as the kernel's quantity operations take it: its normalized
    /// dimension, canonical root and exact path to it. The `unit` argument
    /// of `quire_semantic_value::quantity::convert_quantity`.
    pub fn quantity_unit(&self) -> &QuantityUnit {
        &self.unit
    }

    /// Whether a quantity of this unit converts to `target`: the kernel's
    /// `converts_to` relation, true exactly when the two units have the same
    /// normalized dimension.
    pub fn converts_to(&self, target: &Self) -> bool {
        self.unit.converts_to(&target.unit)
    }
}

/// Why an [`AdmittedContext`] lookup found no key.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum AdmittedLookupRefusal {
    /// The declaration id names no declaration of the looked-up kind (an
    /// enum for the enum lookups, a record, tuple or union for
    /// [`AdmittedContext::composite_declaration`]) of this package or of any
    /// package it depends on, or the enum declaration has no such case.
    #[error("unknown_declaration: {}", unknown_what(.declaration, .case))]
    Unknown {
        /// The declaration id looked up.
        declaration: WireNodeId,
        /// The case looked up, when the lookup named one and the
        /// declaration exists.
        case: Option<Identifier>,
    },
    /// The declaration id names a declaration of the looked-up kind that a
    /// dependency of this package admitted, not this package. Its keys are
    /// that package's.
    #[error("foreign_declaration: declaration {declaration} is admitted by dependency package {}", .package.hex())]
    Foreign {
        /// The declaration id looked up.
        declaration: WireNodeId,
        /// The dependency package that admitted it.
        package: PackageId,
    },
    /// The declaration id names a node this package admitted that is not a
    /// declaration of the looked-up kind (for a record, tuple or union lookup
    /// an enum, a unit, a scalar type, ...; for a unit lookup a dimension, a
    /// record, ...). Only [`AdmittedContext::composite_declaration`],
    /// [`AdmittedContext::unit_declaration`] and
    /// [`AdmittedContext::dimension_declaration`] refuse so; the enum
    /// lookups never do, so an id that is a unit is `Unknown` to
    /// [`AdmittedContext::enum_declaration`].
    #[error("wrong_kind: node {declaration} is admitted by this package but is not a declaration of the looked-up kind")]
    WrongKind {
        /// The declaration id looked up.
        declaration: WireNodeId,
    },
}

fn unknown_what(declaration: &WireNodeId, case: &Option<Identifier>) -> String {
    match case {
        None => format!("no admitted declaration {declaration} of the looked-up kind"),
        Some(case) => format!(
            "admitted enum declaration {declaration} has no case {}",
            case.as_str()
        ),
    }
}

/// The package context a compile admitted: this package's enum, record,
/// tuple, union, dimension and unit tables and the identity of every enum,
/// record, tuple, union, dimension and unit a dependency admitted.
#[derive(Clone, Debug)]
pub struct AdmittedContext {
    package: PackageId,
    enums: BTreeMap<[u8; 32], AdmittedEnum>,
    foreign: BTreeMap<[u8; 32], PackageId>,
    composites: BTreeMap<[u8; 32], AdmittedComposite>,
    foreign_composites: BTreeMap<[u8; 32], PackageId>,
    units: BTreeMap<[u8; 32], AdmittedUnit>,
    foreign_units: BTreeMap<[u8; 32], PackageId>,
    dimensions: BTreeMap<[u8; 32], AdmittedDimension>,
    foreign_dimensions: BTreeMap<[u8; 32], PackageId>,
    unit_table: UnitTable,
    /// Every node of this package's checked graph, and every declared unit
    /// and dimension (a unit enters the graph only when the package uses
    /// it): the ids a composite, unit or dimension lookup refuses as the
    /// wrong kind when it is not of the looked-up kind. One 32-byte id per
    /// node, so the context grows with the package's graph (FR-396, FR-398).
    nodes: BTreeSet<[u8; 32]>,
}

impl AdmittedContext {
    /// The `package_id` of the package whose check admitted this context.
    pub fn package_id(&self) -> PackageId {
        self.package
    }

    /// Every enum declaration this package admitted, ascending by key.
    pub fn enums(&self) -> impl Iterator<Item = &AdmittedEnum> {
        self.enums.values()
    }

    /// The enum declaration the wire node id `declaration` names.
    ///
    /// # Errors
    ///
    /// [`AdmittedLookupRefusal::Foreign`] when a dependency admitted it,
    /// [`AdmittedLookupRefusal::Unknown`] when no package of the closure did.
    pub fn enum_declaration(
        &self,
        declaration: WireNodeId,
    ) -> Result<&AdmittedEnum, AdmittedLookupRefusal> {
        let id = *declaration.as_bytes();
        if let Some(admitted) = self.enums.get(&id) {
            return Ok(admitted);
        }
        Err(match self.foreign.get(&id) {
            Some(&package) => AdmittedLookupRefusal::Foreign {
                declaration,
                package,
            },
            None => AdmittedLookupRefusal::Unknown {
                declaration,
                case: None,
            },
        })
    }

    /// The case `case` of the enum declaration `declaration` names.
    ///
    /// # Errors
    ///
    /// As [`Self::enum_declaration`]; and [`AdmittedLookupRefusal::Unknown`]
    /// naming the case when the declaration has no such case.
    pub fn enum_case(
        &self,
        declaration: WireNodeId,
        case: &Identifier,
    ) -> Result<&AdmittedEnumCase, AdmittedLookupRefusal> {
        self.enum_declaration(declaration)?
            .case(case)
            .ok_or_else(|| AdmittedLookupRefusal::Unknown {
                declaration,
                case: Some(case.clone()),
            })
    }

    /// Every record, tuple and union declaration this package admitted,
    /// ascending by `composite_type` node id.
    pub fn composites(&self) -> impl Iterator<Item = &AdmittedComposite> {
        self.composites.values()
    }

    /// The record, tuple or union declaration the wire node id `declaration`
    /// names.
    ///
    /// # Errors
    ///
    /// [`AdmittedLookupRefusal::Foreign`] when a dependency admitted it as a
    /// record, tuple or union, [`AdmittedLookupRefusal::WrongKind`] when it names
    /// another node of this package, and [`AdmittedLookupRefusal::Unknown`]
    /// otherwise.
    pub fn composite_declaration(
        &self,
        declaration: WireNodeId,
    ) -> Result<&AdmittedComposite, AdmittedLookupRefusal> {
        let id = *declaration.as_bytes();
        if let Some(admitted) = self.composites.get(&id) {
            return Ok(admitted);
        }
        Err(self.refusal(declaration, &self.foreign_composites))
    }

    /// Every unit this package admitted, ascending by node id.
    pub fn units(&self) -> impl Iterator<Item = &AdmittedUnit> {
        self.units.values()
    }

    /// The declared unit the wire node id `declaration` names: its kernel
    /// [`UnitId`], dimension and conversion data (FR-398).
    ///
    /// # Errors
    ///
    /// [`AdmittedLookupRefusal::Foreign`] when a dependency admitted it as a
    /// unit, [`AdmittedLookupRefusal::WrongKind`] when it names another node
    /// of this package (a dimension, a record, ...), and
    /// [`AdmittedLookupRefusal::Unknown`] otherwise.
    pub fn unit_declaration(
        &self,
        declaration: WireNodeId,
    ) -> Result<&AdmittedUnit, AdmittedLookupRefusal> {
        self.units
            .get(declaration.as_bytes())
            .ok_or_else(|| self.refusal(declaration, &self.foreign_units))
    }

    /// Every dimension an admitted unit of this package names, and the base
    /// dimensions those derive from, ascending by node id.
    pub fn dimensions(&self) -> impl Iterator<Item = &AdmittedDimension> {
        self.dimensions.values()
    }

    /// The dimension the wire node id `declaration` names.
    ///
    /// # Errors
    ///
    /// As [`Self::unit_declaration`], for a dimension.
    pub fn dimension_declaration(
        &self,
        declaration: WireNodeId,
    ) -> Result<&AdmittedDimension, AdmittedLookupRefusal> {
        self.dimensions
            .get(declaration.as_bytes())
            .ok_or_else(|| self.refusal(declaration, &self.foreign_dimensions))
    }

    /// The package's unit table: each admitted unit by its kernel id, the
    /// table `quire_semantic_value::quantity::UnitTable::resolve` reads a
    /// quantity's unit from before `convert_quantity` converts it.
    pub fn unit_table(&self) -> &UnitTable {
        &self.unit_table
    }

    /// The refusal for a declaration id none of the looked-up kind's table
    /// holds: `Foreign` when a dependency admitted it (`foreign`),
    /// `WrongKind` when it names another node of this package, else
    /// `Unknown`.
    fn refusal(
        &self,
        declaration: WireNodeId,
        foreign: &BTreeMap<[u8; 32], PackageId>,
    ) -> AdmittedLookupRefusal {
        let id = declaration.as_bytes();
        if let Some(&package) = foreign.get(id) {
            AdmittedLookupRefusal::Foreign {
                declaration,
                package,
            }
        } else if self.nodes.contains(id) {
            AdmittedLookupRefusal::WrongKind { declaration }
        } else {
            AdmittedLookupRefusal::Unknown {
                declaration,
                case: None,
            }
        }
    }
}

/// A unit compiled and emitted, with the context its check admitted.
#[derive(Clone, Debug)]
pub struct AdmittedCompilation {
    package: CompiledPackage,
    context: AdmittedContext,
}

impl AdmittedCompilation {
    /// The emitted package.
    pub fn package(&self) -> &CompiledPackage {
        &self.package
    }

    /// The context the unit's check admitted.
    pub fn context(&self) -> &AdmittedContext {
        &self.context
    }

    /// The emitted package and the admitted context.
    pub fn into_parts(self) -> (CompiledPackage, AdmittedContext) {
        (self.package, self.context)
    }
}

/// A bound that stopped [`compile_admitted`].
#[derive(Debug, thiserror::Error)]
pub enum AdmittedResource {
    /// A stage limit the compile reached (`stage_limit_exceeded`).
    #[error("{0}")]
    StageLimit(Box<CompileRefusal>),
    /// The S1 limit asked for is above the reader limit.
    #[error("invalid-request: the `s1.input_bytes` limit {} is above the reader limit `replay.input_bytes` {}", .0.requested, .0.reader)]
    AboveReader(LimitAboveReader),
}

/// Why [`compile_admitted`] returned no context.
#[derive(Debug, thiserror::Error)]
pub enum AdmittedRefusal {
    /// The unit, a library or the dependency input does not compile to a
    /// package.
    #[error("the unit does not compile to a package: {0}")]
    Package(Box<CompileRefusal>),
    /// A bound stopped the compile.
    #[error(transparent)]
    Resource(AdmittedResource),
    /// A QSL invariant the compile relies on did not hold.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl From<ReplayRefusal> for AdmittedRefusal {
    fn from(refusal: ReplayRefusal) -> Self {
        match refusal {
            ReplayRefusal::Recompile(refusal) => classify(*refusal),
            ReplayRefusal::DependencyInput(refusal) => {
                classify(CompileRefusal::DependencyInput(refusal))
            }
            ReplayRefusal::LimitAboveReader(limit) => {
                Self::Resource(AdmittedResource::AboveReader(limit))
            }
            ReplayRefusal::Fault(fault) => Self::Fault(fault),
            // The compile path raises no replay-only refusal; one here is a
            // broken invariant, not a caller error. Each is named so that a
            // new variant forces a decision.
            ReplayRefusal::Request(_)
            | ReplayRefusal::NotASource(_)
            | ReplayRefusal::SourceCount(_)
            | ReplayRefusal::DependencySelections(_)
            | ReplayRefusal::DependencyIdentityMismatch { .. }
            | ReplayRefusal::PackageIdMismatch { .. }
            | ReplayRefusal::UnknownFunction { .. }
            | ReplayRefusal::UnknownParameter(_)
            | ReplayRefusal::DuplicateArgument(_)
            | ReplayRefusal::UnboundParameter(_)
            | ReplayRefusal::Witness { .. }
            | ReplayRefusal::Input(_)
            | ReplayRefusal::NativePayload { .. }
            | ReplayRefusal::Storage { .. }
            | ReplayRefusal::Supplied(_)
            | ReplayRefusal::UnsupportedHelper(_)
            | ReplayRefusal::Cancelled(_)
            | ReplayRefusal::NotAPredicate { .. }
            | ReplayRefusal::NotAValueFunction { .. }
            | ReplayRefusal::ScalarIdentity(_)
            | ReplayRefusal::ParityBound(_)
            | ReplayRefusal::UnknownOperation { .. }
            | ReplayRefusal::WrongReplayTarget { .. }
            | ReplayRefusal::TargetMismatch { .. }
            | ReplayRefusal::MalformedOperationTarget { .. }
            | ReplayRefusal::AmbiguousOperation { .. }
            | ReplayRefusal::FrameIdentity(_)
            | ReplayRefusal::UnknownClause { .. }
            | ReplayRefusal::ClauseIdentity(_)
            | ReplayRefusal::WrongObservation { .. }
            | ReplayRefusal::Admission(_)
            | ReplayRefusal::UnusedObjectEnvironment
            | ReplayRefusal::ForeignReference { .. }
            | ReplayRefusal::ObjectEnvironment(_)
            | ReplayRefusal::ModelTrace(_)
            | ReplayRefusal::UnfairLasso { .. } => Self::Fault(InternalFault::new(
                "replay",
                "compile-refuses-only-compile-refusals",
            )),
        }
    }
}

fn classify(refusal: CompileRefusal) -> AdmittedRefusal {
    if matches!(
        refusal.code(),
        Code::StageLimitExceeded | Code::ResourceExhausted
    ) {
        AdmittedRefusal::Resource(AdmittedResource::StageLimit(Box::new(refusal)))
    } else {
        AdmittedRefusal::Package(Box::new(refusal))
    }
}

/// Compile `bytes` as [`crate::compile_package`] does and return the emitted
/// package together with the [`AdmittedContext`] its check admitted, the
/// package-bound enum key table a contract generator looks keys up in
/// instead of building them (FR-394).
///
/// The arguments and limits are [`crate::compile_package`]'s.
///
/// # Errors
///
/// [`AdmittedRefusal::Package`] for a unit, library or dependency input that
/// does not compile, [`AdmittedRefusal::Resource`] for a stage limit reached
/// or an S1 limit above the reader limit, and [`AdmittedRefusal::Fault`] for
/// a broken QSL invariant.
pub fn compile_admitted<'a>(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: impl IntoIterator<Item = &'a [u8]>,
    dependencies: &DependencyInput,
    limits: StageLimits,
    replay_limits: ReplayLimits,
) -> Result<AdmittedCompilation, AdmittedRefusal> {
    let compiled = recompile_unit(
        source,
        path,
        bytes,
        packages,
        dependencies,
        limits,
        replay_limits,
    )?;
    let context = context_of(
        compiled.checked.package(),
        compiled.emitted.package().package_id(),
    )
    .map_err(AdmittedRefusal::Fault)?;
    Ok(AdmittedCompilation {
        package: CompiledPackage::new(compiled.emitted),
        context,
    })
}

fn context_of(package: &CheckedPackage, id: PackageId) -> Result<AdmittedContext, InternalFault> {
    let enums = enum_table(package.graph().semantic_graph())?;
    let composites = composite_table(package.graph())?;
    let UnitTables {
        units,
        dimensions,
        table,
    } = unit_tables(package.graph())?;
    let nodes = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| *node.key().as_bytes())
        .chain(units.keys().copied())
        .chain(dimensions.keys().copied())
        .collect();
    let mut foreign = BTreeMap::new();
    let mut foreign_composites = BTreeMap::new();
    let mut foreign_units = BTreeMap::new();
    let mut foreign_dimensions = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut pending: Vec<_> = package.dependencies().iter().collect();
    while let Some((&dependency_id, dependency)) = pending.pop() {
        if !visited.insert(dependency_id) {
            continue;
        }
        for declaration in enum_table(dependency.graph().semantic_graph())?.into_keys() {
            foreign.entry(declaration).or_insert(dependency_id);
        }
        for declaration in composite_table(dependency.graph())?.into_keys() {
            foreign_composites
                .entry(declaration)
                .or_insert(dependency_id);
        }
        let (unit_keys, dimension_keys) = unit_keys(dependency.graph())?;
        for declaration in unit_keys {
            foreign_units.entry(declaration).or_insert(dependency_id);
        }
        for declaration in dimension_keys {
            foreign_dimensions
                .entry(declaration)
                .or_insert(dependency_id);
        }
        pending.extend(dependency.dependencies().iter());
    }
    Ok(AdmittedContext {
        package: id,
        enums,
        foreign,
        composites,
        foreign_composites,
        units,
        foreign_units,
        dimensions,
        foreign_dimensions,
        unit_table: table,
        nodes,
    })
}

/// A graph's admitted units and dimensions, and the unit table they came
/// from.
struct UnitTables {
    units: BTreeMap<[u8; 32], AdmittedUnit>,
    dimensions: BTreeMap<[u8; 32], AdmittedDimension>,
    table: UnitTable,
}

fn qualified_name(segments: &[String]) -> Result<Vec<Identifier>, InternalFault> {
    segments
        .iter()
        .map(|segment| {
            Identifier::new(segment)
                .map_err(|_| InternalFault::new("replay", "admitted-unit-names-are-identifiers"))
        })
        .collect()
}

/// The declared units of `table` and the dimensions they name, walked
/// through the table's nominal nodes to their base dimensions: every unit
/// node, and every dimension node with its nominal terms.
fn declared_nodes(
    table: &UnitTable,
) -> Result<
    (
        Vec<&quire_semantic_value::unit::Unit>,
        Vec<&NominalUnitNode>,
    ),
    InternalFault,
> {
    let named = |key: NodeKey| {
        table
            .nominal_node(key.as_bytes())
            .ok_or_else(|| InternalFault::new("replay", "every-admitted-unit-has-a-nominal-node"))
    };
    let mut units = Vec::new();
    let mut pending = Vec::new();
    for id in table.ids() {
        if let Some(QuantityUnit::Declared(unit)) = table.get(id) {
            pending.push(unit.dimension_node());
            units.push(&**unit);
        }
    }
    let mut seen = BTreeSet::new();
    let mut dimensions = Vec::new();
    while let Some(key) = pending.pop() {
        if !seen.insert(*key.as_bytes()) {
            continue;
        }
        let nominal = named(key)?;
        let NominalUnitForm::Dimension { terms } = &nominal.form else {
            return Err(InternalFault::new(
                "replay",
                "every-admitted-unit-dimension-is-a-dimension",
            ));
        };
        pending.extend(terms.iter().map(|(base, _)| *base));
        dimensions.push(nominal);
    }
    Ok((units, dimensions))
}

/// The unit and dimension node ids of a graph.
type DeclaredKeys = (Vec<[u8; 32]>, Vec<[u8; 32]>);

/// The node ids of `graph`'s declared units and the dimensions they name,
/// for a dependency's `Foreign` refusals: no names, no unit values.
fn unit_keys(graph: &CheckedGraph) -> Result<DeclaredKeys, InternalFault> {
    let (units, dimensions) = declared_nodes(graph.scope().types().units())?;
    Ok((
        units.iter().map(|unit| *unit.key().as_bytes()).collect(),
        dimensions
            .iter()
            .map(|dimension| *dimension.key.as_bytes())
            .collect(),
    ))
}

/// Every declared unit of `graph` and the dimensions they name: the units
/// from the unit table the checked type environment holds (the table the
/// checker resolved `Quantity<u>` against, so each [`AdmittedUnit::id`] is the
/// id the checked signatures carry).
fn unit_tables(graph: &CheckedGraph) -> Result<UnitTables, InternalFault> {
    let table = graph.scope().types().units().clone();
    let (declared, named_dimensions) = declared_nodes(&table)?;
    let mut units = BTreeMap::new();
    for unit in declared {
        let nominal = table.nominal_node(unit.key().as_bytes()).ok_or_else(|| {
            InternalFault::new("replay", "every-admitted-unit-has-a-nominal-node")
        })?;
        units.insert(
            *unit.key().as_bytes(),
            AdmittedUnit {
                node: unit.key(),
                id: unit.id(),
                qualified_name: qualified_name(&nominal.declaration.qualified_declaration)?,
                dimension: unit.dimension_node(),
                unit: QuantityUnit::Declared(Box::new(unit.clone())),
            },
        );
    }
    let mut dimensions = BTreeMap::new();
    for nominal in named_dimensions {
        let NominalUnitForm::Dimension { terms } = &nominal.form else {
            continue;
        };
        dimensions.insert(
            *nominal.key.as_bytes(),
            AdmittedDimension {
                node: nominal.key,
                qualified_name: qualified_name(&nominal.declaration.qualified_declaration)?,
                terms: terms.clone(),
            },
        );
    }
    Ok(UnitTables {
        units,
        dimensions,
        table,
    })
}

/// Every record, tuple and union declaration of `graph`, keyed by its
/// `composite_type` node's id bytes, from the checked type nodes the check
/// minted for the package's admitted composites. A union's members are read
/// from the sealed declaration the checked `TypeEnvironment` holds under the
/// kernel key.
fn composite_table(
    graph: &CheckedGraph,
) -> Result<BTreeMap<[u8; 32], AdmittedComposite>, InternalFault> {
    let mut composites = BTreeMap::new();
    for type_node in graph.checked_type_nodes() {
        let (node, key, shape) = match type_node {
            CheckedTypeNode::Composite {
                node,
                declaration,
                form,
            } => match form {
                CompositeForm::Record => (*node, *declaration, AdmittedShape::Record),
                CompositeForm::Tuple => (*node, *declaration, AdmittedShape::Tuple),
                CompositeForm::Union => (
                    *node,
                    *declaration,
                    AdmittedShape::Union(union_members(graph, *declaration)?),
                ),
            },
            CheckedTypeNode::Scalar { .. }
            | CheckedTypeNode::BoundedDomain { .. }
            | CheckedTypeNode::Sum { .. } => continue,
        };
        let qualified_name = graph
            .semantic_graph()
            .node(node)
            .and_then(|declared| declared.declaration())
            .ok_or_else(|| {
                InternalFault::new("replay", "every-admitted-composite-has-a-declared-node")
            })?
            .to_vec();
        composites.insert(
            *node.as_bytes(),
            AdmittedComposite {
                node,
                key,
                qualified_name,
                shape,
            },
        );
    }
    Ok(composites)
}

/// The members of the sealed union the checked `TypeEnvironment` holds under
/// `key`, each with the `VariantId` of the binding the check sealed it with.
fn union_members(
    graph: &CheckedGraph,
    key: NodeKey,
) -> Result<Vec<AdmittedUnionMember>, InternalFault> {
    let Some(CompositeShape::Union(declared)) = graph
        .scope()
        .types()
        .composite(key)
        .map(CompositeDeclaration::shape)
    else {
        return Err(InternalFault::new(
            "replay",
            "every-admitted-union-is-in-the-type-environment",
        ));
    };
    declared
        .iter()
        .map(|member| {
            let binding = member
                .member()
                .ok_or_else(|| InternalFault::new("replay", "every-admitted-union-is-sealed"))?;
            Ok(AdmittedUnionMember {
                binding: binding.clone(),
                payload: member.positions().to_vec(),
            })
        })
        .collect()
}

/// Every enum declaration of `graph`, keyed by the declaration node's id
/// bytes, with each case's member key read from the member node the checker
/// built for it.
fn enum_table(graph: &SemanticGraph) -> Result<BTreeMap<[u8; 32], AdmittedEnum>, InternalFault> {
    let mut members = BTreeMap::new();
    for node in graph.nodes() {
        if let Some(NominalNode::EnumMember { declaration, case }) = node.nominal() {
            members.insert((*declaration, case.as_str()), node.key());
        }
    }
    let mut enums = BTreeMap::new();
    for node in graph.nodes() {
        let Some(NominalNode::EnumDeclaration(preimage)) = node.nominal() else {
            continue;
        };
        let identifier = |text: &str| {
            Identifier::new(text)
                .map_err(|_| InternalFault::new("replay", "admitted-enum-names-are-identifiers"))
        };
        let cases = preimage
            .members()
            .iter()
            .map(|case| {
                let key = members
                    .get(&(node.key(), case.as_str()))
                    .copied()
                    .ok_or_else(|| {
                        InternalFault::new("replay", "every-admitted-enum-case-has-a-member-node")
                    })?;
                Ok(AdmittedEnumCase {
                    case: identifier(case)?,
                    key,
                })
            })
            .collect::<Result<Vec<_>, InternalFault>>()?;
        let qualified_name = preimage
            .qualified_declaration()
            .iter()
            .map(|name| identifier(name))
            .collect::<Result<Vec<_>, _>>()?;
        enums.insert(
            *node.key().as_bytes(),
            AdmittedEnum {
                key: node.key(),
                qualified_name,
                ordered: preimage.is_ordered(),
                cases,
            },
        );
    }
    Ok(enums)
}
