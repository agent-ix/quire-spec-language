// SPDX-License-Identifier: AGPL-3.0-or-later
//! Name resolution and typing of Complete-V1 value expressions and function
//! declarations into the typed tree, before any definedness or termination
//! judgment.
//!
//! Every refusal here is made before any charge. Typing nesting is bounded
//! by the declared [`CheckingLimits`] depth; reaching a declared limit is
//! `resource_exhausted`, never an admission verdict. The typer itself
//! (`typing`) walks an expression over an explicit heap stack (QSL-228), so
//! its host stack use does not grow with nesting.
//!
//! FR-065 (owner ruling, carried from the QSL-25 spec review): this module
//! retains `infer_form`'s dispatch over [`Expression`] for every `Value`
//! form. **QSL-148 moves function-application checking out of this module.**
//! `Self::call` -- the method `infer_form`'s `Expression::Call` arm used to
//! dispatch to, doing the real name resolution, arity check and
//! per-argument typing -- is deleted; its logic now lives in
//! [`super::family::Application`], reached through this module's own
//! `Typer::scope`/`Typer::signatures`/`Typer::type_named`/`Typer::check_as`
//! accessors (widened from private to `pub(crate)` for exactly this one
//! caller). `infer_form`'s `Expression::Call` arm makes one call into that
//! function and holds no semantic logic of its own (FR-065-AC-4). This
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

use std::collections::{BTreeMap, BTreeSet};

mod typing;

use super::ir::{
    Arithmetic, Connective, DispatchTable, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit,
};
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, Location, Obligation,
};
use crate::value::declaration::{
    admits_equality_conversion, CompositeShape, EqualityOperand, EqualityOperator,
    FieldDeclaration, TypeEnvironment,
};
use crate::value::definition::AdmittedIeeeProfile;
use crate::value::enumeration::{mint_variant_id, EnumDeclaration, EnumMemberIndex, EnumValue};
use crate::value::quantity::{check_comparable, result_unit, UnitOperation, UnitScope};
use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
    FunctionDeclaration,
};
use qsl_foundation::absence::AbsenceMode;
use quire_exact::DecimalType;
use quire_exact::EffectiveId;
use quire_exact::EnumMember;
use quire_exact::EnumShape;
use quire_exact::IllTypedCause;
use quire_exact::Presence;
use quire_exact::Rational;
use quire_exact::{ArithmeticOperator, OrderingOperator};
use quire_exact::{CardinalityBound, CollectionKind, CollectionType, Integer};
use quire_exact::{Value, ValueType};

/// The largest expression nesting depth a checker may declare. The typing,
/// facts and lowering walks run over explicit heap stacks, so this bounds the
/// checked tree's size rather than protecting the host stack.
pub const MAX_CHECKING_DEPTH: u64 = 128;

/// The NFR-010 node admission limits this checker declares before accepting
/// a package.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CheckingLimits {
    nodes: u64,
    depth: u64,
    /// The checked-family contract's own preimage byte-length bound
    /// (QSL-153; `crate::family::StageLimits::input_bytes`'s one
    /// caller-configurable knob). Unlimited (`u64::MAX`) unless
    /// [`Self::with_input_bytes`] narrows it -- the current, unbounded
    /// behavior every existing caller keeps by default.
    input_bytes: u64,
    /// The checked-family contract's own shared-meter `work_units` bound
    /// (QSL-153; PR #302 review finding 3 -- `StageLimitKind::WorkBudget`'s
    /// one caller-configurable knob, since that kind is produced by a
    /// denied charge against the contract meter, not a `StageLimits`
    /// field). Unlimited (`u64::MAX`) unless [`Self::with_work_budget`]
    /// narrows it.
    work_budget: u64,
}

/// A declared checking depth above [`MAX_CHECKING_DEPTH`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("checking depth {depth} exceeds the maximum {MAX_CHECKING_DEPTH}")]
pub struct DepthAboveMaximum {
    /// The requested depth.
    pub depth: u64,
}

impl CheckingLimits {
    /// Admit at most `nodes` expression nodes per checked package or
    /// expression, nested at most `depth` deep.
    pub fn new(nodes: u64, depth: u64) -> Result<Self, DepthAboveMaximum> {
        if depth > MAX_CHECKING_DEPTH {
            return Err(DepthAboveMaximum { depth });
        }
        Ok(Self {
            nodes,
            depth,
            input_bytes: u64::MAX,
            work_budget: u64::MAX,
        })
    }

