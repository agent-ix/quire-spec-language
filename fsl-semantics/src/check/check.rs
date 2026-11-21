// SPDX-License-Identifier: AGPL-3.0-or-later
//! Name resolution and typing of Complete-V1 value expressions and function
//! declarations into the typed tree, before any definedness or termination
//! judgment.
//!
//! Every refusal here is made before any charge. Typing is bounded by the
//! declared [`CheckingLimits`] node ceiling, not by nesting depth (ADR-030
//! D-1); reaching a declared limit is `stage_limit_exceeded`, never an
//! admission verdict. The typer itself (`typing`) walks an expression over
//! an explicit heap stack whose frames the node ceiling charges, so its host
//! stack use does not grow with nesting.
//!
//! FR-065 (owner ruling, carried from the QSL-25 spec review): this module
//! retains `infer_form`'s dispatch over [`Expression`] for every `Value`
//! form. **Function-application checking is moved out of this module.**
//! `Self::call` -- the method `infer_form`'s `Expression::Call` arm used to
//! dispatch to, doing the real name resolution, arity check and
//! per-argument typing -- is deleted; its logic now lives in
//! [`super::family::Application`], reached through this module's own
//! `Typer::scope`/`Typer::signatures`/`Typer::type_named`/`Typer::check_as`
//! accessors (widened from private to `pub(crate)` for exactly this one
//! caller). `infer_form`'s `Expression::Call` arm makes one call into that
//! function and holds no semantic logic of its own (FR-065-CON-3). This
//! module still owns function-declaration *typing*'s underlying engine
//! (`bind_parameters`, `check_declared_type`, `check_as`), because a
//! function body is an arbitrary `Expression` and checking one still needs
//! the same general typer every other `Value` form uses --
//! FR-065-CON-1 forbids reimplementing that engine a second time inside the
//! family module, not calling into this module's existing one. What moved
//! is the *entry point*: `check::family::check_declaration_body` (not this
//! module's own `PackageDeclarations::check`) now constructs the `Typer`
//! and drives it for a function declaration's body and measure, so this
//! module's `check.rs` no longer independently decides whether a
//! declaration or a call is admitted. See `check::family`'s own doc for the
//! full account, and for why termination checking (a whole-package call-graph
//! analysis, not a per-declaration one) cannot move the same way.
//! `infer_form` itself carries `#[deny(clippy::wildcard_enum_match_arm)]`
//! (see its own doc) rather than this whole module: the module also holds
//! several pre-existing, unrelated wildcard arms over *other* enums
//! (`ValueType`, `BinaryOperator`) in small type-eligibility helpers that
//! predate this migration and are outside FR-065-CON-1's scope ("no
//! internal representation of any family other than `Value`'s
//! function-declaration and function-application forms") to rework under
//! this ticket. The denial is pointed at the one seam the review actually
//! flagged.

use filament_checked::node::{
    Arithmetic, CheckedLiteral, Connective, ConnectiveOrigin, NodeKind, OrderedKind, RecordField,
    RecordSlot, Visit,
};
use fsl_foundation::Span;
use fsl_foundation::source::node_table::{
    DebugTable, DebugWith, NodeKeyDebug, NodeList, standalone,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::ControlFlow;
use std::sync::Arc;

#[cfg(test)]
mod declared_type_tests;
mod integer_division_range;
mod modulo_interval;
mod native_literal;
mod typing;
use typing::TypedOperand;

use super::ir::{BodyBuilder, DispatchTable, EqualityProducerData, Node, NodeId, Slot};
use super::model_spelling::{ModelSpellings, Spelled};
use super::navigation::Relationships;
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, IdentityForm, IdentityNode,
    IdentityObserving, KeyFault, Obligation, ReservedSlotName, SlotTag, StageLimitCause,
};
use crate::model::operation::OperationTable;
use crate::value::definition::{AdmittedIeeeProfile, DefinitionRole, admit_ieee_profile};
use crate::value::enumeration::{AdmittedEnumDeclaration, mint_variant_id};
use filament_checked::key::DeclarationKey;
use filament_exact::Cancel;
use filament_exact::EffectiveId;
use filament_exact::EnumMember;
use filament_exact::EnumShape;
use filament_exact::IllTypedCause;
use filament_exact::Presence;
use filament_exact::Rational;
use filament_exact::{ArithmeticOperator, OrderingOperator};
use filament_exact::{CardinalityBound, CollectionKind, CollectionType, Integer};
use filament_exact::{Decimal, DecimalType};
use filament_exact::{NodeKey, VariantId};
use filament_exact::{Value, ValueType};
use filament_semantic_value::absence::AbsenceMode;
use filament_semantic_value::checking::CheckingLimits;
use filament_semantic_value::declaration::{
    CompositeShape, EqualityOperand, EqualityOperator, FieldDeclaration, TypeEnvironment,
    admits_equality_conversion,
};
use filament_semantic_value::enumeration::{EnumDeclaration, EnumMemberIndex, EnumValue};
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use filament_semantic_value::quantity::UnitTable;
use filament_semantic_value::quantity::{
    IdentifiedUnit, QuantityOperation, QuantityRefusal, QuantityUnit, UnitOperation, UnitScope,
    check_comparable, evaluate_quantity_unit, result_unit,
};
use fsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, ExprId, ExprNode, ExprRef,
    FieldInitializer, FunctionDeclaration,
};
use fsl_foundation::source::provenance::RawSourceRef;

/// An enum declaration bound to its source name, with its admitted members.
/// A member `m` is named `name::m`.
#[derive(Clone, Debug)]
pub struct EnumBinding {
    /// The declared source name.
    pub name: String,
    /// The admitted declaration.
    pub declaration: AdmittedEnumDeclaration,
    /// Its admitted members, in FR-141 canonical order (declaration order
    /// for an ordered enum, case-identifier byte order otherwise --
    /// `AdmittedEnumDeclaration::admit` already refuses any other order).
    pub members: Vec<EnumValue>,
}

impl EnumBinding {
    /// This binding's kernel `ValueType::Enum` shape (ADR-013 O-14): the
    /// FR-141 canonical, ranked member list. `self.members` is already in
    /// that order, so each member's rank is exactly its index here.
    pub fn shape(&self) -> EnumShape {
        EnumShape::new(
            self.declaration.declaration().is_ordered(),
            self.members.iter().map(EnumValue::variant),
        )
    }
}

/// A function's resolved signature: its parameters' names paired with each
/// one's resolved `ValueType`, and the resolved result type.
pub(crate) type ResolvedSignature = (Vec<(String, ValueType)>, ValueType);

/// Resolved signatures that stand in for resolving some `functions`
/// entries' own `TypeForm`s, keyed by index into
/// [`PackageDeclarations::functions`]. Only `check` can populate it:
/// `check::checked_dispatch`'s FR-151 synthesized clauses come from an
/// already-resolved model signature, and the FR-091 assembler records the
/// check-owned signature it resolved from each source function's type
/// forms. Every other caller holds the empty default.
#[derive(Clone, Debug, Default)]
pub struct ResolvedSignatures(std::collections::BTreeMap<usize, ResolvedSignature>);

impl ResolvedSignatures {
    pub(crate) fn insert(&mut self, index: usize, signature: ResolvedSignature) {
        self.0.insert(index, signature);
    }

    pub(crate) fn get(&self, index: usize) -> Option<&ResolvedSignature> {
        self.0.get(&index)
    }

    /// The first index at or past `function_count`, if any: an entry naming
    /// no function.
    pub(crate) fn first_out_of_range(&self, function_count: usize) -> Option<usize> {
        self.0
            .range(function_count..)
            .next()
            .map(|(index, _)| *index)
    }
}

/// The runtime members of a model variant type whose checked node is
/// `declaration`, for `cases` in ascending UTF-8 byte order of their names
/// (FSpec FR-144's key order for an unordered enumeration). Each member is
/// keyed by [`mint_variant_id`] over the declaration and its case, and a case
/// name is any non-empty string, kept exactly. `None` when `cases` is empty,
/// repeats a name or has an empty name, which the model's intake refuses
/// before a variant type is declared.
fn variant_type_members(declaration: NodeKey, cases: &[String]) -> Option<Vec<EnumValue>> {
    let runtime = EnumDeclaration::model_owned(declaration.clone(), cases.to_vec()).ok()?;
    cases
        .iter()
        .map(|case| {
            let member = declared_member_key(&declaration, case).ok()?;
            runtime.member(case, member).ok()
        })
        .collect()
}

/// The member's key (FR-322, a derived node): `["value", "enum_value",
/// <declaration node>, <case>]` under the declaration's package, which is
/// `EnumMemberPreimage::key`.
fn declared_member_key(
    declaration: &NodeKey,
    case: &str,
) -> Result<NodeKey, filament_exact::InvalidNodeKey> {
    crate::value::enumeration::enum_value_key(declaration, case)
}

/// A model variant type in the type environment (FR-056 "Variant types",
/// FR-081, FR-360 item 11): the effective type the normalized view gives a
/// `filament.meaning.model.variant-type/v1` declaration, named `M::<artifact
/// id>`, with its cases. One declaration holds both keys of the type and of
/// each case: the FR-081 effective identity and the checked node key, so
/// neither is derived from the other and neither depends on the alias a unit
/// gives the model.
#[derive(Clone, Eq, PartialEq)]
pub struct VariantTypeDeclaration {
    key: EffectiveId,
    name: String,
    declaration: DeclarationKey,
    node: NodeKey,
    cases: Vec<VariantCaseDeclaration>,
    members: Vec<EnumValue>,
}

impl DebugWith for VariantTypeDeclaration {
    fn fmt_with(&self, f: &mut std::fmt::Formatter<'_>, table: &DebugTable) -> std::fmt::Result {
        let Self {
            key,
            name,
            declaration,
            node,
            cases,
            members,
        } = self;
        f.debug_struct("VariantTypeDeclaration")
            .field("key", &key)
            .field("name", &name)
            .field("declaration", &declaration)
            .field("node", &NodeKeyDebug(node, table))
            .field("cases", &cases)
            .field("members", &members)
            .finish()
    }
}

impl std::fmt::Debug for VariantTypeDeclaration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        standalone(f, self)
    }
}

impl VariantTypeDeclaration {
    /// The variant type `name` with effective identity `key`, declared by
    /// the model record `declaration` and checked node key `node`; `cases`
    /// are kept in ascending UTF-8 byte order of their names. Each case's
    /// checked key is the FR-141 member identity over `node` and the case
    /// name. `None` when `cases` is not a valid case set of a model-owned
    /// enumeration (see [`variant_type_members`]).
    pub(crate) fn new(
        key: EffectiveId,
        name: String,
        declaration: DeclarationKey,
        node: NodeKey,
        mut cases: Vec<(EffectiveId, String)>,
    ) -> Option<Self> {
        cases.sort_by(|left, right| left.1.cmp(&right.1));
        let names: Vec<String> = cases.iter().map(|(_, case)| case.clone()).collect();
        let members = variant_type_members(node.clone(), &names)?;
        let cases = cases
            .into_iter()
            .map(|(key, case)| {
                let variant = mint_variant_id(node.clone(), &case).ok()?;
                Some(VariantCaseDeclaration {
                    key,
                    name: case,
                    variant,
                })
            })
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            key,
            name,
            declaration,
            node,
            cases,
            members,
        })
    }

    /// The variant type's effective identity.
    pub fn key(&self) -> EffectiveId {
        self.key.clone()
    }

    /// The qualified name `M::<artifact id>`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The model record that declares the variant type.
    pub(crate) fn declaration(&self) -> &DeclarationKey {
        &self.declaration
    }

    /// The variant type's checked node key (FSpec FR-322 "Model-owned
    /// members" step 2), a function of the selected document's identity and
    /// the type node alone.
    pub fn node(&self) -> NodeKey {
        self.node.clone()
    }

    /// Borrow the same validated checked node key as [`Self::node`].
    pub fn node_ref(&self) -> &NodeKey {
        &self.node
    }

    /// The cases, ascending by the UTF-8 bytes of their names.
    pub fn cases(&self) -> &[VariantCaseDeclaration] {
        &self.cases
    }

    /// The kernel enum shape of the type: its cases' checked keys in
    /// ascending UTF-8 byte order of the case names, which is FSpec FR-144's
    /// key order for an unordered enumeration.
    pub fn shape(&self) -> EnumShape {
        EnumShape::new(
            false,
            self.cases.iter().map(VariantCaseDeclaration::variant),
        )
    }

    /// The runtime members, in the order of [`Self::cases`].
    pub(crate) fn members(&self) -> &[EnumValue] {
        &self.members
    }
}

/// One case of a [`VariantTypeDeclaration`]: an effective member of the
/// variant type, keyed by its own effective identity and by its checked key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantCaseDeclaration {
    key: EffectiveId,
    name: String,
    variant: VariantId,
}

impl VariantCaseDeclaration {
    /// The case's effective identity.
    pub fn key(&self) -> EffectiveId {
        self.key.clone()
    }

    /// The case name, compared as exact UTF-8 bytes.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The case's checked key: the FR-141 member identity over the variant
    /// type's checked node and the case name.
    pub fn variant(&self) -> VariantId {
        self.variant.clone()
    }

    /// Borrow the same validated checked member key as [`Self::variant`].
    pub fn variant_ref(&self) -> &VariantId {
        &self.variant
    }
}

/// The staged model type admission, held until the conformance stage has
/// judged every redefinition.
pub struct StagedTypes(pub filament_semantic_value::declaration::PendingTypeEnvironment);

impl std::fmt::Debug for StagedTypes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("StagedTypes")
    }
}

/// The closed declarations of one package that expressions resolve against.
#[derive(Debug)]
pub struct PackageDeclarations {
    /// The staged model admission, completed once the conformance stage has
    /// judged every model redefinition.
    pub pending_types: Option<StagedTypes>,
    /// The models and the meter their admission counted on, which the
    /// conformance stage continues.
    pub model_inputs: Option<(
        Vec<crate::model::intake::SelectedModel>,
        crate::model::accounting::Meter,
    )>,
    /// Exact identity rows minted by model intake, kept alongside this
    /// package's checked declarations for IR projection after admission.
    pub selected_exact_identity_rows: (
        Vec<filament_contract_model::SelectedExactDeclarationRow>,
        Vec<filament_contract_model::SelectedExactFieldRow>,
    ),
    /// Selected tuple declaration rows retained from model intake.
    pub selected_tuple_declaration_rows: Vec<filament_contract_model::SelectedTupleDeclarationRow>,
    /// The declaring source unit's `RawSourceRef` (FR-001), the reference
    /// its caller named and S0 minted. Its authority and identity are the
    /// unit's owner (ADR-013 O-04, FR-091), carried by every declared
    /// record, tuple and function node key (FR-092). The checked graph carries it on for the lock's
    /// `sources` entry. `check` holds no constant owner.
    pub source: fsl_foundation::source::provenance::RawSourceRef,
    /// All admitted source references in canonical owner order.
    pub(crate) sources: Vec<fsl_foundation::source::provenance::RawSourceRef>,
    /// Source owner for each top-level declaration admitted from the
    /// aggregate unit list. A missing entry means the single-source legacy
    /// assembly path, whose owner is `source`.
    pub(crate) source_owners: BTreeMap<String, fsl_foundation::source::provenance::RawSourceRef>,
    /// Source reference for each ordinary function form, by declaration
    /// index, retained for owner-aware occurrence regions.
    pub(crate) function_sources: Vec<fsl_foundation::source::provenance::RawSourceRef>,
    /// The package's name: the package manifest's top-level `name`, required
    /// on every compile or check request (FR-091-OQ-3). It is the `package`
    /// of every source-owned declared or derived node key. A unit label
    /// (`SourceOwner::identity`) is never a package, and nothing here
    /// defaults, falls back or derives it from source text.
    pub package_name: String,
    /// The `protocol_profile` the unit's header selected (FR-110), the law
    /// `DefinitionRef` a protocol node names (FR-093). The assembler sets it
    /// from the header's catalog resolution.
    pub protocol_profile: Option<crate::value::definition::DefinitionReference>,
    /// FR-110: the header-selectable layers the unit's header profiles
    /// resolve to, ascending; the lock's `definition_selections` hold their
    /// `requires` closure.
    pub header_layers: Vec<crate::value::layer::HeaderLayer>,
    /// Authored identity-field names for each checked source object type,
    /// keyed by the source model's declaration key.
    pub(crate) source_object_identity_fields:
        BTreeMap<filament_checked::key::DeclarationKey, Vec<String>>,
    /// Record, tuple and model object-type declarations.
    pub types: TypeEnvironment,
    /// Where each record keeps the fields its ancestors name (FR-359).
    pub model_records: ModelRecords,
    /// The relationships between the admitted models' object types, which
    /// `r.name` navigates (FR-363).
    pub relationships: Relationships,
    /// The model object types' own declared operations (FR-103), resolved
    /// through [`Self::types`]' effective view.
    pub operations: OperationTable,
    /// Enum declarations.
    pub enums: Vec<EnumBinding>,
    /// The admitted models' variant types, ascending by effective identity.
    pub variant_types: Vec<VariantTypeDeclaration>,
    /// The members the models declare with no checked type (FR-056-AC-43).
    pub untyped_members: Vec<UntypedMember>,
    /// `type Name = T;` aliases, which create no declaration identity.
    pub aliases: Vec<(String, ValueType)>,
    /// The names of the unit's own `type Name = T;` aliases.
    pub unit_alias_names: std::collections::BTreeSet<String>,
    /// Each model alias type by its `alias::T` name, with its declaration.
    pub model_type_aliases: BTreeMap<String, crate::value::TypeArgumentAlias>,
    /// Each package-owned type name's spelling under a further `model`
    /// alias of the same package (FR-480 "Selections and imports").
    pub(crate) model_spellings: super::model_spelling::ModelSpellings,
    /// The unit's own `profile p = ...;` and `model M = ...;` selection
    /// aliases (FR-091), each with which kind it is, independent of
    /// whether the model's domain package admitted (FR-113 "Refusals"
    /// binder no-shadowing rule reads this: a binder naming one of these
    /// refuses regardless of what its `model` declaration eventually
    /// resolves to).
    pub alias_names: std::collections::BTreeMap<String, super::refusal::AliasKind>,
    /// The unit's own `dimension` and `unit` declaration names (FR-091
    /// "Dimension form"/"Unit form"), independent of `Scope::named_types`
    /// (composites, enums, object types and aliases only) and
    /// `Signatures::declares` (functions and predicates): FR-113 "Refusals"
    /// binder no-shadowing rule reads this too, so a binder naming a
    /// declared dimension or unit refuses the same way as one naming any
    /// other native declaration (FSpec `shared-grammar.md`: "Native
    /// declaration names are unique within the linked package").
    pub native_names: std::collections::BTreeSet<String>,
    /// Qualified names of imported model operations, predicates and clauses.
    pub model_operations: Vec<String>,
    /// Function declarations in declaration order.
    pub functions: Vec<FunctionDeclaration>,
    /// The admitted FR-148 IEEE profile, if the package selects one.
    pub ieee_profile: Option<AdmittedIeeeProfile>,
    /// The S3 limits on the written scale of `decimal(c, s)` literals
    /// (`s3.decimal_literal_scale`, `s3.decimal_literal_scale_total`).
    pub decimal_literal_limits: DecimalLiteralLimits,
    /// The package's admitted `div`/`rem` law, when one was selected.
    pub integer_division: Option<filament_checked::integer_division::AdmittedIntegerDivision>,
    /// FR-151 dispatch-eligible operations a `receiver.member(args)` call may
    /// resolve to, keyed by the receiver's static type and the member name.
    /// Built by the caller (the `crate::model` bridge); the checker only
    /// resolves against it.
    pub dispatch_operations: Vec<DispatchOperation>,
    /// The checked dispatch tables `dispatch_operations` indexes into, ready
    /// for the evaluator. Built by the same caller, with function indices
    /// already resolved against `functions`.
    pub dispatch_tables: Vec<DispatchTable>,
    /// FR-094: every admitted domain package whose declarations a
    /// `Reference<T>`, a model row's member or a clause function names.
    /// `check` keys each such declaration's model node and records it in the
    /// model correspondence, whose only writer it is.
    pub models: Vec<super::lowering::AdmittedModel>,
    /// FR-094: the owner and clause kind of each clause function, keyed by
    /// index into [`Self::functions`]. A function with an entry is keyed
    /// with its declaration's `ModelOwner`; every other function is a
    /// source declaration. `check::checked_dispatch` fills it for the
    /// clause functions it synthesizes.
    pub model_clauses: std::collections::BTreeMap<usize, super::lowering::ModelClause>,
    /// FR-098-AC-34: the mode a replay recompile checks a named function body
    /// under, by function name, overriding the requested mode. A declared
    /// clause function is checked `Linked` whatever this holds.
    pub function_modes:
        std::collections::BTreeMap<String, filament_semantic_value::checking::CheckMode>,
    /// Resolved signatures standing in for some `functions` entries' own
    /// type forms; see [`ResolvedSignatures`].
    pub resolved_signatures: ResolvedSignatures,
    /// FR-096: the span of each declared record, tuple, enum, or selected
    /// model alias, by its declared name, for declarations read from the
    /// unit (the FR-091 assembler fills it). A declaration built by hand has
    /// none, and its `declaration` occurrence then names no region.
    pub declared_type_spans: BTreeMap<String, fsl_foundation::Span>,
    /// FR-096, FR-337: the expression spans of each declaration site that
    /// has an expression tree of its own and is located by an
    /// `Origin::TypeDeclaration` site name (a temporal clause's `holds`
    /// condition), by that name. The FR-091 assembler fills it.
    pub declared_site_spans: BTreeMap<String, fsl_forms::ExpressionSpans>,
    /// FR-096, FR-302: the span of the unit declaration each function
    /// synthesized for dispatch without form spans of its own was derived
    /// from, by index into [`Self::functions`]: every occurrence in such a
    /// function resolves to that span. `link_dispatch` fills it for the
    /// precondition functions it synthesizes from the unit's `pre` clauses.
    pub synthesized_spans: BTreeMap<usize, fsl_foundation::Span>,
    /// FR-096-AC-1, ADR-013 C-21: the map of a body embedded in a document.
    /// The spans above are body offsets; with a map, each region resolves
    /// under the document's `RawSourceRef`, shifted by the body's offset in
    /// the document. `None` for a unit that is its own document. `Arc`
    /// so `regions()` shares one map rather than cloning the document and
    /// body text on every call.
    pub embedding: Option<Arc<fsl_foundation::source_map::SourceMap>>,
    /// The `UnitGraph` the assembler admitted over the unit's dimension and
    /// unit declarations (FR-091). Empty when the unit declares none.
    pub units: filament_semantic_value::unit::UnitGraph,
    /// The span of each dimension's and unit's declared name, by its node
    /// key (FR-096).
    pub nominal_spans: BTreeMap<filament_exact::NodeKey, fsl_foundation::Span>,
    /// Each `import ... as a` the S4 source resolution admitted, by its
    /// qualifier `a` (ADR-015 D-1, D-5). An import with no `as` binds no
    /// qualifier (FR-087-AC-13).
    pub imports: BTreeMap<String, super::AdmittedImport>,
    /// FR-091-AC-22: the profile selection each source function's `using`
    /// alias resolved to, by index into [`Self::functions`]. A function
    /// with no `using` alias, or built by hand, has no entry.
    pub function_selections: BTreeMap<usize, fsl_foundation::selection::ProfileSelection>,
    /// FR-104: the unit's state clauses in declaration order, each header
    /// name resolved by the FR-091 assembler. Clauses and functions share
    /// one selection namespace (FR-109).
    pub state_clauses: Vec<super::state_clause::StateClauseDeclaration>,
    /// Exact keys supplied with the accepted execution-point map. S3 checks
    /// them after the existing shared clause/function ambiguity pass.
    pub execution_point_keys: std::collections::BTreeSet<String>,
    /// FR-112: the unit's protocol declarations in declaration order, each
    /// carrying its own scoped anchors and declaration collection for
    /// FR-113's checker to resolve.
    pub protocols: Vec<fsl_forms::ProtocolDeclarationForm>,
    /// FR-114: each protocol's own `attempt`s, resolved by the FR-091
    /// assembler, index-aligned with [`Self::protocols`]; each
    /// inner list is in the same order as its protocol's own
    /// `ProtocolDeclarationForm::attempts`.
    pub protocol_attempts: Vec<Vec<super::state_clause::AttemptDeclaration>>,
    /// FR-325, FR-326: the unit's `temporal` clauses in declaration order,
    /// each as S2 built it, for the `TemporalTrace` family's `check`.
    pub temporal_clauses: Vec<super::temporal_clause::TemporalDeclaration>,
    /// FR-304: the unit's `abstraction` declarations in declaration order,
    /// each as S2 built it, for `CheckedGraph::abstraction`.
    pub abstractions: Vec<fsl_forms::AbstractionDeclarationForm>,
    /// FSpec FR-490: the unit's `guarantee` declarations in declaration
    /// order, each as S2 built it, for `CheckedGraph::implementation_guarantees`.
    pub guarantees: Vec<fsl_forms::GuaranteeDeclarationForm>,
    /// FSpec FR-491: the unit's `compose` declarations in declaration order,
    /// each as S2 built it, for `CheckedGraph::compositions`.
    pub compositions: Vec<fsl_forms::CompositionDeclarationForm>,
    /// FSpec FR-491 "Dependency graph and cycles": the profile the unit
    /// selects that declares FR-176's monotone complete lattice, initial
    /// element and convergence rule, when it selects one.
    pub composition_lattice: Option<String>,
    /// FR-311, FR-392: the unit's `random`, `reward`, `workload` and
    /// `probabilistic` declarations in declaration order, each as S2 built
    /// it, for the `Probabilistic` family's check (FR-185, FR-186).
    pub probabilistic: Vec<super::ProbabilisticDeclaration>,
    /// FR-165: the unit's state-graph claims in declaration order, each with
    /// its profile and `over` type resolved, for the `StateGraph` family's
    /// `check`.
    pub state_graph_claims: Vec<super::state_graph_claim::StateGraphDeclaration>,
    /// FR-754: each bounded synthesis form in declaration order, resolved
    /// against this package's admitted type environment by `check`.
    pub synthesis_forms: Vec<fsl_forms::SynthesisForm>,
    /// FR-110, FR-326: the unit's header profile selections the
    /// `TemporalTrace` family's catalog holds, in source order. A clause is
    /// checked under the one of them, or refuses when there is none or
    /// several.
    pub temporal_selections: Vec<super::temporal_clause::TemporalSelection>,
    /// The complete timed declaration forms S2 built, in source order for
    /// FR-230's S3 check and located refusals.
    pub timed_forms: Vec<super::assemble::TimedDeclaration>,
    /// FR-230: the unit's clock-reading declarations in source order, which
    /// have no checked node; S3 checks each body and expands each call.
    pub clock_reading: Vec<super::assemble::ClockReadingDeclaration>,
    /// The unit's admitted dimension equivalences (FR-091, FSpec FR-142
    /// "Dimension equivalence"), in source order.
    pub dimension_equivalences: Vec<super::assemble::DimensionEquivalence>,
    /// The `taskset`, `schedulable` and `response` forms S2 built, in source
    /// order, for FSL FR-246's S3 check.
    pub task_set_forms: Vec<super::task_set::TaskSetDeclaration>,
    /// Source forms for verification plans. They remain outside the checked
    /// clause graph and are checked at S3 (FR-753).
    pub verification_plans: Vec<fsl_forms::VerificationPlanForm>,
    /// FR-230, FR-233: the time source of each timed model, which a
    /// `model-time` clause binds (FR-234). The assembler starts this empty;
    /// S3 checks source `time` members into it. A caller that already holds
    /// a model's checked time source may supply it directly.
    pub time_sources: Vec<super::temporal_clause::ModelTimeSource>,
    /// FR-110, FR-186: every header profile selection of the unit, in source
    /// order, for a `probabilistic` claim's `using` alias to resolve against.
    pub profile_selections: Vec<fsl_foundation::selection::ProfileSelection>,
    /// FR-171, FR-172: the unit's `hyper` clauses and `relation`s in
    /// declaration order, each as S2 built it, for the `TemporalTrace`
    /// family's `check`.
    pub hyper_clauses: Vec<super::hyper_clause::HyperDeclaration>,
    /// FR-103: each domain-package operation `requires`/`ensures` clause the
    /// driver parsed, with its operation resolved, checked beside the state
    /// clauses. Checking yields a refusal or nothing.
    pub operation_clauses: Vec<super::OperationClauseDeclaration>,
    /// FR-366: each type-level `Invariant` clause the driver parsed, with its
    /// owner resolved, checked beside the state clauses. A checked invariant
    /// adds no declaration, claim or requirement record.
    pub type_invariants: Vec<super::TypeInvariantDeclaration>,
    /// FR-124: the unit's `terminal` declarations in declaration order, each
    /// resolved by the FR-091 assembler and checked beside the state clauses.
    pub terminals: Vec<super::TerminalClauseDeclaration>,
    /// Test-only: check a package whose timed model holds clocks, time
    /// invariants or urgency without refusing it as not emitted (FR-230).
    pub emit_unwritten_timed_members: bool,
    /// FR-135 to FR-137: the unit's `refinement` declarations in declaration
    /// order, each resolved by the FR-091 assembler and checked after the
    /// state clauses.
    pub refinements: Vec<super::RefinementDeclaration>,
    /// FR-447: the unit's `hybrid` declarations in declaration order, each
    /// with its `using` alias resolved by the FR-091 assembler.
    pub hybrids: Vec<super::HybridDeclaration>,
    /// FR-354: the unit's declaration forms of selected extensions in source
    /// order, each as S2 built it.
    pub extension_forms: Vec<fsl_forms::ExtensionForm>,
    /// FR-354: the extension definitions the unit selects, held after the
    /// catalog validated them. S3 reads held definitions only.
    pub held_extensions: crate::library::extension::HeldExtensions,
    /// FR-447: the unit's hybrid claims in declaration order, the `temporal`
    /// clauses whose `over` type names one of [`Self::hybrids`], each with its
    /// `using` alias resolved by the FR-091 assembler.
    pub hybrid_claims: Vec<super::HybridClaimDeclaration>,
    /// FR-147: the unit's `refinement_internals` input, supplied beside the
    /// source and unchecked (FSpec FR-375): a JSON array of `{"refinement":
    /// name, "internal": [class, ...]}` objects. `None` is the empty list.
    pub refinement_internals: Option<serde_json::Value>,
    /// FR-337 "Aggregate formula emission": the selected timestamped-event
    /// clock unit of each clause whose clock the caller already knows, so a
    /// `duration` window converts both bounds to it (FSpec FR-370).
    pub clock_units: Vec<super::temporal_clause::ClockUnitSelection>,
    /// FR-135: the refinement features this build has built. A build
    /// capability, not a request parameter; only a test replaces it.
    pub refinement_support: super::RefinementSupport,
    /// Test-only: check a package that holds a `refinement` declaration
    /// without refusing it as not emitted (FR-135).
    pub emit_unwritten_refinements: bool,
    /// Test-only: check a package whose protocol names a construct S4 does
    /// not write yet ([`ProtocolConstruct`](super::ProtocolConstruct))
    /// without refusing it as not emitted, so the S3 facts and the
    /// refinement rules over it stay testable until S4 writes it (FR-383).
    pub emit_unwritten_protocols: bool,
    /// FR-103, FR-302: the unit's `operation-body` declarations in
    /// declaration order, each resolved to its operation's `DeclarationKey`
    /// by the FR-091 assembler. At most one per key.
    pub operation_bodies: Vec<super::OperationBodyDeclaration>,
    /// FR-103, FR-302: the effective view of each selected `model`
    /// declaration, by its alias, with `has_body` set from the unit's
    /// `operation-body` declarations that resolved into it. A model no body
    /// names holds its admitted view, whose `has_body` is false throughout.
    /// Dispatch linking reads a called operation's family from these views.
    pub selected_views: BTreeMap<String, crate::model::normalize::EffectiveView>,
    /// The byte limit every node identity of this package encodes under
    /// (`identity.input_bytes`), carried from the assembly that built it.
    pub identity: fsl_foundation::IdentityLimits,
}

