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

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ops::ControlFlow;
use std::sync::Arc;

mod native_literal;
mod typing;

use super::ir::{
    Arithmetic, BodyBuilder, CheckedLiteral, Connective, DispatchTable, Node, NodeId, NodeKind,
    OrderedKind, RecordField, RecordSlot, Slot, Visit,
};
use super::navigation::Relationships;
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, Obligation, StageLimitCause,
};
use crate::model::key::DeclarationKey;
use crate::model::operation::OperationTable;
use crate::value::definition::{AdmittedIeeeProfile, AdmittedIntegerDivision};
use crate::value::enumeration::{mint_variant_id, AdmittedEnumDeclaration};
use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, ExprId, ExprNode, ExprRef,
    FieldInitializer, FunctionDeclaration,
};
use qsl_foundation::absence::AbsenceMode;
use quire_exact::Cancel;
use quire_exact::DecimalType;
use quire_exact::EffectiveId;
use quire_exact::EnumMember;
use quire_exact::EnumShape;
use quire_exact::IllTypedCause;
use quire_exact::Presence;
use quire_exact::Rational;
use quire_exact::{ArithmeticOperator, OrderingOperator};
use quire_exact::{CardinalityBound, CollectionKind, CollectionType, Integer};
use quire_exact::{NodeKey, VariantId};
use quire_exact::{Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    admits_equality_conversion, CompositeShape, EqualityOperand, EqualityOperator,
    FieldDeclaration, TypeEnvironment,
};
use quire_semantic_value::enumeration::{EnumDeclaration, EnumMemberIndex, EnumValue};
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::{
    check_comparable, result_unit, IdentifiedUnit, UnitOperation, UnitScope,
};

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
/// (QSpec FR-144's key order for an unordered enumeration). Each member is
/// keyed by [`mint_variant_id`] over the declaration and its case, and a case
/// name is any non-empty string, kept exactly. `None` when `cases` is empty,
/// repeats a name or has an empty name, which the model's intake refuses
/// before a variant type is declared.
fn variant_type_members(declaration: NodeKey, cases: &[String]) -> Option<Vec<EnumValue>> {
    let runtime = EnumDeclaration::model_owned(declaration, cases.to_vec()).ok()?;
    cases
        .iter()
        .map(|case| {
            let member = NodeKey::from_digest(*mint_variant_id(declaration, case).as_bytes());
            runtime.member(case, member).ok()
        })
        .collect()
}

/// A model variant type in the type environment (FR-056 "Variant types",
/// FR-081, FR-360 item 11): the effective type the normalized view gives a
/// `quire.meaning.model.variant-type/v1` declaration, named `M::<artifact
/// id>`, with its cases. One declaration holds both keys of the type and of
/// each case: the FR-081 effective identity and the checked node key, so
/// neither is derived from the other and neither depends on the alias a unit
/// gives the model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantTypeDeclaration {
    key: EffectiveId,
    name: String,
    declaration: DeclarationKey,
    node: NodeKey,
    cases: Vec<VariantCaseDeclaration>,
    members: Vec<EnumValue>,
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
        let members = variant_type_members(node, &names)?;
        let cases = cases
            .into_iter()
            .map(|(key, case)| {
                let variant = mint_variant_id(node, &case);
                VariantCaseDeclaration {
                    key,
                    name: case,
                    variant,
                }
            })
            .collect();
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
        self.key
    }

    /// The qualified name `M::<artifact id>`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The model record that declares the variant type.
    pub(crate) fn declaration(&self) -> &DeclarationKey {
        &self.declaration
    }

    /// The variant type's checked node key (QSpec FR-322 "Model-owned
    /// members" step 2), a function of the selected document's identity and
    /// the type node alone.
    pub fn node(&self) -> NodeKey {
        self.node
    }

    /// The cases, ascending by the UTF-8 bytes of their names.
    pub fn cases(&self) -> &[VariantCaseDeclaration] {
        &self.cases
    }

    /// The kernel enum shape of the type: its cases' checked keys in
    /// ascending UTF-8 byte order of the case names, which is QSpec FR-144's
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
        self.key
    }

    /// The case name, compared as exact UTF-8 bytes.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The case's checked key: the FR-141 member identity over the variant
    /// type's checked node and the case name.
    pub fn variant(&self) -> VariantId {
        self.variant
    }
}