    /// The declared node limit.
    pub fn nodes(self) -> u64 {
        self.nodes
    }

    /// The declared depth limit.
    pub fn depth(self) -> u64 {
        self.depth
    }

    /// The checked-family contract's own preimage byte-length bound
    /// (QSL-153).
    pub fn input_bytes(self) -> u64 {
        self.input_bytes
    }

    /// Bound the checked-family contract's own preimage byte length
    /// (QSL-153): a declaration whose parsed structure encodes to more than
    /// `input_bytes` refuses with a `Limit` outcome naming
    /// `CheckingLimitKind::InputBytes`, before the identity it would have
    /// minted is ever used.
    pub fn with_input_bytes(mut self, input_bytes: u64) -> Self {
        self.input_bytes = input_bytes;
        self
    }

    /// The checked-family contract's own shared-meter `work_units` bound
    /// (QSL-153).
    pub fn work_budget(self) -> u64 {
        self.work_budget
    }

    /// Bound the checked-family contract's own shared-meter `work_units`
    /// spend: once every declaration checked so far has together charged
    /// more than `work_budget` work units, the next declaration refuses
    /// with a `Limit` outcome naming `CheckingLimitKind::WorkBudget`, before
    /// the identity it would have minted is ever used.
    pub fn with_work_budget(mut self, work_budget: u64) -> Self {
        self.work_budget = work_budget;
        self
    }
}

impl Default for CheckingLimits {
    /// Unlimited nodes, input bytes and work budget, at the maximum depth.
    fn default() -> Self {
        Self {
            nodes: u64::MAX,
            depth: MAX_CHECKING_DEPTH,
            input_bytes: u64::MAX,
            work_budget: u64::MAX,
        }
    }
}

/// An enum declaration bound to its source name, with its admitted members.
/// A member `m` is named `name::m`.
#[derive(Clone, Debug)]
pub struct EnumBinding {
    /// The declared source name.
    pub name: String,
    /// The admitted declaration.
    pub declaration: EnumDeclaration,
    /// Its admitted members, in FR-141 canonical order (declaration order
    /// for an ordered enum, case-identifier byte order otherwise --
    /// `EnumDeclaration::admit` already refuses any other order).
    pub members: Vec<EnumValue>,
}

impl EnumBinding {
    /// This binding's kernel `ValueType::Enum` shape (ADR-013 O-14): the
    /// FR-141 canonical, ranked member list. `self.members` is already in
    /// that order, so each member's rank is exactly its index here.
    pub fn shape(&self) -> EnumShape {
        EnumShape::new(
            self.declaration.preimage().is_ordered(),
            self.members.iter().map(EnumValue::variant),
        )
    }
}

/// A function's resolved signature: its parameters' names paired with each
/// one's resolved `ValueType`, and the resolved result type.
pub(crate) type ResolvedSignature = (Vec<(String, ValueType)>, ValueType);

/// Resolved signatures that stand in for resolving some `functions`
/// entries' own `TypeForm`s, keyed by index into
/// [`PackageDeclarations::functions`]. Only `check::checked_dispatch` can
/// populate it: its FR-151 synthesized clauses come from an
/// already-resolved model signature, not from parsed source. Every other
/// caller holds the empty default.
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

/// The closed declarations of one package that expressions resolve against.
#[derive(Clone, Debug)]
pub struct PackageDeclarations {
    /// The declaring source unit's owner (ADR-013 O-04, FR-091): a required
    /// E3 input, carried by every declared record, tuple and function node
    /// key (FR-092). `check` holds no constant owner.
    pub owner: super::node_key::SourceOwner,
    /// The package's lock evidence (ADR-011 §2.4): the law `DefinitionRef`s
    /// a lowered operation may name (FR-093).
    pub lock_evidence: super::lowering::LockEvidence,
    /// Record, tuple and model object-type declarations.
    pub types: TypeEnvironment,
    /// Enum declarations.
    pub enums: Vec<EnumBinding>,
    /// `type Name = T;` aliases, which create no declaration identity.
    pub aliases: Vec<(String, ValueType)>,
    /// Qualified names of imported model operations, predicates and clauses.
    pub model_operations: Vec<String>,
    /// Function declarations in declaration order.
    pub functions: Vec<FunctionDeclaration>,
    /// The admitted FR-148 IEEE profile, if the package selects one.
    pub ieee_profile: Option<AdmittedIeeeProfile>,
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
}