impl PackageDeclarations {
    /// These declarations, to be checked without refusing a timed model's
    /// clocks, time invariants or urgency as not emitted, so the S3 value
    /// (`CheckedGraph::timed_models`) stays testable until S4 writes it
    /// (FR-230).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_unwritten_timed_members(mut self) -> Self {
        self.emit_unwritten_timed_members = true;
        self
    }

    /// These declarations, to be checked without refusing a `refinement`
    /// declaration as not emitted, so the S3 value
    /// (`CheckedGraph::refinements`) stays testable until S4 writes it
    /// (FR-135).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_unwritten_refinements(mut self) -> Self {
        self.emit_unwritten_refinements = true;
        self
    }

    /// These declarations, to be checked without refusing a protocol that
    /// names a construct S4 does not write yet as not emitted (FR-383).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_unwritten_protocols(mut self) -> Self {
        self.emit_unwritten_protocols = true;
        self
    }

    /// These declarations with the unit's `refinement_internals` input
    /// (FR-147).
    #[must_use]
    pub fn with_refinement_internals(mut self, value: serde_json::Value) -> Self {
        self.refinement_internals = Some(value);
        self
    }

    /// These declarations with the profile `profile` the unit selects that
    /// declares FR-176's monotone complete lattice, initial element and
    /// convergence rule (FSpec FR-491 "Dependency graph and cycles").
    #[must_use]
    pub fn with_composition_lattice(mut self, profile: impl Into<String>) -> Self {
        self.composition_lattice = Some(profile.into());
        self
    }

    /// These declarations with the extension definitions the unit selects,
    /// held after validation (FR-354 "Definition validation").
    #[must_use]
    pub fn with_extensions(mut self, held: crate::library::extension::HeldExtensions) -> Self {
        self.held_extensions = held;
        self
    }

    /// These declarations with the selected timestamped-event clock unit of
    /// each clause `units` names (FR-337 "Aggregate formula emission").
    #[must_use]
    pub fn with_clock_units(
        mut self,
        units: Vec<super::temporal_clause::ClockUnitSelection>,
    ) -> Self {
        self.clock_units = units;
        self
    }

    /// These declarations with the clock configuration of each clock role
    /// `clocks` names, keyed by clock name as a request's `clocks` member is
    /// (FR-640): every `temporal` clause that writes that role reads it. An
    /// in-process caller passes the configurations the request path takes, so
    /// a role-clock timed clause has the same binding unit either way (FR-933).
    #[must_use]
    pub fn with_clock_configurations(
        self,
        clocks: &BTreeMap<String, filament_contract_model::ClockConfiguration>,
    ) -> Self {
        let units = self
            .temporal_clauses
            .iter()
            .filter_map(|clause| {
                let clock = clocks.get(&clause.form.clock.as_ref()?.text)?;
                Some(super::temporal_clause::ClockUnitSelection {
                    declaration_span: clause.span,
                    clock: clock.clone(),
                })
            })
            .collect();
        self.with_clock_units(units)
    }

    /// These declarations, to be checked as a build that has built only
    /// `support` (FR-135).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_refinement_support(mut self, support: super::RefinementSupport) -> Self {
        self.refinement_support = support;
        self
    }

    /// The work units the staged model type admission has spent so far
    /// (structural admission plus its metered reads), when one is staged.
    #[cfg(any(test, feature = "test-support"))]
    pub fn staged_work_units(&self) -> Option<u64> {
        self.pending_types.as_ref().map(|staged| staged.0.spent())
    }

    /// Test fixture declarations retaining the genuine admission operation.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_with_selected_models(
        source: fsl_foundation::source::provenance::RawSourceRef,
        package_name: impl Into<String>,
        identity: fsl_foundation::IdentityLimits,
        selected: crate::model::intake::SelectedModels,
    ) -> Self {
        let (entries, meter, exact_declarations, exact_fields, tuple_declarations) =
            selected.into_parts();
        let mut declarations = Self::new(source, package_name, identity);
        declarations.models = entries
            .iter()
            .map(|entry| super::AdmittedModel::from_view(&entry.view))
            .collect();
        declarations.model_inputs = Some((entries, meter));
        declarations.selected_exact_identity_rows = (exact_declarations, exact_fields);
        declarations.selected_tuple_declaration_rows = tuple_declarations;
        declarations
    }

    /// A package declared by the source unit `source` names, with no
    /// declaration yet and no protocol profile, whose node identities encode
    /// under `identity`.
    pub fn new(
        source: fsl_foundation::source::provenance::RawSourceRef,
        package_name: impl Into<String>,
        identity: fsl_foundation::IdentityLimits,
    ) -> Self {
        Self {
            model_inputs: None,
            selected_exact_identity_rows: (Vec::new(), Vec::new()),
            selected_tuple_declaration_rows: Vec::new(),
            pending_types: None,
            source: source.clone(),
            sources: vec![source],
            source_owners: BTreeMap::new(),
            function_sources: Vec::new(),
            package_name: package_name.into(),
            protocol_profile: None,
            header_layers: Vec::new(),
            source_object_identity_fields: BTreeMap::new(),
            types: TypeEnvironment::default(),
            model_records: ModelRecords::default(),
            relationships: Relationships::default(),
            operations: OperationTable::default(),
            enums: Vec::new(),
            variant_types: Vec::new(),
            untyped_members: Vec::new(),
            aliases: Vec::new(),
            unit_alias_names: std::collections::BTreeSet::new(),
            model_type_aliases: BTreeMap::new(),
            model_spellings: super::model_spelling::ModelSpellings::default(),
            alias_names: std::collections::BTreeMap::new(),
            native_names: std::collections::BTreeSet::new(),
            model_operations: Vec::new(),
            functions: Vec::new(),
            ieee_profile: None,
            decimal_literal_limits: DecimalLiteralLimits::default(),
            integer_division: None,
            dispatch_operations: Vec::new(),
            dispatch_tables: Vec::new(),
            models: Vec::new(),
            model_clauses: std::collections::BTreeMap::new(),
            function_modes: std::collections::BTreeMap::new(),
            resolved_signatures: ResolvedSignatures::default(),
            declared_type_spans: BTreeMap::new(),
            declared_site_spans: BTreeMap::new(),
            synthesized_spans: BTreeMap::new(),
            embedding: None,
            units: filament_semantic_value::unit::UnitGraph::default(),
            nominal_spans: BTreeMap::new(),
            imports: BTreeMap::new(),
            function_selections: BTreeMap::new(),
            state_clauses: Vec::new(),
            execution_point_keys: std::collections::BTreeSet::new(),
            protocols: Vec::new(),
            protocol_attempts: Vec::new(),
            temporal_clauses: Vec::new(),
            abstractions: Vec::new(),
            guarantees: Vec::new(),
            compositions: Vec::new(),
            composition_lattice: None,
            probabilistic: Vec::new(),
            state_graph_claims: Vec::new(),
            synthesis_forms: Vec::new(),
            temporal_selections: Vec::new(),
            timed_forms: Vec::new(),
            clock_reading: Vec::new(),
            dimension_equivalences: Vec::new(),
            task_set_forms: Vec::new(),
            verification_plans: Vec::new(),
            time_sources: Vec::new(),
            profile_selections: Vec::new(),
            hyper_clauses: Vec::new(),
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
            terminals: Vec::new(),
            emit_unwritten_timed_members: false,
            refinements: Vec::new(),
            hybrids: Vec::new(),
            extension_forms: Vec::new(),
            held_extensions: crate::library::extension::HeldExtensions::default(),
            hybrid_claims: Vec::new(),
            refinement_internals: None,
            clock_units: Vec::new(),
            refinement_support: super::RefinementSupport::BUILD,
            emit_unwritten_refinements: false,
            emit_unwritten_protocols: false,
            operation_bodies: Vec::new(),
            selected_views: BTreeMap::new(),
            identity,
        }
    }

    /// FR-302: the unit's body for the operation `key`, if one resolved to
    /// it.
    pub fn operation_body(
        &self,
        key: &filament_checked::key::DeclarationKey,
    ) -> Option<&super::OperationBodyDeclaration> {
        self.operation_bodies.iter().find(|body| &body.key == key)
    }

    /// FR-103, FR-302: the unit `pre` and `post` clauses that resolved to
    /// the operation `key`, in declaration order.
    pub fn unit_clauses_for(
        &self,
        key: &filament_checked::key::DeclarationKey,
    ) -> Vec<&super::StateClauseDeclaration> {
        self.state_clauses
            .iter()
            .filter(|clause| clause.operation_key.as_ref() == Some(key))
            .collect()
    }

    /// The unit's owner, `SourceOwner{authority, identity}` taken from its
    /// `RawSourceRef` (FR-001).
    pub fn owner(&self) -> super::node_key::SourceOwner {
        super::node_key::SourceOwner::from(&self.source)
    }
}

/// One FR-151 dispatch-eligible operation: a `receiver.member(args)` call
/// whose receiver's static type is `receiver_type` and whose member name is
/// `member` resolves to this signature, checked against `parameters` and
/// `result`, and its call site becomes a `NodeKind::Dispatch`
/// naming `table` (an index into the package's `dispatch_tables`).
#[derive(Clone, Debug)]
pub struct DispatchOperation {
    /// The receiver's required static type, by its effective identity
    /// (ADR-013 O-05).
    pub receiver_type: EffectiveId,
    /// The unqualified member name a dispatch call site names.
    pub member: String,
    /// The declared parameter types, in order.
    pub parameters: Vec<ValueType>,
    /// Minimum number of positional arguments accepted by this signature.
    pub required_arity: usize,
    /// The declared result type.
    pub result: ValueType,
    /// The declaring model type selected for this operation family.
    pub selected_operation_owner: EffectiveId,
    /// The producer-selected operation member name.
    pub selected_operation: String,
    /// The dispatch table this operation's call sites resolve to.
    pub table: usize,
}

/// What the checker knows of a model's record value types and event types
/// beyond their composite declarations (FR-359).
///
/// A checked field read names its field by the record it is typed at and the
/// name there. The value read is a child of that record, whose effective
/// fields are ordered by their own keys, and which may redefine the field
/// under another name (FR-082). This holds, for each record that redefines an
/// ancestor's field under a different name, the name of its own field; a
/// record with no such field keeps every field under its name. It also holds
/// which declarations are event types, which FR-208 gives no equality.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct ModelRecords {
    /// By the value's declaration and the declaration the read is typed at,
    /// then the name there: a read looks a borrowed name up and allocates
    /// nothing, and a package with no redefinition under another name
    /// holds an empty map.
    renamed: BTreeMap<(filament_exact::NodeKey, filament_exact::NodeKey), BTreeMap<String, String>>,
    events: BTreeSet<filament_exact::NodeKey>,
    /// Authored tuple positions mapped to data positions; clocks have no data slot.
    tuple_positions: BTreeMap<filament_exact::NodeKey, Vec<Option<usize>>>,
}

impl DebugWith for ModelRecords {
    fn fmt_with(&self, f: &mut std::fmt::Formatter<'_>, table: &DebugTable) -> std::fmt::Result {
        let Self {
            renamed,
            events,
            tuple_positions,
        } = self;
        f.debug_struct("ModelRecords")
            .field(
                "renamed",
                &renamed
                    .iter()
                    .map(|(key, value)| {
                        (
                            (NodeKeyDebug(&key.0, table), NodeKeyDebug(&key.1, table)),
                            value,
                        )
                    })
                    .collect::<Vec<_>>(),
            )
            .field("events", &NodeList(events.iter(), table))
            .field(
                "tuple_positions",
                &tuple_positions
                    .iter()
                    .map(|(key, positions)| (NodeKeyDebug(key, table), positions))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl std::fmt::Debug for ModelRecords {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        standalone(f, self)
    }
}

impl ModelRecords {
    /// Record that the record `value` keeps the field `name` of the record
    /// `read_at` under its own field `own`.
    pub(crate) fn rename(
        &mut self,
        value: filament_exact::NodeKey,
        read_at: filament_exact::NodeKey,
        name: impl Into<String>,
        own: impl Into<String>,
    ) {
        self.renamed
            .entry((value, read_at))
            .or_default()
            .insert(name.into(), own.into());
    }

    /// Record that `declaration` is an event type.
    pub(crate) fn declare_event(&mut self, declaration: filament_exact::NodeKey) {
        self.events.insert(declaration);
    }

    pub(crate) fn declare_tuple_positions(
        &mut self,
        declaration: filament_exact::NodeKey,
        positions: Vec<Option<usize>>,
    ) {
        self.tuple_positions.insert(declaration, positions);
    }

    /// Every entry of `other`.
    pub(crate) fn extend(&mut self, other: ModelRecords) {
        for (key, names) in other.renamed {
            self.renamed.entry(key).or_default().extend(names);
        }
        self.events.extend(other.events);
        self.tuple_positions.extend(other.tuple_positions);
    }

    /// The name under which a value of `declaration` keeps `field`.
    fn own_name<'f>(
        &'f self,
        declaration: filament_exact::NodeKey,
        field: &'f RecordField,
    ) -> &'f str {
        self.renamed
            .get(&(declaration, field.declaration()))
            .and_then(|names| names.get(field.name()))
            .map_or(field.name(), String::as_str)
    }
}

/// The S3 limits on the written scale of `decimal(c, s)` literals (FR-093
/// "Decimal constructors"; FR-255).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecimalLiteralLimits {
    /// `s3.decimal_literal_scale`: the largest written scale of one literal.
    pub scale: u64,
    /// `s3.decimal_literal_scale_total`: the largest total of the written
    /// scales of the literals of one checked package.
    pub total: u64,
}

impl Default for DecimalLiteralLimits {
    fn default() -> Self {
        Self {
            scale: 65_535,
            total: 1_048_576,
        }
    }
}

/// Everything names resolve against, apart from function bodies.
#[derive(Clone)]
pub struct Scope {
    /// The limits on decimal literal scale, and the total counted so far.
    decimal_literal_limits: DecimalLiteralLimits,
    decimal_literal_total: fsl_foundation::tally::Tally,
    /// Fixed once built: [`Self::index`] is derived from it.
    types: TypeEnvironment,
    /// Where a record value keeps the fields its ancestors name (FR-359).
    model_records: ModelRecords,
    /// The relationship ends a receiver type navigates (FR-363).
    relationships: Relationships,
    /// Fixed once built, like `types`.
    enums: Vec<EnumBinding>,
    /// The admitted models' variant types, as the assembler ordered them.
    /// A source name `M::T` resolves to one through [`Self::index`].
    variant_types: Vec<VariantTypeDeclaration>,
    /// The model object types' own declared operations (FR-103).
    operations: OperationTable,
    pub(crate) ieee_profile: Option<AdmittedIeeeProfile>,
    /// The IEEE package-selection trigger recorded from resolved declarations.
    /// Every body float originates in a declaration: constants and conversion
    /// into floats are unavailable, and model float intake is refused. Any new
    /// source of floats must also extend this trigger before checking its body.
    ieee_operation: bool,
    pub(crate) integer_division:
        Option<filament_checked::integer_division::AdmittedIntegerDivision>,
    pub(crate) complete_value: bool,
    /// The names of the concrete populations of the unit's models, which
    /// refuse `unsupported_construct`/`expression-form` where no refinement
    /// reads them (FR-139).
    pub(crate) known_populations: Vec<String>,
    /// The object type and path of every clock place of the unit's timed
    /// models, which `ref(p)` names (FR-230 "Clock references").
    clock_places: Vec<(
        EffectiveId,
        Vec<super::lowering::PlaceStep>,
        Option<NodeKey>,
    )>,
    /// The guarded clock places named by presence reads in data clauses.
    clock_presence_places: Vec<super::timed_model::PresencePlace>,
    /// The names of the unit's clock-reading declarations (FR-230): a call
    /// of one that reaches the value checker is outside every place that
    /// admits it.
    clock_reading: BTreeSet<String>,
    /// The names of the history fields the unit's refinements declare
    /// (FR-138). A field of that name is the refinement's own, so a state
    /// clause that reads it refuses `missing-name`.
    pub(crate) history_names: Vec<String>,
    /// Every admitted model currently uses `filament.model.complete/v1`; an
    /// absent model keeps the historical exact-edge typing rule.
    pub(crate) complete_model: bool,
    /// The package-owned type names each unit's model aliases spell (FR-480
    /// "Selections and imports").
    model_spellings: ModelSpellings,
    /// The members the models declare with no checked type (FR-056-AC-43).
    untyped_members: Vec<UntypedMember>,
    /// The source unit of each top-level declaration, by declared name; a
    /// name with no entry is the primary unit's.
    source_owners: BTreeMap<String, RawSourceRef>,
    /// The primary source unit, owner of a declaration with no entry above.
    primary_source: Option<RawSourceRef>,
    /// The names of the unit's own `type Name = T;` aliases.
    own_aliases: BTreeSet<String>,
    /// Each model alias type by its `alias::T` name, with its declaration.
    model_aliases: BTreeMap<String, crate::value::TypeArgumentAlias>,
    /// Admitted systems-model ports by their package identity and source
    /// artifact name, with the exact declaration and owning object type
    /// used by a role.
    model_ports: BTreeMap<(String, String), (DeclarationKey, EffectiveId)>,
    pub(crate) dispatch_operations: Vec<DispatchOperation>,
    /// FR-150: each object type a `symmetric` population covers, with that
    /// population's declaration key.
    symmetric_objects: BTreeMap<EffectiveId, DeclarationKey>,
    /// The by-name lookups over the declared types, enums, aliases and
    /// model operations, built once by [`Self::new`]. Aliases and
    /// model operations are only ever looked up by name, so only this index
    /// holds them.
    index: ScopeIndex,
    /// Each admitted import by its qualifier (ADR-015 D-5).
    imports: BTreeMap<String, super::AdmittedImport>,
}

fsl_foundation::debug_with_struct!(Scope, "Scope" { types: plain, model_records: with, relationships: plain, enums: plain, variant_types: with, operations: plain, ieee_profile: plain, ieee_operation: plain, integer_division: plain, complete_value: plain, known_populations: plain, clock_places: plain, clock_presence_places: plain, clock_reading: plain, history_names: plain, complete_model: plain, dispatch_operations: plain, symmetric_objects: plain, index: plain, imports: values });

/// Drops the chain of imported checked graphs over an explicit stack
/// (ADR-030): each import holds its library's graph, whose scope holds that
/// library's own imports, so a derived drop would recurse once per library
/// of an import chain.
/// A [`Scope`] as one source unit reads its type names: a package-owned
/// type `M::T` resolves through that unit's model alias `M` (FR-480
/// "Selections and imports").
#[derive(Clone, Copy)]
pub(crate) struct SourceScope<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) source: Option<&'a RawSourceRef>,
}

impl<'a> SourceScope<'a> {
    /// Every type `name` binds in this unit; see [`Scope::named_types`].
    pub(crate) fn named_types(&self, name: &str) -> &'a [ValueType] {
        self.scope.named_types(self.source, name)
    }

    /// The unit `name` names in this unit; see [`Scope::unit_named`].
    pub(crate) fn unit_named(&self, name: &str) -> Option<filament_exact::UnitId> {
        self.scope.unit_named(self.source, name)
    }

    /// The packages declaring the bare unit `name`; see
    /// [`Scope::unit_packages`].
    pub(crate) fn unit_packages(&self, name: &str) -> Vec<String> {
        self.scope.unit_packages(self.source, name)
    }

    /// Whether `name` names an unbounded package Rational in this unit; see
    /// [`ModelSpellings::unbounded_rational`](super::model_spelling::ModelSpellings::unbounded_rational).
    pub(crate) fn unbounded_rational(&self, name: &str) -> bool {
        self.scope
            .model_spellings
            .unbounded_rational(self.source, name)
    }

    /// The object type `name` names in this unit; see
    /// [`Scope::object_type_named`].
    pub(crate) fn object_type_named(&self, name: &str) -> Option<EffectiveId> {
        self.scope.object_type_named(self.source, name)
    }

    /// The selected package this unit binds its model alias `alias` to, for
    /// a static protocol role that acts on a whole model (FR-480
    /// "Selections and imports").
    pub(crate) fn model_selection(
        &self,
        alias: &str,
    ) -> Option<&'a crate::model::domain_package::DomainPackageRef> {
        self.scope.model_spellings.selection(self.source, alias)
    }

    /// The selected Port declaration `M::artifact` names in this unit, through
    /// the package the unit binds `M` to, and the owning effective object
    /// type that provides its runtime value context.
    pub(crate) fn model_port(&self, name: &str) -> Option<(&'a DeclarationKey, &'a EffectiveId)> {
        let (alias, artifact) = name.split_once("::")?;
        let package = self.model_selection(alias)?;
        self.scope
            .model_ports
            .get(&(package.identity.clone(), artifact.to_owned()))
            .map(|(key, owner)| (key, owner))
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        let mut pending: Vec<Arc<super::CheckedGraph>> = std::mem::take(&mut self.imports)
            .into_values()
            .map(|import| import.graph)
            .collect();
        while let Some(graph) = pending.pop() {
            if let Ok(mut graph) = Arc::try_unwrap(graph) {
                pending.extend(
                    std::mem::take(&mut graph.scope.imports)
                        .into_values()
                        .map(|import| import.graph),
                );
            }
        }
    }
}

/// `Scope`'s by-name lookups, keyed by the parsed parts of a name,
/// so resolving a name is a map lookup, not a scan of every declaration.
#[derive(Clone, Debug, Default)]
struct ScopeIndex {
    /// Every type a declared name binds, in resolution order: aliases,
    /// composites, enums, then object types. More than one is ambiguous.
    types: BTreeMap<String, Vec<ValueType>>,
    /// Enum name, then member case, to each (binding, member) position in
    /// [`Scope::enums`] that declares it. A member `m` of enum `E` is
    /// written `E::m`.
    enum_members: BTreeMap<String, BTreeMap<String, Vec<(usize, usize)>>>,
    /// Every declared model operation name.
    model_operations: BTreeSet<String>,
    /// Each enum shape's first binding position in [`Scope::enums`], and
    /// the member index holding exactly that shape's members.
    enum_shapes: BTreeMap<EnumShape, (usize, EnumMemberIndex)>,
    /// Each object type name's first declaration, in `object_types()`
    /// order.
    object_types: BTreeMap<String, EffectiveId>,
    /// The checked `VariantId -> EnumValue` index over every binding's
    /// members, in binding then member order (ADR-013 T-6).
    enum_member_index: EnumMemberIndex,
    /// Variant type name, then case, to the position of the variant type in
    /// [`Scope::variant_types`] and of the case in its runtime members.
    variant_cases: BTreeMap<String, BTreeMap<String, (usize, usize)>>,
    /// Each variant type's enum shape to the type's position in
    /// [`Scope::variant_types`].
    variant_shapes: BTreeMap<EnumShape, usize>,
    /// Every case name of every variant type, for the bare-case refusal.
    bare_cases: BTreeSet<String>,
    /// Each protocol declaration's name to its index in the unit's
    /// protocols (FR-218: a protocol reference binds a local protocol only).
    protocols: BTreeMap<String, usize>,
    /// Each unit the unit declares, by its declared name (FR-091 "Dimension
    /// and unit declarations": a unit is used by name outside a type form).
    /// A quantity literal names its unit here (FR-185).
    units: BTreeMap<String, filament_exact::UnitId>,
}

/// Compound units a checked stage formed, each identified once when the
/// stage was checked. Evaluation and later clause checking seed their unit
/// scope from it without re-identifying or copying a table (FR-142).
#[derive(Clone, Debug, Default)]
pub struct FormedUnits(Vec<IdentifiedUnit>);

impl FormedUnits {
    /// Identify every unit of `table` once.
    ///
    /// # Errors
    ///
    /// As [`IdentifiedUnit::new`].
    pub fn identify(
        table: &UnitTable,
    ) -> Result<Self, filament_semantic_value::semantic_node::IdentityRefusal> {
        table
            .ids()
            .filter_map(|id| table.get(id))
            .map(|unit| IdentifiedUnit::new(unit.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    /// Record every unit in `scope`, which keeps the ones its package does
    /// not already hold.
    pub fn seed(&self, scope: &mut UnitScope<'_>) {
        for unit in &self.0 {
            scope.form(unit.clone());
        }
    }
}

/// The units `graph` declares by their declared name, read from `declared`,
/// the table built over the same graph, leaving out the units of a selected
/// package (`packages`), which a bare name reaches only when exactly one
/// package declares it ([`ModelSpellings::declaring_packages`]).
fn unit_ids_by_name(
    graph: &filament_semantic_value::unit::UnitGraph,
    declared: &filament_semantic_value::quantity::UnitTable,
    packages: &[filament_exact::UnitId],
) -> BTreeMap<String, filament_exact::UnitId> {
    graph
        .units()
        .filter(|unit| !packages.contains(&unit.id()))
        .filter_map(|unit| {
            let name = declared
                .nominal_node(&unit.key())
                .map(|node| node.declaration.qualified_declaration.as_slice());
            match name {
                Some([name]) => Some((name.clone(), unit.id())),
                _ => None,
            }
        })
        .collect()
}

/// The units a domain package declares in its own document, by their authored
/// name: the unit table a type of that package resolves its `unit` against
/// (FR-505-AC-7, AC-9), whether or not the compiling unit declares a unit of
/// the same name (the compiling unit's own unit of that name takes the name).
pub(crate) fn model_unit_ids(
    model: &crate::check::AdmittedModel,
) -> BTreeMap<String, filament_exact::UnitId> {
    model
        .units
        .iter()
        .filter_map(|unit| {
            let key = crate::check::assemble::package_unit_key(model, unit)?;
            Some((unit.name.clone(), filament_exact::UnitId::declared(key)))
        })
        .collect()
}

/// The units a type of `model` resolves its `unit` against: `declared`, the
/// compiling unit's own units, then the units the package's document declares
/// for the names the compiling unit leaves free (FR-505-AC-7, AC-9).
pub(crate) fn package_unit_ids(
    declared: &BTreeMap<String, filament_exact::UnitId>,
    model: &crate::check::AdmittedModel,
) -> BTreeMap<String, filament_exact::UnitId> {
    let mut units = declared.clone();
    for (name, id) in model_unit_ids(model) {
        units.entry(name).or_insert(id);
    }
    units
}

/// The units `graph` declares by their declared name (FSpec FR-142
/// "Quantity type reference": the unit namespace `Quantity<u>` looks up).
pub(crate) fn declared_unit_ids(
    graph: &filament_semantic_value::unit::UnitGraph,
    packages: &[filament_exact::UnitId],
) -> BTreeMap<String, filament_exact::UnitId> {
    unit_ids_by_name(
        graph,
        &filament_semantic_value::quantity::UnitTable::declared(graph),
        packages,
    )
}

/// Whether an untyped member is a field or an operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UntypedKind {
    /// A field.
    Field,
    /// An operation, untyped in a parameter or its result.
    Operation,
}

/// A member of a model's object type declared with no checked type: a bare
/// native `Text`, `Rational` or `Decimal` (FR-056-AC-43). It is admitted, and
/// refused where a clause or an input names it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UntypedMember {
    /// The declaring object type.
    pub owner: filament_exact::EffectiveId,
    /// The declaring object type's name.
    pub owner_name: String,
    /// The member name.
    pub name: String,
    /// Field or operation.
    pub kind: UntypedKind,
    /// Where its type reference is written.
    pub locus: String,
    /// For an operation, each parameter of no checked type and its locus.
    pub parameters: Vec<(String, String)>,
}

impl ScopeIndex {
    fn new(
        types: &TypeEnvironment,
        enums: &[EnumBinding],
        aliases: &[(String, ValueType)],
        model_operations: &[String],
    ) -> Self {
        let mut index = Self::default();
        for (alias, value_type) in aliases {
            index.named_type(alias, value_type.clone());
        }
        for declaration in types.composites() {
            index.named_type(declaration.name(), ValueType::Composite(declaration.key()));
        }
        for (position, binding) in enums.iter().enumerate() {
            index.named_type(&binding.name, ValueType::Enum(binding.shape()));
            for member in &binding.members {
                index.enum_member_index.record(member.clone());
            }
            let cases = index.enum_members.entry(binding.name.clone()).or_default();
            for (member, value) in binding.members.iter().enumerate() {
                cases
                    .entry(value.case().to_owned())
                    .or_default()
                    .push((position, member));
            }
        }
        for declaration in types.object_types() {
            index.named_type(declaration.name(), ValueType::Reference(declaration.key()));
            index
                .object_types
                .entry(declaration.name().to_owned())
                .or_insert(declaration.key());
        }
        // Filtered from the whole index, as `check_equality_in` once did
        // per equality, so each shape's table is the same one it built.
        for (position, binding) in enums.iter().enumerate() {
            let shape = binding.shape();
            if !index.enum_shapes.contains_key(&shape) {
                let members = index.enum_member_index.filtered(shape.variants().cloned());
                index.enum_shapes.insert(shape, (position, members));
            }
        }
        index.model_operations = model_operations.iter().cloned().collect();
        index
    }