/// The staged model type admission, held until the conformance stage has
/// judged every redefinition.
pub struct StagedTypes(pub quire_semantic_value::declaration::PendingTypeEnvironment);

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
    /// The declaring source unit's `RawSourceRef` (FR-001), the reference
    /// its caller named and S0 minted. Its authority and identity are the
    /// unit's owner (ADR-013 O-04, FR-091), carried by every declared
    /// record, tuple and function node key (FR-092); its revision and digest
    /// enter no key. The checked graph carries it on for the lock's
    /// `sources` entry. `check` holds no constant owner.
    pub source: qsl_foundation::source::provenance::RawSourceRef,
    /// The package's lock evidence (ADR-011 §2.4): the law `DefinitionRef`s
    /// a lowered operation may name (FR-093).
    pub lock_evidence: super::lowering::LockEvidence,
    /// FR-110: the header-selectable layers the unit's header profiles
    /// resolve to, ascending; the lock's `definition_selections` hold their
    /// `requires` closure.
    pub header_layers: Vec<crate::value::layer::HeaderLayer>,
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
    /// `type Name = T;` aliases, which create no declaration identity.
    pub aliases: Vec<(String, ValueType)>,
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
    /// other native declaration (QSpec `shared-grammar.md`: "Native
    /// declaration names are unique within the linked package").
    pub native_names: std::collections::BTreeSet<String>,
    /// Qualified names of imported model operations, predicates and clauses.
    pub model_operations: Vec<String>,
    /// Function declarations in declaration order.
    pub functions: Vec<FunctionDeclaration>,
    /// The admitted FR-148 IEEE profile, if the package selects one.
    pub ieee_profile: Option<AdmittedIeeeProfile>,
    /// The package's admitted `div`/`rem` law, when one was selected.
    pub integer_division: Option<AdmittedIntegerDivision>,
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
    /// Resolved signatures standing in for some `functions` entries' own
    /// type forms; see [`ResolvedSignatures`].
    pub resolved_signatures: ResolvedSignatures,
    /// FR-096: the span of each declared record, tuple, enum, or selected
    /// model alias, by its declared name, for declarations read from the
    /// unit (the FR-091 assembler fills it). A declaration built by hand has
    /// none, and its `declaration` occurrence then names no region.
    pub declared_type_spans: BTreeMap<String, qsl_foundation::Span>,
    /// FR-096, FR-302: the span of the unit declaration each function
    /// synthesized for dispatch without form spans of its own was derived
    /// from, by index into [`Self::functions`]: every occurrence in such a
    /// function resolves to that span. `link_dispatch` fills it for the
    /// precondition functions it synthesizes from the unit's `pre` clauses.
    pub synthesized_spans: BTreeMap<usize, qsl_foundation::Span>,
    /// FR-096-AC-1, ADR-013 C-21: the map of a body embedded in a document.
    /// The spans above are body offsets; with a map, each region resolves
    /// under the document's `RawSourceRef`, shifted by the body's offset in
    /// the document. `None` for a unit that is its own document. `Arc`
    /// so `regions()` shares one map rather than cloning the document and
    /// body text on every call.
    pub embedding: Option<Arc<qsl_foundation::source_map::SourceMap>>,
    /// The `UnitGraph` the assembler admitted over the unit's dimension and
    /// unit declarations (FR-091). Empty when the unit declares none.
    pub units: quire_semantic_value::unit::UnitGraph,
    /// The span of each dimension's and unit's declared name, by its node
    /// key (FR-096).
    pub nominal_spans: BTreeMap<quire_exact::NodeKey, qsl_foundation::Span>,
    /// Each `import ... as a` the S4 source resolution admitted, by its
    /// qualifier `a` (ADR-015 D-1, D-5). An import with no `as` binds no
    /// qualifier (FR-087-AC-13).
    pub imports: BTreeMap<String, super::AdmittedImport>,
    /// FR-091-AC-22: the profile selection each source function's `using`
    /// alias resolved to, by index into [`Self::functions`]. A function
    /// with no `using` alias, or built by hand, has no entry.
    pub function_selections: BTreeMap<usize, qsl_foundation::selection::ProfileSelection>,
    /// FR-104: the unit's state clauses in declaration order, each header
    /// name resolved by the FR-091 assembler. Clauses and functions share
    /// one selection namespace (FR-109).
    pub state_clauses: Vec<super::state_clause::StateClauseDeclaration>,
    /// FR-112: the unit's protocol declarations in declaration order, each
    /// carrying its own scoped anchors and declaration collection for
    /// FR-113's checker to resolve.
    pub protocols: Vec<qsl_forms::ProtocolDeclarationForm>,
    /// FR-114: each protocol's own `attempt`s, resolved by the FR-091
    /// assembler, index-aligned with [`Self::protocols`]; each
    /// inner list is in the same order as its protocol's own
    /// `ProtocolDeclarationForm::attempts`.
    pub protocol_attempts: Vec<Vec<super::state_clause::AttemptDeclaration>>,
    /// FR-325, FR-326: the unit's `temporal` clauses in declaration order,
    /// each as S2 built it, for the `TemporalTrace` family's `check`.
    pub temporal_clauses: Vec<super::temporal_clause::TemporalDeclaration>,
    /// FR-311, FR-392: the unit's `random`, `reward`, `workload` and
    /// `probabilistic` declarations in declaration order, each as S2 built
    /// it, for the `Probabilistic` family's check (FR-185, FR-186).
    pub probabilistic: Vec<super::ProbabilisticDeclaration>,
    /// FR-110, FR-326: the unit's header profile selections the
    /// `TemporalTrace` family's catalog holds, in source order. A clause is
    /// checked under the one of them, or refuses when there is none or
    /// several.
    pub temporal_selections: Vec<super::temporal_clause::TemporalSelection>,
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
    /// Test-only: check a package that holds a `terminal` declaration
    /// without refusing it as not emitted (FR-124).
    pub emit_unwritten_terminals: bool,
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
    pub identity: qsl_foundation::IdentityLimits,
}