impl PackageDeclarations {
    /// A package declared by `owner`'s source unit, with no declaration yet
    /// and no lock evidence.
    pub fn new(owner: super::node_key::SourceOwner) -> Self {
        Self {
            owner,
            lock_evidence: super::lowering::LockEvidence::default(),
            types: TypeEnvironment::default(),
            enums: Vec::new(),
            aliases: Vec::new(),
            model_operations: Vec::new(),
            functions: Vec::new(),
            ieee_profile: None,
            dispatch_operations: Vec::new(),
            dispatch_tables: Vec::new(),
            models: Vec::new(),
            model_clauses: std::collections::BTreeMap::new(),
            resolved_signatures: ResolvedSignatures::default(),
        }
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

/// Everything names resolve against, apart from function bodies.
#[derive(Clone, Debug)]
pub struct Scope {
    /// Fixed once built: [`Self::index`] is derived from it (QSL-205).
    types: TypeEnvironment,
    /// Fixed once built, like `types`.
    enums: Vec<EnumBinding>,
    pub(crate) ieee_profile: Option<AdmittedIeeeProfile>,
    pub(crate) dispatch_operations: Vec<DispatchOperation>,
    /// The by-name lookups over the declared types, enums, aliases and
    /// model operations, built once by [`Self::new`] (QSL-205). Aliases and
    /// model operations are only ever looked up by name, so only this index
    /// holds them.
    index: ScopeIndex,
}

/// `Scope`'s by-name lookups (QSL-205), keyed by the parsed parts of a name,
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
        enums: Vec<EnumBinding>,
        aliases: Vec<(String, ValueType)>,
        model_operations: Vec<String>,
        ieee_profile: Option<AdmittedIeeeProfile>,
        dispatch_operations: Vec<DispatchOperation>,
    ) -> Self {
        let index = ScopeIndex::new(&types, &enums, &aliases, &model_operations);
        Self {
            types,
            enums,
            ieee_profile,
            dispatch_operations,
            index,
        }
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

    /// The package's admitted enum declarations, in declaration order.
    pub(crate) fn enums(&self) -> &[EnumBinding] {
        &self.enums
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
    /// §6.1's own module split). Built once with the scope (QSL-205).
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
/// functions, with a by-name index built once (QSL-205) so resolving a call
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

    /// Every signature, in declaration order.
    pub fn iter(&self) -> std::slice::Iter<'_, Signature> {
        self.entries.iter()
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
    /// Whether an ordinary named [`Expression::Call`] may resolve to this
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
    signatures: &'a Signatures,
    limits: CheckingLimits,
    nodes: &'a mut u64,
    depth: u64,
    locals: Vec<Local>,
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
    /// The package's quantity units, then every compound unit this pass
    /// formed as a product or quotient type.
    units: UnitScope<'a>,
}

fn refuse(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
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
/// The walk keeps its pending sub-expressions on a heap stack (QSL-228): it
/// runs before the typer has entered `expression`'s own levels, so no
/// checking limit has bounded its depth yet.
fn contains_pre_eligible_read(expression: &Expression) -> bool {
    let mut pending = vec![expression];
    while let Some(expression) = pending.pop() {
        if matches!(
            expression,
            Expression::AllInstances { .. } | Expression::Lookup { .. } | Expression::Pre(_)
        ) {
            return true;
        }
        pending.extend(expression.children());
    }
    false
}

/// One `let` a captured-alias walk has passed (QSL-228): its name, whether
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
    Operand(&'e Expression, Option<usize>),
    /// The value of `let name = value in body` is resolved on top of the
    /// result stack: bind it, then resolve `body`.
    Bind(&'e str, &'e Expression, Option<usize>),
    /// One branch of an `if` is resolved on top of the result stack: keep a
    /// `true`, otherwise resolve the other branch.
    OrElse(&'e Expression, Option<usize>),
}

/// Whether an expression takes its type from its context.
fn contextual(expression: &Expression) -> bool {
    match expression {
        Expression::Integer(_) | Expression::Rational(..) | Expression::Collection { .. } => true,
        Expression::Negate(operand) => matches!(**operand, Expression::Integer(_)),
        Expression::Binary {
            operator: BinaryOperator::Divide,
            ..
        } => true,
        _ => false,
    }
}

/// Admit `node` where `required` is expected: identical types, an integer into
/// `Integer` or a containing `Int[..]`, or an integer into another `Int[..]`
/// through a range-obligated [`NodeKind::Coerce`].
pub(crate) fn coerce(node: Node, required: &ValueType) -> Result<Node, CheckRefusal> {
    if &node.value_type == required {
        return Ok(node);
    }
    match (&node.value_type, required) {
        (ValueType::Integer | ValueType::Int(_), ValueType::Integer) => Ok(node),
        (ValueType::Int(source), ValueType::Int(target))
            if target.lower() <= source.lower() && source.upper() <= target.upper() =>
        {
            Ok(node)
        }
        (ValueType::Integer | ValueType::Int(_), ValueType::Int(target)) => {
            let location = node.location.clone();
            Ok(Node {
                kind: NodeKind::Coerce(Box::new(node), target.clone()),
                value_type: required.clone(),
                location,
            })
        }
        _ => Err(mismatch(&node.location)),
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
            signatures,
            limits,
            nodes,
            depth: 0,
            locals: Vec::new(),
            slots: 0,
            slot_names: Vec::new(),
            clause_kind,
            units: UnitScope::new(scope.types.units()),
        }
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
    pub(crate) fn into_formed_units(self) -> crate::value::quantity::UnitTable {
        self.units.into_formed()
    }

    /// The package-level declarations this typing pass resolves names
    /// against. QSL-148: `check::family::Application` (the relocated
    /// function-application checker) reads this to resolve a call's callee
    /// against `model_operations` and tuple-constructor types, the same way
    /// this `Typer`'s own other methods already do.
    pub(crate) fn scope(&self) -> &'a Scope {
        self.scope
    }

    /// Every function signature this typing pass may resolve an ordinary
    /// named [`Expression::Call`] against. QSL-148: `check::family::
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
        if let Some(existing) = self.locals.iter().find(|local| local.name == name) {
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
        self.locals.truncate(keep);
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
        self.locals
            .iter()
            .rev()
            .find(|local| local.name == name)
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
    /// sub-expressions and `let` bindings on the heap (QSL-228): they run
    /// before the typer has entered the operand's own levels, so no
    /// checking limit has bounded its depth yet.
    fn contains_captured_pre_alias(&self, expression: &Expression, boundary: usize) -> bool {
        let mut bindings: Vec<AliasBinding<'_>> = Vec::new();
        let mut pending: Vec<(&Expression, Option<usize>)> = vec![(expression, None)];
        while let Some((expression, scope)) = pending.pop() {
            match expression {
                Expression::AllInstances { population, .. }
                | Expression::Lookup { population, .. } => {
                    if self.resolves_to_captured_alias(population, boundary, &mut bindings, scope) {
                        return true;
                    }
                }
                Expression::Let { name, value, body } => {
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
    /// - [`Expression::Name`] resolves against the scope first (innermost
    ///   within this walk wins, matching ordinary shadowing), falling back
    ///   to [`Self::captured_before`] for a name this walk never rebound.
    /// - [`Expression::Let`] resolves its own value first (the value may
    ///   itself be a further `let`/`if`), binds that result as `name`, and
    ///   resolves through its body under that extended scope.
    /// - [`Expression::If`] resolves `true` when *either* branch does: a
    ///   checker refusing statically cannot rule out the branch that
    ///   escapes, so both must be clear.
    /// - Every other shape is not itself alias-bearing syntax and resolves
    ///   `false`.
    fn resolves_to_captured_alias<'e>(
        &self,
        operand: &'e Expression,
        boundary: usize,
        bindings: &mut Vec<AliasBinding<'e>>,
        scope: Option<usize>,
    ) -> bool {
        let mut resolved: Vec<bool> = Vec::new();
        let mut pending = vec![AliasStep::Operand(operand, scope)];
        while let Some(step) = pending.pop() {
            match step {
                AliasStep::Operand(Expression::Name(name), scope) => {
                    let mut at = scope;
                    let mut alias = None;
                    while let Some(binding) = at.and_then(|index| bindings.get(index)) {
                        if binding.name == name {
                            alias = Some(binding.alias);
                            break;
                        }
                        at = binding.outer;
                    }
                    resolved.push(alias.unwrap_or_else(|| self.captured_before(name, boundary)));
                }
                AliasStep::Operand(Expression::Let { name, value, body }, scope) => {
                    pending.push(AliasStep::Bind(name, body, scope));
                    pending.push(AliasStep::Operand(value, scope));
                }
                AliasStep::Operand(
                    Expression::If {
                        then, otherwise, ..
                    },
                    scope,
                ) => {
                    pending.push(AliasStep::OrElse(otherwise, scope));
                    pending.push(AliasStep::Operand(then, scope));
                }
                AliasStep::Operand(_, _) => resolved.push(false),
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
        let exhausted = |kind, limit| {
            refuse(
                location,
                CheckCause::ResourceExhausted {
                    stage: CheckingStage::Typing,
                    kind,
                    limit,
                },
            )
        };
        if *self.nodes >= self.limits.nodes {
            return Err(exhausted(CheckingLimitKind::Nodes, self.limits.nodes));
        }
        if self.depth >= self.limits.depth {
            return Err(exhausted(CheckingLimitKind::Depth, self.limits.depth));
        }
        *self.nodes = self.nodes.saturating_add(1);
        self.depth = self.depth.saturating_add(1);
        Ok(())
    }

    fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
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
                    if self.scope.enum_binding_of(shape).is_none() {
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
                | ValueType::Composite(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => {}
            }
        }
        Ok(())
    }

    /// Resolve a qualified type name: an alias, a record or tuple, or an enum.
    ///
    /// `pub(crate)` (QSL-148): `check::family::Application` (the
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
            NodeKind::Literal(Value::Rational(value)),
            ValueType::Rational(domain.clone()),
            location,
        ))
    }

    fn name(&self, name: &str, location: &Location) -> Result<Node, CheckRefusal> {
        if let Some(local) = self.locals.iter().rev().find(|local| local.name == name) {
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
                // `EnumDeclaration::admit_member` (`value/enumeration.rs`).
                let rank = u32::try_from(member.position())
                    .expect("an admitted enum has far fewer than u32::MAX members");
                Ok(node(
                    NodeKind::Literal(Value::Enum(EnumMember::new(
                        mint_variant_id(binding.declaration.key(), member.case()),
                        rank,
                    ))),
                    ValueType::Enum(binding.shape()),
                    location,
                ))
            }
            [] if self.signatures.declares(name) => Err(mismatch(location)),
            [] => Err(refuse(location, CheckCause::MissingName(name.to_owned()))),
            _ => Err(refuse(
                location,
                CheckCause::AmbiguousName {
                    name: name.to_owned(),
                    loci: vec![location.clone()],
                },
            )),
        }
    }

    /// `left = right` or `left != right` over its two typed operands, each
    /// with the equality operand it compares as.
    fn equality(
        &self,
        operator: EqualityOperator,
        (left, left_operand): (Node, EqualityOperand),
        (right, right_operand): (Node, EqualityOperand),
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
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
            NodeKind::Equality(operator, Box::new(checked), Box::new(left), Box::new(right)),
            ValueType::Boolean,
            location,
        ))
    }

    /// An ordering over its two typed operands.
    fn ordering(
        &self,
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
            | (ValueType::Reference(_), ValueType::Reference(_)) => {
                return Err(ineligible(location))
            }
            _ => return Err(mismatch(location)),
        };
        Ok(node(
            NodeKind::Order(operator, kind, Box::new(left), Box::new(right)),
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
        let (left_box, right_box) = (Box::new(left), Box::new(right));
        let (kind, value_type) = match (&left_type, &right_type, operator) {
            (l, r, ArithmeticOperator::Divide) if is_integer(l) && is_integer(r) => match hint {
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
            },
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
                    NodeKind::Ieee(operator, left_box, right_box),
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
        &self,
        operand: Node,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let (kind, value_type) = match &operand.value_type {
            ValueType::Integer | ValueType::Int(_) => {
                (NodeKind::Negate(Box::new(operand)), ValueType::Integer)
            }
            ValueType::Rational(domain) => {
                let domain = match hint {
                    Some(ValueType::Rational(expected)) => expected.clone(),
                    _ => domain.negated(),
                };
                (
                    NodeKind::RationalNegate(Box::new(operand), domain.clone()),
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
                    NodeKind::DecimalNegate(Box::new(operand), target.clone()),
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
            | ValueType::Population(_) => return Err(mismatch(location)),
        };
        Ok(node(kind, value_type, location))
    }

    /// `deref(r).f` over the typed reference `r`, read at
    /// `operand_location` (the `deref(r)` operand's own location).
    fn attribute(
        &self,
        reference: Node,
        field: &str,
        operand_location: &Location,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Reference(key) = reference.value_type else {
            return Err(mismatch(operand_location));
        };
        let attribute = self
            .scope
            .types
            .object_type(key)
            .and_then(|object| {
                object
                    .attributes()
                    .iter()
                    .find(|attribute| attribute.name() == field)
            })
            .ok_or_else(|| mismatch(location))?;
        let optional = attribute.presence() == Presence::Optional;
        let value_type = if optional {
            ValueType::option(attribute.value_type().clone())
        } else {
            attribute.value_type().clone()
        };
        Ok(node(
            NodeKind::Attribute {
                reference: Box::new(reference),
                name: field.to_owned(),
                optional,
            },
            value_type,
            location,
        ))
    }

    /// `e.f` over the typed record `e`.
    fn field(&self, operand: Node, field: &str, location: &Location) -> Result<Node, CheckRefusal> {
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
        let (index, declared) = fields
            .iter()
            .enumerate()
            .find(|(_, declared)| declared.name() == field)
            .ok_or_else(|| mismatch(location))?;
        let optional = declared.presence() == Presence::Optional;
        let value_type = if optional {
            ValueType::option(declared.value_type().clone())
        } else {
            declared.value_type().clone()
        };
        Ok(node(
            NodeKind::Field {
                operand: Box::new(operand),
                index,
                optional,
            },
            value_type,
            location,
        ))
    }

    /// `receiver.member(args)` (FR-151, `quire.model.dispatch.single/v1`):
    /// whether a dispatch call checks in this clause at all. It checks only
    /// inside an invariant, precondition or postcondition (TC-196 D07).
    fn dispatch_admitted(&self, location: &Location) -> Result<(), CheckRefusal> {
        if matches!(
            self.clause_kind,
            ClauseKind::Invariant | ClauseKind::Precondition | ClauseKind::Postcondition
        ) {
            Ok(())
        } else {
            Err(ineligible(location))
        }
    }

    /// The dispatch-eligible operation `member` a call with `arity`
    /// arguments names on its typed receiver: the receiver is `self`, a
    /// `deref(...)` result or another `Reference<T>` value, and `member`
    /// resolves statically against `T`'s exposed dispatch-eligible
    /// operations. Returns the operation's index and declaration.
    fn dispatch_operation(
        &self,
        receiver: &Node,
        member: &str,
        arity: usize,
        location: &Location,
    ) -> Result<(usize, &'a DispatchOperation), CheckRefusal> {
        let scope: &'a Scope = self.scope;
        let ValueType::Reference(receiver_type) = receiver.value_type else {
            return Err(ineligible(location));
        };
        let Some((operation_index, operation)) =
            scope
                .dispatch_operations
                .iter()
                .enumerate()
                .find(|(_, operation)| {
                    operation.receiver_type == receiver_type && operation.member == member
                })
        else {
            return Err(ineligible(location));
        };
        if operation.parameters.len() != arity {
            return Err(mismatch(location));
        }
        Ok((operation_index, operation))
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
        &self,
        mut typed: Node,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if &typed.value_type == required {
            Ok(typed)
        } else if let (ValueType::Reference(actual), ValueType::Reference(expected)) =
            (&typed.value_type, required)
        {
            if self.scope.types.conforms(*actual, *expected) {
                typed.value_type = required.clone();
                Ok(typed)
            } else {
                Err(mismatch(location))
            }
        } else {
            Err(mismatch(location))
        }
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
    fn record_field<'d, 'i>(
        declared: &'d [FieldDeclaration],
        supplied: &mut [Option<RecordSlot>],
        field: &str,
        initializer: &'i FieldInitializer,
        field_location: &Location,
    ) -> Result<Option<(usize, &'d ValueType, &'i Expression)>, CheckRefusal> {
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
        &self,
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
                        operand: Box::new(operand),
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
                    NodeKind::IeeeToRational(Box::new(operand), domain.clone()),
                    target.clone(),
                    location,
                ))
            }
            (ValueType::Rational(_) | ValueType::Decimal(_), ValueType::Decimal(decimal))
                if !admits_equality_conversion(&operand.value_type, target, &self.units) =>
            {
                Ok(node(
                    NodeKind::ConvertDecimal(Box::new(operand), decimal.clone()),
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
                    NodeKind::ConvertScalar(converted, Box::new(operand)),
                    target.clone(),
                    location,
                ))
            }
        }
    }

    /// The queried type `T` of `allInstances<T>(p)` or `lookup<T>(p, r)`
    /// (FR-153), resolved and checked: an object reference type.
    fn population_target(
        &self,
        target: &qsl_forms::TypeForm,
        location: &Location,
    ) -> Result<ValueType, CheckRefusal> {
        let target = self.declared_target(target, location)?;
        if !matches!(target, ValueType::Reference(_)) {
            return Err(mismatch(location));
        }
        Ok(target)
    }

    /// `allInstances<T>(p)` (FR-153) over the typed population `p`: `p`'s
    /// own checked `Population<T>[N]` type (`ValueType::Population`) gives
    /// the result's declared bound `[0,N]` directly; no runtime value is
    /// consulted at check time.
    fn all_instances(
        target: &ValueType,
        population: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        // FR-153 requires `p` to be a population binding with a declared
        // maximum, otherwise `ill_typed`/`operator-ineligible` (TC-198 L06):
        // the operand is the wrong kind, not merely the wrong type name.
        let ValueType::Population(maximum) = population.value_type else {
            return Err(ineligible(&population.location));
        };
        let collection_type = CollectionType::new(
            CollectionKind::Set,
            target.clone(),
            bound(0, maximum, location)?,
        );
        Ok(node(
            NodeKind::AllInstances {
                population: Box::new(population),
            },
            ValueType::collection(collection_type),
            location,
        ))
    }

    /// `lookup<T>(p, r)`'s typed population operand `p`: FR-153 requires a
    /// population binding with a declared maximum, otherwise
    /// `ill_typed`/`operator-ineligible` (TC-198 L06): the operand is the
    /// wrong kind, not merely the wrong type name.
    fn lookup_population(population: &Node) -> Result<(), CheckRefusal> {
        if matches!(population.value_type, ValueType::Population(_)) {
            Ok(())
        } else {
            Err(ineligible(&population.location))
        }
    }

    /// `lookup<T>(p, r) absent m` (FR-153) over its typed operands. `r`'s own
    /// checked static type `S` (never `T`) is `reference.value_type` at
    /// evaluation time (`qsl-eval`'s `value::expression::evaluate`), so
    /// [`NodeKind::Lookup`] does not restate it. FR-153 also refuses
    /// `ill_typed`/`type-mismatch` at check time when `S` does not conform to
    /// `T` (TC-198 L03's last case, "before any charge") -- this checker
    /// cannot decide that here without the model's own generalization graph,
    /// which `TypeEnvironment` does not carry (the "TypeEnvironment island",
    /// tracked at <https://github.com/agent-ix/quire-spec-language/issues/164>),
    /// so that refusal is deferred to evaluation, inside
    /// `crate::model::population::lookup`'s own `ModelIndex::conforms` call
    /// (`crate::value::evaluate_lookup`).
    fn lookup(
        target: &ValueType,
        population: Node,
        reference: Node,
        absence: AbsenceMode,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if !matches!(reference.value_type, ValueType::Reference(_)) {
            return Err(mismatch(&reference.location));
        }
        let value_type = match absence {
            AbsenceMode::Undefined | AbsenceMode::Refused => target.clone(),
            AbsenceMode::Empty => ValueType::option(target.clone()),
        };
        Ok(node(
            NodeKind::Lookup {
                population: Box::new(population),
                reference: Box::new(reference),
                absence,
            },
            value_type,
            location,
        ))
    }

    /// Bind a one-binder form's binder to the element type of its typed
    /// collection operand `source`; returns the binder's slot and the
    /// source's collection type.
    fn bind_element(
        &mut self,
        source: &Node,
        binder: &str,
        location: &Location,
    ) -> Result<(Slot, Box<CollectionType>), CheckRefusal> {
        let ValueType::Collection(source_type) = source.value_type.clone() else {
            return Err(mismatch(location));
        };
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
        (slot, source_type): (Slot, Box<CollectionType>),
        source: Node,
        body: Node,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let typed = match query {
            BinderQuery::Map | BinderQuery::FlatMap => {
                let bound = source_type.bound();
                let minimum = if source_type.kind().is_unique() {
                    bound.minimum().min(1)
                } else {
                    bound.minimum()
                };
                let mapped = CollectionType::new(
                    source_type.kind(),
                    body.value_type.clone(),
                    super::check::bound(minimum, bound.maximum(), location)?,
                );
                let value_type = ValueType::collection(mapped);
                self.check_declared_type(&value_type, location)?;
                let map = node(
                    NodeKind::Query {
                        visit: Visit::Map,
                        slot,
                        source: Box::new(source),
                        body: Box::new(body),
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
                    super::check::bound(0, source_type.bound().maximum(), location)?,
                );
                node(
                    NodeKind::Query {
                        visit: Visit::Filter,
                        slot,
                        source: Box::new(source),
                        body: Box::new(body),
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
                        source: Box::new(source),
                        body: Box::new(body),
                    },
                    ValueType::Boolean,
                    location,
                )
            }
        };
        self.unbind(1);
        Ok(typed)
    }

    fn flatten(&self, source: Node, location: &Location) -> Result<Node, CheckRefusal> {
        let ValueType::Collection(outer) = &source.value_type else {
            return Err(mismatch(location));
        };
        let ValueType::Collection(inner) = outer.element() else {
            return Err(mismatch(location));
        };
        if outer.kind().is_ordered() && !inner.kind().is_ordered() {
            return Err(mismatch(location));
        }
        let unrepresentable = || refuse(location, CheckCause::UnrepresentableBound);
        let minimum = outer
            .bound()
            .minimum()
            .checked_mul(inner.bound().minimum())
            .ok_or_else(unrepresentable)?;
        let maximum = outer
            .bound()
            .maximum()
            .checked_mul(inner.bound().maximum())
            .ok_or_else(unrepresentable)?;
        let minimum = if outer.kind().is_unique() {
            minimum.min(1)
        } else {
            minimum
        };
        let flattened = CollectionType::new(
            outer.kind(),
            inner.element().clone(),
            bound(minimum, maximum, location)?,
        );
        let value_type = ValueType::collection(flattened);
        self.check_declared_type(&value_type, location)?;
        Ok(node(
            NodeKind::Flatten(Box::new(source)),
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
    ) -> Result<(Slot, Slot, Box<CollectionType>), CheckRefusal> {
        let ValueType::Collection(source_type) = source.value_type.clone() else {
            return Err(mismatch(location));
        };
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
        &self,
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
            NodeKind::Contains(Box::new(collection), Box::new(item)),
            ValueType::Boolean,
            location,
        ))
    }
}

/// Whether a well-typed step is in the FR-145 set-and-bag catalog: `acc OP t`
/// or `t OP acc`, `acc` not in `t`, `t` of type `A`, and `+` or `*` on
/// `Integer` or `and` or `or` on `Boolean`.
fn catalogued_step(step: &Node, accumulator: Slot, value_type: &ValueType) -> bool {
    let (left, right) = match (&step.kind, value_type) {
        (
            NodeKind::Arithmetic(Arithmetic::Add | Arithmetic::Multiply, left, right),
            ValueType::Integer,
        )
        | (
            NodeKind::Connective(Connective::And | Connective::Or, left, right),
            ValueType::Boolean,
        ) => (left, right),
        _ => return false,
    };
    let is_accumulator =
        |operand: &Node| matches!(operand.kind, NodeKind::Local(slot) if slot == accumulator);
    let other = if is_accumulator(left) {
        right
    } else if is_accumulator(right) {
        left
    } else {
        return false;
    };
    &other.value_type == value_type && !other.descendants().into_iter().any(is_accumulator)
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