    /// Index `variant_types`: each type's name resolves to its enum shape
    /// and each `T::Case` to its runtime member (FR-360 item 4).
    fn variant_types(&mut self, variant_types: &[VariantTypeDeclaration]) {
        for (position, declared) in variant_types.iter().enumerate() {
            let shape = declared.shape();
            self.named_type(declared.name(), ValueType::Enum(shape.clone()));
            self.variant_shapes.insert(shape, position);
            let cases = self
                .variant_cases
                .entry(declared.name().to_owned())
                .or_default();
            for (member, value) in declared.members().iter().enumerate() {
                cases.insert(value.case().to_owned(), (position, member));
                self.bare_cases.insert(value.case().to_owned());
                self.enum_member_index.record(value.clone());
            }
        }
    }

    fn named_type(&mut self, name: &str, value_type: ValueType) {
        self.types
            .entry(name.to_owned())
            .or_default()
            .push(value_type);
    }
}

impl Scope {
    /// A scope over these declarations, with its by-name lookups built once.
    pub(crate) fn new(
        types: TypeEnvironment,
        operations: OperationTable,
        enums: Vec<EnumBinding>,
        aliases: Vec<(String, ValueType)>,
        model_operations: Vec<String>,
        ieee_profile: Option<AdmittedIeeeProfile>,
        dispatch_operations: Vec<DispatchOperation>,
    ) -> Self {
        let index = ScopeIndex::new(&types, &enums, &aliases, &model_operations);
        Self {
            decimal_literal_limits: DecimalLiteralLimits::default(),
            decimal_literal_total: fsl_foundation::tally::Tally::default(),
            types,
            model_records: ModelRecords::default(),
            relationships: Relationships::default(),
            enums,
            variant_types: Vec::new(),
            untyped_members: Vec::new(),
            operations,
            ieee_profile,
            ieee_operation: false,
            integer_division: None,
            complete_value: false,
            known_populations: Vec::new(),
            clock_places: Vec::new(),
            clock_presence_places: Vec::new(),
            clock_reading: BTreeSet::new(),
            history_names: Vec::new(),
            complete_model: false,
            model_spellings: ModelSpellings::default(),
            source_owners: BTreeMap::new(),
            primary_source: None,
            own_aliases: BTreeSet::new(),
            model_aliases: BTreeMap::new(),
            model_ports: BTreeMap::new(),
            dispatch_operations,
            symmetric_objects: BTreeMap::new(),
            index,
            imports: BTreeMap::new(),
        }
    }

    /// The admitted operation declaration of `name` on its declaring type.
    pub(crate) fn operation_declaration(
        &self,
        declaring: filament_exact::EffectiveId,
        name: &str,
    ) -> Option<&crate::model::operation::OperationDeclaration> {
        self.operations
            .declared_by(declaring)
            .iter()
            .find(|operation| operation.name() == name)
    }

    /// This scope with a plan synthesis's plan value declared (FSpec
    /// FR-800): its `composites` and the `operations` enum, whose cases a
    /// qualified name `Alias::Op::case` reads. `None` when the type
    /// environment does not admit the composites beside its own.
    pub(crate) fn with_plan_types(
        &self,
        composites: Vec<filament_semantic_value::declaration::CompositeDeclaration>,
        operations: EnumBinding,
    ) -> Option<Self> {
        let mut scope = self.clone();
        scope.types = TypeEnvironment::new(
            self.types.composites().cloned().chain(composites),
            self.types.object_types().cloned(),
        )
        .ok()?
        .with_units(self.types.units().clone());
        let position = scope.enums.len();
        for member in &operations.members {
            scope.index.enum_member_index.record(member.clone());
        }
        let cases = scope
            .index
            .enum_members
            .entry(operations.name.clone())
            .or_default();
        for (member, value) in operations.members.iter().enumerate() {
            cases
                .entry(value.case().to_owned())
                .or_default()
                .push((position, member));
        }
        let shape = operations.shape();
        if !scope.index.enum_shapes.contains_key(&shape) {
            let members = scope
                .index
                .enum_member_index
                .filtered(shape.variants().cloned());
            scope.index.enum_shapes.insert(shape, (position, members));
        }
        scope.enums.push(operations);
        Some(scope)
    }

    /// This scope with the object types the admitted models' `symmetric`
    /// populations cover (FR-150).
    pub(crate) fn with_symmetric_objects(
        mut self,
        symmetric_objects: BTreeMap<EffectiveId, DeclarationKey>,
    ) -> Self {
        self.symmetric_objects = symmetric_objects;
        self
    }

    /// FR-150: the `symmetric` population whose objects `value_type` is, or
    /// contains, a reference to: through `Option`, a collection's element, a
    /// record's field or a tuple's position. `None` when it reads no such
    /// population.
    pub(crate) fn symmetric_population_in(
        &self,
        value_type: &ValueType,
    ) -> Option<&DeclarationKey> {
        if self.symmetric_objects.is_empty() {
            return None;
        }
        let mut pending = vec![value_type];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            match current {
                ValueType::Reference(object) => {
                    if let Some(population) = self.symmetric_objects.get(object) {
                        return Some(population);
                    }
                }
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Map(map) => pending.push(map.values()),
                ValueType::Composite(key) if seen.insert(key.clone()) => {
                    match self
                        .types
                        .composite(key.clone())
                        .map(|declaration| declaration.shape())
                    {
                        Some(CompositeShape::Record(fields)) => {
                            pending.extend(fields.iter().map(|field| field.value_type()));
                        }
                        Some(CompositeShape::Tuple(positions)) => pending.extend(positions),
                        Some(CompositeShape::Union(members)) => {
                            pending.extend(members.iter().flat_map(|member| member.positions()))
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// This scope with `imports`, each admitted import by its qualifier.
    pub(crate) fn with_imports(mut self, imports: BTreeMap<String, super::AdmittedImport>) -> Self {
        self.imports = imports;
        self
    }

    pub(crate) fn with_integer_division(
        mut self,
        division: Option<filament_checked::integer_division::AdmittedIntegerDivision>,
    ) -> Self {
        self.integer_division = division;
        self
    }

    pub(crate) fn with_complete_value(mut self, complete_value: bool) -> Self {
        self.complete_value = complete_value;
        self
    }

    pub(crate) fn with_decimal_literal_limits(mut self, limits: DecimalLiteralLimits) -> Self {
        self.decimal_literal_limits = limits;
        self
    }

    pub(crate) fn with_clock_reading(mut self, names: BTreeSet<String>) -> Self {
        self.clock_reading = names;
        self
    }

    /// Whether `name` names a clock-reading declaration of the unit
    /// (FR-230), which has no function node.
    pub(crate) fn is_clock_reading(&self, name: &str) -> bool {
        self.clock_reading.contains(name)
    }

    /// The clock places of `models`, which `ref(p)` may name.
    #[must_use]
    pub(crate) fn with_clock_places(mut self, models: &[super::AdmittedModel]) -> Self {
        self.clock_places = models
            .iter()
            .flat_map(|model| super::AdmittedModel::clock_places(model).iter())
            .map(|place| (place.owner.clone(), place.path.clone(), None))
            .collect();
        self
    }

    /// Attach the structural keys S3 assigns to timed models' clock
    /// places, in their time-source and place order (FSpec FR-322).
    pub(crate) fn with_clock_place_keys(
        mut self,
        models: &[super::AdmittedModel],
        time_sources: &[super::temporal_clause::ModelTimeSource],
        package_name: &str,
    ) -> Self {
        let mut ordinal = 0_u64;
        let keys: Vec<_> = time_sources
            .iter()
            .filter_map(|source| models.iter().find(|model| source.times(model)))
            .flat_map(|model| {
                self.clock_presence_places
                    .extend(super::timed_model::presence_places(
                        &super::timed_model::clock_fields(model),
                    ));
                super::AdmittedModel::clock_places(model)
                    .iter()
                    .map(|place| (place.owner.clone(), place.path.clone()))
                    .collect::<Vec<_>>()
            })
            .map(|(owner, path)| {
                let key = super::clock_place_node_key(package_name, ordinal).ok();
                ordinal = ordinal.saturating_add(1);
                (owner, path, key)
            })
            .collect();
        for (owner, path, key) in keys {
            if let Some(place) =
                self.clock_places
                    .iter_mut()
                    .find(|(candidate, candidate_path, _)| {
                        *candidate == owner && *candidate_path == path
                    })
            {
                place.2 = key;
            }
        }
        self
    }

    pub(crate) fn with_known_populations(mut self, names: Vec<String>) -> Self {
        self.known_populations = names;
        self
    }

    pub(crate) fn with_history_names(mut self, names: Vec<String>) -> Self {
        self.history_names = names;
        self
    }

    /// This scope with where its record values keep the fields their
    /// ancestors name.
    pub(crate) fn with_model_records(mut self, model_records: ModelRecords) -> Self {
        self.model_records = model_records;
        self
    }

    /// This scope with the relationship ends a receiver type navigates.
    pub(crate) fn with_relationships(mut self, relationships: Relationships) -> Self {
        self.relationships = relationships;
        self
    }

    /// The relationship ends a receiver type navigates (FR-363).
    pub(crate) fn relationships(&self) -> &Relationships {
        &self.relationships
    }

    /// This scope with the admitted models' variant types.
    pub(crate) fn with_untyped_members(mut self, untyped: Vec<UntypedMember>) -> Self {
        self.untyped_members = untyped;
        self
    }

    /// The member `name` of kind `kind` that `owner` declares with no checked
    /// type.
    pub(crate) fn untyped_member(
        &self,
        owner: &filament_exact::EffectiveId,
        name: &str,
        kind: UntypedKind,
    ) -> Option<&UntypedMember> {
        self.untyped_members
            .iter()
            .find(|member| member.kind == kind && &member.owner == owner && member.name == name)
    }

    pub(crate) fn with_variant_types(mut self, variant_types: Vec<VariantTypeDeclaration>) -> Self {
        self.index.variant_types(&variant_types);
        self.variant_types = variant_types;
        self
    }

    /// This scope with each unit's model aliases, through which a
    /// package-owned type name `M::T` written in that unit resolves (FR-480
    /// "Selections and imports").
    pub(crate) fn with_model_spellings(mut self, spellings: ModelSpellings) -> Self {
        self.model_spellings = spellings;
        self
    }

    /// This scope with the source unit of each top-level declaration, by
    /// declared name.
    pub(crate) fn with_source_owners(
        mut self,
        source_owners: BTreeMap<String, RawSourceRef>,
    ) -> Self {
        self.source_owners = source_owners;
        self
    }

    /// The source unit declaring the top-level declaration `name`, `None`
    /// for the primary unit.
    /// This scope with the unit's own alias names and its model alias types,
    /// the two kinds of named type argument that carry an alias (FSpec FR-322).
    pub(crate) fn with_type_argument_aliases(
        mut self,
        own: BTreeSet<String>,
        model: BTreeMap<String, crate::value::TypeArgumentAlias>,
    ) -> Self {
        self.own_aliases = own;
        self.model_aliases = model;
        self
    }

    /// This scope with its primary source unit.
    pub(crate) fn with_primary_source(mut self, source: RawSourceRef) -> Self {
        self.primary_source = Some(source);
        self
    }

    pub(crate) fn source_of(&self, name: &str) -> Option<&RawSourceRef> {
        self.source_owners.get(name)
    }

    /// The model of `models` that the unit declaring the top-level
    /// declaration `name` reads its model alias `alias` as (FR-480
    /// "Selections and imports").
    pub(crate) fn declared_model<'m>(
        &self,
        models: &'m [super::lowering::AdmittedModel],
        name: &str,
        alias: &str,
    ) -> Option<&'m super::lowering::AdmittedModel> {
        self.model_spellings
            .model(models, self.source_of(name), alias)
    }

    /// This scope as the unit declaring the top-level declaration `name`
    /// reads its type names.
    pub(crate) fn for_declaration(&self, name: &str) -> SourceScope<'_> {
        self.in_source(self.source_of(name))
    }

    pub(crate) fn with_complete_model(mut self, complete_model: bool) -> Self {
        self.complete_model = complete_model;
        self
    }

    /// This scope with the selected models' typed Port declarations, by
    /// package identity and source artifact name.
    pub(crate) fn with_model_ports(
        mut self,
        ports: BTreeMap<(String, String), (DeclarationKey, EffectiveId)>,
    ) -> Self {
        self.model_ports = ports;
        self
    }

    /// Each unit's model aliases bound to their packages.
    pub(crate) fn model_spellings(&self) -> &ModelSpellings {
        &self.model_spellings
    }

    /// The declaration key of the type alias `written` names, when the unit's
    /// own source or a selected model declares one (FSpec FR-322 "Named type
    /// arguments"): a model alias type is its declaration, the unit's own
    /// alias its qualified name beneath its source unit's identity, and a
    /// model scalar names none.
    pub(crate) fn alias_declaration(
        &self,
        written: &str,
    ) -> Option<crate::value::TypeArgumentAlias> {
        if let Some(alias) = self.model_aliases.get(written) {
            return Some(alias.clone());
        }
        if !self.own_aliases.contains(written) {
            return None;
        }
        let identity = self
            .source_owners
            .get(written)
            .or(self.primary_source.as_ref())?
            .identity();
        let path = written.split("::").collect::<Vec<_>>().join("/");
        Some(crate::value::TypeArgumentAlias {
            package: identity.to_owned(),
            node: format!("ix://{identity}/{path}"),
        })
    }

    /// The source unit of each top-level declaration, by declared name.
    pub(crate) fn source_owners(&self) -> &BTreeMap<String, RawSourceRef> {
        &self.source_owners
    }

    /// This scope with the unit's admitted dimension and unit declarations
    /// (FR-091): every unit of `graph` joins the type environment's unit
    /// table, so a quantity of it types and lowers (FR-094), and is named by
    /// its declared name for a quantity literal (FR-185). An empty graph
    /// leaves the scope unchanged.
    pub(crate) fn with_declared_units(
        mut self,
        graph: &filament_semantic_value::unit::UnitGraph,
    ) -> Self {
        if graph.units().next().is_none() {
            return self;
        }
        let declared = filament_semantic_value::quantity::UnitTable::declared(graph);
        let packages = self.model_spellings.package_unit_ids();
        self.index
            .units
            .extend(unit_ids_by_name(graph, &declared, &packages));
        let mut units = self.types.units().clone();
        units.append(declared);
        self.types = std::mem::take(&mut self.types).with_units(units);
        self
    }

    /// The unit `name`, written in the source unit `source`, names as a
    /// quantity's unit id: a declared unit of the unit, or `Alias::unit` of
    /// the admitted package its model alias selects.
    pub(crate) fn unit_named(
        &self,
        source: Option<&RawSourceRef>,
        name: &str,
    ) -> Option<filament_exact::UnitId> {
        match self.model_spellings.spelled(source, name) {
            Spelled::Model(_) => self.model_spellings.unit(source, name),
            Spelled::Other => self.index.units.get(name).cloned(),
        }
    }

    /// The selected packages that declare a unit of the bare name `name`,
    /// for the unresolved-unit-name diagnostic.
    pub(crate) fn unit_packages(&self, source: Option<&RawSourceRef>, name: &str) -> Vec<String> {
        self.model_spellings.declaring_packages(source, name)
    }

    /// This scope with the unit's protocol declarations named `names`, in
    /// declaration order (FR-218). A name declared twice keeps its first.
    pub(crate) fn with_protocols<'n>(mut self, names: impl IntoIterator<Item = &'n str>) -> Self {
        for (position, name) in names.into_iter().enumerate() {
            self.index
                .protocols
                .entry(name.to_owned())
                .or_insert(position);
        }
        self
    }

    /// The index of the unit's protocol declaration written `name`.
    pub(crate) fn protocol_named(&self, name: &str) -> Option<usize> {
        self.index.protocols.get(name).copied()
    }

    /// Every name a type of the unit binds, in name order.
    pub(crate) fn type_names(&self) -> impl Iterator<Item = &str> {
        self.index.types.keys().map(String::as_str)
    }

    /// Whether `name` is a declared type or enum of the unit, which a
    /// protocol reference cannot name (FR-218).
    pub(crate) fn declares_type(&self, name: &str) -> bool {
        self.index.types.contains_key(name)
    }

    /// The checked graph of the admitted import exporting `callee`, where an
    /// imported call to that structural function identity is evaluated.
    pub fn imported_graph(&self, callee: &filament_exact::NodeKey) -> Option<&super::CheckedGraph> {
        self.imports
            .values()
            .find(|import| import.view.exports().any(|(_, node)| node == callee))
            .map(|import| import.graph.as_ref())
    }

    /// The admitted import whose qualifier is `qualifier`.
    pub(crate) fn imported<'a>(&'a self, qualifier: &str) -> Option<&'a super::AdmittedImport> {
        self.imports.get(qualifier)
    }

    pub(crate) fn imported_for_source<'a>(
        &'a self,
        source: Option<&fsl_foundation::source::provenance::RawSourceRef>,
        qualifier: &str,
    ) -> Option<&'a super::AdmittedImport> {
        match source {
            Some(source) => self.imports.get(&source_import_key(source, qualifier)),
            None => self.imported(qualifier),
        }
    }

    /// Every admitted import, for typed consumers that resolve declarations
    /// by their selected model identity rather than the library alias.
    pub(crate) fn imported_packages(&self) -> impl Iterator<Item = &super::AdmittedImport> {
        let mut seen = BTreeSet::new();
        self.imports
            .values()
            .filter(move |import| seen.insert(import.identity.as_str()))
    }

    /// This scope as the source unit `source` (the primary unit when `None`)
    /// reads its type names.
    pub(crate) fn in_source<'a>(&'a self, source: Option<&'a RawSourceRef>) -> SourceScope<'a> {
        SourceScope {
            scope: self,
            source,
        }
    }

    /// Every type `name`, written in the source unit `source`, binds -- an
    /// alias, composite, enum or object type -- in resolution order. Empty
    /// when `name` binds no type.
    pub(crate) fn named_types(&self, source: Option<&RawSourceRef>, name: &str) -> &[ValueType] {
        self.model_spellings.types(source, name, &self.index.types)
    }

    /// Every enum member the qualified name `name` (`E::m`) names, with its
    /// binding. A case is an identifier, so the last `::` separates the
    /// enum's name from the case.
    pub(crate) fn enum_members_named(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (&EnumBinding, &EnumValue)> {
        name.rsplit_once("::")
            .and_then(|(enum_name, case)| self.index.enum_members.get(enum_name)?.get(case))
            .into_iter()
            .flatten()
            .filter_map(|&(binding, member)| {
                let binding = self.enums.get(binding)?;
                Some((binding, binding.members.get(member)?))
            })
    }

    /// The admitted models' variant types, ascending by effective identity.
    pub fn variant_types(&self) -> &[VariantTypeDeclaration] {
        &self.variant_types
    }

    /// The variant type whose effective identity is `key`.
    pub fn variant_type(&self, key: EffectiveId) -> Option<&VariantTypeDeclaration> {
        self.variant_types
            .iter()
            .find(|declared| declared.key() == key)
    }

    /// Whether `name` is the name of an enum of the unit.
    pub(crate) fn names_enum(&self, name: &str) -> bool {
        self.index.enum_members.contains_key(name)
    }

    /// The package's admitted enum declarations, in declaration order.
    pub fn enums(&self) -> &[EnumBinding] {
        &self.enums
    }

    /// Whether `shape` is the enum shape of a variant type.
    pub(crate) fn is_variant_shape(&self, shape: &EnumShape) -> bool {
        self.index.variant_shapes.contains_key(shape)
    }

    /// The variant type whose enum shape is `shape`.
    pub(crate) fn variant_type_of(&self, shape: &EnumShape) -> Option<&VariantTypeDeclaration> {
        self.variant_types
            .get(*self.index.variant_shapes.get(shape)?)
    }

    /// The variant type `name`, written in the source unit `source`, names,
    /// when it is a variant type name.
    pub(crate) fn variant_type_named(
        &self,
        source: Option<&RawSourceRef>,
        name: &str,
    ) -> Option<&VariantTypeDeclaration> {
        match self.model_spellings.spelled(source, name) {
            Spelled::Model(found) => self.variant_type(found?.variant.clone()?),
            Spelled::Other => {
                let (position, _) = *self.index.variant_cases.get(name)?.values().next()?;
                self.variant_types.get(position)
            }
        }
    }

    /// The runtime member of the case `name` (`T::Case`), written in the
    /// source unit `source`, names, with its variant type's enum shape.
    pub(crate) fn variant_case_named(
        &self,
        source: Option<&RawSourceRef>,
        name: &str,
    ) -> Option<(EnumShape, &EnumValue)> {
        let (type_name, case) = name.rsplit_once("::")?;
        let declared = self.variant_type_named(source, type_name)?;
        let member = declared
            .members()
            .iter()
            .find(|value| value.case() == case)?;
        Some((declared.shape(), member))
    }

    /// Whether `name` is the case name of some variant type.
    pub(crate) fn is_variant_case_name(&self, name: &str) -> bool {
        self.index.bare_cases.contains(name)
    }

    /// The first enum binding whose shape is `shape`.
    pub(crate) fn enum_binding_of(&self, shape: &EnumShape) -> Option<&EnumBinding> {
        self.enums.get(self.index.enum_shapes.get(shape)?.0)
    }

    /// The member index holding exactly `shape`'s members (SR-511 M2),
    /// shared, not copied, when `shape` is a declared enum's.
    pub(crate) fn enum_members_of(&self, shape: &EnumShape) -> EnumMemberIndex {
        match self.index.enum_shapes.get(shape) {
            Some((_, members)) => members.clone(),
            None => self
                .index
                .enum_member_index
                .filtered(shape.variants().cloned()),
        }
    }

    /// The object type `name`, written in the source unit `source`, names,
    /// by its effective identity.
    pub(crate) fn object_type_named(
        &self,
        source: Option<&RawSourceRef>,
        name: &str,
    ) -> Option<EffectiveId> {
        match self.model_spellings.spelled(source, name) {
            Spelled::Model(found) => found?.object.clone(),
            // A source object type is seen in the unit that declares it only.
            Spelled::Other if self.model_spellings.is_source_type(name) => None,
            Spelled::Other => self.index.object_types.get(name).cloned(),
        }
    }

    /// ADR-013 T-6 (last sentence): the checked `VariantId -> EnumValue`
    /// index `TypeEnvironment::check_equality_in`'s `Enum` schedule needs to
    /// evaluate a comparison later, and `value::expression::evaluate::Machine`
    /// needs for `OrderedKind::Enums` -- `TypeEnvironment` itself holds no
    /// enum declarations (those are the scope's own enum bindings, ADR-011
    /// §6.1's own module split). Built once with the scope.
    pub fn enum_member_index(&self) -> &EnumMemberIndex {
        &self.index.enum_member_index
    }

    /// Whether `name` is a declared model operation.
    pub(crate) fn declares_model_operation(&self, name: &str) -> bool {
        self.index.model_operations.contains(name)
    }

    /// The package's composite and object type declarations.
    pub fn types(&self) -> &TypeEnvironment {
        &self.types
    }

    /// Seal every declared union under the checked node id of its type
    /// node, `nodes` mapping each declaration to its node (FR-319): each
    /// member is bound to its FSpec FR-441 key over that node id, minted
    /// here once, so the `VariantId` to (union, member, position) index the
    /// type environment keeps is built exactly once and no other stage
    /// derives it. Lowering produced `nodes`; a union it left out is an
    /// internal fault.
    pub(crate) fn seal_unions(
        &mut self,
        nodes: &BTreeMap<filament_exact::NodeKey, filament_exact::NodeKey>,
    ) -> Result<(), CheckRefusal> {
        let fault = |invariant: &'static str| CheckRefusal {
            location: Location::root(
                filament_semantic_value::location::ExpressionOwner::Expression,
            ),
            cause: CheckCause::InternalFault(Box::new(super::refusal::KeyFault::StageFault(
                filament_semantic_value::catalog::InternalFault::new("S3", invariant),
            ))),
        };
        let unions: Vec<(filament_exact::NodeKey, Vec<String>)> = self
            .types
            .composites()
            .filter_map(|declaration| match declaration.shape() {
                CompositeShape::Union(members) => Some((
                    declaration.key(),
                    members
                        .iter()
                        .map(|member| member.identifier().to_owned())
                        .collect(),
                )),
                CompositeShape::Record(_) | CompositeShape::Tuple(_) => None,
            })
            .collect();
        for (handle, members) in unions {
            let node = nodes
                .get(&handle)
                .ok_or_else(|| fault("union-node-missing"))?;
            let bindings = members
                .into_iter()
                .map(|member| filament_exact::UnionMember::from_admitted(node.clone(), member))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| fault("union-empty-member"))?;
            self.types
                .seal_union_verified(handle, node.clone(), bindings)
                .map_err(|_| fault("union-seal"))?;
        }
        Ok(())
    }

    /// Whether a value of `value_type` holds an event type anywhere inside
    /// it: FR-208 gives an event type no equality, so none compares one.
    pub(crate) fn reaches_event(&self, value_type: &ValueType) -> bool {
        if self.model_records.events.is_empty() {
            return false;
        }
        reaches_leaf(
            value_type,
            |leaf| {
                matches!(
                    leaf,
                    ValueType::Composite(declaration)
                        if self.model_records.events.contains(declaration)
                )
            },
            |declaration| self.composite_members(declaration),
        )
    }

    /// Whether a value type contains an IEEE float directly or through an
    /// option, collection, map value, or declared composite field.
    pub(crate) fn reaches_float(&self, value_type: &ValueType) -> bool {
        reaches_leaf(
            value_type,
            |leaf| matches!(leaf, ValueType::Float(_)),
            |declaration| self.composite_members(declaration),
        )
    }

    /// The types a declared composite holds: a record's fields, a tuple's
    /// positions or every position of a union's members.
    fn composite_members(&self, declaration: &filament_exact::NodeKey) -> Vec<&ValueType> {
        match self.types.composite(declaration.clone()).map(|c| c.shape()) {
            Some(CompositeShape::Record(fields)) => {
                fields.iter().map(FieldDeclaration::value_type).collect()
            }
            Some(CompositeShape::Tuple(positions)) => positions.iter().collect(),
            Some(CompositeShape::Union(members)) => members
                .iter()
                .flat_map(|member| member.positions())
                .collect(),
            None => Vec::new(),
        }
    }

    /// The position, in the record `declaration`'s own fields, of the field
    /// `field` names (FR-359 item 2): found in the value's own declaration by
    /// the field's identity, never by a position computed from the record
    /// the read was typed at. `None` when the declaration is no record or
    /// holds no such field.
    pub fn record_slot(
        &self,
        declaration: filament_exact::NodeKey,
        field: &RecordField,
    ) -> Option<usize> {
        let CompositeShape::Record(fields) = self.types.composite(declaration.clone())?.shape()
        else {
            return None;
        };
        let name = self.model_records.own_name(declaration, field);
        fields.iter().position(|candidate| candidate.name() == name)
    }

    pub(crate) fn tuple_data_position(
        &self,
        declaration: &filament_exact::NodeKey,
        position: usize,
    ) -> Option<usize> {
        self.model_records
            .tuple_positions
            .get(declaration)
            .map_or(Some(position), |positions| {
                positions.get(position).copied().flatten()
            })
    }

    /// The package's model object types' declared operations (FR-103).
    pub fn operations(&self) -> &OperationTable {
        &self.operations
    }

    /// The package's admitted IEEE profile, if it selects one.
    pub fn ieee_profile(&self) -> Option<&AdmittedIeeeProfile> {
        self.ieee_profile.as_ref()
    }

    /// The admitted policy selected by this package's declared-float trigger.
    /// An available policy alone does not select a row for a float-free package.
    pub fn ieee_profile_selection(&self) -> Option<&AdmittedIeeeProfile> {
        self.ieee_profile.as_ref().filter(|_| self.ieee_operation)
    }

    /// Select the catalog's IEEE policy when a resolved declaration contains
    /// a float, including unused fields and nested option/collection types.
    pub(crate) fn select_declared_ieee_profile(
        &mut self,
        signatures: &[Signature],
        has_ieee_literal: bool,
    ) -> Result<(), CheckRefusal> {
        let mut declared: Vec<&ValueType> = self.index.types.values().flatten().collect();
        for signature in signatures {
            declared.extend(signature.parameters.iter().map(|(_, ty)| ty));
            declared.push(&signature.result);
        }
        for declaration in self.types.composites() {
            match declaration.shape() {
                CompositeShape::Record(fields) => {
                    declared.extend(fields.iter().map(FieldDeclaration::value_type));
                }
                CompositeShape::Tuple(positions) => declared.extend(positions),
                CompositeShape::Union(members) => {
                    declared.extend(members.iter().flat_map(|member| member.positions()));
                }
            }
        }
        declared.extend(self.types.object_types().flat_map(|declaration| {
            declaration
                .attributes()
                .iter()
                .map(FieldDeclaration::value_type)
        }));
        let has_float = has_ieee_literal || declared.into_iter().any(holds_float);
        self.ieee_operation = has_float;
        if has_float && self.ieee_profile.is_none() {
            let definition = DefinitionRole::IeeeProfile.reference();
            self.ieee_profile =
                Some(
                    admit_ieee_profile(&[definition], &[]).map_err(|cause| CheckRefusal {
                        location: Location::root(Origin::Expression),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::IeeeProfileAdmission(
                            cause,
                        ))),
                    })?,
                );
        }
        Ok(())
    }

    /// The package's FR-151 dispatch operations, indexed by a checked
    /// `NodeKind::Dispatch`'s `operation`.
    pub fn dispatch_operations(&self) -> &[DispatchOperation] {
        &self.dispatch_operations
    }
}