impl PackageDeclarations {
    /// These declarations, to be checked without refusing a `terminal`
    /// declaration as not emitted, so the S3 value
    /// (`CheckedGraph::terminals`) stays testable until S4 writes it
    /// (FR-124).
    #[cfg(any(test, feature = "test-support"))]
    #[must_use]
    pub fn with_unwritten_terminals(mut self) -> Self {
        self.emit_unwritten_terminals = true;
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
        source: qsl_foundation::source::provenance::RawSourceRef,
        identity: qsl_foundation::IdentityLimits,
        selected: crate::model::intake::SelectedModels,
    ) -> Self {
        let (entries, meter) = selected.into_parts();
        let mut declarations = Self::new(source, identity);
        declarations.models = entries
            .iter()
            .map(|entry| super::AdmittedModel::from_view(&entry.view))
            .collect();
        declarations.model_inputs = Some((entries, meter));
        declarations
    }

    /// A package declared by the source unit `source` names, with no
    /// declaration yet and no lock evidence, whose node identities encode
    /// under `identity`.
    pub fn new(
        source: qsl_foundation::source::provenance::RawSourceRef,
        identity: qsl_foundation::IdentityLimits,
    ) -> Self {
        Self {
            model_inputs: None,
            pending_types: None,
            source,
            lock_evidence: super::lowering::LockEvidence::default(),
            header_layers: Vec::new(),
            types: TypeEnvironment::default(),
            model_records: ModelRecords::default(),
            relationships: Relationships::default(),
            operations: OperationTable::default(),
            enums: Vec::new(),
            variant_types: Vec::new(),
            aliases: Vec::new(),
            alias_names: std::collections::BTreeMap::new(),
            native_names: std::collections::BTreeSet::new(),
            model_operations: Vec::new(),
            functions: Vec::new(),
            ieee_profile: None,
            integer_division: None,
            dispatch_operations: Vec::new(),
            dispatch_tables: Vec::new(),
            models: Vec::new(),
            model_clauses: std::collections::BTreeMap::new(),
            resolved_signatures: ResolvedSignatures::default(),
            declared_type_spans: BTreeMap::new(),
            synthesized_spans: BTreeMap::new(),
            embedding: None,
            units: quire_semantic_value::unit::UnitGraph::default(),
            nominal_spans: BTreeMap::new(),
            imports: BTreeMap::new(),
            function_selections: BTreeMap::new(),
            state_clauses: Vec::new(),
            protocols: Vec::new(),
            protocol_attempts: Vec::new(),
            temporal_clauses: Vec::new(),
            probabilistic: Vec::new(),
            temporal_selections: Vec::new(),
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
            terminals: Vec::new(),
            emit_unwritten_terminals: false,
            operation_bodies: Vec::new(),
            selected_views: BTreeMap::new(),
            identity,
        }
    }

    /// FR-302: the unit's body for the operation `key`, if one resolved to
    /// it.
    pub fn operation_body(
        &self,
        key: &crate::model::key::DeclarationKey,
    ) -> Option<&super::OperationBodyDeclaration> {
        self.operation_bodies.iter().find(|body| &body.key == key)
    }

    /// FR-103, FR-302: the unit `pre` and `post` clauses that resolved to
    /// the operation `key`, in declaration order.
    pub fn unit_clauses_for(
        &self,
        key: &crate::model::key::DeclarationKey,
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
    /// The declared result type.
    pub result: ValueType,
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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModelRecords {
    /// By the value's declaration and the declaration the read is typed at,
    /// then the name there: a read looks a borrowed name up and allocates
    /// nothing, and a package with no redefinition under another name
    /// holds an empty map.
    renamed: BTreeMap<(quire_exact::NodeKey, quire_exact::NodeKey), BTreeMap<String, String>>,
    events: BTreeSet<quire_exact::NodeKey>,
}

impl ModelRecords {
    /// Record that the record `value` keeps the field `name` of the record
    /// `read_at` under its own field `own`.
    pub(crate) fn rename(
        &mut self,
        value: quire_exact::NodeKey,
        read_at: quire_exact::NodeKey,
        name: impl Into<String>,
        own: impl Into<String>,
    ) {
        self.renamed
            .entry((value, read_at))
            .or_default()
            .insert(name.into(), own.into());
    }

    /// Record that `declaration` is an event type.
    pub(crate) fn declare_event(&mut self, declaration: quire_exact::NodeKey) {
        self.events.insert(declaration);
    }

    /// Every entry of `other`.
    pub(crate) fn extend(&mut self, other: ModelRecords) {
        for (key, names) in other.renamed {
            self.renamed.entry(key).or_default().extend(names);
        }
        self.events.extend(other.events);
    }

    /// The name under which a value of `declaration` keeps `field`.
    fn own_name<'f>(
        &'f self,
        declaration: quire_exact::NodeKey,
        field: &'f RecordField,
    ) -> &'f str {
        self.renamed
            .get(&(declaration, field.declaration()))
            .and_then(|names| names.get(field.name()))
            .map_or(field.name(), String::as_str)
    }
}