pub(crate) fn source_import_key(
    source: &fsl_foundation::source::provenance::RawSourceRef,
    qualifier: &str,
) -> String {
    format!(
        "\0{}:{}{}:{}{}",
        source.authority().len(),
        source.authority(),
        source.identity().len(),
        source.identity(),
        qualifier
    )
}

/// Every declared function signature, index-aligned with the package's
/// functions, with a by-name index built once so resolving a call
/// is a map lookup, not a scan of every signature.
#[derive(Clone, Debug, Default)]
pub struct Signatures {
    entries: Vec<Signature>,
    /// Each declared name's first positions in `entries`.
    names: BTreeMap<String, NamePositions>,
}

/// One name's first positions among the [`Signatures`] that declare it.
#[derive(Clone, Copy, Debug)]
struct NamePositions {
    /// The first signature with this name, callable by name or not.
    first: usize,
    /// The first one an ordinary named call may resolve to.
    callable: Option<usize>,
}

impl Signatures {
    /// Index `entries` by name.
    pub fn new(entries: Vec<Signature>) -> Self {
        let mut names: BTreeMap<String, NamePositions> = BTreeMap::new();
        for (index, signature) in entries.iter().enumerate() {
            let callable = signature.callable_by_name.then_some(index);
            names
                .entry(signature.name.clone())
                .and_modify(|positions| {
                    positions.callable = positions.callable.or(callable);
                })
                .or_insert(NamePositions {
                    first: index,
                    callable,
                });
        }
        Self { entries, names }
    }

    /// The first signature named `name` that an ordinary named call may
    /// resolve to, with its index.
    pub(crate) fn callable(&self, name: &str) -> Option<(usize, &Signature)> {
        let index = self.names.get(name)?.callable?;
        Some((index, self.entries.get(index)?))
    }

    /// The index of the first signature, callable by name or not, named
    /// `name`.
    pub(crate) fn position(&self, name: &str) -> Option<usize> {
        self.names.get(name).map(|positions| positions.first)
    }

    /// Whether any signature, callable by name or not, is named `name`.
    pub(crate) fn declares(&self, name: &str) -> bool {
        self.names.contains_key(name)
    }

    /// Every signature, in declaration order.
    pub fn as_slice(&self) -> &[Signature] {
        &self.entries
    }
}

impl From<Vec<Signature>> for Signatures {
    fn from(entries: Vec<Signature>) -> Self {
        Self::new(entries)
    }
}

/// A function's declared signature.
#[derive(Clone, Debug)]
pub struct Signature {
    pub(crate) name: String,
    pub(crate) parameters: Vec<(String, ValueType)>,
    pub(crate) result: ValueType,
    /// Whether an ordinary named [`ExprNode::Call`] may resolve to this
    /// signature (TC-196 D07's bypass: a crate-internal FR-151 synthesized
    /// dispatch candidate body or precondition clause is never callable by
    /// plain name, only reachable through a real dispatched call).
    pub(crate) callable_by_name: bool,
}

/// Whether a [`Local`] is an operation/function parameter (bound once, for
/// the whole declaration, never re-bound) or an ordinary bound local (a
/// `let`, or a query/count/sum/accumulate binder) -- the distinction
/// [`Typer::captured_before`] needs for FR-042-AC-3's let-alias rule: only a
/// `Bound` local, never a `Parameter`, counts as "captured" by a `let`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LocalKind {
    Parameter,
    Bound,
}

/// A local name in scope.
struct Local {
    name: String,
    value_type: ValueType,
    slot: Slot,
    location: Location,
    kind: LocalKind,
}

/// One typing pass over a function or standalone expression.
pub(crate) struct Typer<'a> {
    scope: &'a Scope,
    source_owner: Option<fsl_foundation::source::provenance::RawSourceRef>,
    complete_value: bool,
    /// The derived StateModel family of a helper function body.
    model_family: bool,
    signatures: &'a Signatures,
    context: Option<crate::family::CheckContext<'a, Scope>>,
    limits: CheckingLimits,
    nodes: &'a mut u64,
    integer_division_work: u64,
    /// Let slots whose bound integer value lowers at Integer despite a
    /// bounded checked type. Slots are unique throughout this typing pass.
    integer_lowered_locals: BTreeSet<Slot>,
    /// The caller's cancellation handle, polled at every node charge
    /// (FR-276).
    cancel: Option<&'a Cancel>,
    locals: Vec<Local>,
    /// The index in `locals` of the local each name is bound to. A name is
    /// bound at most once at a time ([`Self::bind`] refuses a second
    /// binding), so a name resolves in constant time at any nesting.
    local_names: HashMap<String, usize>,
    slots: usize,
    /// The name each slot was bound under, indexed by slot (FR-092: a
    /// parameter node binds its binder's name).
    slot_names: Vec<String>,
    /// The clause this declaration is (FR-151's dispatch-call restriction;
    /// also the only signal `pre(...)` needs: `pre(...)` is legal exactly in
    /// [`ClauseKind::Postcondition`] -- FR-153's own anchor table;
    /// shared-grammar.md's "self, result and pre(...) are caller-side anchor
    /// operations" -- and refused `wrong_snapshot`/`wrong-anchor` in every
    /// other clause kind, including a standalone `check_expression` call).
    clause_kind: ClauseKind,
    /// The state clause this pass types (FR-104), or `None` for a function,
    /// a measure or a standalone expression: only a state clause binds
    /// `self`, `result` and admits `reaches`.
    state: Option<StateContext>,
    /// The package's quantity units, then every compound unit this pass
    /// formed as a product or quotient type.
    units: UnitScope<'a>,
    /// The nodes typed so far, each stored once a parent names it.
    body: BodyBuilder,
    /// The concrete populations a refinement row reads (FR-139); `None`
    /// outside a refinement declaration, where a population name resolves
    /// through `named` (FR-391) and `seq` refuses
    /// `unsupported_construct`/`expression-form`.
    populations: Option<PopulationScope>,
    /// The populations a population position may name (FR-391); `None`
    /// outside one, where a `M::P` name names no population.
    named: Option<NamedScope>,
    /// The history fields a refinement row reads (FR-138); empty outside a
    /// refinement declaration, where the name of one is a missing field.
    history: Vec<ScopedHistory>,
    /// What a `hyper` clause's condition reads beyond a state clause's
    /// (FR-487); `None` everywhere else.
    hyper: Option<HyperTyping>,
    /// The guarded clock places `present(r.f)` reads by the owner `r`
    /// (FR-230 "Presence is derived"); empty outside a timed predicate.
    pub(super) presence_places: Vec<super::timed_model::PresencePlace>,
}

/// What the typer reads of a `hyper` clause's atom or `match` conjunct
/// (FR-487): an indexed read `e @ t` types as `e` and a step label as the
/// slot [`hyper_lowering::step_slot_name`] names, bound beforehand.
#[derive(Clone, Debug, Default)]
pub(crate) struct HyperTyping {
    /// Hyper roots that must not be rebound by a value-level local.
    pub(crate) reserved_roots: BTreeSet<String>,
    /// The text of each operation a label constant may name, by the
    /// qualified name the constant is written as.
    pub(crate) labels: BTreeMap<String, String>,
    /// The fields of each HP-1 execution variable, with their shared body
    /// binder slots and checked types.
    pub(crate) executions: BTreeMap<String, BTreeMap<String, (usize, ValueType)>>,
}

/// One history field a refinement row's expression may read (FR-138).
#[derive(Clone, Debug)]
pub(crate) struct ScopedHistory {
    /// The concrete object type the refinement adds the field to.
    pub(crate) object_type: EffectiveId,
    /// The field's name.
    pub(crate) name: String,
    /// The field's declared type.
    pub(crate) value_type: ValueType,
}

/// The value of the enum member `member` of `binding`: its identity and its
/// own canonical rank (ADR-013 O-14/OQ-D), which `member.position()` is,
/// verified at `AdmittedEnumDeclaration::admit_member`
/// (`value/enumeration.rs`).
pub(crate) fn enum_value(binding: &EnumBinding, member: &EnumValue) -> Value {
    let rank = u32::try_from(member.position())
        .expect("an admitted enum has far fewer than u32::MAX members");
    Value::Enum(EnumMember::new(
        mint_variant_id(binding.declaration.key(), member.case())
            .expect("an admitted enum member has a nonempty case"),
        rank,
    ))
}

/// The concrete populations a refinement row's expression may name (FR-139,
/// ADR-020 RM-8).
#[derive(Clone, Debug, Default)]
pub(crate) struct PopulationScope {
    pub(crate) populations: Vec<ScopedPopulation>,
}

/// One concrete population a row may name, with the name a row spells it
/// by.
#[derive(Clone, Debug)]
pub(crate) struct ScopedPopulation {
    /// The qualified name a row writes, `Alias::name`.
    pub(crate) name: String,
    /// The population's declaration.
    pub(crate) key: DeclarationKey,
    /// The object type its members are, `T` of `Set<Reference<T>>[0, n]`.
    pub(crate) member: EffectiveId,
}

/// The domain-package populations a population position may name
/// (FR-391): the body of a unit-level state clause or a `holds` atom.
#[derive(Clone, Debug, Default)]
pub(crate) struct NamedScope {
    /// Every population of the unit's models, by the name `M::P` spells.
    pub(crate) populations: Vec<NamedPopulation>,
    /// The model aliases that also name an enum of the unit: a two-segment
    /// name under one is ambiguous.
    pub(crate) ambiguous: Vec<String>,
    /// Every model alias of the unit: a two-segment name under one that
    /// names no population is missing at its second segment.
    pub(crate) aliases: Vec<String>,
}

/// One population a population position may name (FR-391).
#[derive(Clone, Debug)]
pub(crate) struct NamedPopulation {
    /// The qualified name a clause writes, `Alias::P`.
    pub(crate) name: String,
    /// The population's declaration.
    pub(crate) key: DeclarationKey,
    /// Its declared member types.
    pub(crate) members: Vec<EffectiveId>,
    /// The requirement-record domain it adds (FR-104 "Requirements").
    pub(crate) domain: super::state_clause::PopulationDomain,
    /// The unit's authored bound on it (FR-380).
    pub(crate) bound: Option<filament_checked::node::AuthoredBound>,
}

impl NamedScope {
    /// The scope over `models`, with `scope`'s enums for the ambiguity rule.
    pub(crate) fn of(
        models: &[super::lowering::AdmittedModel],
        scope: &Scope,
        unit: Option<&RawSourceRef>,
    ) -> Self {
        let mut populations = Vec::new();
        let mut ambiguous = Vec::new();
        let mut aliases = Vec::new();
        for model in models {
            let Some(alias) = model.alias() else {
                continue;
            };
            // A source model's populations are visible in its own unit only.
            if !model.visible_in(unit) {
                continue;
            }
            aliases.push(alias.to_owned());
            if scope.names_enum(alias) {
                ambiguous.push(alias.to_owned());
            }
            populations.extend(model.named_populations().map(|record| NamedPopulation {
                name: super::source_model::qualified(alias, record.name),
                key: record.key.clone(),
                members: record.members,
                domain: super::state_clause::PopulationDomain {
                    object: record.canonical,
                    ordinal: record.ordinal,
                    maximum: record.bound.map(|bound| bound.maximum),
                },
                bound: record.bound.cloned(),
            }));
        }
        Self {
            populations,
            ambiguous,
            aliases,
        }
    }

    /// This scope narrowed to the populations of the model alias `alias`:
    /// a population of another alias is then missing at its second segment.
    pub(crate) fn of_alias(mut self, alias: &str) -> Self {
        self.populations
            .retain(|population| alias_of(&population.name) == Some(alias));
        self
    }

    /// The population `name` spells, when it spells one.
    pub(crate) fn named(&self, name: &str) -> Option<&NamedPopulation> {
        self.populations
            .iter()
            .find(|population| population.name == name)
    }

    /// The population whose declaration is `key`.
    pub(crate) fn keyed(&self, key: &DeclarationKey) -> Option<&NamedPopulation> {
        self.populations
            .iter()
            .find(|population| &population.key == key)
    }

    /// Whether `name` is two segments whose first is a model alias that
    /// also names an enum of the unit.
    pub(crate) fn is_ambiguous(&self, name: &str) -> bool {
        alias_of(name).is_some_and(|alias| self.ambiguous.iter().any(|known| known == alias))
    }

    /// Whether `name` is two segments whose first is a model alias of the
    /// unit (FR-391 "Resolution").
    pub(crate) fn is_under_alias(&self, name: &str) -> bool {
        alias_of(name).is_some_and(|alias| self.aliases.iter().any(|known| known == alias))
    }
}

/// The first segment of `name` when it is exactly two segments.
fn alias_of(name: &str) -> Option<&str> {
    let mut segments = name.split("::");
    match (segments.next(), segments.next(), segments.next()) {
        (Some(alias), Some(_), None) => Some(alias),
        _ => None,
    }
}

/// A state clause's own typing context (FR-104 "Typing"): its kind, the
/// operation a `pre` or `post` clause names, and the slots `self` and
/// `result` are bound to.
#[derive(Clone, Debug)]
pub(crate) struct StateContext {
    pub(crate) parent_projections:
        BTreeMap<filament_semantic_value::declaration::FieldRef, (ValueType, Presence)>,
    /// The clause kind.
    pub(crate) kind: fsl_forms::StateClauseKind,
    /// The operation's name, for a `pre` or `post` clause.
    pub(crate) operation: Option<String>,
    /// The slot `self` reads.
    pub(crate) self_slot: Slot,
    /// The slot `result` reads: only in a postcondition of an operation
    /// that declares a result.
    pub(crate) result_slot: Option<Slot>,
}

/// A meter that admits no charge: the first charge of an operation is denied,
/// so an operation typed against it computes nothing (`Typer::power`).
pub(super) const NO_CHARGE: filament_exact::ScalarLimits = filament_exact::ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

fn refuse(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
}

/// The refusal a charge returns when it saw a cancelled [`Cancel`]. It is a
/// stand-in for the denial: the operation that owns the handle reports the
/// cancellation, not this refusal.
pub(crate) fn cancelled_refusal(location: &Location) -> CheckRefusal {
    refuse(
        location,
        CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::WorkBudget,
            limit: 0,
            actual: 0,
            region: None,
        })),
    )
}

/// `value_type` with every `Reference<Clock>` replaced by `Boolean`, a leaf
/// the type environment admits wherever a reference is; `None` when a map
/// built from the result would have a key that is no leaf.
fn clock_reference_as_leaf(value_type: &ValueType) -> Option<ValueType> {
    Some(match value_type {
        ValueType::Reference(target) if target == &super::clock_reference_target() => {
            ValueType::Boolean
        }
        ValueType::Option(payload) => {
            ValueType::Option(Arc::new(clock_reference_as_leaf(payload)?))
        }
        ValueType::Collection(collection) => ValueType::collection(CollectionType::new(
            collection.kind(),
            clock_reference_as_leaf(collection.element())?,
            collection.bound(),
        )),
        ValueType::Map(map) => ValueType::Map(Arc::new(
            filament_exact::MapType::new(
                clock_reference_as_leaf(map.key())?,
                clock_reference_as_leaf(map.values())?,
                map.bound(),
            )
            .ok()?,
        )),
        other => other.clone(),
    })
}

fn mismatch(location: &Location) -> CheckRefusal {
    CheckRefusal::ill_typed(location, IllTypedCause::TypeMismatch)
}

fn operand_mismatch(
    location: &Location,
    operator: fsl_forms::BinaryOperator,
    left_type: ValueType,
    right_type: ValueType,
) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause: CheckCause::OperandTypeMismatch {
            operator,
            left_type: Box::new(left_type),
            right_type: Box::new(right_type),
        },
    }
}

fn ordering_binary_operator(operator: OrderingOperator) -> fsl_forms::BinaryOperator {
    match operator {
        OrderingOperator::Less => fsl_forms::BinaryOperator::Less,
        OrderingOperator::LessOrEqual => fsl_forms::BinaryOperator::LessOrEqual,
        OrderingOperator::Greater => fsl_forms::BinaryOperator::Greater,
        OrderingOperator::GreaterOrEqual => fsl_forms::BinaryOperator::GreaterOrEqual,
    }
}

/// Whether the steps `path` read the clock place whose steps are `steps`: a
/// union case is the field read after its union field, and a selector is a
/// slot step. Each selector's index in `path` comes back, in order.
fn named_by(
    steps: &[super::lowering::PlaceStep],
    path: &[(Option<&str>, Option<Location>)],
) -> Option<Vec<usize>> {
    use super::lowering::PlaceStep;
    if steps.len() != path.len() {
        return None;
    }
    let mut slots = Vec::new();
    for (index, (step, read)) in steps.iter().zip(path).enumerate() {
        match (step, read) {
            (PlaceStep::Field(name) | PlaceStep::Union(name), (Some(read), _)) if name == read => {}
            (PlaceStep::Slot(_), (None, Some(_))) => slots.push(index),
            // A different field name, a field read where a slot is, a selector
            // where a field is.
            (PlaceStep::Field(_) | PlaceStep::Union(_), (Some(_), _))
            | (PlaceStep::Field(_) | PlaceStep::Union(_), (None, _))
            | (PlaceStep::Slot(_), (Some(_), _))
            | (PlaceStep::Slot(_), (None, None)) => return None,
        }
    }
    Some(slots)
}

fn quantity_unit(value_type: &ValueType) -> Option<&filament_exact::UnitId> {
    match value_type {
        ValueType::Quantity(unit) => Some(unit),
        ValueType::BoundedQuantity(bounded) => Some(bounded.unit()),
        ValueType::CarriedQuantity(carried) => Some(carried.unit()),
        _ => None,
    }
}

fn uncarried_quantity_unit(value_type: &ValueType) -> Option<&filament_exact::UnitId> {
    match value_type {
        ValueType::Quantity(unit) => Some(unit),
        ValueType::BoundedQuantity(bounded) => Some(bounded.unit()),
        _ => None,
    }
}

/// Exact scalar operands that preserve the explicitly written quantity unit
/// under FR-142 quantity scaling.
fn is_exact_quantity_factor(value_type: &ValueType) -> bool {
    matches!(
        value_type,
        ValueType::Integer | ValueType::Int(_) | ValueType::Rational(_) | ValueType::Decimal(_)
    )
}

fn ineligible(location: &Location) -> CheckRefusal {
    CheckRefusal::ill_typed(location, IllTypedCause::OperatorIneligible)
}

fn node(kind: NodeKind<NodeId>, value_type: ValueType, location: &Location) -> Node {
    Node {
        kind,
        value_type,
        location: location.clone(),
        equality_producer: None,
        field_population: None,
        equality_range_context: None,
        type_argument: None,
        binder_annotation: None,
    }
}

fn is_integer(value_type: &ValueType) -> bool {
    matches!(value_type, ValueType::Integer | ValueType::Int(_))
}

/// A comparison of a guarded clock place with `absent`, read as the place's
/// presence (FR-230 "Presence and element members").
pub(super) struct AbsentComparison<'e> {
    /// The place's owner `r` of `r.f` or `r.f.c`.
    pub(super) owner: ExprRef<'e>,
    /// The owner's location.
    pub(super) owner_location: Location,
    /// The place's field.
    pub(super) field: String,
    /// The union case of the place, when it is one.
    pub(super) case: Option<String>,
    /// Whether the comparison requires the place present (`!=`).
    pub(super) present: bool,
}

/// Whether `expression`'s own syntax contains a form `pre(...)`'s anchor can
/// act on: `allInstances`/`lookup` (the only reads FR-153 anchors), or a
/// nested `pre(...)` (idempotent by construction: a nested `pre` with no
/// eligible read of its own is refused independently when *it* is checked).
///
/// `Call` is deliberately *not* one of these forms, even though a call's own
/// body is legitimately anchor-sensitive: `Expression::Call` itself is never
/// eligible, but `Expression::children()` still walks into its own
/// `arguments`, so `pre(F(allInstances(p)))` stays eligible (the eligible
/// read lives in the argument, exactly as FR-042-AC-1's own example,
/// `pre(P(self.version, delta))`, needs), while `pre(F(p))` -- bare-name
/// arguments only -- is not: `F`'s callee body is a separate declaration
/// this checker cannot see into (`children()` never exposes it), so treating
/// the call itself as eligible would accept a `pre(...)` whose own visible
/// syntax is nothing but a bare parameter, silently returning `F`'s *post*
/// value (its own body runs at `Anchor::Post`, `evaluate.rs`'s
/// `NodeKind::Call` reset) under a `pre` label.
///
/// This is a syntactic, pre-typing check on purpose: `let v = allInstances(p)
/// in pre(v)` and `let q = pre(p) in allInstances(q)` (FR-042-AC-3's capture-
/// drift shapes) both fail it, because `pre(...)`'s own operand in each case
/// is nothing but a bare `Name` -- the eligible read that produced `v`, or
/// the population `pre(p)` never actually reads, sits in a sibling/ancestor
/// `Let`, not in this `pre(...)`'s own subtree. `pre(1)` and `pre(delta)`
/// (a bare parameter) fail it for the same reason: a literal or a `Name`
/// alone is never itself an eligible read.
///
/// The walk keeps its pending sub-expressions on a heap stack: it runs
/// before the typer has entered `expression`'s own levels, and its stack
/// grows by one entry per node it names, which the typer then charges.
fn contains_pre_eligible_read(expression: ExprRef<'_>) -> bool {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        if matches!(
            expression.node(),
            ExprNode::AllInstances { .. } | ExprNode::Lookup { .. } | ExprNode::Pre(_)
        ) {
            return true;
        }
        pending.extend(expression.children());
    }
    false
}

/// The name a reference expression reads through, looking past `deref`,
/// `value` and field projections: `target` for `deref(value(target.f))`,
/// `None` when the reference is not rooted at a name.
fn reference_root(expression: ExprRef<'_>) -> Option<&str> {
    let mut expression = expression;
    loop {
        match expression.node() {
            ExprNode::Name(name) => return Some(name.name.as_str()),
            ExprNode::Deref(operand)
            | ExprNode::Value(operand)
            | ExprNode::Field { operand, .. } => {
                expression = expression.at(*operand);
            }
            _ => return None,
        }
    }
}

/// One `let` a captured-alias walk has passed: its name, whether
/// its value resolves to a captured alias, and the binding it was made
/// under, as an index into the walk's binding arena. A chain of `outer`
/// links from one binding is the `let` scope, innermost first.
struct AliasBinding<'e> {
    name: &'e str,
    alias: bool,
    outer: Option<usize>,
}

/// One pending step of [`Typer::resolves_to_captured_alias`]'s walk.
enum AliasStep<'e> {
    /// Resolve an operand under the scope ending at the binding index.
    Operand(ExprRef<'e>, Option<usize>),
    /// The value of `let name = value in body` is resolved on top of the
    /// result stack: bind it, then resolve `body`.
    Bind(&'e str, ExprRef<'e>, Option<usize>),
    /// One branch of an `if` is resolved on top of the result stack: keep a
    /// `true`, otherwise resolve the other branch.
    OrElse(ExprRef<'e>, Option<usize>),
}

/// Whether a call's callee names the `Text` constructor.
#[ix_trace_rs::string_edge]
fn is_text_constructor(callee: &str) -> bool {
    callee == "Text"
}

/// Whether an expression takes its type from its context.
fn contextual(expression: ExprRef<'_>) -> bool {
    match expression.node() {
        ExprNode::Integer(_)
        | ExprNode::Float32(_)
        | ExprNode::Float64(_)
        | ExprNode::Rational(..)
        | ExprNode::DecimalValue { .. }
        | ExprNode::Collection { .. }
        | ExprNode::Absent => true,
        ExprNode::Call { name, .. } if is_text_constructor(&name.name) => true,
        ExprNode::Negate(operand) => matches!(
            expression.at(*operand).node(),
            ExprNode::Integer(_) | ExprNode::DecimalValue { .. }
        ),
        ExprNode::Binary {
            operator: BinaryOperator::Divide,
            ..
        } => true,
        _ => false,
    }
}

impl Typer<'_> {
    /// Admit `node` where `required` is expected: identical types, a record
    /// into a record type it conforms to (FR-359), an integer into `Integer` or
    /// a containing `Int[..]`, or an integer into another `Int[..]` through a
    /// range-obligated [`NodeKind::Coerce`].
    pub(crate) fn coerce(
        &mut self,
        node: Node,
        required: &ValueType,
    ) -> Result<Node, CheckRefusal> {
        if &node.value_type == required {
            if let (NodeKind::IntegerDivision { .. } | NodeKind::Local(_), ValueType::Int(target)) =
                (&node.kind, required)
            {
                if self.lowers_as_integer(&node) {
                    let location = node.location.clone();
                    return Ok(Node {
                        kind: NodeKind::Coerce(self.push(node), target.clone()),
                        value_type: required.clone(),
                        location,
                        equality_producer: None,
                        field_population: None,
                        equality_range_context: None,
                        type_argument: None,
                        binder_annotation: None,
                    });
                }
            }
            return Ok(node);
        }
        match (&node.value_type, required) {
            // FR-359 items 1 and 6: a record whose most-specific type
            // conforms to the slot's record type is admitted as it is, with
            // no conversion node; its own type stays on the node. A type
            // constructor over a record is not covariant, so only the slot's
            // own record type reaches this arm.
            (ValueType::Composite(sub), ValueType::Composite(sup))
                if self
                    .scope
                    .types
                    .composite_conforms(sub.clone(), sup.clone()) =>
            {
                Ok(node)
            }
            (ValueType::Integer | ValueType::Int(_), ValueType::Integer) => Ok(node),
            (ValueType::Int(source), ValueType::Int(target))
                if matches!(
                    node.kind,
                    NodeKind::IntegerDivision { .. } | NodeKind::Local(_)
                ) && target.lower() <= source.lower()
                    && source.upper() <= target.upper()
                    && self.lowers_as_integer(&node) =>
            {
                let location = node.location.clone();
                Ok(Node {
                    kind: NodeKind::Coerce(self.push(node), target.clone()),
                    value_type: required.clone(),
                    location,
                    equality_producer: None,
                    field_population: None,
                    equality_range_context: None,
                    type_argument: None,
                    binder_annotation: None,
                })
            }
            (ValueType::Int(source), ValueType::Int(target))
                if target.lower() <= source.lower() && source.upper() <= target.upper() =>
            {
                Ok(node)
            }
            (ValueType::Integer | ValueType::Int(_), ValueType::Int(target)) => {
                let location = node.location.clone();
                Ok(Node {
                    kind: NodeKind::Coerce(self.push(node), target.clone()),
                    value_type: required.clone(),
                    location,
                    equality_producer: None,
                    field_population: None,
                    equality_range_context: None,
                    type_argument: None,
                    binder_annotation: None,
                })
            }
            _ => Err(mismatch(&node.location)),
        }
    }
}

fn bound(
    minimum: u64,
    maximum: u64,
    location: &Location,
) -> Result<CardinalityBound, CheckRefusal> {
    CardinalityBound::new(minimum, maximum)
        .map_err(|_| refuse(location, CheckCause::UnrepresentableBound))
}

impl<'a> Typer<'a> {
    pub(crate) fn new(
        scope: &'a Scope,
        signatures: &'a Signatures,
        limits: CheckingLimits,
        nodes: &'a mut u64,
        clause_kind: ClauseKind,
    ) -> Self {
        Self {
            scope,
            source_owner: None,
            complete_value: scope.complete_value,
            model_family: false,
            signatures,
            context: None,
            limits,
            nodes,
            integer_division_work: 0,
            integer_lowered_locals: BTreeSet::new(),
            cancel: None,
            locals: Vec::new(),
            local_names: HashMap::new(),
            slots: 0,
            slot_names: Vec::new(),
            clause_kind,
            state: None,
            units: UnitScope::new(scope.types.units()),
            body: BodyBuilder::default(),
            populations: None,
            named: None,
            history: Vec::new(),
            hyper: None,
            presence_places: Vec::new(),
        }
    }

    #[must_use]
    pub(crate) fn with_source_owner(
        mut self,
        source: Option<fsl_foundation::source::provenance::RawSourceRef>,
    ) -> Self {
        // A nested model check reads type names through the same unit.
        self.context = self
            .context
            .take()
            .map(|cx| cx.with_source_owner(source.clone()));
        self.source_owner = source;
        self
    }

    /// The source unit this pass checks, `None` for the primary unit.
    pub(crate) fn source_owner(&self) -> Option<&RawSourceRef> {
        self.source_owner.as_ref()
    }