/// Everything names resolve against, apart from function bodies.
#[derive(Clone, Debug)]
pub struct Scope {
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
    pub(crate) integer_division: Option<AdmittedIntegerDivision>,
    pub(crate) complete_value: bool,
    /// Every admitted model currently uses `quire.model.complete/v1`; an
    /// absent model keeps the historical exact-edge typing rule.
    pub(crate) complete_model: bool,
    pub(crate) dispatch_operations: Vec<DispatchOperation>,
    /// The by-name lookups over the declared types, enums, aliases and
    /// model operations, built once by [`Self::new`]. Aliases and
    /// model operations are only ever looked up by name, so only this index
    /// holds them.
    index: ScopeIndex,
    /// Each admitted import by its qualifier (ADR-015 D-5).
    imports: BTreeMap<String, super::AdmittedImport>,
}

/// Drops the chain of imported checked graphs over an explicit stack
/// (ADR-030): each import holds its library's graph, whose scope holds that
/// library's own imports, so a derived drop would recurse once per library
/// of an import chain.
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
                let members = index.enum_member_index.filtered(shape.variants());
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
            types,
            model_records: ModelRecords::default(),
            relationships: Relationships::default(),
            enums,
            variant_types: Vec::new(),
            operations,
            ieee_profile,
            integer_division: None,
            complete_value: false,
            complete_model: false,
            dispatch_operations,
            index,
            imports: BTreeMap::new(),
        }
    }

    /// This scope with `imports`, each admitted import by its qualifier.
    pub(crate) fn with_imports(mut self, imports: BTreeMap<String, super::AdmittedImport>) -> Self {
        self.imports = imports;
        self
    }

    pub(crate) fn with_integer_division(
        mut self,
        division: Option<AdmittedIntegerDivision>,
    ) -> Self {
        self.integer_division = division;
        self
    }

    pub(crate) fn with_complete_value(mut self, complete_value: bool) -> Self {
        self.complete_value = complete_value;
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
    pub(crate) fn with_variant_types(mut self, variant_types: Vec<VariantTypeDeclaration>) -> Self {
        self.index.variant_types(&variant_types);
        self.variant_types = variant_types;
        self
    }

    pub(crate) fn with_complete_model(mut self, complete_model: bool) -> Self {
        self.complete_model = complete_model;
        self
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

    /// Whether `name` is a declared type or enum of the unit, which a
    /// protocol reference cannot name (FR-218).
    pub(crate) fn declares_type(&self, name: &str) -> bool {
        self.index.types.contains_key(name)
    }

    /// The checked graph of the admitted import whose library has
    /// `package_id` `package` (ADR-015 D-5): where an imported call's callee
    /// is evaluated.
    pub fn imported_graph(
        &self,
        package: crate::library::PackageId,
    ) -> Option<&super::CheckedGraph> {
        self.imports
            .values()
            .find(|import| import.view.package() == package)
            .map(|import| import.graph.as_ref())
    }

    /// The admitted import whose qualifier is `qualifier`.
    pub(crate) fn imported(&self, qualifier: &str) -> Option<&super::AdmittedImport> {
        self.imports.get(qualifier)
    }

    /// Every type `name` binds -- an alias, composite, enum or object type --
    /// in resolution order. Empty when `name` binds no type.
    pub(crate) fn named_types(&self, name: &str) -> &[ValueType] {
        self.index.types.get(name).map_or(&[], Vec::as_slice)
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

    /// The package's admitted enum declarations, in declaration order.
    pub(crate) fn enums(&self) -> &[EnumBinding] {
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

    /// The variant type `name` names, when it is a variant type name.
    pub(crate) fn variant_type_named(&self, name: &str) -> Option<&VariantTypeDeclaration> {
        let (position, _) = *self.index.variant_cases.get(name)?.values().next()?;
        self.variant_types.get(position)
    }

    /// The runtime member of the case `name` (`T::Case`) names, with its
    /// variant type's enum shape.
    pub(crate) fn variant_case_named(&self, name: &str) -> Option<(EnumShape, &EnumValue)> {
        let (type_name, case) = name.rsplit_once("::")?;
        let &(position, member) = self.index.variant_cases.get(type_name)?.get(case)?;
        let declared = self.variant_types.get(position)?;
        Some((declared.shape(), declared.members().get(member)?))
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
            None => self.index.enum_member_index.filtered(shape.variants()),
        }
    }

    /// The first object type named `name`, by its effective identity.
    pub(crate) fn object_type_named(&self, name: &str) -> Option<EffectiveId> {
        self.index.object_types.get(name).copied()
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
    /// member is bound to its QSpec FR-441 key over that node id, minted
    /// here once, so the `VariantId` to (union, member, position) index the
    /// type environment keeps is built exactly once and no other stage
    /// derives it. Lowering produced `nodes`; a union it left out is an
    /// internal fault.
    pub(crate) fn seal_unions(
        &mut self,
        nodes: &BTreeMap<quire_exact::NodeKey, quire_exact::NodeKey>,
    ) -> Result<(), CheckRefusal> {
        let fault = |invariant: &'static str| CheckRefusal {
            location: Location::root(quire_semantic_value::location::Origin::Expression),
            cause: CheckCause::InternalFault(Box::new(super::refusal::KeyFault::StageFault(
                qsl_foundation::InternalFault::new("S3", invariant),
            ))),
        };
        let unions: Vec<(quire_exact::NodeKey, Vec<quire_exact::Identifier>)> = self
            .types
            .composites()
            .filter_map(|declaration| match declaration.shape() {
                CompositeShape::Union(members) => Some((
                    declaration.key(),
                    members
                        .iter()
                        .map(|member| member.identifier().clone())
                        .collect(),
                )),
                CompositeShape::Record(_) | CompositeShape::Tuple(_) => None,
            })
            .collect();
        for (handle, members) in unions {
            let node = *nodes
                .get(&handle)
                .ok_or_else(|| fault("union-node-missing"))?;
            let bindings = members
                .into_iter()
                .map(|member| {
                    let key = super::sum_case::mint_union_member_key(node, member.as_str());
                    quire_exact::UnionMember::from_admitted(node, key, member)
                })
                .collect();
            self.types
                .seal_union_verified(handle, node, bindings)
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
        let mut seen = BTreeSet::new();
        let mut pending = vec![value_type];
        while let Some(value_type) = pending.pop() {
            match value_type {
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Composite(declaration) => {
                    if self.model_records.events.contains(declaration) {
                        return true;
                    }
                    if !seen.insert(*declaration) {
                        continue;
                    }
                    match self.types.composite(*declaration).map(|c| c.shape()) {
                        Some(CompositeShape::Record(fields)) => {
                            pending.extend(fields.iter().map(FieldDeclaration::value_type));
                        }
                        Some(CompositeShape::Tuple(positions)) => pending.extend(positions),
                        Some(CompositeShape::Union(members)) => {
                            pending.extend(members.iter().flat_map(|member| member.positions()));
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// The position, in the record `declaration`'s own fields, of the field
    /// `field` names (FR-359 item 2): found in the value's own declaration by
    /// the field's identity, never by a position computed from the record
    /// the read was typed at. `None` when the declaration is no record or
    /// holds no such field.
    pub fn record_slot(
        &self,
        declaration: quire_exact::NodeKey,
        field: &RecordField,
    ) -> Option<usize> {
        let CompositeShape::Record(fields) = self.types.composite(declaration)?.shape() else {
            return None;
        };
        let name = self.model_records.own_name(declaration, field);
        fields.iter().position(|candidate| candidate.name() == name)
    }

    /// The package's model object types' declared operations (FR-103).
    pub fn operations(&self) -> &OperationTable {
        &self.operations
    }

    /// The package's admitted IEEE profile, if it selects one.
    pub fn ieee_profile(&self) -> Option<&AdmittedIeeeProfile> {
        self.ieee_profile.as_ref()
    }

    /// The package's FR-151 dispatch operations, indexed by a checked
    /// `NodeKind::Dispatch`'s `operation`.
    pub fn dispatch_operations(&self) -> &[DispatchOperation] {
        &self.dispatch_operations
    }
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
    complete_value: bool,
    signatures: &'a Signatures,
    context: Option<crate::family::CheckContext<'a, Scope>>,
    limits: CheckingLimits,
    nodes: &'a mut u64,
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
}

/// A state clause's own typing context (FR-104 "Typing"): its kind, the
/// operation a `pre` or `post` clause names, and the slots `self` and
/// `result` are bound to.
#[derive(Clone, Debug)]
pub(crate) struct StateContext {
    pub(crate) parent_projections:
        BTreeMap<quire_semantic_value::declaration::FieldRef, (ValueType, Presence)>,
    /// The clause kind.
    pub(crate) kind: qsl_forms::StateClauseKind,
    /// The operation's name, for a `pre` or `post` clause.
    pub(crate) operation: Option<String>,
    /// The slot `self` reads.
    pub(crate) self_slot: Slot,
    /// The slot `result` reads: only in a postcondition of an operation
    /// that declares a result.
    pub(crate) result_slot: Option<Slot>,
}

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

fn mismatch(location: &Location) -> CheckRefusal {
    CheckRefusal::ill_typed(location, IllTypedCause::TypeMismatch)
}

fn ineligible(location: &Location) -> CheckRefusal {
    CheckRefusal::ill_typed(location, IllTypedCause::OperatorIneligible)
}

fn node(kind: NodeKind, value_type: ValueType, location: &Location) -> Node {
    Node {
        kind,
        value_type,
        location: location.clone(),
    }
}

fn is_integer(value_type: &ValueType) -> bool {
    matches!(value_type, ValueType::Integer | ValueType::Int(_))
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
            ExprNode::Name(name) => return Some(name),
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

/// Whether an expression takes its type from its context.
fn contextual(expression: ExprRef<'_>) -> bool {
    match expression.node() {
        ExprNode::Integer(_) | ExprNode::Rational(..) | ExprNode::Collection { .. } => true,
        ExprNode::Negate(operand) => matches!(expression.at(*operand).node(), ExprNode::Integer(_)),
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
            return Ok(node);
        }
        match (&node.value_type, required) {
            // FR-359 items 1 and 6: a record whose most-specific type
            // conforms to the slot's record type is admitted as it is, with
            // no conversion node; its own type stays on the node. A type
            // constructor over a record is not covariant, so only the slot's
            // own record type reaches this arm.
            (ValueType::Composite(sub), ValueType::Composite(sup))
                if self.scope.types.composite_conforms(*sub, *sup) =>
            {
                Ok(node)
            }
            (ValueType::Integer | ValueType::Int(_), ValueType::Integer) => Ok(node),
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
            complete_value: scope.complete_value,
            signatures,
            context: None,
            limits,
            nodes,
            cancel: None,
            locals: Vec::new(),
            local_names: HashMap::new(),
            slots: 0,
            slot_names: Vec::new(),
            clause_kind,
            state: None,
            units: UnitScope::new(scope.types.units()),
            body: BodyBuilder::default(),
        }
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

    /// Borrow the enclosing family's stage state for nested model checks.
    pub(crate) fn with_context(mut self, cx: crate::family::CheckContext<'a, Scope>) -> Self {
        self.context = Some(cx);
        self
    }

    fn model_check(
        &mut self,
        form: super::state_model::Form,
        location: &Location,
    ) -> Result<super::state_model::Checked, CheckRefusal> {
        use crate::family::FamilyContract;
        use qsl_foundation::diagnostic::StageFailure;
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
            StageFailure::Limit(limit) => CheckRefusal {
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

    /// Every compound unit this pass formed (FR-094: kept until lowering
    /// keys each one's type node).
    pub(crate) fn into_formed_units(self) -> quire_semantic_value::quantity::UnitTable {
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

    /// Bind a parameter or local name, refusing a name already in scope.
    pub(crate) fn bind(
        &mut self,
        name: &str,
        value_type: ValueType,
        location: &Location,
        kind: LocalKind,
    ) -> Result<Slot, CheckRefusal> {
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
                ExprNode::Let { name, value, body } => {
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
                            if binding.name == name {
                                alias = Some(binding.alias);
                                break;
                            }
                            at = binding.outer;
                        }
                        resolved
                            .push(alias.unwrap_or_else(|| self.captured_before(name, boundary)));
                    }
                    ExprNode::Let { name, value, body } => {
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
        self.scope
            .types
            .check_type(value_type)
            .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
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
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Float(_)
                | ValueType::Quantity(_)
                | ValueType::Text(_)
                | ValueType::String
                | ValueType::Composite(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => {}
                ValueType::Uuid | ValueType::Timestamp => {}
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
        super::type_form::resolve_named_type(self.scope, name, location)
    }

    /// Resolve a declared `TypeForm` (S2) to the kernel `ValueType` (E3,
    /// ADR-013 O-14/C-26). The one production entry into
    /// [`super::type_form::resolve_type_form`], which [`Self::type_named`]
    /// above's [`super::type_form::resolve_named_type`] also backs for the
    /// qualified-name case.
    pub(crate) fn resolve_type(
        &self,
        form: &qsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        super::type_form::resolve_type_form(self.scope, form, location)
    }

    fn element_type(&self, collection: &Node) -> Result<ValueType, CheckRefusal> {
        match &collection.value_type {
            ValueType::Collection(collection_type) => Ok(collection_type.element().clone()),
            _ => Err(mismatch(&collection.location)),
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
                ))
            }
            Some(ValueType::Rational(domain)) => domain,
            Some(_) => return Err(mismatch(location)),
        };
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
                let rank = u32::try_from(member.position())
                    .expect("an admitted enum has far fewer than u32::MAX members");
                Ok(node(
                    NodeKind::Literal(CheckedLiteral(Value::Enum(EnumMember::new(
                        mint_variant_id(binding.declaration.key(), member.case()),
                        rank,
                    )))),
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
        if let Some((shape, member)) = self.scope.variant_case_named(name) {
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
        let undeclared_case = name
            .rsplit_once("::")
            .is_some_and(|(type_name, _)| self.scope.variant_type_named(type_name).is_some());
        if undeclared_case
            || self.signatures.declares(name)
            || (self.scope.is_variant_case_name(name)
                && matches!(expected, Some(ValueType::Enum(shape)) if self.scope.is_variant_shape(shape)))
        {
            return Err(mismatch(location));
        }
        Err(refuse(location, CheckCause::MissingName(name.to_owned())))
    }

    /// `left = right` or `left != right` over its two typed operands, each
    /// with the equality operand it compares as.
    fn equality(
        &mut self,
        operator: EqualityOperator,
        (left, left_operand): (Node, EqualityOperand),
        (right, right_operand): (Node, EqualityOperand),
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        // FR-359 item 4: FR-208 gives an event type no equality, so an
        // operand holding one is ineligible, not compared.
        if self.scope.reaches_event(&left.value_type) || self.scope.reaches_event(&right.value_type)
        {
            return Err(ineligible(location));
        }
        let checked = self
            .scope
            .types
            .check_equality_in(
                &self.units,
                operator,
                left_operand,
                right_operand,
                &|shape: &EnumShape| self.scope.enum_members_of(shape),
            )
            .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
        Ok(node(
            NodeKind::Equality(
                operator,
                Box::new(checked),
                self.push(left),
                self.push(right),
            ),
            ValueType::Boolean,
            location,
        ))
    }

    /// An ordering over its two typed operands.
    fn ordering(
        &mut self,
        operator: OrderingOperator,
        left: Node,
        right: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
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
            (ValueType::Quantity(l), ValueType::Quantity(r)) => {
                let (Some(l), Some(r)) = (self.units.get(*l), self.units.get(*r)) else {
                    return Err(mismatch(location));
                };
                check_comparable(l, r)
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
                OrderedKind::Quantities
            }
            // FR-148 selects IEEE ordering only through the `totalOrder`
            // intrinsic; the grammar's orderings select none.
            (ValueType::Float(_), ValueType::Float(_))
            | (ValueType::Boolean, ValueType::Boolean)
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
            _ => return Err(mismatch(location)),
        };
        Ok(node(
            NodeKind::Order(operator, kind, self.push(left), self.push(right)),
            ValueType::Boolean,
            location,
        ))
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
                        ))
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
                    _ => l.result_of(operator, r),
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
            (ValueType::Decimal(_), ValueType::Decimal(_), _) => {
                let target = match hint {
                    Some(ValueType::Decimal(target)) => target.clone(),
                    Some(_) => return Err(mismatch(location)),
                    None => {
                        return Err(CheckRefusal::ill_typed(
                            location,
                            IllTypedCause::AmbiguousLiteral,
                        ))
                    }
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
            (ValueType::Quantity(l), ValueType::Quantity(r), _) => {
                let operation = match operator {
                    ArithmeticOperator::Add => UnitOperation::Add,
                    ArithmeticOperator::Subtract => UnitOperation::Subtract,
                    ArithmeticOperator::Multiply => UnitOperation::Multiply,
                    ArithmeticOperator::Divide => UnitOperation::Divide,
                };
                let (Some(l), Some(r)) = (self.units.get(*l), self.units.get(*r)) else {
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
            _ => return Err(mismatch(location)),
        };
        Ok(node(kind, value_type, location))
    }

    /// Unary `-` over its typed operand: integers, rationals and decimals.
    /// FR-148 and FR-142 define no negation of an IEEE value or a quantity.
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
            ValueType::Float(_) | ValueType::Quantity(_) => return Err(ineligible(location)),
            ValueType::Boolean
            | ValueType::Text(_)
            | ValueType::Enum(_)
            | ValueType::Option(_)
            | ValueType::Composite(_)
            | ValueType::Collection(_)
            | ValueType::Reference(_)
            | ValueType::Population(_)
            | ValueType::Uuid
            | ValueType::String
            | ValueType::Timestamp => return Err(mismatch(location)),
        };
        Ok(node(kind, value_type, location))
    }

    /// Whether `x.name` over an `x` of `value_type` is a model read (FR-104,
    /// FR-363) rather than a record field read. A reference is one in a state
    /// clause, and elsewhere when `name` names a relationship end navigable
    /// from it. An `Option`, a collection or a population of references is one
    /// in a function body as in a state clause, which the hook refuses at the
    /// operand (FR-363).
    pub(crate) fn is_model_receiver(&self, value_type: &ValueType, name: &str) -> bool {
        match value_type {
            ValueType::Reference(object) => {
                self.in_state_clause()
                    || self
                        .scope
                        .relationships()
                        .navigable(&self.scope.types, *object, name)
                        .next()
                        .is_some()
            }
            ValueType::Option(element) => Self::holds_references(element),
            ValueType::Collection(collection) => Self::holds_references(collection.element()),
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
            | ValueType::Quantity(_) => false,
        }
    }

    fn holds_references(value_type: &ValueType) -> bool {
        match value_type {
            ValueType::Reference(_) => true,
            ValueType::Option(element) => Self::holds_references(element),
            ValueType::Collection(collection) => Self::holds_references(collection.element()),
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
            | ValueType::Population(_) => false,
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
        operand_location: &Location,
        location: &Location,
        derefed: bool,
    ) -> Result<Node, CheckRefusal> {
        let field_reads = derefed || self.in_state_clause();
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
            return Err(refuse(location, CheckCause::MissingName("self".to_owned())));
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
    /// outside the `pre` is a capture (QSpec's `let s = self in
    /// pre(s.version)` row), and one bound inside it names no parameter.
    /// Parameters, literals and `result` alone are never eligible (QSpec:
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
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Composite(key) = operand.value_type else {
            return Err(mismatch(location));
        };
        let Some(CompositeShape::Record(fields)) = self
            .scope
            .types
            .composite(key)
            .map(|declaration| declaration.shape())
        else {
            return Err(mismatch(location));
        };
        let declared = fields
            .iter()
            .find(|declared| declared.name() == field)
            .ok_or_else(|| mismatch(location))?;
        let optional = declared.presence() == Presence::Optional;
        let value_type = if optional {
            ValueType::option(declared.value_type().clone())
        } else {
            declared.value_type().clone()
        };
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

    /// `receiver.member(args)` (FR-151, `quire.model.dispatch.single/v1`):
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
    /// (`quire.model.dispatch.single/v1`: "its arguments are type-checked
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
        mut typed: Node,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        typed.value_type = self.model_type(
            super::state_model::Form::Argument {
                actual: typed.value_type.clone(),
                required: required.clone(),
            },
            location,
        )?;
        Ok(typed)
    }

    /// The record declaration `name` a record literal builds, and its
    /// declared fields.
    fn record_declaration(
        &self,
        name: &str,
        location: &Location,
    ) -> Result<(quire_exact::NodeKey, &'a [FieldDeclaration]), CheckRefusal> {
        let scope: &'a Scope = self.scope;
        let ValueType::Composite(key) = self.type_named(name, location)? else {
            return Err(mismatch(location));
        };
        let Some(CompositeShape::Record(declared)) = scope
            .types
            .composite(key)
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
        supplied: &mut [Option<RecordSlot>],
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
                *slot = Some(RecordSlot::Null);
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
        key: quire_exact::NodeKey,
        declared: &[FieldDeclaration],
        supplied: Vec<Option<RecordSlot>>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let mut slots = Vec::with_capacity(declared.len());
        for (declaration, slot) in declared.iter().zip(supplied) {
            slots.push(match slot {
                Some(slot) => slot,
                None if declaration.presence() == Presence::Optional => RecordSlot::Absent,
                None => return Err(mismatch(location)),
            });
        }
        Ok(node(
            NodeKind::Record {
                declaration: key,
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
        target: &qsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        let target = self.resolve_type(target, location)?;
        self.check_declared_type(&target, location)?;
        Ok(target)
    }

    /// `convert<target>(e)` over the typed operand `e`.
    fn convert(
        &mut self,
        target: &ValueType,
        operand: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match (&operand.value_type, target) {
            (ValueType::Collection(source), ValueType::Collection(to)) => {
                if source.element() != to.element() {
                    return Err(mismatch(location));
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
            (ValueType::Rational(_) | ValueType::Decimal(_), ValueType::Decimal(decimal))
                if !admits_equality_conversion(&operand.value_type, target, &self.units) =>
            {
                Ok(node(
                    NodeKind::ConvertDecimal(self.push(operand), decimal.clone()),
                    target.clone(),
                    location,
                ))
            }
            (source, _) => {
                if matches!(source, ValueType::Float(_))
                    || !admits_equality_conversion(source, target, &self.units)
                {
                    return Err(mismatch(location));
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
        target: &qsl_forms::TypeForm,
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
    /// evaluation time (`qsl-eval`'s `value::expression::evaluate`), so
    /// [`NodeKind::Lookup`] does not restate it. FR-153 refuses
    /// `ill_typed`/`type-mismatch` at check time when `S` does not conform to
    /// `T` (TC-198 L03's last case, "before any charge"): decided here from
    /// the package's own admitted generalization graph
    /// ([`quire_semantic_value::declaration::TypeEnvironment::conforms`]), whose
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
                return Err(mismatch(location))
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
        self.scope
            .types
            .check_equality_in(
                &self.units,
                EqualityOperator::Equal,
                EqualityOperand::typed(element.clone()),
                EqualityOperand::typed(element),
                &|shape: &EnumShape| self.scope.enum_members_of(shape),
            )
            .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
        Ok(node(
            NodeKind::Contains(self.push(collection), self.push(item)),
            ValueType::Boolean,
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
            NodeKind::Connective(Connective::And | Connective::Or, left, right),
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
    impl<F: Fn(&Node) -> bool> quire_walk::Walk for FreeOfAccumulator<'_, F> {
        type Node = NodeId;
        type Frame = ();
        type Stop = ();

        fn enter(
            &mut self,
            id: NodeId,
            children: &mut quire_walk::Children<'_, NodeId>,
        ) -> ControlFlow<(), ()> {
            match self.body.get(id) {
                Some(node) if !(self.is_accumulator)(node) => {
                    children.extend(node.kind.children());
                    ControlFlow::Continue(())
                }
                _ => ControlFlow::Break(()),
            }
        }

        fn exit(&mut self, (): ()) -> ControlFlow<()> {
            ControlFlow::Continue(())
        }
    }
    quire_walk::walk(
        &mut FreeOfAccumulator {
            body,
            is_accumulator,
        },
        other,
    )
    .is_continue()
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