    /// The package scope as this pass's source unit reads its type names.
    pub(crate) fn source_scope(&self) -> SourceScope<'_> {
        self.scope.in_source(self.source_owner.as_ref())
    }

    pub(crate) fn imported(&self, qualifier: &str) -> Option<&'a super::AdmittedImport> {
        let scope: &'a Scope = self.scope;
        scope.imported_for_source(self.source_owner.as_ref(), qualifier)
    }

    /// The static range of the integer `node` computes (FR-139).
    pub(crate) fn static_range(&self, node: &Node) -> Option<super::ir::IntegerRange> {
        super::ir::static_integer_range((&node.kind, &node.value_type), &|id| {
            self.body
                .get(id)
                .map(|found| (&found.kind, &found.value_type))
        })
    }

    /// Whether an integer argument already has the catalog's Integer type
    /// when lowered, so its call needs no synthetic widening conversion.
    fn lowers_as_integer(&self, root: &Node) -> bool {
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            match &node.value_type {
                ValueType::Integer => return true,
                ValueType::Int(_) => {}
                _ => continue,
            }
            let children = match &node.kind {
                NodeKind::IntegerDivision { .. } => return true,
                NodeKind::Local(slot) if self.integer_lowered_locals.contains(slot) => return true,
                NodeKind::If {
                    then, otherwise, ..
                } => vec![*then, *otherwise],
                NodeKind::Let { body, .. } => vec![*body],
                NodeKind::Case { arms, .. } => arms.iter().map(|arm| arm.body).collect(),
                _ => Vec::new(),
            };
            pending.extend(children.into_iter().filter_map(|id| self.body.get(id)));
        }
        false
    }

    /// Whether an unconverted integer equality operand lowers at the builtin
    /// `Integer` node by the rule `lowers_as_integer` states (the typer form of
    /// `lowered_integer_types`): FR-495-AC-24 compares such an operand as that
    /// node, never as its checked interval.
    fn equality_operand_lowers_as_integer(&self, operand: &TypedOperand) -> bool {
        operand.conversion_target.is_none()
            && matches!(operand.node.value_type, ValueType::Int(_) | ValueType::Integer)
            && self.lowers_as_integer(&operand.node)
    }

    /// This pass reading the concrete populations `populations` as a
    /// refinement row does (FR-139).
    #[must_use]
    pub(crate) fn with_populations(mut self, populations: PopulationScope) -> Self {
        self.populations = Some(populations);
        self
    }

    /// This pass typing a population position (FR-391): `M::P` names a
    /// population of `named`.
    #[must_use]
    pub(crate) fn with_named_populations(mut self, named: NamedScope) -> Self {
        self.named = Some(named);
        self
    }

    /// This pass typing a timed predicate, whose `present(r.f)` may read
    /// the guarded clock places `places` (FR-230 "Presence is derived").
    #[must_use]
    pub(crate) fn with_presence_places(
        mut self,
        places: Vec<super::timed_model::PresencePlace>,
    ) -> Self {
        self.presence_places = places;
        self
    }

    fn clock_presence_places(&self) -> &[super::timed_model::PresencePlace] {
        if self.presence_places.is_empty() {
            &self.scope.clock_presence_places
        } else {
            &self.presence_places
        }
    }

    /// This pass typing the atom or `match` conjunct of a `hyper` clause
    /// (FR-487).
    #[must_use]
    pub(crate) fn with_hyper(mut self, hyper: HyperTyping) -> Self {
        self.hyper = Some(hyper);
        self
    }

    /// This pass reading the history fields `history` as a refinement row
    /// does (FR-138).
    #[must_use]
    pub(crate) fn with_history(mut self, history: Vec<ScopedHistory>) -> Self {
        self.history = history;
        self
    }

    /// This pass polling `cancel` at every node charge (FR-276). A cancelled
    /// handle refuses the next node as a reached work budget; the caller
    /// that owns the handle reads [`Cancel::tripped`] to tell the two apart.
    #[must_use]
    pub(crate) fn with_cancel(mut self, cancel: Option<&'a Cancel>) -> Self {
        self.cancel = cancel;
        self
    }

    pub(crate) fn with_complete_value(mut self, complete_value: bool) -> Self {
        self.complete_value = complete_value;
        self
    }

    /// Type this body as a derived StateModel helper family.
    #[must_use]
    pub(crate) fn with_model_family(mut self, derived: bool) -> Self {
        self.model_family = derived;
        self
    }

    /// Appends a resolved predicate call to the enclosing check's stage
    /// state. A standalone pass without a stage context has no package to
    /// report to and records nothing.
    pub(crate) fn record_dependency_reference(
        &mut self,
        reference: super::PendingDependencyReference,
    ) {
        if let Some(cx) = self.context.as_mut() {
            cx.diagnostics.record_dependency_reference(reference);
        }
    }

    /// Borrow the enclosing family's stage state for nested model checks.
    /// The context's source unit, when it names one, is the unit this pass
    /// checks; otherwise the context takes this pass's unit, so a nested
    /// model check (`allInstances<T>`, `lookup<T>`) reads `T` through the
    /// unit that writes it (FR-480).
    pub(crate) fn with_context(mut self, cx: crate::family::CheckContext<'a, Scope>) -> Self {
        let cx = match cx.source_owner() {
            Some(owner) => {
                self.source_owner = Some(owner.clone());
                cx
            }
            None => cx.with_source_owner(self.source_owner.clone()),
        };
        self.context = Some(cx);
        self
    }

    fn model_check(
        &mut self,
        form: super::state_model::Form,
        location: &Location,
    ) -> Result<super::state_model::Checked, CheckRefusal> {
        use crate::family::FamilyContract;
        use fsl_foundation::diagnostic::StageFailure;
        let cx = self
            .context
            .as_mut()
            .ok_or_else(|| super::state_model::fault(location))?;
        super::state_model::StateModelFamily::check(
            &super::state_model::Request {
                form,
                location: location.clone(),
            },
            cx,
        )
        .map(|staged| staged.into_value())
        .map_err(|failure| match failure {
            StageFailure::Refused(refusal) => refusal,
            StageFailure::RefusedWithPayload(refusal, _) => *refusal,
            StageFailure::Limit(limit) => CheckRefusal {
                location: location.clone(),
                cause: super::limit_cause(&limit),
            },
            StageFailure::LimitWithPayload(limit, _) => CheckRefusal {
                location: location.clone(),
                cause: super::limit_cause(&limit),
            },
            StageFailure::Fault(fault) => CheckRefusal {
                location: location.clone(),
                cause: CheckCause::InternalFault(Box::new(super::refusal::KeyFault::StageFault(
                    fault,
                ))),
            },
            StageFailure::Cancelled(_) => super::state_model::fault(location),
        })
    }

    fn model_node(
        &mut self,
        form: super::state_model::Form,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match self.model_check(form, location)? {
            super::state_model::Checked::Node(node) => Ok(*node),
            _ => Err(super::state_model::fault(location)),
        }
    }

    fn model_type(
        &mut self,
        form: super::state_model::Form,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        match self.model_check(form, location)? {
            super::state_model::Checked::Type(value_type) => Ok(value_type),
            _ => Err(super::state_model::fault(location)),
        }
    }

    /// Store `node` as an operand of the node being built, and name it.
    fn push(&mut self, node: Node) -> NodeId {
        self.body.push(node)
    }

    /// Type this pass as the state clause `state` (FR-104): `self`, `result`
    /// and `reaches` resolve against it. Its slots are already bound.
    pub(crate) fn enter_state_clause(&mut self, state: StateContext) {
        self.state = Some(state);
    }

    /// Whether this pass types a state clause.
    pub(crate) fn in_state_clause(&self) -> bool {
        self.state.is_some()
    }

    /// The number of slots allocated so far.
    pub(crate) fn slots(&self) -> usize {
        self.slots
    }

    /// The name each slot was bound under, indexed by slot.
    pub(crate) fn slot_names(&self) -> &[String] {
        &self.slot_names
    }

    /// The declared name of `slot` (FR-400 "Canonical read projections"). A slot
    /// bound without a recorded name is an internal fault, never an empty name.
    pub(crate) fn slot_name(
        &self,
        slot: Slot,
        location: &Location,
    ) -> Result<String, CheckRefusal> {
        self.slot_names.get(slot).cloned().ok_or_else(|| {
            refuse(
                location,
                CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                    filament_semantic_value::catalog::InternalFault::new(
                        "S3",
                        "checked-binding-slot-has-a-declared-name",
                    ),
                ))),
            )
        })
    }

    /// Seed this typing pass with the checked graph's retained compound
    /// units so an externally supplied frame can name one as a value type.
    pub(crate) fn include_formed_units(&mut self, formed: &FormedUnits) {
        formed.seed(&mut self.units);
    }

    /// Every compound unit this pass formed (FR-094: kept until lowering
    /// keys each one's type node).
    pub(crate) fn into_formed_units(self) -> filament_semantic_value::quantity::UnitTable {
        self.units.into_formed()
    }

    /// The package-level declarations this typing pass resolves names
    /// against. `check::family::Application` (the relocated
    /// function-application checker) reads this to resolve a call's callee
    /// against `model_operations` and tuple-constructor types, the same way
    /// this `Typer`'s own other methods already do.
    pub(crate) fn scope(&self) -> &'a Scope {
        self.scope
    }

    /// Every function signature this typing pass may resolve an ordinary
    /// named [`ExprNode::Call`] against. `check::family::
    /// Application` reads this for name resolution and arity
    /// checking, exactly as this `Typer`'s own (now deleted) `call` method
    /// did.
    pub(crate) fn signatures(&self) -> &'a Signatures {
        self.signatures
    }

    /// Bind a parameter or local name, refusing a name in FR-488's reserved
    /// slot form or already in scope.
    pub(crate) fn bind(
        &mut self,
        name: &str,
        value_type: ValueType,
        location: &Location,
        kind: LocalKind,
    ) -> Result<Slot, CheckRefusal> {
        if let Some(refusal) = reserved_slot_refusal(name, None, location) {
            return Err(refusal);
        }
        if kind == LocalKind::Bound
            && self
                .hyper
                .as_ref()
                .is_some_and(|hyper| hyper.reserved_roots.contains(name))
        {
            return Err(refuse(
                location,
                CheckCause::AmbiguousName {
                    name: name.to_owned(),
                    loci: vec![location.clone()],
                },
            ));
        }
        if let Some(existing) = self.local(name) {
            return Err(refuse(
                location,
                CheckCause::AmbiguousName {
                    name: name.to_owned(),
                    loci: vec![existing.location.clone(), location.clone()],
                },
            ));
        }
        let slot = self.slots;
        self.slots = self.slots.saturating_add(1);
        self.slot_names.push(name.to_owned());
        self.local_names.insert(name.to_owned(), self.locals.len());
        self.locals.push(Local {
            name: name.to_owned(),
            value_type,
            slot,
            location: location.clone(),
            kind,
        });
        Ok(slot)
    }

    fn unbind(&mut self, count: usize) {
        let keep = self.locals.len().saturating_sub(count);
        for local in self.locals.drain(keep..) {
            self.local_names.remove(&local.name);
        }
    }

    /// The local bound to `name`, if any.
    fn local(&self, name: &str) -> Option<&Local> {
        self.locals.get(*self.local_names.get(name)?)
    }

    /// Whether `name` resolves (innermost binding first, matching ordinary
    /// shadowing) to a [`LocalKind::Bound`] local -- never a
    /// [`LocalKind::Parameter`] -- whose own slot was allocated before
    /// `boundary`. `boundary` is `self.slots` at the point this `pre(...)`'s
    /// own operand began checking, so a slot below it was bound outside that
    /// operand: a `let`, or a query/count/sum/accumulate binder, introduced
    /// before this `pre(...)` was ever reached. A parameter never counts,
    /// even though it is also always bound before `boundary`: it is bound
    /// once, for the whole declaration, and FR-042-AC-3's own analogue
    /// (`let s = self in pre(s.version)`) is specifically about a `let`
    /// re-binding a state root, never about the root parameter itself
    /// (`pre(self.version)`/`pre(allInstances(p))` stay legal).
    fn captured_before(&self, name: &str, boundary: usize) -> bool {
        self.local(name)
            .is_some_and(|local| local.kind == LocalKind::Bound && local.slot < boundary)
    }

    /// FR-042-AC-3's `let s = self in pre(s.version)` analogue, generalized
    /// to `allInstances`/`lookup`: whether `expression`'s own syntax reads
    /// one of them over a population operand that [`Self::resolves_to_captured_alias`]
    /// resolves to a `let`-bound alias of a state root, captured *outside*
    /// this `pre(...)`'s own operand, re-anchored only because the read
    /// syntax happens to sit inside it.
    ///
    /// The walk tracks every name a `let` *within this same walk* (i.e.
    /// within this `pre(...)`'s own operand) has since rebound, most recent
    /// last, paired with whether *that* binding's own value is itself a
    /// captured alias. A rebinding to a fresh, non-alias value (say,
    /// `allInstances(other)`) is `false` -- this operand's own `let`
    /// introducing it for itself, never "outside" it, so it does not falsely
    /// trip this check inside its own body. A rebinding that is itself a
    /// captured alias, however it is spelled -- a bare reference
    /// (`let r = q in ...`), an `if` with an aliasing arm (`let r = if c
    /// then q else q in ...`), or a further nested `let` resolving to one
    /// (`let r = (let s = q in s) in ...`) -- is `true`: aliasing an alias
    /// is still aliasing, transitively, through any of the shapes FR-153
    /// lets a population-typed expression take between `let`s, however many
    /// sit in between (PR #168 review round 4, finding 1). Neither FR-042
    /// nor FR-153's own spec text states a "bare identifier only"
    /// restriction for this specific alias-capture rule -- `bind_parameters`'s
    /// own FR-153 "direct operand" doc governs the unrelated `Population`
    /// *value-type* placement, not this syntactic alias check -- so this
    /// resolves the general case rather than special-casing one syntax.
    ///
    /// The walk and [`Self::resolves_to_captured_alias`] keep their pending
    /// sub-expressions and `let` bindings on the heap: they run before the
    /// typer has entered the operand's own levels, and each stack grows by
    /// one entry per node or `let` they name, which the typer then charges.
    fn contains_captured_pre_alias(&self, expression: ExprRef<'_>, boundary: usize) -> bool {
        let mut bindings: Vec<AliasBinding<'_>> = Vec::new();
        let mut pending: Vec<(ExprRef<'_>, Option<usize>)> = vec![(expression, None)];
        while let Some((expression, scope)) = pending.pop() {
            match expression.node() {
                ExprNode::AllInstances { population, .. } | ExprNode::Lookup { population, .. } => {
                    let population = expression.at(*population);
                    if self.resolves_to_captured_alias(population, boundary, &mut bindings, scope) {
                        return true;
                    }
                }
                ExprNode::Let {
                    name, value, body, ..
                } => {
                    let (value, body) = (expression.at(*value), expression.at(*body));
                    let alias =
                        self.resolves_to_captured_alias(value, boundary, &mut bindings, scope);
                    bindings.push(AliasBinding {
                        name,
                        alias,
                        outer: scope,
                    });
                    pending.push((body, Some(bindings.len() - 1)));
                    pending.push((value, scope));
                    continue;
                }
                _ => {}
            }
            pending.extend(
                expression
                    .children()
                    .into_iter()
                    .map(|child| (child, scope)),
            );
        }
        false
    }

    /// Whether `operand` -- a population-typed sub-expression this walk is
    /// considering as `allInstances`/`lookup`'s direct operand, or as a
    /// `let`'s own value -- resolves to a captured alias of a state root
    /// bound outside this `pre(...)`'s operand, however many `let`s or `if`
    /// branches it is spelled through. `scope` is the innermost binding in
    /// `bindings` the operand sits under; the bindings its own `let`s make
    /// are appended to `bindings`.
    ///
    /// - [`ExprNode::Name`] resolves against the scope first (innermost
    ///   within this walk wins, matching ordinary shadowing), falling back
    ///   to [`Self::captured_before`] for a name this walk never rebound.
    /// - [`ExprNode::Let`] resolves its own value first (the value may
    ///   itself be a further `let`/`if`), binds that result as `name`, and
    ///   resolves through its body under that extended scope.
    /// - [`ExprNode::If`] resolves `true` when *either* branch does: a
    ///   checker refusing statically cannot rule out the branch that
    ///   escapes, so both must be clear.
    /// - Every other shape is not itself alias-bearing syntax and resolves
    ///   `false`.
    fn resolves_to_captured_alias<'e>(
        &self,
        operand: ExprRef<'e>,
        boundary: usize,
        bindings: &mut Vec<AliasBinding<'e>>,
        scope: Option<usize>,
    ) -> bool {
        let mut resolved: Vec<bool> = Vec::new();
        let mut pending = vec![AliasStep::Operand(operand, scope)];
        while let Some(step) = pending.pop() {
            match step {
                AliasStep::Operand(operand, scope) => match operand.node() {
                    ExprNode::Name(name) => {
                        let mut at = scope;
                        let mut alias = None;
                        while let Some(binding) = at.and_then(|index| bindings.get(index)) {
                            if binding.name == name.name {
                                alias = Some(binding.alias);
                                break;
                            }
                            at = binding.outer;
                        }
                        resolved.push(
                            alias.unwrap_or_else(|| self.captured_before(&name.name, boundary)),
                        );
                    }
                    ExprNode::Let {
                        name, value, body, ..
                    } => {
                        pending.push(AliasStep::Bind(name, operand.at(*body), scope));
                        pending.push(AliasStep::Operand(operand.at(*value), scope));
                    }
                    ExprNode::If {
                        then, otherwise, ..
                    } => {
                        pending.push(AliasStep::OrElse(operand.at(*otherwise), scope));
                        pending.push(AliasStep::Operand(operand.at(*then), scope));
                    }
                    _ => resolved.push(false),
                },
                AliasStep::Bind(name, body, scope) => {
                    let alias = resolved.pop() == Some(true);
                    bindings.push(AliasBinding {
                        name,
                        alias,
                        outer: scope,
                    });
                    pending.push(AliasStep::Operand(body, Some(bindings.len() - 1)));
                }
                AliasStep::OrElse(otherwise, scope) => {
                    if resolved.last() == Some(&false) {
                        resolved.pop();
                        pending.push(AliasStep::Operand(otherwise, scope));
                    }
                }
            }
        }
        resolved.pop() == Some(true)
    }

    fn enter(&mut self, location: &Location) -> Result<(), CheckRefusal> {
        // The refused entry would have taken the counter one past the
        // current one (the actual counter a rendered diagnostic
        // names, not folded into the bound).
        let exhausted = |kind, limit: u64, actual: u64| {
            refuse(
                location,
                CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                    stage: CheckingStage::Typing,
                    kind,
                    limit,
                    actual: u128::from(actual),
                    region: None,
                })),
            )
        };
        if self.cancel.is_some_and(Cancel::poll) {
            return Err(cancelled_refusal(location));
        }
        if *self.nodes >= self.limits.nodes() {
            return Err(exhausted(
                CheckingLimitKind::Nodes,
                self.limits.nodes(),
                self.nodes.saturating_add(1),
            ));
        }
        *self.nodes = self.nodes.saturating_add(1);
        Ok(())
    }

    /// Check a declared type: its named declarations, reference targets, keyed
    /// element types and enum declarations.
    pub(crate) fn check_declared_type(
        &self,
        value_type: &ValueType,
        location: &Location,
    ) -> Result<(), CheckRefusal> {
        // `Reference<Clock>` names no declared object type, so the whole type
        // is checked with each clock reference read as a leaf of its own.
        let whole = clock_reference_as_leaf(value_type).ok_or_else(|| mismatch(location))?;
        self.scope
            .types
            .try_check_type(&whole)
            .map_err(|refusal| CheckRefusal::from_type_check(location, refusal))?;
        let mut pending = vec![value_type];
        while let Some(value_type) = pending.pop() {
            match value_type {
                ValueType::Enum(shape) => {
                    // The checker only ever builds a `ValueType::Enum` from
                    // one of `self.scope.enums`' own bindings (`Typer::name`,
                    // `resolve_named_type`), never from a caller-supplied
                    // shape, so a shape naming no admitted binding is a
                    // checker-internal mismatch, not something an author's
                    // source can trigger today. The check stays here as
                    // defense in depth, matching `Composite`/`Reference`'s
                    // own (`TypeEnvironment`-level) declaration checks.
                    if self.scope.enum_binding_of(shape).is_none()
                        && !self.scope.is_variant_shape(shape)
                    {
                        return Err(mismatch(location));
                    }
                }
                ValueType::Reference(target) if target == &super::clock_reference_target() => {
                    // `Reference<Clock>` is the checker's synthetic handle
                    // type for a clock place. The native Clock declaration
                    // is not an object type in the admitted TypeEnvironment.
                }
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Map(map) => {
                    pending.push(map.key());
                    pending.push(map.values());
                }
                ValueType::Reference(_) | ValueType::Composite(_) => self
                    .scope
                    .types
                    .try_check_type(value_type)
                    .map_err(|refusal| CheckRefusal::from_type_check(location, refusal))?,
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Float(_)
                | ValueType::Quantity(_)
                | ValueType::BoundedQuantity(_)
                | ValueType::CarriedQuantity(_)
                | ValueType::Text(_)
                | ValueType::String
                | ValueType::Population(_)
                | ValueType::Uuid
                | ValueType::Timestamp
                | ValueType::Bytes
                | ValueType::JsonObject => self
                    .scope
                    .types
                    .try_check_type(value_type)
                    .map_err(|refusal| CheckRefusal::from_type_check(location, refusal))?,
            }
        }
        Ok(())
    }

    /// Resolve a qualified type name: an alias, a record or tuple, or an enum.
    ///
    /// `pub(crate)`: `check::family::Application` (the
    /// relocated function-application checker) calls this the same way this
    /// `Typer`'s own (now deleted) `call` method did, to resolve a call's
    /// callee against a tuple-constructor type when no function signature
    /// matches.
    pub(crate) fn type_named(
        &self,
        name: &str,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        super::type_form::resolve_named_type(&self.source_scope(), name, location)
    }

    /// Resolve a declared `TypeForm` (S2) to the kernel `ValueType` (E3,
    /// ADR-013 O-14/C-26). The one production entry into
    /// [`super::type_form::resolve_type_form`], which [`Self::type_named`]
    /// above's [`super::type_form::resolve_named_type`] also backs for the
    /// qualified-name case.
    pub(crate) fn resolve_type(
        &self,
        form: &fsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        super::type_form::resolve_type_form(&self.source_scope(), form, location)
    }

    fn element_type(&self, collection: &Node) -> Result<ValueType, CheckRefusal> {
        match &collection.value_type {
            ValueType::Collection(collection_type) => Ok(collection_type.element().clone()),
            _ => Err(mismatch(&collection.location)),
        }
    }

    /// `absent`: the absent option of the `Option<T>` its context expects
    /// (FR-230 "Presence bit"; STD-493 ruling: an `absent` option literal
    /// typed by context). With no expected type it is ambiguous; where no
    /// option is expected it is a type mismatch.
    fn absent_literal(
        &self,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match hint {
            None => Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::AmbiguousLiteral,
            )),
            Some(ValueType::Option(payload)) => Ok(node(
                NodeKind::Literal(CheckedLiteral(filament_exact::OptionValue::none(
                    (**payload).clone(),
                ))),
                ValueType::Option(payload.clone()),
                location,
            )),
            Some(_) => Err(mismatch(location)),
        }
    }

    fn rational_literal(
        &self,
        numerator: &Integer,
        denominator: &Integer,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let domain = match hint {
            None => {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::AmbiguousLiteral,
                ));
            }
            Some(ValueType::Rational(domain)) => domain,
            Some(_) => return Err(mismatch(location)),
        };
        // FR-039: a zero denominator is refused before any normalization work;
        // the normalization (a gcd over both written integers) is then charged
        // by their larger bit length against `s3.rational_bits`, so exhausting
        // it is resource incompleteness and never a value (FR-039-AC-5).
        if denominator.is_zero() {
            return Err(refuse(location, CheckCause::Unproved(Obligation::Nonzero)));
        }
        let bits = numerator.magnitude_bits().max(denominator.magnitude_bits());
        if bits > self.limits.rational_bits() {
            return Err(refuse(
                location,
                CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::RationalBits,
                    limit: self.limits.rational_bits(),
                    actual: u128::from(bits),
                    region: None,
                })),
            ));
        }
        let value = Rational::new(numerator.clone(), denominator.clone())
            .map_err(|_| refuse(location, CheckCause::Unproved(Obligation::Nonzero)))?;
        if !domain.contains(&value) {
            return Err(refuse(
                location,
                CheckCause::Unproved(Obligation::RationalRange),
            ));
        }
        Ok(node(
            NodeKind::Literal(CheckedLiteral(Value::Rational(value))),
            ValueType::Rational(domain.clone()),
            location,
        ))
    }

    /// `decimal(c, s)`: the decimal of coefficient `c` and scale `s` as
    /// written, typed by its unique expected `Decimal[..]` type (FR-104-AC-13,
    /// FR-104-AC-16). With no expected type it is ambiguous; where no decimal
    /// is expected it is a type mismatch. A literal that is not a member of
    /// that type by value (FSpec FR-140: membership scale `max(s, smin)`,
    /// membership coefficient the exact integer at that scale) is a type
    /// mismatch at check time, before any charge (FSpec FR-140-AC-8).
    fn decimal_literal(
        &self,
        coefficient: &Integer,
        scale: u32,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let domain = match hint {
            None => {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::AmbiguousLiteral,
                ));
            }
            Some(ValueType::Decimal(domain)) => domain,
            Some(_) => return Err(mismatch(location)),
        };
        // The literal scale limits come before membership: a zero coefficient
        // is a member whatever its written scale, and the literal keeps that
        // scale as written, so its wire spelling is bounded (FR-093 steps 2
        // and 3).
        let limits = self.scope.decimal_literal_limits;
        let written = u64::from(scale);
        let exhausted = |kind, limit: u64, actual: u64| {
            refuse(
                location,
                CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                    stage: CheckingStage::Typing,
                    kind,
                    limit,
                    actual: u128::from(actual),
                    region: None,
                })),
            )
        };
        if written > limits.scale {
            return Err(exhausted(
                CheckingLimitKind::DecimalLiteralScale,
                limits.scale,
                written,
            ));
        }
        let total = self.scope.decimal_literal_total.add(written);
        if total > limits.total {
            return Err(exhausted(
                CheckingLimitKind::DecimalLiteralScaleTotal,
                limits.total,
                total,
            ));
        }
        let value = Decimal::new(coefficient.clone(), scale);
        if !domain.contains(&value) {
            return Err(mismatch(location));
        }
        Ok(node(
            NodeKind::Literal(CheckedLiteral(Value::Decimal(value))),
            ValueType::Decimal(domain.clone()),
            location,
        ))
    }

    /// `protocol <target>` (FR-218): the unit's protocol declaration the
    /// written name resolves to, once, in the unit's own scope. The
    /// reference is typed `Boolean`, the semantic type every protocol node
    /// carries. A name no declaration of the unit holds is `missing-name`
    /// (an import or a qualified name offers no local protocol); a name that
    /// resolves to a function or a type is `operator-ineligible`.
    fn protocol_reference(&self, target: &str, location: &Location) -> Result<Node, CheckRefusal> {
        if let Some(protocol) = self.scope.protocol_named(target) {
            return Ok(node(
                NodeKind::ProtocolReference { protocol },
                ValueType::Boolean,
                location,
            ));
        }
        if self.signatures.declares(target) || self.scope.declares_type(target) {
            return Err(ineligible(location));
        }
        Err(refuse(location, CheckCause::MissingName(target.to_owned())))
    }

    fn name(
        &self,
        name: &str,
        expected: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if let Some(local) = self.local(name) {
            return Ok(node(
                NodeKind::Local(local.slot),
                local.value_type.clone(),
                location,
            ));
        }
        let members: Vec<(&EnumBinding, &EnumValue)> =
            self.scope.enum_members_named(name).collect();
        match members.as_slice() {
            [(binding, member)] => {
                // ADR-013 O-14/OQ-D: the literal's rank is its case's own
                // canonical position, never a value this site invents --
                // `member.position()` is exactly that, verified at
                // `AdmittedEnumDeclaration::admit_member` (`value/enumeration.rs`).
                Ok(node(
                    NodeKind::Literal(CheckedLiteral(enum_value(binding, member))),
                    ValueType::Enum(binding.shape()),
                    location,
                ))
            }
            [] => self.variant_case(name, expected, location),
            _ => Err(refuse(
                location,
                CheckCause::AmbiguousName {
                    name: name.to_owned(),
                    loci: vec![location.clone()],
                },
            )),
        }
    }

    /// A name no local or enum binds: a case of a variant type (FR-360
    /// item 4), a case the type does not declare, a bare case name where a
    /// variant value is expected, which is no variant value, or a name nothing
    /// binds.
    fn variant_case(
        &self,
        name: &str,
        expected: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if let Some((shape, member)) = self
            .scope
            .variant_case_named(self.source_owner.as_ref(), name)
        {
            let rank = u32::try_from(member.position())
                .expect("an admitted variant type has far fewer than u32::MAX members");
            return Ok(node(
                NodeKind::Literal(CheckedLiteral(Value::Enum(EnumMember::new(
                    member.variant(),
                    rank,
                )))),
                ValueType::Enum(shape),
                location,
            ));
        }
        if let Some((type_name, case)) = name.rsplit_once("::") {
            if let Some(declared) = self
                .scope
                .variant_type_named(self.source_owner.as_ref(), type_name)
            {
                let owner = declared
                    .name()
                    .rsplit("::")
                    .next()
                    .unwrap_or(declared.name())
                    .to_owned();
                return Err(refuse(
                    location,
                    CheckCause::UndeclaredVariantCase {
                        owner,
                        case: case.to_owned(),
                    },
                ));
            }
        }
        if self.signatures.declares(name)
            || (self.scope.is_variant_case_name(name)
                && matches!(expected, Some(ValueType::Enum(shape)) if self.scope.is_variant_shape(shape)))
        {
            return Err(mismatch(location));
        }
        Err(refuse(location, CheckCause::MissingName(name.to_owned())))
    }

    /// `left = right` or `left != right` over its two typed operands. The
    /// pair is first checked with the operands as written, so a conversion
    /// is admitted or refused as such; the stored equality then compares the
    /// operands as they are typed, a converted operand being its own
    /// `ConvertScalar` node.
    fn equality(
        &mut self,
        operator: EqualityOperator,
        left: TypedOperand,
        right: TypedOperand,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if matches!(&left.node.value_type, ValueType::CarriedQuantity(_))
            || matches!(&right.node.value_type, ValueType::CarriedQuantity(_))
        {
            return Err(mismatch(location));
        }
        // FR-359 item 4: FR-208 gives an event type no equality, so an
        // operand holding one is ineligible, not compared.
        if self.scope.reaches_event(&left.node.value_type)
            || self.scope.reaches_event(&right.node.value_type)
        {
            return Err(ineligible(location));
        }
        let mismatch_operator = match operator {
            EqualityOperator::Equal => fsl_forms::BinaryOperator::Equal,
            EqualityOperator::NotEqual => fsl_forms::BinaryOperator::NotEqual,
        };
        if matches!(&left.node.value_type, ValueType::Float(_))
            || matches!(&right.node.value_type, ValueType::Float(_))
        {
            let same_float_type = matches!(
                (&left.node.value_type, &right.node.value_type),
                (ValueType::Float(left), ValueType::Float(right)) if left == right
            );
            if !same_float_type
                || left.conversion_target.is_some()
                || right.conversion_target.is_some()
            {
                return Err(operand_mismatch(
                    location,
                    mismatch_operator,
                    left.node.value_type.clone(),
                    right.node.value_type.clone(),
                ));
            }
        }
        if let (ValueType::Float(left_type), ValueType::Float(right_type)) =
            (&left.node.value_type, &right.node.value_type)
        {
            if left_type != right_type {
                return Err(mismatch(location));
            }
            if left.conversion_target.is_some() || right.conversion_target.is_some() {
                return Err(mismatch(location));
            }
            let operand_type_facts = [(0, &left.source_type), (1, &right.source_type)]
                .into_iter()
                .map(
                    |(operand_position, value_type)| super::ir::EqualityOperandTypeFact {
                        operand_position,
                        role: super::ir::EqualityUnitOperandRole::Source,
                        value_type: value_type.clone(),
                    },
                )
                .collect();
            let producer = EqualityProducerData {
                operator,
                schedule: filament_semantic_value::declaration::EqualitySchedule::Float,
                authored_operands: [left.authored.clone(), right.authored.clone()],
                compared_operands: [left.stored.clone(), right.stored.clone()],
                operand_type_facts,
                units: UnitTable::default(),
                unit_type_facts: Vec::new(),
                enum_members: EnumMemberIndex::default(),
            };
            let mut equal = node(
                NodeKind::IeeeNumericEquality(self.push(left.node), self.push(right.node)),
                ValueType::Boolean,
                location,
            );
            equal.equality_producer = Some(Box::new(producer));
            return Ok(if operator == EqualityOperator::Equal {
                equal
            } else {
                node(
                    NodeKind::Not(self.push(equal)),
                    ValueType::Boolean,
                    location,
                )
            });
        }
        // FR-495-AC-24: two integer operands that both lower at the builtin
        // `Integer` node (an integer division, or a control node over one) are
        // compared as that node, never as their checked intervals, so
        // `n mod 3 = n mod 2` checks. An operand converted or of another type
        // keeps the equality matrix.
        let (mut left, mut right) = (left, right);
        if self.equality_operand_lowers_as_integer(&left)
            && self.equality_operand_lowers_as_integer(&right)
        {
            for operand in [&mut left, &mut right] {
                let integer = EqualityOperand::typed(ValueType::Integer);
                operand.checked = integer.clone();
                operand.stored = integer;
            }
        }
        let check = |left: &EqualityOperand, right: &EqualityOperand| {
            self.scope
                .types
                .check_equality_in(
                    &self.units,
                    operator,
                    left.clone(),
                    right.clone(),
                    &|shape: &EnumShape| self.scope.enum_members_of(shape),
                )
                .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))
        };
        let mut checked = check(&left.checked, &right.checked)?;
        if (&left.stored, &right.stored) != (&left.checked, &right.checked) {
            checked = check(&left.stored, &right.stored)?;
        }
        let mut units = UnitTable::default();
        for value_type in [
            &left.source_type,
            left.conversion_target
                .as_ref()
                .unwrap_or(&left.node.value_type),
            &left.node.value_type,
            &right.source_type,
            right
                .conversion_target
                .as_ref()
                .unwrap_or(&right.node.value_type),
            &right.node.value_type,
        ] {
            let unit_id = match value_type {
                ValueType::Quantity(unit) => Some(unit.clone()),
                ValueType::BoundedQuantity(quantity) => Some(quantity.unit().clone()),
                ValueType::CarriedQuantity(quantity) => Some(quantity.unit().clone()),
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Float(_)
                | ValueType::Text(_)
                | ValueType::String
                | ValueType::Uuid
                | ValueType::Timestamp
                | ValueType::Bytes
                | ValueType::JsonObject
                | ValueType::Enum(_)
                | ValueType::Option(_)
                | ValueType::Composite(_)
                | ValueType::Collection(_)
                | ValueType::Map(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => None,
            };
            if let Some(unit_id) = unit_id {
                let unit = self
                    .units
                    .get(unit_id.clone())
                    .ok_or_else(|| CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::InternalFault(Box::new(KeyFault::UnheldUnit(unit_id))),
                    })?;
                units.insert(unit.clone()).map_err(|refusal| CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::Identity(refusal),
                })?;
            }
        }
        let mut unit_type_facts = Vec::new();
        let mut operand_type_facts = Vec::new();
        for (operand_position, operand) in [(0, &left), (1, &right)] {
            let mut typed_roles = vec![(
                &operand.source_type,
                super::ir::EqualityUnitOperandRole::Source,
            )];
            if operand.conversion_target.is_some() {
                typed_roles.push((
                    &operand.node.value_type,
                    super::ir::EqualityUnitOperandRole::ConversionTarget,
                ));
            }
            for (value_type, role) in typed_roles {
                operand_type_facts.push(super::ir::EqualityOperandTypeFact {
                    operand_position,
                    role,
                    value_type: value_type.clone(),
                });
                let unit_id = match value_type {
                    ValueType::Quantity(unit) => Some(unit.clone()),
                    ValueType::BoundedQuantity(quantity) => Some(quantity.unit().clone()),
                    ValueType::CarriedQuantity(quantity) => Some(quantity.unit().clone()),
                    _ => None,
                };
                if let Some(unit_id) = unit_id {
                    if !units.ids().any(|held| held == unit_id) {
                        return Err(CheckRefusal {
                            location: location.clone(),
                            cause: CheckCause::InternalFault(Box::new(KeyFault::UnheldUnit(
                                unit_id,
                            ))),
                        });
                    }
                    unit_type_facts.push(super::ir::EqualityUnitTypeFact {
                        operand_position,
                        role,
                        unit_id,
                        value_type: value_type.clone(),
                    });
                }
            }
        }
        let producer = EqualityProducerData {
            operator,
            schedule: checked.schedule(),
            authored_operands: [left.authored.clone(), right.authored.clone()],
            compared_operands: [left.stored.clone(), right.stored.clone()],
            operand_type_facts,
            units,
            unit_type_facts,
            enum_members: match (checked.schedule(), &left.node.value_type) {
                (
                    filament_semantic_value::declaration::EqualitySchedule::Enum,
                    ValueType::Enum(shape),
                ) => self.scope.enum_members_of(shape),
                _ => EnumMemberIndex::default(),
            },
        };
        let (left, right) = (left.node, right.node);
        let mut equality = node(
            NodeKind::Equality(
                operator,
                Box::new(checked),
                self.push(left),
                self.push(right),
            ),
            ValueType::Boolean,
            location,
        );
        equality.equality_producer = Some(Box::new(producer));
        Ok(equality)
    }

    /// An ordering over its two typed operands.
    fn ordering(
        &mut self,
        operator: OrderingOperator,
        left: Node,
        right: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if matches!(left.value_type, ValueType::Float(_))
            || matches!(right.value_type, ValueType::Float(_))
        {
            return self.float_ordering(operator, left, right, location);
        }
        let kind = match (&left.value_type, &right.value_type) {
            (l, r) if is_integer(l) && is_integer(r) => OrderedKind::Integers,
            (ValueType::Rational(_), ValueType::Rational(_)) => OrderedKind::Rationals,
            (ValueType::Decimal(_), ValueType::Decimal(_)) => OrderedKind::Decimals,
            (ValueType::Enum(l), ValueType::Enum(r)) if l == r => {
                // ADR-013 O-14: the shape itself carries whether its
                // declaration selects `ordered enum` semantics, so this
                // needs no `scope.enums` lookup (FR-141-AC-5).
                if !l.is_ordered() {
                    return Err(ineligible(location));
                }
                OrderedKind::Enums
            }
            (ValueType::Text(l), ValueType::Text(r)) => {
                if l.profile() != r.profile() {
                    return Err(mismatch(location));
                }
                OrderedKind::Texts
            }
            (l, r) if uncarried_quantity_unit(l).is_some() && uncarried_quantity_unit(r).is_some() => {
                let (Some(l), Some(r)) = (uncarried_quantity_unit(l).and_then(|id| self.units.get(id.clone())), uncarried_quantity_unit(r).and_then(|id| self.units.get(id.clone()))) else {
                    return Err(mismatch(location));
                };
                check_comparable(l, r)
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
                OrderedKind::Quantities
            }
            (ValueType::Boolean, ValueType::Boolean)
            | (ValueType::Option(_), ValueType::Option(_))
            | (ValueType::Composite(_), ValueType::Composite(_))
            | (ValueType::Collection(_), ValueType::Collection(_))
            | (ValueType::Reference(_), ValueType::Reference(_))
            // FR-056 "Unit-language equality on native String, UUID and
            // Timestamp": their key order is a representation order, not an
            // ordering capability.
            | (ValueType::String, ValueType::String)
            | (ValueType::Uuid, ValueType::Uuid)
            | (ValueType::Timestamp, ValueType::Timestamp) => return Err(ineligible(location)),
            (ValueType::Bytes, ValueType::Bytes)
            | (ValueType::JsonObject, ValueType::JsonObject) => return Err(ineligible(location)),
            _ => return Err(mismatch(location)),
        };
        Ok(node(
            NodeKind::Order(operator, kind, self.push(left), self.push(right)),
            ValueType::Boolean,
            location,
        ))
    }

    /// FR-397 "Comparison": an ordering over two operands of one float type
    /// is the IEEE quiet comparison, a Boolean composition of
    /// `numericEqual` and `totalOrder`. With `N(x)` for `numericEqual(x, x)`,
    /// `a < b` is `N(a) and N(b) and (totalOrder(a, b) and not
    /// numericEqual(a, b))` and `a <= b` is `N(a) and N(b) and
    /// (totalOrder(a, b) or numericEqual(a, b))`; `a > b` and `a >= b` are
    /// `b < a` and `b <= a` over the same operand nodes.
    fn float_ordering(
        &mut self,
        operator: OrderingOperator,
        left: Node,
        right: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let (ValueType::Float(left_type), ValueType::Float(right_type)) =
            (&left.value_type, &right.value_type)
        else {
            return Err(operand_mismatch(
                location,
                ordering_binary_operator(operator),
                left.value_type.clone(),
                right.value_type.clone(),
            ));
        };
        if left_type != right_type {
            return Err(operand_mismatch(
                location,
                ordering_binary_operator(operator),
                left.value_type.clone(),
                right.value_type.clone(),
            ));
        }
        // Each operand is evaluated once and read by every part of the
        // composition: an operand that is not already a slot read or a
        // literal is bound by a `let` around the composition, the left one
        // outermost so evaluation keeps source order.
        let mut bindings = Vec::new();
        let mut operand = |this: &mut Self, operand: Node| {
            if matches!(operand.kind, NodeKind::Local(_) | NodeKind::Literal(_)) {
                return this.push(operand);
            }
            let value_type = operand.value_type.clone();
            let value = this.push(operand);
            let slot = this.fresh_slot("#ieee_operand");
            bindings.push((slot, value));
            this.push(node(NodeKind::Local(slot), value_type, location))
        };
        let (left, right) = (operand(self, left), operand(self, right));
        let (lesser, greater, strict) = match operator {
            OrderingOperator::Less => (left, right, true),
            OrderingOperator::LessOrEqual => (left, right, false),
            OrderingOperator::Greater => (right, left, true),
            OrderingOperator::GreaterOrEqual => (right, left, false),
        };
        let boolean = |this: &mut Self, kind| this.push(node(kind, ValueType::Boolean, location));
        let part = |this: &mut Self, kind| this.push(node(kind, ValueType::Boolean, location));
        let lesser_ordinary = part(self, NodeKind::IeeeNumericEquality(lesser, lesser));
        let greater_ordinary = part(self, NodeKind::IeeeNumericEquality(greater, greater));
        let both_ordinary = boolean(
            self,
            NodeKind::Connective(
                Connective::And,
                ConnectiveOrigin::Synthesized,
                lesser_ordinary,
                greater_ordinary,
            ),
        );
        let precedes = boolean(
            self,
            NodeKind::IeeeTotalOrder(OrderedKind::Float, lesser, greater),
        );
        let equal = part(self, NodeKind::IeeeNumericEquality(lesser, greater));
        let relation = if strict {
            let apart = boolean(self, NodeKind::Not(equal));
            boolean(
                self,
                NodeKind::Connective(
                    Connective::And,
                    ConnectiveOrigin::Synthesized,
                    precedes,
                    apart,
                ),
            )
        } else {
            boolean(
                self,
                NodeKind::Connective(
                    Connective::Or,
                    ConnectiveOrigin::Synthesized,
                    precedes,
                    equal,
                ),
            )
        };
        let mut composed = node(
            NodeKind::Connective(
                Connective::And,
                ConnectiveOrigin::Synthesized,
                both_ordinary,
                relation,
            ),
            ValueType::Boolean,
            location,
        );
        for (slot, value) in bindings.into_iter().rev() {
            let body = self.push(composed);
            composed = node(
                NodeKind::Let {
                    slot,
                    name: self.slot_name(slot, location)?,
                    value,
                    body,
                },
                ValueType::Boolean,
                location,
            );
        }
        Ok(composed)
    }

    /// A slot that no source name can read, for a value the checker binds
    /// itself. Its declared name is the fixed text `name`, which begins with
    /// `#` so no source identifier can take it (FR-397 "Operand binders").
    fn fresh_slot(&mut self, name: &str) -> Slot {
        let slot = self.slots;
        self.slots = self.slots.saturating_add(1);
        self.slot_names.push(name.to_owned());
        slot
    }

    /// `left op right` for `+`, `-`, `*` and `/`, over its two typed
    /// operands. Both operands are of one numeric family; mixing families
    /// needs an explicit conversion.
    fn arithmetic(
        &mut self,
        operator: ArithmeticOperator,
        left: Node,
        right: Node,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let (left_type, right_type) = (left.value_type.clone(), right.value_type.clone());
        let (left_box, right_box) = (self.push(left), self.push(right));
        let (kind, value_type) = match (&left_type, &right_type, operator) {
            (ValueType::Quantity(unit), factor, ArithmeticOperator::Multiply)
                if is_exact_quantity_factor(factor) =>
            {
                let Some(quantity_unit) = self.units.get(unit.clone()) else {
                    return Err(mismatch(location));
                };
                if matches!(quantity_unit, QuantityUnit::Declared(unit) if unit.is_affine()) {
                    return Err(mismatch(location));
                }
                (
                    NodeKind::QuantityScale {
                        quantity: left_box,
                        factor: right_box,
                    },
                    ValueType::Quantity(unit.clone()),
                )
            }
            (factor, ValueType::Quantity(unit), ArithmeticOperator::Multiply)
                if is_exact_quantity_factor(factor) =>
            {
                let Some(quantity_unit) = self.units.get(unit.clone()) else {
                    return Err(mismatch(location));
                };
                if matches!(quantity_unit, QuantityUnit::Declared(unit) if unit.is_affine()) {
                    return Err(mismatch(location));
                }
                (
                    NodeKind::QuantityScale {
                        quantity: right_box,
                        factor: left_box,
                    },
                    ValueType::Quantity(unit.clone()),
                )
            }
            (l, r, ArithmeticOperator::Divide) if is_integer(l) && is_integer(r) => {
                if !self.complete_value {
                    return Err(refuse(
                        location,
                        CheckCause::ProfileProhibitedOperator(BinaryOperator::Divide),
                    ));
                }
                match hint {
                    Some(ValueType::Rational(domain)) => (
                        NodeKind::Divide {
                            left: left_box,
                            right: right_box,
                            domain: domain.clone(),
                        },
                        ValueType::Rational(domain.clone()),
                    ),
                    Some(_) => return Err(mismatch(location)),
                    None => {
                        return Err(CheckRefusal::ill_typed(
                            location,
                            IllTypedCause::AmbiguousLiteral,
                        ));
                    }
                }
            }
            (l, r, _) if is_integer(l) && is_integer(r) => {
                // `Divide` is taken by the preceding arm; naming it here keeps
                // a later change from silently lowering `/` as `*`.
                let integer = match operator {
                    ArithmeticOperator::Add => Arithmetic::Add,
                    ArithmeticOperator::Subtract => Arithmetic::Subtract,
                    ArithmeticOperator::Multiply => Arithmetic::Multiply,
                    ArithmeticOperator::Divide => return Err(ineligible(location)),
                };
                (
                    NodeKind::Arithmetic(integer, left_box, right_box),
                    ValueType::Integer,
                )
            }
            (ValueType::Rational(l), ValueType::Rational(r), _) => {
                let domain = match hint {
                    Some(ValueType::Rational(expected)) => expected.clone(),
                    _ => super::facts::rational_result_of(l, operator, r),
                };
                (
                    NodeKind::Rational {
                        operator,
                        left: left_box,
                        right: right_box,
                        domain: domain.clone(),
                    },
                    ValueType::Rational(domain),
                )
            }
            (ValueType::Decimal(left_decimal), ValueType::Decimal(right_decimal), _) => {
                if left_decimal != right_decimal {
                    return Err(mismatch(location));
                }
                let target = match hint {
                    Some(ValueType::Decimal(target)) if target == left_decimal => target.clone(),
                    Some(ValueType::Decimal(_)) => return Err(mismatch(location)),
                    Some(_) => return Err(mismatch(location)),
                    // No expected type: decimal operands and result share one
                    // named type (FSpec FR-140-AC-7), so `+`, `-`, `*` and `/`
                    // are typed by their operands' type wherever they are
                    // written (FR-104). A literal operand took its peer's type.
                    None => left_decimal.clone(),
                };
                (
                    NodeKind::Decimal {
                        operator,
                        left: left_box,
                        right: right_box,
                        target: target.clone(),
                    },
                    ValueType::Decimal(target),
                )
            }
            (ValueType::Float(l), ValueType::Float(r), _) => {
                if l != r {
                    return Err(mismatch(location));
                }
                if self.scope.ieee_profile.is_none() {
                    return Err(refuse(location, CheckCause::IeeeProfileNotAdmitted));
                }
                (
                    NodeKind::Ieee(operator, l.rounding(), left_box, right_box),
                    ValueType::Float(*l),
                )
            }
            (l, r, _)
                if matches!((l, r), (ValueType::Quantity(_), ValueType::Quantity(_)))
                    || operator == ArithmeticOperator::Add
                        && uncarried_quantity_unit(l).is_some()
                        && uncarried_quantity_unit(r).is_some() =>
            {
                let operation = match operator {
                    ArithmeticOperator::Add => UnitOperation::Add,
                    ArithmeticOperator::Subtract => UnitOperation::Subtract,
                    ArithmeticOperator::Multiply => UnitOperation::Multiply,
                    ArithmeticOperator::Divide => UnitOperation::Divide,
                };
                let (Some(l), Some(r)) = (
                    uncarried_quantity_unit(l).and_then(|id| self.units.get(id.clone())),
                    uncarried_quantity_unit(r).and_then(|id| self.units.get(id.clone())),
                ) else {
                    return Err(mismatch(location));
                };
                let unit = result_unit(operation, l, r)
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
                let unit = IdentifiedUnit::new(unit).map_err(|refusal| CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::Identity(refusal),
                })?;
                (
                    NodeKind::Quantity(operator, left_box, right_box),
                    ValueType::Quantity(self.units.form(unit)),
                )
            }
            _ => {
                let operator = match operator {
                    ArithmeticOperator::Add => BinaryOperator::Add,
                    ArithmeticOperator::Subtract => BinaryOperator::Subtract,
                    ArithmeticOperator::Multiply => BinaryOperator::Multiply,
                    ArithmeticOperator::Divide => BinaryOperator::Divide,
                };
                return Err(CheckRefusal {
                    location: location.clone(),
                    cause: CheckCause::OperandTypeMismatch {
                        operator,
                        left_type: Box::new(left_type),
                        right_type: Box::new(right_type),
                    },
                });
            }
        };
        Ok(node(kind, value_type, location))
    }

    /// `operand ^ exponent` over a typed quantity operand and its exponent
    /// literal (FR-093 "Integer power"): a non-affine quantity, the result in
    /// the compound unit with every root exponent multiplied by `exponent`.
    /// The unit is the quantity core's own: the power is typed against a
    /// probe quantity under a meter that admits no charge, so only the
    /// type-time half of the operation runs (FSpec FR-142 "integer power").
    fn power(
        &mut self,
        operand: Node,
        exponent: Integer,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let unit = match &operand.value_type {
            ValueType::Quantity(unit) => unit.clone(),
            // FR-497: a carried quantity is refused at the expression.
            ValueType::CarriedQuantity(_) => return Err(mismatch(location)),
            _ => return Err(mismatch(&location.child(0))),
        };
        let probe = filament_exact::Quantity::new(Rational::from_integer(Integer::one()), unit);
        let Some(base) = self.units.resolve(&probe) else {
            return Err(mismatch(&location.child(0)));
        };
        // An affine base is the only type-time refusal of a power.
        let (_, result) = evaluate_quantity_unit(
            QuantityOperation::Power(base, &exponent),
            &mut filament_exact::Meter::new(NO_CHARGE),
        )
        .map_err(|refusal| match refusal {
            QuantityRefusal::IllTyped(_) => ineligible(location),
            QuantityRefusal::Identity(refusal) => CheckRefusal {
                location: location.clone(),
                cause: CheckCause::Identity(refusal),
            },
        })?;
        let base = self.push(operand);
        let exponent = self.push(node(
            NodeKind::Literal(CheckedLiteral(Value::Integer(exponent))),
            ValueType::Integer,
            location,
        ));
        Ok(node(
            NodeKind::QuantityPower(base, exponent),
            ValueType::Quantity(self.units.form(result)),
            location,
        ))
    }

    /// Unary `-` over its typed operand: integers, rationals, decimals and
    /// IEEE values.
    fn negate(
        &mut self,
        operand: Node,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let (kind, value_type) = match &operand.value_type {
            ValueType::Integer | ValueType::Int(_) => {
                (NodeKind::Negate(self.push(operand)), ValueType::Integer)
            }
            ValueType::Rational(domain) => {
                let domain = match hint {
                    Some(ValueType::Rational(expected)) => expected.clone(),
                    _ => domain.negated(),
                };
                (
                    NodeKind::RationalNegate(self.push(operand), domain.clone()),
                    ValueType::Rational(domain),
                )
            }
            ValueType::Decimal(source) => {
                let target = match hint {
                    Some(ValueType::Decimal(expected)) => expected.clone(),
                    _ => DecimalType::new(
                        source.upper().neg(),
                        source.lower().neg(),
                        u64::from(source.min_scale()),
                        u64::from(source.max_scale()),
                        source.rounding(),
                    )
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?,
                };
                (
                    NodeKind::DecimalNegate(self.push(operand), target.clone()),
                    ValueType::Decimal(target),
                )
            }
            ValueType::Float(float) => {
                if self.scope.ieee_profile.is_none() {
                    return Err(refuse(location, CheckCause::IeeeProfileNotAdmitted));
                }
                let float = *float;
                (
                    NodeKind::Negate(self.push(operand)),
                    ValueType::Float(float),
                )
            }
            // FR-093 "Negation": a non-affine quantity keeps its own unit; the
            // type-time half of the quantity core's negate refuses an affine
            // one, as a power does.
            ValueType::Quantity(unit) => {
                let unit = unit.clone();
                let probe = filament_exact::Quantity::new(
                    Rational::from_integer(Integer::one()),
                    unit.clone(),
                );
                let Some(base) = self.units.resolve(&probe) else {
                    return Err(mismatch(location));
                };
                let (_, _) = evaluate_quantity_unit(
                    QuantityOperation::Negate(base),
                    &mut filament_exact::Meter::new(NO_CHARGE),
                )
                .map_err(|refusal| match refusal {
                    QuantityRefusal::IllTyped(_) => ineligible(location),
                    QuantityRefusal::Identity(refusal) => CheckRefusal {
                        location: location.clone(),
                        cause: CheckCause::Identity(refusal),
                    },
                })?;
                (
                    NodeKind::QuantityNegate(self.push(operand)),
                    ValueType::Quantity(unit),
                )
            }
            // FR-093 "Negation": a bounded quantity is no operand of unary `-`.
            ValueType::BoundedQuantity(_) => return Err(ineligible(location)),
            ValueType::Boolean
            | ValueType::Text(_)
            | ValueType::Enum(_)
            | ValueType::Option(_)
            | ValueType::Composite(_)
            | ValueType::Collection(_)
            | ValueType::Map(_)
            | ValueType::Reference(_)
            | ValueType::Population(_)
            | ValueType::Uuid
            | ValueType::String
            | ValueType::Timestamp => return Err(mismatch(&location.child(0))),
            ValueType::Bytes | ValueType::JsonObject => return Err(mismatch(&location.child(0))),
            // FR-497-AC-2: a carried quantity is no operand of unary `-`.
            ValueType::CarriedQuantity(_) => return Err(mismatch(location)),
        };
        Ok(node(kind, value_type, location))
    }

    /// Whether `x.name` over an `x` of `value_type` is a model read (FR-104,
    /// FR-363) rather than a record field read. A reference is one in a state
    /// clause, and elsewhere when `name` names a relationship end navigable
    /// from it. An `Option`, a collection or a population of references is one
    /// in a function body as in a state clause, which the hook refuses at the
    /// operand (FR-363).
    /// The owner, field and union case of `operand` when it names a guarded
    /// clock place this pass reads the presence of: `r.f` for an
    /// `Option<Clock>` field `f`, `r.f.c` for the case `c` of a union field
    /// `f` (FR-230 "Presence is derived"); `None` otherwise.
    pub(super) fn place_operand<'e>(
        &self,
        operand: fsl_forms::ExprRef<'e>,
    ) -> Option<(fsl_forms::ExprRef<'e>, String, Option<String>, usize)> {
        if self.clock_presence_places().is_empty() {
            return None;
        }
        let mut path = Vec::new();
        let mut prefixes = Vec::new();
        let mut owner = operand;
        loop {
            match owner.node() {
                fsl_forms::ExprNode::Field {
                    operand: before,
                    field,
                    ..
                } => {
                    path.push(field.clone());
                    owner = owner.at(*before);
                    prefixes.push(owner);
                }
                fsl_forms::ExprNode::Index {
                    operand: before,
                    index,
                } => {
                    let fsl_forms::ExprNode::Integer(position) = owner.at(*index).node() else {
                        return None;
                    };
                    path.push(position.to_u64()?.to_string());
                    owner = owner.at(*before);
                    prefixes.push(owner);
                }
                fsl_forms::ExprNode::SelfRef if !path.is_empty() => break,
                fsl_forms::ExprNode::Name(name)
                    if !path.is_empty()
                        && name.segments.len() == 1
                        && self.local(&name.name).is_some_and(|local| {
                            matches!(local.value_type, ValueType::Reference(_))
                        }) =>
                {
                    // A population binder is already the visited object
                    // reference. Preserve it as the presence-read owner;
                    // do not route it through the ordinary state-field
                    // lookup, where clock places have no data field.
                    break;
                }
                _ => return None,
            }
        }
        path.reverse();
        prefixes.reverse();
        let names_place = |place: &&super::timed_model::PresencePlace, path: &[String]| {
            !place.slot
                && place.path.len() == path.len()
                && place.path.iter().zip(path).all(|(step, read)| match step {
                    super::lowering::PlaceStep::Field(name)
                    | super::lowering::PlaceStep::Union(name) => name == read,
                    super::lowering::PlaceStep::Slot(_) => false,
                })
        };
        if let Some(place) = self
            .clock_presence_places()
            .iter()
            .find(|place| names_place(place, &path))
        {
            return Some((owner, place.field.clone(), place.case.clone(), path.len()));
        }
        for prefix in (1..path.len()).rev() {
            let Some(owner) = prefixes.get(prefix).copied() else {
                continue;
            };
            if let Some(place) = self
                .clock_presence_places()
                .iter()
                .find(|place| names_place(place, &path[prefix..]))
            {
                return Some((
                    owner,
                    place.field.clone(),
                    place.case.clone(),
                    path.len().saturating_sub(prefix),
                ));
            }
        }
        None
    }

    /// Whether `operand`, the operand of a field read `operand.op`, is
    /// `t.step` over a name `t` this scope does not resolve: the read is
    /// then the step label `t.step.op` of a trace variable (FR-612).
    pub(super) fn unbound_step_label(&self, operand: fsl_forms::ExprRef<'_>) -> bool {
        let fsl_forms::ExprNode::Field {
            operand: trace,
            field,
            ..
        } = operand.node()
        else {
            return false;
        };
        let fsl_forms::ExprNode::Name(name) = operand.at(*trace).node() else {
            return false;
        };
        field == "step"
            && self
                .name(&name.name, None, &Location::root(Origin::Expression))
                .is_err_and(|refusal| matches!(refusal.cause, CheckCause::MissingName(_)))
    }

    /// `comparison`, located at `location`, when it is `p = absent` or `p
    /// != absent` (either operand order) of a guarded clock place `p` this
    /// pass reads the presence of.
    pub(super) fn absent_comparison<'e>(
        &self,
        comparison: fsl_forms::ExprRef<'e>,
        location: &Location,
    ) -> Option<AbsentComparison<'e>> {
        let fsl_forms::ExprNode::Binary {
            operator,
            left,
            right,
        } = comparison.node()
        else {
            return None;
        };
        let present = match operator {
            fsl_forms::BinaryOperator::Equal => false,
            fsl_forms::BinaryOperator::NotEqual => true,
            _ => return None,
        };
        let (place, side) = match (comparison.at(*left).node(), comparison.at(*right).node()) {
            (_, fsl_forms::ExprNode::Absent) => (comparison.at(*left), 0),
            (fsl_forms::ExprNode::Absent, _) => (comparison.at(*right), 1),
            _ => return None,
        };
        let (owner, field, case, depth) = self.place_operand(place)?;
        Some(AbsentComparison {
            owner,
            owner_location: (0..depth).fold(location.child(side), |at, _| at.child(0)),
            field,
            case,
            present,
        })
    }

    /// The owner and field of `query` when it is `forall(e in r.q:
    /// present(e))` or `exists(..)`, `q` a `Sequence<Option<Clock>>` place
    /// this pass reads the presences of, with whether it is `forall`
    /// (FSpec FR-415-AC-20, STD-526 PR-03).
    pub(super) fn slot_presence_operand<'e>(
        &self,
        query: fsl_forms::ExprRef<'e>,
    ) -> Option<(fsl_forms::ExprRef<'e>, &'e str, bool)> {
        let (all, source, _) = super::timed_model::slot_presence_query(query)?;
        let source = query.at(source);
        let fsl_forms::ExprNode::Field { operand, field, .. } = source.node() else {
            return None;
        };
        self.clock_presence_places()
            .iter()
            .any(|place| place.slot && place.field == *field)
            .then(|| (source.at(*operand), field.as_str(), all))
    }

    /// `forall(e in r.q: present(e))` (`all`) or `exists(..)` over the typed
    /// owner `owner`: whether every (some) element the collection holds is
    /// present, `Boolean`. An owner that is no reference to an object type
    /// holding the place refuses `ill_typed`/`type-mismatch`.
    pub(super) fn slot_presence(
        &mut self,
        owner: Node,
        field: &str,
        all: bool,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Reference(object) = &owner.value_type else {
            return Err(mismatch(location));
        };
        let place = self
            .clock_presence_places()
            .iter()
            .find(|place| {
                place.slot
                    && place.field == field
                    && self
                        .scope
                        .types
                        .conforms(object.clone(), place.owner.clone())
            })
            .cloned()
            .ok_or_else(|| mismatch(location))?;
        Ok(node(
            NodeKind::SlotPresence {
                reference: self.push(owner),
                field: filament_semantic_value::declaration::FieldRef::new(
                    place.owner,
                    place.field,
                ),
                all,
            },
            ValueType::Boolean,
            location,
        ))
    }

    /// `present(r.f)` (`present(r.f.c)`) over the typed owner `owner`: the
    /// place's presence, `Boolean`. An owner of an object type holding no
    /// such place reads `r.f` as a field and `present` over it; any other
    /// owner refuses `ill_typed`/`type-mismatch` (FSpec FR-322-AC-156).
    pub(super) fn place_presence(
        &mut self,
        owner: Node,
        field: &str,
        case: Option<&str>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let place = match &owner.value_type {
            ValueType::Reference(object) => self
                .clock_presence_places()
                .iter()
                .find(|place| {
                    !place.slot
                        && place.field == field
                        && place.case.as_deref() == case
                        && self
                            .scope
                            .types
                            .conforms(object.clone(), place.owner.clone())
                })
                .cloned(),
            _ => None,
        };
        let Some(place) = place else {
            if case.is_some() {
                return Err(mismatch(location));
            }
            // A field of the same name that is no guarded place: `present`
            // of the field read, as the typer reads it elsewhere.
            let field_location = location.child(0);
            let read = if self.is_model_receiver(&owner.value_type, field) {
                self.attribute(
                    owner,
                    field,
                    None,
                    &field_location.child(0),
                    &field_location,
                    false,
                )?
            } else {
                self.field(owner, field, None, &field_location)?
            };
            if !matches!(read.value_type, ValueType::Option(_)) {
                return Err(mismatch(location));
            }
            return Ok(node(
                NodeKind::Present(self.push(read)),
                ValueType::Boolean,
                location,
            ));
        };
        Ok(node(
            NodeKind::PlacePresence {
                reference: self.push(owner),
                field: filament_semantic_value::declaration::FieldRef::new(
                    place.owner,
                    place.field,
                ),
                case: place.case,
            },
            ValueType::Boolean,
            location,
        ))
    }

    /// `ref(r.f[s]...)` over the typed owner `owner`, the `path` of fields
    /// and selectors below it and the types of the selectors (FR-230 "Clock
    /// references", "Selection"): the place id of one clock place, a
    /// `Reference<Clock>`. A name or selector that is no place of the owner
    /// refuses `ill_typed`/`type-mismatch` at the `ref`; so does a place
    /// under a `Map` value, which has no place id (FR-230-AC-62), and a
    /// selector that is no `Integer` index of a `Sequence` or `OrderedSet`,
    /// at the selector.
    ///
    /// A longer path can cross a field that is itself an object reference
    /// (`ref(self.r.x)`); the typed owner is that field's value, and the
    /// remaining path names a place of its target object.
    pub(super) fn clock_ref(
        &mut self,
        owner: Node,
        path: &[(Option<&str>, Option<Location>)],
        selectors: &[Node],
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Reference(object) = &owner.value_type else {
            return Err(mismatch(location));
        };
        let object = object.clone();
        let scope = self.scope;
        let find_place = |object: EffectiveId, path: &[(Option<&str>, Option<Location>)]| {
            scope
                .clock_places
                .iter()
                .find_map(|(place_owner, steps, node)| {
                    (scope.types.conforms(object.clone(), place_owner.clone()))
                        .then(|| {
                            named_by(steps, path).map(|slots| (steps.clone(), slots, node.clone()))
                        })
                        .flatten()
                })
        };
        let mut selected_owner = owner;
        let mut path_offset = 0;
        let mut place = find_place(object, path);
        if place.is_none() && path.len() > 1 {
            // A longer path can cross an object-reference field. Read that
            // field as the owner of the remaining clock place, preserving
            // the same direct Reference<T> term the checked graph carries.
            for (index, (field, _)) in path.iter().enumerate().take(path.len() - 1) {
                let Some(field) = field else {
                    break;
                };
                let field_location = location;
                selected_owner = match selected_owner.value_type.clone() {
                    ValueType::Reference(_) => match self.attribute(
                        selected_owner,
                        field,
                        None,
                        field_location,
                        field_location,
                        false,
                    ) {
                        Ok(value) => value,
                        Err(_) => return Err(mismatch(location)),
                    },
                    ValueType::Composite(_) => {
                        match self.field(selected_owner, field, None, field_location) {
                            Ok(value) => value,
                            Err(_) => return Err(mismatch(location)),
                        }
                    }
                    ValueType::Option(_) => return Err(mismatch(field_location)),
                    _ => break,
                };
                if let ValueType::Reference(object) = &selected_owner.value_type {
                    path_offset = index + 1;
                    place = find_place(object.clone(), &path[path_offset..]);
                    if place.is_some() {
                        break;
                    }
                }
            }
        }
        let Some((steps, slots, place)) = place else {
            return Err(mismatch(location));
        };
        let Some(place) = place else {
            return Err(refuse(
                location,
                CheckCause::InternalFault(Box::new(KeyFault::StageFault(
                    filament_semantic_value::catalog::InternalFault::new(
                        "S3",
                        "clock place has no emitted timed-model node",
                    ),
                ))),
            ));
        };
        let selector_offset = path[..path_offset]
            .iter()
            .filter(|(field, _)| field.is_none())
            .count();
        let clock_is_terminal_field = matches!(
            steps.last(),
            Some(super::lowering::PlaceStep::Field(_) | super::lowering::PlaceStep::Union(_))
        );
        let mut selected = selectors.iter().skip(selector_offset).zip(slots);
        for step in steps {
            let super::lowering::PlaceStep::Slot(shape) = step else {
                continue;
            };
            let Some((selector, index)) = selected.next() else {
                return Err(mismatch(location));
            };
            if shape.map {
                return Err(mismatch(location));
            }
            if !matches!(&selector.value_type, ValueType::Integer | ValueType::Int(_)) {
                return Err(mismatch(
                    path[path_offset + index].1.as_ref().unwrap_or(location),
                ));
            }
        }

        // Keep each selected collection element in the checked body as an
        // Index node. Facts can then prove its slot live from `size(q) > i`
        // (or refuse `unproved-presence` at the selector) while S4 unwraps
        // the same node back to its authored selector for the place-ref call.
        let mut current = selected_owner.clone();
        let mut checked_slots = Vec::new();
        let mut selector = selectors.iter().skip(selector_offset);
        for (index, (field, selector_location)) in path[path_offset..].iter().enumerate() {
            if clock_is_terminal_field && path_offset + index + 1 == path.len() {
                // The clock leaf (or union payload case) is represented by
                // the place template itself; it is not an ordinary model
                // attribute to type as a value read.
                continue;
            }
            if let Some(field) = field {
                current = if let Some(collection) =
                    self.clock_collection_field(current.clone(), field, location)?
                {
                    collection
                } else {
                    match &current.value_type {
                        ValueType::Reference(_) => {
                            self.attribute(current, field, None, location, location, false)?
                        }
                        ValueType::Composite(_) => self.field(current, field, None, location)?,
                        _ => return Err(mismatch(location)),
                    }
                };
                continue;
            }
            let Some(index) = selector.next() else {
                return Err(mismatch(location));
            };
            let (kind, value_type) = match &current.value_type {
                ValueType::Collection(collection) => {
                    let kind = match collection.kind() {
                        filament_exact::CollectionKind::Sequence => {
                            filament_checked::node::IndexedKind::Sequence
                        }
                        filament_exact::CollectionKind::OrderedSet => {
                            filament_checked::node::IndexedKind::OrderedSet
                        }
                        _ => return Err(mismatch(selector_location.as_ref().unwrap_or(location))),
                    };
                    (kind, collection.element().clone())
                }
                _ => return Err(mismatch(selector_location.as_ref().unwrap_or(location))),
            };
            let selector_location = selector_location.as_ref().unwrap_or(location);
            let checked = node(
                NodeKind::Index {
                    collection: self.push(current),
                    index: self.push(index.clone()),
                    kind,
                },
                value_type,
                selector_location,
            );
            checked_slots.push(self.push(checked.clone()));
            current = checked;
        }
        let reference = self.push(selected_owner);
        Ok(node(
            NodeKind::PlaceReference {
                reference,
                place: place.clone(),
                slots: checked_slots,
            },
            super::clock_reference_type(),
            location,
        ))
    }

    /// A clock collection is a place, not an ordinary model attribute. Keep
    /// a typed field-shaped path under an Index so FR-1040's interval proof
    /// can compare `size(self.q)` with `self.q[i]`. PlaceReference lowering
    /// emits only the selector, so this synthetic field never becomes a
    /// package value read.
    fn clock_collection_field(
        &mut self,
        owner: Node,
        field: &str,
        location: &Location,
    ) -> Result<Option<Node>, CheckRefusal> {
        let ValueType::Reference(object) = &owner.value_type else {
            return Ok(None);
        };
        let Some((shape, place_key)) =
            self.scope
                .clock_places
                .iter()
                .find_map(|(candidate, path, key)| {
                    (self.scope.types.conforms(object.clone(), candidate.clone()))
                        .then(|| match path.as_slice() {
                            [
                                super::lowering::PlaceStep::Field(name),
                                super::lowering::PlaceStep::Slot(shape),
                            ] if name == field && !shape.map => {
                                key.as_ref().map(|key| (*shape, key.clone()))
                            }
                            _ => None,
                        })
                        .flatten()
                })
        else {
            return Ok(None);
        };
        let kind = match (shape.ordered, shape.unique) {
            (true, false) => CollectionKind::Sequence,
            (true, true) => CollectionKind::OrderedSet,
            (false, true) => CollectionKind::Set,
            (false, false) => return Ok(None),
        };
        let cardinality = shape
            .max
            .map(|maximum| bound(shape.min, maximum, location))
            .transpose()?;
        let value_type = ValueType::collection(CollectionType::new(
            kind,
            super::clock_reference_type(),
            cardinality,
        ));
        Ok(Some(node(
            NodeKind::Field {
                operand: self.push(owner),
                field: filament_checked::node::RecordField::new(place_key, field),
                optional: false,
            },
            value_type,
            location,
        )))
    }

    pub(crate) fn is_model_receiver(&self, value_type: &ValueType, name: &str) -> bool {
        match value_type {
            ValueType::Reference(object) => {
                self.in_state_clause()
                    || self.model_family
                    || self
                        .scope
                        .relationships()
                        .navigable(&self.scope.types, object.clone(), name)
                        .next()
                        .is_some()
            }
            ValueType::Option(element) => Self::holds_references(element),
            ValueType::Collection(collection) => Self::holds_references(collection.element()),
            ValueType::Map(map) => Self::holds_references(map.values()),
            ValueType::Population(_) => true,
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Integer
            | ValueType::Text(_)
            | ValueType::String
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_)
            | ValueType::Composite(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_) => false,
            ValueType::Bytes | ValueType::JsonObject => false,
        }
    }

    fn holds_references(value_type: &ValueType) -> bool {
        match value_type {
            ValueType::Reference(_) => true,
            ValueType::Option(element) => Self::holds_references(element),
            ValueType::Collection(collection) => Self::holds_references(collection.element()),
            ValueType::Map(map) => Self::holds_references(map.values()),
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Integer
            | ValueType::Text(_)
            | ValueType::String
            | ValueType::Uuid
            | ValueType::Timestamp
            | ValueType::Enum(_)
            | ValueType::Composite(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::CarriedQuantity(_)
            | ValueType::Population(_) => false,
            ValueType::Bytes | ValueType::JsonObject => false,
        }
    }

    /// `deref(r).f` over the typed reference `r`, read at
    /// `operand_location` (the `deref(r)` operand's own location), or the
    /// navigation `r.name` over one of its relationship ends (FR-363).
    /// `derefed` is whether the source wrote an explicit `deref(...)`
    /// (`self.f`/`r.f` inside a state clause never does; SR-750 FND-008
    /// round 2).
    fn attribute(
        &mut self,
        reference: Node,
        field: &str,
        member_span: Option<Span>,
        operand_location: &Location,
        location: &Location,
        derefed: bool,
    ) -> Result<Node, CheckRefusal> {
        if let ValueType::Reference(object) = &reference.value_type {
            let history = self.history.iter().find(|history| {
                history.name == field
                    && self
                        .scope
                        .types
                        .conforms(object.clone(), history.object_type.clone())
            });
            if let Some(history) = history {
                let (object_type, value_type) =
                    (history.object_type.clone(), history.value_type.clone());
                return Ok(node(
                    NodeKind::History {
                        reference: self.push(reference),
                        object_type,
                        field: field.to_owned(),
                    },
                    value_type,
                    location,
                ));
            }
        }
        let field_reads = derefed || self.in_state_clause() || self.model_family;
        let value_type = reference.value_type.clone();
        let on_self = self.state.as_ref().is_some_and(
            |state| matches!(&reference.kind, NodeKind::Local(slot) if *slot == state.self_slot),
        );
        let reference = self.push(reference);
        let mut attribute = self.model_node(
            super::state_model::Form::Attribute {
                reference,
                value_type,
                field: field.to_owned(),
                member_span,
                operand_location: operand_location.clone(),
                derefed,
                field_reads,
            },
            location,
        )?;
        // A field the writing operation narrows reads through its
        // immediate parent's declared type and presence.
        if on_self {
            if let NodeKind::Attribute {
                field, optional, ..
            } = &mut attribute.kind
            {
                let projected = self
                    .state
                    .as_ref()
                    .and_then(|state| state.parent_projections.get(&*field));
                if let Some((declared, presence)) = projected {
                    *optional = *presence == Presence::Optional;
                    attribute.value_type = if *optional {
                        ValueType::option(declared.clone())
                    } else {
                        declared.clone()
                    };
                }
            }
        }
        Ok(attribute)
    }

    /// `self` (FR-104): in a state clause, the context object,
    /// `Reference<M::T>`. Outside a state clause no declaration binds it, so
    /// it refuses `missing_declaration`/`missing-name`.
    fn self_reference(&self, location: &Location) -> Result<Node, CheckRefusal> {
        let Some(state) = &self.state else {
            return Err(refuse(location, CheckCause::MissingSelf));
        };
        self.bound_slot(state.self_slot, location)
    }

    /// `result` (FR-104): admitted only in a postcondition of an operation
    /// that declares a result, typed as that result. Anywhere else it
    /// refuses `wrong_snapshot`/`wrong-anchor`, naming the clause kind and
    /// the operation.
    fn operation_result(&self, location: &Location) -> Result<Node, CheckRefusal> {
        match &self.state {
            Some(StateContext {
                result_slot: Some(slot),
                ..
            }) => self.bound_slot(*slot, location),
            state => Err(refuse(
                location,
                CheckCause::UnanchoredResult {
                    clause: state.as_ref().map(|state| state.kind),
                    operation: state.as_ref().and_then(|state| state.operation.clone()),
                },
            )),
        }
    }

    /// A read of `slot`, a state clause's own `self` or `result` binding,
    /// bound for the whole clause before its body is typed.
    fn bound_slot(&self, slot: Slot, location: &Location) -> Result<Node, CheckRefusal> {
        // Locals are pushed in slot order, so they are sorted by slot.
        let local = self
            .locals
            .binary_search_by_key(&slot, |local| local.slot)
            .ok()
            .and_then(|index| self.locals.get(index))
            .ok_or_else(|| mismatch(location))?;
        Ok(node(
            NodeKind::Local(slot),
            local.value_type.clone(),
            location,
        ))
    }

    /// `reaches(source, target, edge)` over the typed `source` and `target`
    /// (FR-104): for a complete model, each endpoint conforms to the
    /// inherited edge's declaring type `T`, and the edge targets
    /// `Reference<T>`. An absent model retains exact `Reference<T>` endpoint
    /// typing. Anything else,
    /// a set, bag or ordered set of references included, refuses
    /// `ill_typed`/`operator-ineligible` at the `reaches`.
    fn reaches(
        &mut self,
        source: Node,
        target: Node,
        edge: &str,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let source_type = source.value_type.clone();
        let target_type = target.value_type.clone();
        let source = self.push(source);
        let target = self.push(target);
        self.model_node(
            super::state_model::Form::Reaches {
                source,
                source_type,
                target,
                target_type,
                edge: edge.to_owned(),
            },
            location,
        )
    }

    /// Whether a state clause's `pre(e)` operand contains, in its own
    /// syntax, an eligible state read (FR-104 "Observations of reads"):
    /// `self`, a field read through `self` or through a reference-typed
    /// operation parameter, `allInstances`, `lookup` or a nested `pre`. A
    /// field read through a `let` or binder name is never one: a name bound
    /// outside the `pre` is a capture (FSpec's `let s = self in
    /// pre(s.version)` row), and one bound inside it names no parameter.
    /// Parameters, literals and `result` alone are never eligible (FSpec:
    /// "a bare or parameter-only historical selection refuses").
    fn state_pre_eligible(&self, operand: ExprRef<'_>) -> bool {
        let mut inner: BTreeSet<&str> = BTreeSet::new();
        let mut pending = vec![operand];
        while let Some(expression) = pending.pop() {
            match expression.node() {
                ExprNode::Let { name, .. } => {
                    inner.insert(name);
                }
                ExprNode::Query { binder, .. }
                | ExprNode::Count { binder, .. }
                | ExprNode::Sum { binder, .. } => {
                    inner.insert(binder);
                }
                ExprNode::Accumulate {
                    accumulator,
                    binder,
                    ..
                } => {
                    inner.insert(accumulator);
                    inner.insert(binder);
                }
                _ => {}
            }
            pending.extend(expression.children());
        }
        let parameter = |name: &str| {
            !inner.contains(name)
                && self
                    .local(name)
                    .is_some_and(|local| local.kind == LocalKind::Parameter)
        };
        let mut pending = vec![operand];
        while let Some(expression) = pending.pop() {
            match expression.node() {
                ExprNode::SelfRef
                | ExprNode::AllInstances { .. }
                | ExprNode::Lookup { .. }
                | ExprNode::Pre(_) => return true,
                ExprNode::Field {
                    operand: reference, ..
                }
                | ExprNode::Deref(reference)
                    if reference_root(expression.at(*reference)).is_some_and(parameter) =>
                {
                    return true;
                }
                _ => {}
            }
            pending.extend(expression.children());
        }
        false
    }

    /// `e.f` over the typed record `e`.
    fn field(
        &mut self,
        operand: Node,
        field: &str,
        member_span: Option<Span>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Composite(key) = &operand.value_type else {
            return Err(mismatch(location));
        };
        let Some(declaration) = self.scope.types.composite(key.clone()) else {
            return Err(mismatch(location));
        };
        let CompositeShape::Record(fields) = declaration.shape() else {
            return Err(mismatch(location));
        };
        let declared = fields
            .iter()
            .find(|declared| declared.name() == field)
            .ok_or_else(|| {
                refuse(
                    location,
                    if key.package() == "filament.plan-value" {
                        CheckCause::MissingName(field.into())
                    } else {
                        CheckCause::UnknownField {
                            owner: declaration.name().into(),
                            field: field.into(),
                            span: member_span,
                        }
                    },
                )
            })?;
        let optional = declared.presence() == Presence::Optional;
        let value_type = if optional {
            ValueType::option(declared.value_type().clone())
        } else {
            declared.value_type().clone()
        };
        let key = key.clone();
        Ok(node(
            NodeKind::Field {
                operand: self.push(operand),
                field: RecordField::new(key, field),
                optional,
            },
            value_type,
            location,
        ))
    }

    /// `receiver.member(args)` (FR-151, `filament.model.dispatch.single/v1`):
    /// whether a dispatch call checks in this clause at all. It checks only
    /// inside an invariant, precondition or postcondition (TC-196 D07).
    fn dispatch_admitted(&mut self, location: &Location) -> Result<(), CheckRefusal> {
        self.model_check(
            super::state_model::Form::DispatchContext(self.clause_kind),
            location,
        )
        .map(|_| ())
    }

    /// The dispatch-eligible operation `member` a call with `arity`
    /// arguments names on its typed receiver: the receiver is `self`, a
    /// `deref(...)` result or another `Reference<T>` value, and `member`
    /// resolves statically against `T`'s exposed dispatch-eligible
    /// operations. Returns the operation's index and declaration.
    fn dispatch_operation(
        &mut self,
        receiver: &Node,
        member: &str,
        arity: usize,
        location: &Location,
    ) -> Result<(usize, &'a DispatchOperation), CheckRefusal> {
        let index = match self.model_check(
            super::state_model::Form::DispatchOperation {
                receiver_type: receiver.value_type.clone(),
                member: member.to_owned(),
                arity,
            },
            location,
        )? {
            super::state_model::Checked::Operation(index) => index,
            _ => return Err(super::state_model::fault(location)),
        };
        self.scope
            .dispatch_operations
            .get(index)
            .map(|operation| (index, operation))
            .ok_or_else(|| super::state_model::fault(location))
    }

    /// A dispatch call argument typed against its declared parameter type
    /// (`filament.model.dispatch.single/v1`: "its arguments are type-checked
    /// statically against `o`'s signature with reference upcasts only").
    /// Unlike a checked operand, this never calls [`coerce`]: an ordinary
    /// call admits an `Integer`/`Int[..]` argument into a wider or
    /// differently-bounded `Int[..]` parameter, which a dispatch call must
    /// not. A `Reference<T>` argument is admitted where `T` exactly matches
    /// the declared parameter (reflexive), or where `T` is a proper subtype
    /// of it in `self.scope.types`'s own admitted generalization graph (H1,
    /// #204 round 1) -- the static upcast case FR-151 names, never a wider
    /// admission than the spec allows. The upcast changes only this node's
    /// own static/declared type at the binding site; the runtime
    /// `ObjectReference` triple underneath is untouched. A conditional or
    /// `let` passes the parameter type into its branches or body, so only
    /// the value an argument's branches yield reaches this.
    fn upcast(
        &mut self,
        typed: Node,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let upcast_type = self.model_type(
            super::state_model::Form::Argument {
                actual: typed.value_type.clone(),
                required: required.clone(),
            },
            location,
        )?;
        if matches!(typed.value_type, ValueType::Integer | ValueType::Int(_))
            && matches!(
                typed.kind,
                NodeKind::IntegerDivision { .. } | NodeKind::Local(_)
            )
        {
            if let ValueType::Int(target) = required {
                let location = typed.location.clone();
                return Ok(Node {
                    kind: NodeKind::Coerce(self.push(typed), target.clone()),
                    value_type: required.clone(),
                    location,
                    equality_producer: None,
                    field_population: None,
                    equality_range_context: None,
                    type_argument: None,
                    binder_annotation: None,
                });
            }
            return Ok(typed);
        }
        let mut typed = typed;
        typed.value_type = upcast_type;
        Ok(typed)
    }

    /// The record declaration `name` a record literal builds, and its
    /// declared fields.
    fn record_declaration(
        &self,
        name: &str,
        location: &Location,
    ) -> Result<(filament_exact::NodeKey, &'a [FieldDeclaration]), CheckRefusal> {
        let scope: &'a Scope = self.scope;
        let value_type = self.type_named(name, location)?;
        let ValueType::Composite(key) = &value_type else {
            return Err(mismatch(location));
        };
        let key = key.clone();
        let Some(CompositeShape::Record(declared)) = scope
            .types
            .composite(key.clone())
            .map(|declaration| declaration.shape())
        else {
            return Err(mismatch(location));
        };
        Ok((key, declared))
    }

    /// Admit a record literal's initializer of `field` at `field_location`
    /// into `supplied`: a `null` is admitted now; a value expression names
    /// the slot it fills and the type it is checked against once typed.
    fn record_field<'d>(
        declared: &'d [FieldDeclaration],
        supplied: &mut [Option<RecordSlot<NodeId>>],
        field: &str,
        initializer: FieldInitializer,
        field_location: &Location,
    ) -> Result<Option<(usize, &'d ValueType, ExprId)>, CheckRefusal> {
        let (index, declaration) = declared
            .iter()
            .enumerate()
            .find(|(_, declaration)| declaration.name() == field)
            .ok_or_else(|| mismatch(field_location))?;
        let slot = supplied
            .get_mut(index)
            .ok_or_else(|| mismatch(field_location))?;
        if slot.is_some() {
            return Err(mismatch(field_location));
        }
        let optional = declaration.presence() == Presence::Optional;
        match initializer {
            FieldInitializer::Null if optional => {
                *slot = Some(RecordSlot::<NodeId>::Null);
                Ok(None)
            }
            FieldInitializer::Null => Err(mismatch(field_location)),
            FieldInitializer::Value(expression) => {
                Ok(Some((index, declaration.value_type(), expression)))
            }
        }
    }

    /// A record literal's node over every field it supplied: an optional
    /// field it left out is absent, a required one refuses.
    fn record(
        key: filament_exact::NodeKey,
        declared: &[FieldDeclaration],
        supplied: Vec<Option<RecordSlot<NodeId>>>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let mut slots = Vec::with_capacity(declared.len());
        for (declaration, slot) in declared.iter().zip(supplied) {
            slots.push(match slot {
                Some(slot) => slot,
                None if declaration.presence() == Presence::Optional => {
                    RecordSlot::<NodeId>::Absent
                }
                None => return Err(mismatch(location)),
            });
        }
        Ok(node(
            NodeKind::Record {
                declaration: key.clone(),
                slots,
            },
            ValueType::Composite(key),
            location,
        ))
    }

    /// The collection type a `kind` literal takes: its unique expected
    /// collection type.
    fn collection_type(
        kind: CollectionKind,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<CollectionType, CheckRefusal> {
        match hint {
            None => Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::AmbiguousLiteral,
            )),
            Some(ValueType::Collection(collection_type)) if collection_type.kind() == kind => {
                Ok((**collection_type).clone())
            }
            Some(_) => Err(mismatch(location)),
        }
    }

    /// A declared conversion or query target, resolved and checked.
    fn declared_target(
        &self,
        target: &fsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        let target = self.resolve_type(target, location)?;
        self.check_declared_type(&target, location)?;
        Ok(target)
    }

    /// FR-150: the `identity-observing-form` refusal of the `node` form at
    /// `location` when `operand` is, or contains, a reference into a
    /// `symmetric` population; `None` otherwise.
    fn identity_observing(
        &self,
        location: &Location,
        node: IdentityNode,
        form: IdentityForm,
        operand: &ValueType,
    ) -> Option<CheckRefusal> {
        let population = self.scope.symmetric_population_in(operand)?.clone();
        Some(refuse(
            location,
            CheckCause::IdentityObservingForm(Box::new(IdentityObserving {
                node,
                form,
                population,
            })),
        ))
    }

    /// `convert<target>(e)` over the typed operand `e`.
    fn convert(
        &mut self,
        target: &ValueType,
        operand: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if &operand.value_type == target
            && matches!(&operand.value_type, ValueType::Text(_) | ValueType::String)
        {
            return Ok(operand);
        }
        match (&operand.value_type, target) {
            (ValueType::Text(_) | ValueType::String, target)
                if matches!(
                    target,
                    ValueType::Integer
                        | ValueType::Int(_)
                        | ValueType::Decimal(_)
                        | ValueType::Rational(_)
                        | ValueType::Float(_)
                        | ValueType::Quantity(_)
                ) =>
            {
                Ok(node(
                    NodeKind::ConvertText(target.clone(), self.push(operand)),
                    target.clone(),
                    location,
                ))
            }
            (ValueType::Text(_) | ValueType::String, _) => Err(ineligible(location)),
            (ValueType::Collection(source), ValueType::Collection(to)) => {
                if source.element() != to.element() {
                    return Err(mismatch(location));
                }
                // A Set or Bag read as an ordered collection is read in the
                // identity-sorted order of its elements.
                if !source.kind().is_ordered() && to.kind().is_ordered() {
                    if let Some(refusal) = self.identity_observing(
                        location,
                        IdentityNode::Convert,
                        IdentityForm::OrderSensitiveTraversal,
                        &operand.value_type,
                    ) {
                        return Err(refusal);
                    }
                }
                Ok(node(
                    NodeKind::ConvertCollection {
                        target: (**to).clone(),
                        operand: self.push(operand),
                    },
                    target.clone(),
                    location,
                ))
            }
            (ValueType::Collection(_), _) | (_, ValueType::Collection(_)) => {
                Err(mismatch(location))
            }
            (ValueType::Float(_), ValueType::Rational(domain)) => {
                if self.scope.ieee_profile.is_none() {
                    return Err(refuse(location, CheckCause::IeeeProfileNotAdmitted));
                }
                Ok(node(
                    NodeKind::IeeeToRational(self.push(operand), domain.clone()),
                    target.clone(),
                    location,
                ))
            }
            // FR-397 "Conversion into a float": a float of the target's width
            // whose mode differs is the operand retyped. The mode is part of
            // the type; no node is built, nothing is charged.
            (ValueType::Float(source), ValueType::Float(to)) if source.width() == to.width() => {
                let mut retyped = operand;
                retyped.value_type = target.clone();
                Ok(retyped)
            }
            // A cross-width float, or an `Integer`, `Int[..]`, `Rational[..]`
            // or `Decimal[..]`, converts through the owned IEEE operation.
            (
                ValueType::Float(_)
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_),
                ValueType::Float(_),
            ) => {
                if self.scope.ieee_profile.is_none() {
                    return Err(refuse(location, CheckCause::IeeeProfileNotAdmitted));
                }
                let converted =
                    EqualityOperand::converted(operand.value_type.clone(), target.clone());
                Ok(node(
                    NodeKind::ConvertScalar(converted, self.push(operand)),
                    target.clone(),
                    location,
                ))
            }
            (
                ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_),
                ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_),
            ) => {
                use super::numeric_conversion::{Class, classify};
                let class = classify(&operand.value_type, target).ok_or_else(|| {
                    CheckRefusal::ill_typed(location, IllTypedCause::OperatorIneligible)
                })?;
                let kind =
                    if let (Class::ScaleReduction, ValueType::Decimal(decimal)) = (class, target) {
                        NodeKind::ConvertDecimal(self.push(operand), decimal.clone())
                    } else {
                        let converted =
                            EqualityOperand::converted(operand.value_type.clone(), target.clone());
                        NodeKind::ConvertScalar(converted, self.push(operand))
                    };
                Ok(node(kind, target.clone(), location))
            }
            (
                ValueType::Quantity(_) | ValueType::CarriedQuantity(_),
                ValueType::Quantity(_) | ValueType::CarriedQuantity(_),
            )
            | (
                ValueType::BoundedQuantity(_),
                ValueType::Quantity(_) | ValueType::BoundedQuantity(_),
            )
            | (ValueType::Quantity(_), ValueType::BoundedQuantity(_)) => {
                let Some(source_unit) = quantity_unit(&operand.value_type) else {
                    return Err(mismatch(location));
                };
                let Some(target_unit) = quantity_unit(target) else {
                    return Err(mismatch(location));
                };
                let compatible = self
                    .units
                    .get(source_unit.clone())
                    .zip(self.units.get(target_unit.clone()))
                    .is_some_and(|(source, target)| source.converts_to(target));
                if !compatible {
                    return Err(mismatch(location));
                }
                let converted =
                    EqualityOperand::converted(operand.value_type.clone(), target.clone());
                Ok(node(
                    NodeKind::ConvertScalar(converted, self.push(operand)),
                    target.clone(),
                    location,
                ))
            }
            (source, _) => {
                if matches!(source, ValueType::Float(_))
                    || !admits_equality_conversion(source, target, &self.units)
                {
                    return Err(self
                        .identity_observing(
                            location,
                            IdentityNode::Convert,
                            IdentityForm::ReferenceConversion,
                            source,
                        )
                        .unwrap_or_else(|| mismatch(location)));
                }
                let converted = EqualityOperand::converted(source.clone(), target.clone());
                Ok(node(
                    NodeKind::ConvertScalar(converted, self.push(operand)),
                    target.clone(),
                    location,
                ))
            }
        }
    }

    /// The queried type `T` of `allInstances<T>(p)` or `lookup<T>(p, r)`
    /// (FR-153), resolved and checked: an object reference type.
    fn population_target(
        &mut self,
        target: &fsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        self.model_type(super::state_model::Form::Target(target.clone()), location)
    }

    /// `allInstances<T>(p)` (FR-153) over the typed population `p`: `p`'s
    /// own checked `Population<T>[N]` type (`ValueType::Population`) gives
    /// the result's declared bound `[0,N]` directly; no runtime value is
    /// consulted at check time.
    fn all_instances(
        &mut self,
        target: &ValueType,
        population: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let population_type = population.value_type.clone();
        let operand_location = population.location.clone();
        let population = self.push(population);
        self.model_node(
            super::state_model::Form::AllInstances {
                target: target.clone(),
                population,
                population_type,
                operand_location,
            },
            location,
        )
    }

    /// `lookup<T>(p, r)`'s typed population operand `p`: FR-153 requires a
    /// population binding with a declared maximum, otherwise
    /// `ill_typed`/`operator-ineligible` (TC-198 L06): the operand is the
    /// wrong kind, not merely the wrong type name.
    fn lookup_population(&mut self, population: &Node) -> Result<(), CheckRefusal> {
        self.model_check(
            super::state_model::Form::Population {
                value_type: population.value_type.clone(),
            },
            &population.location,
        )
        .map(|_| ())
    }

    /// `lookup<T>(p, r) absent m` (FR-153) over its typed operands. `r`'s own
    /// checked static type `S` (never `T`) is `reference.value_type` at
    /// evaluation time (`fsl-eval`'s `value::expression::evaluate`), so
    /// [`NodeKind::Lookup`] does not restate it. FR-153 refuses
    /// `ill_typed`/`type-mismatch` at check time when `S` does not conform to
    /// `T` (TC-198 L03's last case, "before any charge"): decided here from
    /// the package's own admitted generalization graph
    /// ([`filament_semantic_value::declaration::TypeEnvironment::conforms`]), whose
    /// admission bound makes it agree with the model's own walk at
    /// evaluation (`crate::model::population::lookup`), which still decides
    /// `S` against `T` for a runtime binding.
    fn lookup(
        &mut self,
        target: &ValueType,
        population: Node,
        reference: Node,
        absence: AbsenceMode,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let reference_type = reference.value_type.clone();
        let reference_location = reference.location.clone();
        let population = self.push(population);
        let reference = self.push(reference);
        self.model_node(
            super::state_model::Form::Lookup {
                target: target.clone(),
                population,
                reference,
                reference_type,
                reference_location,
                absence,
            },
            location,
        )
    }

    /// Bind a one-binder form's binder to the element type of its typed
    /// collection operand `source`; returns the binder's slot and the
    /// source's collection type.
    fn bind_element(
        &mut self,
        source: &Node,
        binder: &str,
        location: &Location,
    ) -> Result<(Slot, Arc<CollectionType>), CheckRefusal> {
        let ValueType::Collection(source_type) = &source.value_type else {
            return Err(mismatch(location));
        };
        let source_type = source_type.clone();
        let slot = self.bind(
            binder,
            source_type.element().clone(),
            location,
            LocalKind::Bound,
        )?;
        Ok((slot, source_type))
    }

    /// A one-binder query `q(x in source: body)` over its typed source and
    /// body, the binder bound at `slot`; unbinds the binder.
    fn query(
        &mut self,
        query: BinderQuery,
        (slot, source_type): (Slot, Arc<CollectionType>),
        source: Node,
        body: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let typed = match query {
            BinderQuery::Map | BinderQuery::FlatMap => {
                // An unbounded source maps to an unbounded result (ADR-014
                // §2): no maximum exists to carry over.
                let mapped_bound = source_type
                    .bound()
                    .map(|bound| {
                        let minimum = if source_type.kind().is_unique() {
                            bound.minimum().min(1)
                        } else {
                            bound.minimum()
                        };
                        super::check::bound(minimum, bound.maximum(), location)
                    })
                    .transpose()?;
                let mapped =
                    CollectionType::new(source_type.kind(), body.value_type.clone(), mapped_bound);
                let value_type = ValueType::collection(mapped);
                self.check_declared_type(&value_type, location)?;
                let map = node(
                    NodeKind::Query {
                        visit: Visit::Map,
                        slot,
                        name: self.slot_name(slot, location)?,
                        source: self.push(source),
                        body: self.push(body),
                    },
                    value_type,
                    location,
                );
                if query == BinderQuery::FlatMap {
                    self.flatten(map, location)?
                } else {
                    map
                }
            }
            BinderQuery::Filter => {
                let filtered = CollectionType::new(
                    source_type.kind(),
                    source_type.element().clone(),
                    source_type
                        .bound()
                        .map(|bound| super::check::bound(0, bound.maximum(), location))
                        .transpose()?,
                );
                node(
                    NodeKind::Query {
                        visit: Visit::Filter,
                        slot,
                        name: self.slot_name(slot, location)?,
                        source: self.push(source),
                        body: self.push(body),
                    },
                    ValueType::collection(filtered),
                    location,
                )
            }
            BinderQuery::Forall | BinderQuery::Exists => {
                let visit = if query == BinderQuery::Forall {
                    Visit::Forall
                } else {
                    Visit::Exists
                };
                node(
                    NodeKind::Query {
                        visit,
                        slot,
                        name: self.slot_name(slot, location)?,
                        source: self.push(source),
                        body: self.push(body),
                    },
                    ValueType::Boolean,
                    location,
                )
            }
        };
        self.unbind(1);
        Ok(typed)
    }

    fn flatten(&mut self, source: Node, location: &Location) -> Result<Node, CheckRefusal> {
        let ValueType::Collection(outer) = &source.value_type else {
            return Err(mismatch(location));
        };
        let ValueType::Collection(inner) = outer.element() else {
            return Err(mismatch(location));
        };
        if outer.kind().is_ordered() && !inner.kind().is_ordered() {
            return Err(mismatch(location));
        }
        // Flattening an unbounded outer or inner collection gives an
        // unbounded result (ADR-014 §2): the product has no maximum.
        let flattened_bound = match (outer.bound(), inner.bound()) {
            (Some(outer_bound), Some(inner_bound)) => {
                let unrepresentable = || refuse(location, CheckCause::UnrepresentableBound);
                let minimum = outer_bound
                    .minimum()
                    .checked_mul(inner_bound.minimum())
                    .ok_or_else(unrepresentable)?;
                let maximum = outer_bound
                    .maximum()
                    .checked_mul(inner_bound.maximum())
                    .ok_or_else(unrepresentable)?;
                let minimum = if outer.kind().is_unique() {
                    minimum.min(1)
                } else {
                    minimum
                };
                Some(bound(minimum, maximum, location)?)
            }
            (None, _) | (_, None) => None,
        };
        let flattened = CollectionType::new(outer.kind(), inner.element().clone(), flattened_bound);
        let value_type = ValueType::collection(flattened);
        self.check_declared_type(&value_type, location)?;
        Ok(node(
            NodeKind::Flatten(self.push(source)),
            value_type,
            location,
        ))
    }

    /// Bind a `fold`/`reduce`'s accumulator and element binder over its
    /// typed source, the accumulator typed `value_type`; `identity` is
    /// whether an `identity:` is written. Returns the accumulator's and the
    /// binder's slots and the source's collection type.
    fn bind_accumulation(
        &mut self,
        (form, identity): (Accumulation, bool),
        value_type: &ValueType,
        (accumulator, binder): (&str, &str),
        source: &Node,
        location: &Location,
    ) -> Result<(Slot, Slot, Arc<CollectionType>), CheckRefusal> {
        let ValueType::Collection(source_type) = &source.value_type else {
            return Err(mismatch(location));
        };
        let source_type = source_type.clone();
        match (form, identity) {
            (Accumulation::Fold, false) | (Accumulation::Reduce, true) => {
                return Err(mismatch(location));
            }
            (Accumulation::Fold, true) | (Accumulation::Reduce, false) => {}
        }
        if form == Accumulation::Reduce && source_type.element() != value_type {
            return Err(mismatch(location));
        }
        let accumulator_slot =
            self.bind(accumulator, value_type.clone(), location, LocalKind::Bound)?;
        let binder_slot = self.bind(
            binder,
            source_type.element().clone(),
            location,
            LocalKind::Bound,
        )?;
        Ok((accumulator_slot, binder_slot, source_type))
    }

    /// `contains(c, v)` over its typed collection `c`, of element type
    /// `element`, and its typed item `v`: the element type admits equality.
    fn contains(
        &mut self,
        collection: Node,
        item: Node,
        element: ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.require_member_equality(element, location)?;
        Ok(node(
            NodeKind::Contains(self.push(collection), self.push(item)),
            ValueType::Boolean,
            location,
        ))
    }
}

impl Typer<'_> {
    /// The element type admits the equality `contains` and `removeOne` use.
    fn require_member_equality(
        &self,
        element: ValueType,
        location: &Location,
    ) -> Result<(), CheckRefusal> {
        self.scope
            .types
            .check_equality_in(
                &self.units,
                EqualityOperator::Equal,
                EqualityOperand::typed(element.clone()),
                EqualityOperand::typed(element),
                &|shape: &EnumShape| self.scope.enum_members_of(shape),
            )
            .map(|_| ())
            .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))
    }

    /// `removeOne(c, v)` over its typed collection `c`, of element type
    /// `element`, and its typed item `v`: the same kind with the bound
    /// `[max(a-1,0), b]` (FSpec FR-145-AC-11).
    fn remove_one(
        &mut self,
        collection: Node,
        item: Node,
        element: ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.require_member_equality(element, location)?;
        let ValueType::Collection(source) = &collection.value_type else {
            return Err(mismatch(&collection.location));
        };
        let shrunk = match source.bound() {
            Some(source_bound) => Some(bound(
                source_bound.minimum().saturating_sub(1),
                source_bound.maximum(),
                location,
            )?),
            None => None,
        };
        let value_type = ValueType::collection(CollectionType::new(
            source.kind(),
            source.element().clone(),
            shrunk,
        ));
        self.check_declared_type(&value_type, location)?;
        Ok(node(
            NodeKind::RemoveOne(self.push(collection), self.push(item)),
            value_type,
            location,
        ))
    }

    /// `dropFirst(c)` over a sequence or an ordered set: the same kind with
    /// the bound `[max(a-1,0), max(b-1,0)]`; a set or a bag has no first
    /// occurrence (FSpec FR-145-AC-10).
    fn drop_first(&mut self, source: Node, location: &Location) -> Result<Node, CheckRefusal> {
        let ValueType::Collection(collection) = &source.value_type else {
            return Err(mismatch(&source.location));
        };
        if !matches!(
            collection.kind(),
            CollectionKind::Sequence | CollectionKind::OrderedSet
        ) {
            return Err(ineligible(location));
        }
        let shrunk = match collection.bound() {
            Some(source_bound) => Some(bound(
                source_bound.minimum().saturating_sub(1),
                source_bound.maximum().saturating_sub(1),
                location,
            )?),
            None => None,
        };
        let value_type = ValueType::collection(CollectionType::new(
            collection.kind(),
            collection.element().clone(),
            shrunk,
        ));
        self.check_declared_type(&value_type, location)?;
        Ok(node(
            NodeKind::DropFirst(self.push(source)),
            value_type,
            location,
        ))
    }
}

/// Whether a well-typed step is in the FR-145 set-and-bag catalog: `acc OP t`
/// or `t OP acc`, `acc` not in `t`, `t` of type `A`, and `+` or `*` on
/// `Integer` or `and` or `or` on `Boolean`.
fn catalogued_step(
    body: &BodyBuilder,
    step: &Node,
    accumulator: Slot,
    value_type: &ValueType,
) -> bool {
    let (left, right) = match (&step.kind, value_type) {
        (
            NodeKind::Arithmetic(Arithmetic::Add | Arithmetic::Multiply, left, right),
            ValueType::Integer,
        )
        | (
            NodeKind::Connective(Connective::And | Connective::Or, _, left, right),
            ValueType::Boolean,
        ) => (*left, *right),
        _ => return false,
    };
    let is_accumulator =
        |operand: &Node| matches!(operand.kind, NodeKind::Local(slot) if slot == accumulator);
    let (Some(left_node), Some(right_node)) = (body.get(left), body.get(right)) else {
        return false;
    };
    let other = if is_accumulator(left_node) {
        right
    } else if is_accumulator(right_node) {
        left
    } else {
        return false;
    };
    if body.get(other).map(|node| &node.value_type) != Some(value_type) {
        return false;
    }
    // Every node of the operand is in the body and none is the accumulator.
    struct FreeOfAccumulator<'b, F: Fn(&Node) -> bool> {
        body: &'b BodyBuilder,
        is_accumulator: F,
    }
    impl<F: Fn(&Node) -> bool> filament_walk::Walk for FreeOfAccumulator<'_, F> {
        type Node = NodeId;
        type Frame = ();
        type Stop = ();

        fn enter(
            &mut self,
            id: NodeId,
            children: &mut filament_walk::Children<'_, NodeId>,
        ) -> ControlFlow<(), ()> {
            match self.body.get(id) {
                Some(node) if !(self.is_accumulator)(node) => {
                    children.extend(super::ir::node_children(&node.kind));
                    ControlFlow::Continue(())
                }
                _ => ControlFlow::Break(()),
            }
        }

        fn exit(&mut self, (): ()) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
    }
    filament_walk::walk(
        &mut FreeOfAccumulator {
            body,
            is_accumulator,
        },
        other,
    )
    .is_continue()
}

/// FR-488 "Reserved slot names": the refusal of `name` when FSpec FR-402's
/// slot decoder accepts it, at its own `span` when the declaring form records
/// one and otherwise at `location`. The decoder is contract-ir's, so the
/// reserved set is exactly the set a reader classifies as a slot parameter.
pub(crate) fn reserved_slot_refusal(
    name: &str,
    span: Option<Span>,
    location: &Location,
) -> Option<CheckRefusal> {
    reserved_slot_cause(name, span).map(|cause| refuse(location, cause))
}

/// [`reserved_slot_refusal`]'s cause, for a site that reports a bare cause.
pub(crate) fn reserved_slot_cause(name: &str, span: Option<Span>) -> Option<CheckCause> {
    if !filament_contract_model::is_slot_parameter_name(name) {
        return None;
    }
    Some(CheckCause::ReservedSlotName(Box::new(ReservedSlotName {
        name: name.to_owned(),
        tag: SlotTag::of(name)?,
        span,
    })))
}

/// Check a function signature's declared types and bind its parameters.
/// FR-153's `Population<T>[N]` parameter type (`ValueType::Population`) is
/// the one context that names a population binding directly, so it bypasses
/// [`Typer::check_declared_type`]: that walk (`TypeEnvironment::type_refusal`)
/// refuses `Population` everywhere else (an equality operand, an `Option`
/// payload, a collection element, a record/tuple/object-type member, or any
/// other named type), since FR-153 gives it Outputs only as the direct
/// operand of `allInstances`/`lookup`.
pub(crate) fn bind_parameters(
    typer: &mut Typer<'_>,
    parameters: &[(String, ValueType)],
    location: &Location,
) -> Result<(), CheckRefusal> {
    for (name, value_type) in parameters {
        if !matches!(value_type, ValueType::Population(_)) {
            typer.check_declared_type(value_type, location)?;
        }
        typer.bind(name, value_type.clone(), location, LocalKind::Parameter)?;
    }
    Ok(())
}

/// Whether `ty` is, or holds through an option, a collection or a map's
/// values, a `Float` (the IEEE profile is needed exactly then).
pub(super) fn holds_float(ty: &ValueType) -> bool {
    reaches_leaf(
        ty,
        |leaf| matches!(leaf, ValueType::Float(_)),
        |_| Vec::new(),
    )
}

/// Whether `root` is, or holds through an option, a collection, a map's
/// values or a declared composite, a type `leaf` accepts. `members` gives the
/// types a composite declaration holds; each declaration is visited once.
fn reaches_leaf<'a>(
    root: &'a ValueType,
    leaf: impl Fn(&ValueType) -> bool,
    members: impl Fn(&filament_exact::NodeKey) -> Vec<&'a ValueType>,
) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(value_type) = pending.pop() {
        if leaf(value_type) {
            return true;
        }
        match value_type {
            ValueType::Option(payload) => pending.push(payload),
            ValueType::Collection(collection) => pending.push(collection.element()),
            ValueType::Map(map) => pending.push(map.values()),
            ValueType::Composite(declaration) if seen.insert(declaration.clone()) => {
                pending.extend(members(declaration));
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod holds_float_tests {
    use super::*;

    /// The IEEE profile is needed exactly when a `Float` is declared, also
    /// behind an option, a collection or a map's values (filament-exact
    /// FR-386); a bounded quantity is not a float. No FSL criterion pins the
    /// map arm itself.
    #[test]
    fn a_float_behind_a_map_is_seen() {
        let float = ValueType::Float(filament_exact::FloatType::new(
            filament_exact::IeeeWidth::Binary64,
            filament_exact::RoundingMode::Exact,
        ));
        let map = |values| {
            ValueType::Map(std::sync::Arc::new(
                filament_exact::MapType::new(ValueType::String, values, None).expect("a leaf key"),
            ))
        };
        assert!(holds_float(&float));
        assert!(holds_float(&map(float.clone())));
        assert!(holds_float(&map(ValueType::option(map(float)))));
        assert!(!holds_float(&map(ValueType::Boolean)));
        let bounded = ValueType::BoundedQuantity(std::sync::Arc::new(
            filament_exact::BoundedQuantityType::new(
                filament_exact::UnitId::compound(Vec::new()),
                None,
                None,
            )
            .expect("in the unit"),
        ));
        assert!(!holds_float(&bounded));
    }
}

#[cfg(test)]
mod source_scope_tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::model::domain_package::DomainPackageRef;

    fn source(identity: &str) -> RawSourceRef {
        RawSourceRef::new("repo".to_owned(), identity.to_owned()).expect("a source reference")
    }

    fn port(package: &str) -> (DeclarationKey, EffectiveId) {
        (
            DeclarationKey {
                package: package.to_owned(),
                node: format!("ix://{package}/out"),
            },
            EffectiveId::new(
                None,
                filament_exact::OriginalDeclarationKey::new(package, format!("ix://{package}/Sys")),
            ),
        )
    }

    /// FR-480 "Selections and imports" (QSL-1638): unit `a` binds `N` to
    /// package `x` and unit `b` binds `N` to package `y`, both declaring the
    /// Port `out`. A protocol of each unit reads its own unit's package for
    /// a whole-model role `N` and a Port role `N::out`, whichever package is
    /// selected last; the primary unit reads `a`'s.
    #[trace("FR-480")]
    #[test]
    fn a_shared_model_alias_names_each_units_own_selection_and_port() {
        let mut spellings = ModelSpellings::new(source("a"));
        for (unit, identity) in [("a", "x"), ("b", "y")] {
            let package = spellings.add_package(DomainPackageRef {
                identity: identity.to_owned(),
            });
            spellings.bind_model(&source(unit), "N", Some(package));
        }
        let scope = crate::check::family::fixtures::empty_scope()
            .with_model_spellings(spellings)
            .with_source_owners(BTreeMap::from([
                ("Pa".to_owned(), source("a")),
                ("Pb".to_owned(), source("b")),
            ]))
            .with_model_ports(BTreeMap::from([
                (("x".to_owned(), "out".to_owned()), port("x")),
                (("y".to_owned(), "out".to_owned()), port("y")),
            ]));
        for (protocol, identity) in [("Pa", "x"), ("Pb", "y"), ("Unowned", "x")] {
            let unit = scope.for_declaration(protocol);
            assert_eq!(
                unit.model_selection("N")
                    .map(|selection| selection.identity.as_str()),
                Some(identity),
                "{protocol}"
            );
            let (expected_key, expected_owner) = port(identity);
            assert_eq!(
                unit.model_port("N::out"),
                Some((&expected_key, &expected_owner)),
                "{protocol}"
            );
            assert_eq!(unit.model_port("M::out"), None, "{protocol}: no alias M");
        }
    }

    /// FSpec FR-322 "Named type arguments": a unit alias is its qualified name
    /// beneath the unit, a model alias type is its own declaration, and a
    /// model scalar, though it resolves as a named type, names no alias.
    #[trace("FR-931-AC-16")]
    #[test]
    fn a_type_argument_alias_is_the_unit_s_or_a_model_alias_types_never_a_model_scalars() {
        let model_alias = crate::value::TypeArgumentAlias {
            package: "ix://domain".to_owned(),
            node: "ix://domain/Total".to_owned(),
        };
        let scope = crate::check::family::fixtures::scope_with(
            Default::default(),
            vec![
                ("Small".to_owned(), ValueType::Boolean),
                ("M::Total".to_owned(), ValueType::Boolean),
                ("M::Count".to_owned(), ValueType::Boolean),
            ],
        )
        .with_primary_source(source("unit"))
        .with_type_argument_aliases(
            std::collections::BTreeSet::from(["Small".to_owned()]),
            BTreeMap::from([("M::Total".to_owned(), model_alias.clone())]),
        );
        let own = scope.alias_declaration("Small").expect("the unit's alias");
        assert_eq!(own.node, "ix://unit/Small");
        assert_eq!(scope.alias_declaration("M::Total"), Some(model_alias));
        assert_eq!(scope.alias_declaration("M::Count"), None, "a model scalar");
        assert_eq!(scope.alias_declaration("Absent"), None);
    }
}
