// SPDX-License-Identifier: AGPL-3.0-or-later
//! Name resolution and typing of Complete-V1 value expressions and function
//! declarations into the typed tree, before any definedness or termination
//! judgment.
//!
//! Every refusal here is made before any charge. Typing recursion is bounded
//! by the declared [`CheckingLimits`] depth, whose maximum keeps checking off
//! the host stack limit; reaching a declared limit is `resource_exhausted`,
//! never an admission verdict.
//!
//! FR-065 (owner ruling, carried from the QSL-25 spec review): this module
//! retains `infer_form`'s dispatch over [`Expression`] for every `Value`
//! form. **QSL-148 moves function-application checking out of this module.**
//! `Self::call` -- the method `infer_form`'s `Expression::Call` arm used to
//! dispatch to, doing the real name resolution, arity check and
//! per-argument typing -- is deleted; its logic now lives in
//! [`super::family::check_application`], reached through this module's own
//! `Typer::scope`/`Typer::signatures`/`Typer::type_named`/`Typer::check_as`
//! accessors (widened from private to `pub(crate)` for exactly this one
//! caller). `infer_form`'s `Expression::Call` arm makes one call into that
//! function and holds no semantic logic of its own (FR-065-AC-4). This
//! module still owns function-declaration *typing*'s underlying engine
//! (`bind_parameters`, `check_declared_type`, `check_as`), because a
//! function body is an arbitrary `Expression` and checking one still needs
//! the same general recursive typer every other `Value` form uses --
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

use super::ir::{
    Arithmetic, Connective, DispatchTable, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit,
};
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, Location, Obligation,
    WrongSnapshotCause,
};
use crate::value::collection::CollectionType;
use crate::value::composite::{Value, ValueType};
use crate::value::decimal::DecimalType;
use crate::value::declaration::{
    admits_equality_conversion, CompositeShape, EqualityOperand, EqualityOperator, TypeEnvironment,
};
use crate::value::definition::AdmittedIeeeProfile;
use crate::value::enumeration::{EnumDeclaration, EnumValue};
use crate::value::quantity::{check_comparable, result_unit, UnitOperation};
use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
    FunctionDeclaration,
};
use qsl_foundation::absence::AbsenceMode;
use quire_exact::EffectiveId;
use quire_exact::IllTypedCause;
use quire_exact::Presence;
use quire_exact::Rational;
use quire_exact::{ArithmeticOperator, OrderingOperator};
use quire_exact::{CardinalityBound, CollectionKind, Integer};

/// The largest expression nesting depth a checker may declare. It keeps every
/// recursive checking pass well inside the host stack.
///
/// #141 P2: [`crate::model::normalize::MAX_GENERALIZATION_DEPTH`] shares this
/// value for the same reason (a safe host-stack bound for a *different*
/// recursion, the FR-150 generalization ancestor-path walk), not because
/// either limit constrains the other.
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
    /// Its admitted members.
    pub members: Vec<EnumValue>,
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
#[derive(Clone, Debug, Default)]
pub struct PackageDeclarations {
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
    /// ADR-013 O-04 (PR #300 review finding 1): the checked-node-id ->
    /// domain-declaration correspondence entries this package's declarations
    /// are already known to require, in the same "built by the caller (the
    /// `model` bridge); the checker only records/resolves against it"
    /// division `dispatch_operations`/`dispatch_tables` above already use.
    /// `check` records every entry here onto `CheckedGraph`'s
    /// [`super::identity::ModelCorrespondence`] verbatim -- it never derives
    /// or searches for one itself (R-05) -- so [`super::CheckedGraph::resolve_declaration`]
    /// reads back exactly what a caller supplied. Always empty from every
    /// production call site today: no #213 slice before S-3b gives `model`'s
    /// bridge a domain-package declaration to resolve a
    /// `crate::model::key::DeclarationKey` from in the first place
    /// (FR-088-CON-2 leaves FR-340's frame semantics, the first real
    /// consumer, to #210). The field exists now so that bridge has a real
    /// place to hand its entries to, and so this crate's own tests can
    /// exercise the recording step through a genuine `check()` run rather
    /// than a hand-built `ModelCorrespondence` (`check/mod.rs`'s own test
    /// module).
    pub model_correspondence: Vec<(quire_exact::NodeKey, crate::model::key::DeclarationKey)>,
    /// Resolved signatures standing in for some `functions` entries' own
    /// type forms; see [`ResolvedSignatures`].
    pub resolved_signatures: ResolvedSignatures,
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
pub(crate) struct Scope {
    pub(crate) types: TypeEnvironment,
    pub(crate) enums: Vec<EnumBinding>,
    pub(crate) aliases: Vec<(String, ValueType)>,
    pub(crate) model_operations: Vec<String>,
    pub(crate) ieee_profile: Option<AdmittedIeeeProfile>,
    pub(crate) dispatch_operations: Vec<DispatchOperation>,
}

/// A function's declared signature.
#[derive(Clone, Debug)]
pub(crate) struct Signature {
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

/// Two typed operands, each with its per-operator extra.
type OperandPair<T> = ((Node, T), (Node, T));

/// One typing pass over a function or standalone expression.
pub(crate) struct Typer<'a> {
    scope: &'a Scope,
    signatures: &'a [Signature],
    limits: CheckingLimits,
    nodes: &'a mut u64,
    depth: u64,
    locals: Vec<Local>,
    slots: usize,
    /// The clause this declaration is (FR-151's dispatch-call restriction;
    /// also the only signal `pre(...)` needs: `pre(...)` is legal exactly in
    /// [`ClauseKind::Postcondition`] -- FR-153's own anchor table;
    /// shared-grammar.md's "self, result and pre(...) are caller-side anchor
    /// operations" -- and refused `wrong_snapshot`/`wrong-anchor` in every
    /// other clause kind, including a standalone `check_expression` call).
    clause_kind: ClauseKind,
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
fn contains_pre_eligible_read(expression: &Expression) -> bool {
    matches!(
        expression,
        Expression::AllInstances { .. } | Expression::Lookup { .. } | Expression::Pre(_)
    ) || expression
        .children()
        .into_iter()
        .any(contains_pre_eligible_read)
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
        signatures: &'a [Signature],
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
            clause_kind,
        }
    }

    /// The number of slots allocated so far.
    pub(crate) fn slots(&self) -> usize {
        self.slots
    }

    /// The package-level declarations this typing pass resolves names
    /// against. QSL-148: `check::family::check_application` (the relocated
    /// function-application checker) reads this to resolve a call's callee
    /// against `model_operations` and tuple-constructor types, the same way
    /// this `Typer`'s own other methods already do.
    pub(crate) fn scope(&self) -> &'a Scope {
        self.scope
    }

    /// Every function signature this typing pass may resolve an ordinary
    /// named [`Expression::Call`] against. QSL-148: `check::family::
    /// check_application` reads this for name resolution and arity
    /// checking, exactly as this `Typer`'s own (now deleted) `call` method
    /// did.
    pub(crate) fn signatures(&self) -> &'a [Signature] {
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
    /// `bindings` tracks every name a `let` *within this same walk* (i.e.
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
    fn contains_captured_pre_alias(
        &self,
        expression: &Expression,
        boundary: usize,
        bindings: &[(String, bool)],
    ) -> bool {
        let direct = match expression {
            Expression::AllInstances { population, .. } => {
                self.resolves_to_captured_alias(population, boundary, bindings)
            }
            Expression::Lookup { population, .. } => {
                self.resolves_to_captured_alias(population, boundary, bindings)
            }
            _ => false,
        };
        if direct {
            return true;
        }
        if let Expression::Let { name, value, body } = expression {
            if self.contains_captured_pre_alias(value, boundary, bindings) {
                return true;
            }
            let mut bindings = bindings.to_vec();
            bindings.push((
                name.clone(),
                self.resolves_to_captured_alias(value, boundary, &bindings),
            ));
            return self.contains_captured_pre_alias(body, boundary, &bindings);
        }
        expression
            .children()
            .into_iter()
            .any(|child| self.contains_captured_pre_alias(child, boundary, bindings))
    }

    /// Whether `operand` -- a population-typed sub-expression this walk is
    /// considering as `allInstances`/`lookup`'s direct operand, or as a
    /// `let`'s own value -- resolves to a captured alias of a state root
    /// bound outside this `pre(...)`'s operand, however many `let`s or `if`
    /// branches it is spelled through.
    ///
    /// - [`Expression::Name`] resolves against `bindings` first (innermost
    ///   within this walk wins, matching ordinary shadowing), falling back
    ///   to [`Self::captured_before`] for a name this walk never rebound.
    /// - [`Expression::Let`] resolves its own value first (recursively --
    ///   the value may itself be a further `let`/`if`), pushes that result
    ///   as `name`'s own binding, and resolves through its body under that
    ///   extended `bindings`.
    /// - [`Expression::If`] resolves `true` when *either* branch does: a
    ///   checker refusing statically cannot rule out the branch that
    ///   escapes, so both must be clear.
    /// - Every other shape is not itself alias-bearing syntax and resolves
    ///   `false`.
    fn resolves_to_captured_alias(
        &self,
        operand: &Expression,
        boundary: usize,
        bindings: &[(String, bool)],
    ) -> bool {
        match operand {
            Expression::Name(name) => bindings
                .iter()
                .rev()
                .find(|(bound, _)| bound == name)
                .map_or_else(|| self.captured_before(name, boundary), |(_, alias)| *alias),
            Expression::Let { name, value, body } => {
                let alias = self.resolves_to_captured_alias(value, boundary, bindings);
                let mut bindings = bindings.to_vec();
                bindings.push((name.clone(), alias));
                self.resolves_to_captured_alias(body, boundary, &bindings)
            }
            Expression::If {
                then, otherwise, ..
            } => {
                self.resolves_to_captured_alias(then, boundary, bindings)
                    || self.resolves_to_captured_alias(otherwise, boundary, bindings)
            }
            _ => false,
        }
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
                ValueType::Enum(key) => {
                    if !self
                        .scope
                        .enums
                        .iter()
                        .any(|binding| binding.declaration.key() == *key)
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
                | ValueType::Composite(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => {}
            }
        }
        Ok(())
    }

    /// Resolve a qualified type name: an alias, a record or tuple, or an enum.
    ///
    /// `pub(crate)` (QSL-148): `check::family::check_application` (the
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

    /// Type `expression` where `required` is expected. A conditional or `let`
    /// passes the requirement into its branches or body.
    pub(crate) fn check_as(
        &mut self,
        expression: &Expression,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match expression {
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                self.enter(location)?;
                let condition =
                    self.check_as(condition, &ValueType::Boolean, &location.child(0))?;
                let then = self.check_as(then, required, &location.child(1))?;
                let otherwise = self.check_as(otherwise, required, &location.child(2))?;
                self.leave();
                Ok(node(
                    NodeKind::If {
                        condition: Box::new(condition),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    required.clone(),
                    location,
                ))
            }
            Expression::Let { name, value, body } => {
                self.enter(location)?;
                let value = self.infer(value, None, &location.child(0))?;
                let slot = self.bind(name, value.value_type.clone(), location, LocalKind::Bound)?;
                let body = self.check_as(body, required, &location.child(1))?;
                self.unbind(1);
                self.leave();
                Ok(node(
                    NodeKind::Let {
                        slot,
                        value: Box::new(value),
                        body: Box::new(body),
                    },
                    required.clone(),
                    location,
                ))
            }
            _ => {
                let typed = self.infer(expression, Some(required), location)?;
                coerce(typed, required)
            }
        }
    }

    /// Type `expression`, with `hint` as the expected type of a contextual
    /// literal.
    pub(crate) fn infer(
        &mut self,
        expression: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        self.enter(location)?;
        let typed = self.infer_form(expression, hint, location)?;
        self.leave();
        Ok(typed)
    }

    /// FR-065's dispatch seam over [`Expression`] (ADR-012 §4.3).
    ///
    /// **`Expression::Call` is thin (QSL-148).** The arm below makes exactly
    /// one call, into [`super::family::check_application`] -- `Value`'s
    /// family check code for function application -- and holds no semantic
    /// logic of its own (FR-065-AC-4): no name resolution, arity check or
    /// per-argument typing loop runs directly in this arm. `Self::call`, the
    /// method that used to hold that logic, is deleted; `check_application`
    /// is its relocated replacement, reached through `Typer`'s own
    /// `scope`/`signatures`/`type_named`/`check_as` accessors (see this
    /// module's own doc).
    ///
    /// **`Expression::Call` itself is still present in [`Expression`]
    /// (FR-065-AC-5 remains unmet for this reason).** FR-065-AC-5 requires
    /// the composed checker's input form-kind enum to carry neither a
    /// function-declaration nor a function-application variant once this
    /// requirement lands; `Expression::Call` is that variant, and it has not
    /// been removed. An earlier version of this doc argued a call should
    /// stay because it is "an ordinary, nestable operand of every other
    /// `Value` form" -- that argument reasons about a migration that has
    /// not happened, not about one AC-5 already excuses, and FR-065-AC-5's
    /// own text says plainly that a variant left in place "with or without
    /// an arm for it, does not satisfy this criterion." See FR-065's own
    /// Status section for why AC-5 stays recorded unbacked rather than
    /// retagged onto what this arm's own shape does satisfy.
    ///
    /// `#[deny(...)]` (FR-063's residual paragraph, carried into the
    /// QSL-25 implementation by owner ruling): a future change that wants
    /// to delete an arm from this `match` cannot restore exhaustiveness
    /// with a `_ => ...` catch-all -- that is the exact "escape hatch"
    /// closed here, not merely by convention; what this attribute forbids
    /// is silently absorbing a *future* removed arm behind a catch-all
    /// instead of deleting the corresponding variant.
    #[deny(clippy::wildcard_enum_match_arm)]
    #[deny(clippy::match_wildcard_for_single_variants)]
    fn infer_form(
        &mut self,
        expression: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match expression {
            Expression::Boolean(value) => Ok(node(
                NodeKind::Literal(Value::Boolean(*value)),
                ValueType::Boolean,
                location,
            )),
            Expression::Integer(value) => Ok(node(
                NodeKind::Literal(Value::Integer(value.clone())),
                ValueType::Integer,
                location,
            )),
            Expression::Rational(numerator, denominator) => {
                self.rational_literal(numerator, denominator, hint, location)
            }
            Expression::Name(name) => self.name(name, location),
            Expression::Let { name, value, body } => {
                let value = self.infer(value, None, &location.child(0))?;
                let slot = self.bind(name, value.value_type.clone(), location, LocalKind::Bound)?;
                let body = self.infer(body, hint, &location.child(1))?;
                self.unbind(1);
                let value_type = body.value_type.clone();
                Ok(node(
                    NodeKind::Let {
                        slot,
                        value: Box::new(value),
                        body: Box::new(body),
                    },
                    value_type,
                    location,
                ))
            }
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                let condition =
                    self.check_as(condition, &ValueType::Boolean, &location.child(0))?;
                let then = self.infer(then, hint, &location.child(1))?;
                let otherwise = self.infer(otherwise, hint, &location.child(2))?;
                let value_type = if then.value_type == otherwise.value_type {
                    then.value_type.clone()
                } else if is_integer(&then.value_type) && is_integer(&otherwise.value_type) {
                    ValueType::Integer
                } else {
                    return Err(mismatch(&otherwise.location));
                };
                Ok(node(
                    NodeKind::If {
                        condition: Box::new(condition),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    value_type,
                    location,
                ))
            }
            Expression::Binary {
                operator,
                left,
                right,
            } => self.binary(*operator, left, right, hint, location),
            Expression::Negate(operand) => self.negate(operand, hint, location),
            Expression::Not(operand) => {
                let operand = self.check_as(operand, &ValueType::Boolean, &location.child(0))?;
                Ok(node(
                    NodeKind::Not(Box::new(operand)),
                    ValueType::Boolean,
                    location,
                ))
            }
            Expression::Field { operand, field } => self.field(operand, field, location),
            Expression::Present(operand) => {
                let operand = self.infer(operand, None, &location.child(0))?;
                if !matches!(operand.value_type, ValueType::Option(_)) {
                    return Err(mismatch(location));
                }
                Ok(node(
                    NodeKind::Present(Box::new(operand)),
                    ValueType::Boolean,
                    location,
                ))
            }
            Expression::Value(operand) => {
                let operand = self.infer(operand, None, &location.child(0))?;
                let ValueType::Option(payload) = operand.value_type.clone() else {
                    return Err(mismatch(location));
                };
                Ok(node(NodeKind::Value(Box::new(operand)), *payload, location))
            }
            Expression::Deref(operand) => {
                // Only `deref(r).f` is a value.
                self.infer(operand, None, &location.child(0))?;
                Err(mismatch(location))
            }
            Expression::Pre(operand) => {
                // FR-153/FR-042 (FR-208 applies FR-042 to invariants and
                // preconditions, naming this same cause explicitly): `pre(...)`
                // is legal only in an operation's postcondition -- every other
                // clause kind (a function body, its measure, an invariant, a
                // precondition, or a bare `check_expression` call) refuses it
                // here, before looking at `operand` at all, as
                // `wrong_snapshot`/`forbidden-pre-read`. `wrong-anchor` is a
                // different, catalogued cause for a different case this
                // checker cannot see: a postcondition it does admit, evaluated
                // at runtime over a population with no attached pre anchor
                // (`evaluate.rs`'s `select_anchor`) -- never a `pre(...)`
                // written in the wrong clause.
                if self.clause_kind != ClauseKind::Postcondition {
                    return Err(refuse(
                        location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-042's Behavior clause: "Pre is refused ... on bare
                // parameters/constants/captures". `operand` must contain, in
                // its own syntax, at least one form `pre(...)`'s anchor can
                // actually act on -- see `contains_pre_eligible_read`'s own
                // doc for exactly which forms count and why a bare `Name`
                // (a parameter, or a `let` bound outside this very
                // `pre(...)`) never does.
                if !contains_pre_eligible_read(operand) {
                    return Err(refuse(
                        location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-042-AC-3's `let s = self in pre(s.version)` analogue:
                // an eligible read this operand does contain must not take
                // its population operand from a `let` local captured before
                // this `pre(...)` was reached -- see
                // `contains_captured_pre_alias`'s own doc.
                if self.contains_captured_pre_alias(operand, self.slots, &[]) {
                    return Err(refuse(
                        location,
                        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead),
                    ));
                }
                // FR-153: `pre(e)` is identity-typed; only the anchor that
                // `allInstances`/`lookup` read underneath it changes.
                let operand = self.infer(operand, hint, &location.child(0))?;
                let value_type = operand.value_type.clone();
                Ok(node(NodeKind::Pre(Box::new(operand)), value_type, location))
            }
            Expression::Call { name, arguments } => {
                super::family::check_application(self, name, arguments, location)
            }
            Expression::Record { name, fields } => self.record(name, fields, location),
            Expression::Collection { kind, elements } => {
                self.collection_literal(*kind, elements, hint, location)
            }
            Expression::Convert { target, operand } => self.convert(target, operand, location),
            Expression::Query {
                query,
                binder,
                source,
                body,
            } => self.query(*query, binder, source, body, location),
            Expression::Flatten(source) => {
                let source = self.infer(source, None, &location.child(0))?;
                self.flatten(source, location)
            }
            Expression::Accumulate {
                form,
                accumulator_type,
                accumulator,
                binder,
                source,
                step,
                identity,
            } => self.accumulate(
                *form,
                accumulator_type,
                accumulator,
                binder,
                source,
                step,
                identity.as_deref(),
                location,
            ),
            Expression::Count {
                result_type,
                binder,
                source,
                predicate,
            } => {
                let value_type = self.type_named(result_type, location)?;
                if !is_integer(&value_type) {
                    return Err(mismatch(location));
                }
                let source = self.infer(source, None, &location.child(0))?;
                let element = self.element_type(&source)?;
                let slot = self.bind(binder, element, location, LocalKind::Bound)?;
                let predicate =
                    self.check_as(predicate, &ValueType::Boolean, &location.child(1))?;
                self.unbind(1);
                Ok(node(
                    NodeKind::Query {
                        visit: Visit::Count,
                        slot,
                        source: Box::new(source),
                        body: Box::new(predicate),
                    },
                    value_type,
                    location,
                ))
            }
            Expression::Sum {
                result_type,
                binder,
                source,
                summand,
            } => {
                let value_type = self.type_named(result_type, location)?;
                if !is_integer(&value_type) {
                    return Err(mismatch(location));
                }
                let source = self.infer(source, None, &location.child(0))?;
                let element = self.element_type(&source)?;
                let slot = self.bind(binder, element, location, LocalKind::Bound)?;
                let summand = self.infer(summand, None, &location.child(1))?;
                self.unbind(1);
                if !is_integer(&summand.value_type) {
                    return Err(mismatch(&summand.location));
                }
                Ok(node(
                    NodeKind::Query {
                        visit: Visit::Sum,
                        slot,
                        source: Box::new(source),
                        body: Box::new(summand),
                    },
                    value_type,
                    location,
                ))
            }
            Expression::Size(operand) => {
                let operand = self.infer(operand, None, &location.child(0))?;
                self.element_type(&operand)?;
                Ok(node(
                    NodeKind::Size(Box::new(operand)),
                    ValueType::Integer,
                    location,
                ))
            }
            Expression::Contains { collection, item } => {
                let collection = self.infer(collection, None, &location.child(0))?;
                let element = self.element_type(&collection)?;
                let item = self.check_as(item, &element, &location.child(1))?;
                self.scope
                    .types
                    .check_equality(
                        EqualityOperator::Equal,
                        EqualityOperand::typed(element.clone()),
                        EqualityOperand::typed(element),
                    )
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
                Ok(node(
                    NodeKind::Contains(Box::new(collection), Box::new(item)),
                    ValueType::Boolean,
                    location,
                ))
            }
            Expression::AllInstances { target, population } => {
                self.all_instances(target, population, location)
            }
            Expression::Lookup {
                target,
                population,
                reference,
                absence,
            } => self.lookup(target, population, reference, *absence, location),
            Expression::Dispatch {
                receiver,
                member,
                arguments,
            } => self.dispatch_call(receiver, member, arguments, location),
        }
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
        let members: Vec<&EnumValue> =
            self.scope
                .enums
                .iter()
                .flat_map(|binding| {
                    binding.members.iter().filter(move |member| {
                        format!("{}::{}", binding.name, member.case()) == name
                    })
                })
                .collect();
        match members.as_slice() {
            [member] => Ok(node(
                NodeKind::Literal(Value::Enum((*member).clone())),
                ValueType::Enum(member.declaration()),
                location,
            )),
            [] if self
                .signatures
                .iter()
                .any(|signature| signature.name == name) =>
            {
                Err(mismatch(location))
            }
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

    fn binary(
        &mut self,
        operator: BinaryOperator,
        left: &Expression,
        right: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let (left_location, right_location) = (location.child(0), location.child(1));
        match operator {
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide => {
                let operator = match operator {
                    BinaryOperator::Add => ArithmeticOperator::Add,
                    BinaryOperator::Subtract => ArithmeticOperator::Subtract,
                    BinaryOperator::Multiply => ArithmeticOperator::Multiply,
                    BinaryOperator::Divide => ArithmeticOperator::Divide,
                    _ => return Err(ineligible(location)),
                };
                self.arithmetic(operator, left, right, hint, location)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                let operator = if operator == BinaryOperator::Equal {
                    EqualityOperator::Equal
                } else {
                    EqualityOperator::NotEqual
                };
                self.equality(operator, left, right, location)
            }
            BinaryOperator::Less
            | BinaryOperator::LessOrEqual
            | BinaryOperator::Greater
            | BinaryOperator::GreaterOrEqual => {
                let ordering = match operator {
                    BinaryOperator::Less => OrderingOperator::Less,
                    BinaryOperator::LessOrEqual => OrderingOperator::LessOrEqual,
                    BinaryOperator::Greater => OrderingOperator::Greater,
                    BinaryOperator::GreaterOrEqual => OrderingOperator::GreaterOrEqual,
                    _ => return Err(ineligible(location)),
                };
                self.ordering(ordering, left, right, location)
            }
            BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies => {
                let connective = match operator {
                    BinaryOperator::And => Connective::And,
                    BinaryOperator::Or => Connective::Or,
                    BinaryOperator::Implies => Connective::Implies,
                    _ => return Err(ineligible(location)),
                };
                let left = self.check_as(left, &ValueType::Boolean, &left_location)?;
                let right = self.check_as(right, &ValueType::Boolean, &right_location)?;
                Ok(node(
                    NodeKind::Connective(connective, Box::new(left), Box::new(right)),
                    ValueType::Boolean,
                    location,
                ))
            }
        }
    }

    /// Type both operands, the non-contextual one first so a literal can take
    /// its peer's type.
    fn peer_operands<T>(
        &mut self,
        left: &Expression,
        right: &Expression,
        location: &Location,
        mut operand: impl FnMut(
            &mut Self,
            &Expression,
            Option<&ValueType>,
            &Location,
        ) -> Result<(Node, T, ValueType), CheckRefusal>,
    ) -> Result<OperandPair<T>, CheckRefusal> {
        let (left_location, right_location) = (location.child(0), location.child(1));
        if contextual(left) && !contextual(right) {
            let (right_node, right_extra, right_type) =
                operand(self, right, None, &right_location)?;
            let (left_node, left_extra, _) =
                operand(self, left, Some(&right_type), &left_location)?;
            Ok(((left_node, left_extra), (right_node, right_extra)))
        } else {
            let (left_node, left_extra, left_type) = operand(self, left, None, &left_location)?;
            let (right_node, right_extra, _) =
                operand(self, right, Some(&left_type), &right_location)?;
            Ok(((left_node, left_extra), (right_node, right_extra)))
        }
    }

    fn equality_operand(
        &mut self,
        expression: &Expression,
        peer: Option<&ValueType>,
        location: &Location,
    ) -> Result<(Node, EqualityOperand, ValueType), CheckRefusal> {
        if let Expression::Convert { target, operand } = expression {
            let target = self.resolve_type(target, location)?;
            if !matches!(target, ValueType::Collection(_)) {
                self.enter(location)?;
                self.check_declared_type(&target, location)?;
                let inner = self.infer(operand, None, &location.child(0))?;
                self.leave();
                if !matches!(inner.value_type, ValueType::Float(_)) {
                    let source = inner.value_type.clone();
                    return Ok((
                        inner,
                        EqualityOperand::converted(source, target.clone()),
                        target,
                    ));
                }
                return Err(mismatch(location));
            }
        }
        let typed = match peer {
            Some(peer) if contextual(expression) => self.check_as(expression, peer, location)?,
            _ => self.infer(expression, peer, location)?,
        };
        let value_type = typed.value_type.clone();
        Ok((
            typed,
            EqualityOperand::typed(value_type.clone()),
            value_type,
        ))
    }

    fn equality(
        &mut self,
        operator: EqualityOperator,
        left: &Expression,
        right: &Expression,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ((left, left_operand), (right, right_operand)) =
            self.peer_operands(left, right, location, Self::equality_operand)?;
        let checked = self
            .scope
            .types
            .check_equality(operator, left_operand, right_operand)
            .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
        Ok(node(
            NodeKind::Equality(operator, Box::new(checked), Box::new(left), Box::new(right)),
            ValueType::Boolean,
            location,
        ))
    }

    fn ordering(
        &mut self,
        operator: OrderingOperator,
        left: &Expression,
        right: &Expression,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ((left, ()), (right, ())) =
            self.peer_operands(left, right, location, |typer, expression, peer, at| {
                let typed = typer.infer(expression, peer, at)?;
                let value_type = typed.value_type.clone();
                Ok((typed, (), value_type))
            })?;
        let kind = match (&left.value_type, &right.value_type) {
            (l, r) if is_integer(l) && is_integer(r) => OrderedKind::Integers,
            (ValueType::Rational(_), ValueType::Rational(_)) => OrderedKind::Rationals,
            (ValueType::Decimal(_), ValueType::Decimal(_)) => OrderedKind::Decimals,
            (ValueType::Enum(l), ValueType::Enum(r)) if l == r => {
                let ordered = self
                    .scope
                    .enums
                    .iter()
                    .find(|binding| binding.declaration.key() == *l)
                    .map(|binding| binding.declaration.preimage().is_ordered())
                    .ok_or_else(|| mismatch(location))?;
                if !ordered {
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

    /// `left op right` for `+`, `-`, `*` and `/`. Both operands are of one
    /// numeric family; mixing families needs an explicit conversion.
    fn arithmetic(
        &mut self,
        operator: ArithmeticOperator,
        left: &Expression,
        right: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ((left, ()), (right, ())) =
            self.peer_operands(left, right, location, |typer, expression, peer, at| {
                let typed = typer.infer(expression, peer, at)?;
                let value_type = typed.value_type.clone();
                Ok((typed, (), value_type))
            })?;
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
                let unit = result_unit(operation, l, r)
                    .map_err(|refusal| CheckRefusal::from_ill_typed(location, refusal))?;
                (
                    NodeKind::Quantity(operator, left_box, right_box),
                    ValueType::Quantity(unit),
                )
            }
            _ => return Err(mismatch(location)),
        };
        Ok(node(kind, value_type, location))
    }

    /// Unary `-`: integers, rationals and decimals. FR-148 and FR-142 define
    /// no negation of an IEEE value or a quantity.
    fn negate(
        &mut self,
        operand: &Expression,
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let operand = self.infer(operand, None, &location.child(0))?;
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

    fn field(
        &mut self,
        operand: &Expression,
        field: &str,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let operand_location = location.child(0);
        if let Expression::Deref(reference) = operand {
            self.enter(&operand_location)?;
            let reference = self.infer(reference, None, &operand_location.child(0))?;
            self.leave();
            let ValueType::Reference(key) = reference.value_type else {
                return Err(mismatch(&operand_location));
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
            return Ok(node(
                NodeKind::Attribute {
                    reference: Box::new(reference),
                    name: field.to_owned(),
                    optional,
                },
                value_type,
                location,
            ));
        }
        let operand = self.infer(operand, None, &operand_location)?;
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
    /// only checks inside an invariant, precondition or postcondition
    /// (TC-196 D07); the receiver is `self`, a `deref(...)` result or
    /// another `Reference<T>` value, and `member` resolves statically
    /// against `T`'s exposed dispatch-eligible operations.
    fn dispatch_call(
        &mut self,
        receiver: &Expression,
        member: &str,
        arguments: &[Expression],
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        if !matches!(
            self.clause_kind,
            ClauseKind::Invariant | ClauseKind::Precondition | ClauseKind::Postcondition
        ) {
            return Err(ineligible(location));
        }
        let receiver_location = location.child(0);
        let receiver_node = if let Expression::Deref(inner) = receiver {
            self.enter(&receiver_location)?;
            let inner = self.infer(inner, None, &receiver_location.child(0))?;
            self.leave();
            inner
        } else {
            self.infer(receiver, None, &receiver_location)?
        };
        let ValueType::Reference(receiver_type) = receiver_node.value_type else {
            return Err(ineligible(location));
        };
        let Some((operation_index, operation)) = self
            .scope
            .dispatch_operations
            .iter()
            .enumerate()
            .find(|(_, operation)| {
                operation.receiver_type == receiver_type && operation.member == member
            })
        else {
            return Err(ineligible(location));
        };
        if operation.parameters.len() != arguments.len() {
            return Err(mismatch(location));
        }
        let mut typed_arguments = Vec::with_capacity(arguments.len());
        for (index, (argument, parameter)) in
            arguments.iter().zip(&operation.parameters).enumerate()
        {
            typed_arguments.push(self.check_dispatch_argument(
                argument,
                parameter,
                &location.child(index + 1),
            )?);
        }
        Ok(node(
            NodeKind::Dispatch {
                receiver: Box::new(receiver_node),
                table: operation.table,
                operation: operation_index,
                arguments: typed_arguments,
            },
            operation.result.clone(),
            location,
        ))
    }

    /// A dispatch call argument against its declared parameter type
    /// (`quire.model.dispatch.single/v1`: "its arguments are type-checked
    /// statically against `o`'s signature with reference upcasts only").
    /// Unlike [`Self::check_as`], this never calls [`coerce`]: an ordinary
    /// call admits an `Integer`/`Int[..]` argument into a wider or
    /// differently-bounded `Int[..]` parameter, which a dispatch call must
    /// not. A `Reference<T>` argument is admitted where `T` exactly matches
    /// the declared parameter (reflexive), or where `T` is a proper subtype
    /// of it in `self.scope.types`'s own admitted generalization graph (H1,
    /// #204 round 1) -- the static upcast case FR-151 names, never a wider
    /// admission than the spec allows. The upcast changes only this node's
    /// own static/declared type at the binding site; the runtime
    /// `ObjectReference` triple underneath is untouched.
    fn check_dispatch_argument(
        &mut self,
        expression: &Expression,
        required: &ValueType,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        match expression {
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                self.enter(location)?;
                let condition =
                    self.check_as(condition, &ValueType::Boolean, &location.child(0))?;
                let then = self.check_dispatch_argument(then, required, &location.child(1))?;
                let otherwise =
                    self.check_dispatch_argument(otherwise, required, &location.child(2))?;
                self.leave();
                Ok(node(
                    NodeKind::If {
                        condition: Box::new(condition),
                        then: Box::new(then),
                        otherwise: Box::new(otherwise),
                    },
                    required.clone(),
                    location,
                ))
            }
            Expression::Let { name, value, body } => {
                self.enter(location)?;
                let value = self.infer(value, None, &location.child(0))?;
                let slot = self.bind(name, value.value_type.clone(), location, LocalKind::Bound)?;
                let body = self.check_dispatch_argument(body, required, &location.child(1))?;
                self.unbind(1);
                self.leave();
                Ok(node(
                    NodeKind::Let {
                        slot,
                        value: Box::new(value),
                        body: Box::new(body),
                    },
                    required.clone(),
                    location,
                ))
            }
            _ => {
                let mut typed = self.infer(expression, Some(required), location)?;
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
        }
    }

    fn record(
        &mut self,
        name: &str,
        fields: &[(String, FieldInitializer)],
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let ValueType::Composite(key) = self.type_named(name, location)? else {
            return Err(mismatch(location));
        };
        let Some(CompositeShape::Record(declared)) = self
            .scope
            .types
            .composite(key)
            .map(|declaration| declaration.shape())
        else {
            return Err(mismatch(location));
        };
        let mut supplied: Vec<Option<RecordSlot>> = declared.iter().map(|_| None).collect();
        let mut child = 0;
        for (field, initializer) in fields {
            let field_location = match initializer {
                FieldInitializer::Value(_) => {
                    let at = location.child(child);
                    child += 1;
                    at
                }
                FieldInitializer::Null => location.clone(),
            };
            let (index, declaration) = declared
                .iter()
                .enumerate()
                .find(|(_, declaration)| declaration.name() == field)
                .ok_or_else(|| mismatch(&field_location))?;
            let slot = supplied
                .get_mut(index)
                .ok_or_else(|| mismatch(&field_location))?;
            if slot.is_some() {
                return Err(mismatch(&field_location));
            }
            let optional = declaration.presence() == Presence::Optional;
            *slot = Some(match initializer {
                FieldInitializer::Null if optional => RecordSlot::Null,
                FieldInitializer::Null => return Err(mismatch(&field_location)),
                FieldInitializer::Value(expression) => RecordSlot::Present(Box::new(
                    self.check_as(expression, declaration.value_type(), &field_location)?,
                )),
            });
        }
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

    fn collection_literal(
        &mut self,
        kind: CollectionKind,
        elements: &[Expression],
        hint: Option<&ValueType>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let collection_type = match hint {
            None => {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::AmbiguousLiteral,
                ))
            }
            Some(ValueType::Collection(collection_type)) if collection_type.kind() == kind => {
                (**collection_type).clone()
            }
            Some(_) => return Err(mismatch(location)),
        };
        let mut typed = Vec::with_capacity(elements.len());
        for (index, element) in elements.iter().enumerate() {
            typed.push(self.check_as(
                element,
                collection_type.element(),
                &location.child(index),
            )?);
        }
        let value_type = ValueType::collection(collection_type.clone());
        Ok(node(
            NodeKind::Collection {
                collection_type,
                elements: typed,
            },
            value_type,
            location,
        ))
    }

    fn convert(
        &mut self,
        target: &qsl_forms::TypeForm,
        operand: &Expression,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let target = self.resolve_type(target, location)?;
        let target = &target;
        self.check_declared_type(target, location)?;
        let operand = self.infer(operand, None, &location.child(0))?;
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
                if !admits_equality_conversion(&operand.value_type, target) =>
            {
                Ok(node(
                    NodeKind::ConvertDecimal(Box::new(operand), decimal.clone()),
                    target.clone(),
                    location,
                ))
            }
            (source, _) => {
                if matches!(source, ValueType::Float(_))
                    || !admits_equality_conversion(source, target)
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

    /// `allInstances<T>(p)` (FR-153): `p`'s own checked `Population<T>[N]`
    /// type (`ValueType::Population`) gives the result's declared bound
    /// `[0,N]` directly; no runtime value is consulted at check time.
    fn all_instances(
        &mut self,
        target: &qsl_forms::TypeForm,
        population: &Expression,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let target = self.resolve_type(target, location)?;
        let target = &target;
        self.check_declared_type(target, location)?;
        if !matches!(target, ValueType::Reference(_)) {
            return Err(mismatch(location));
        }
        let population = self.infer(population, None, &location.child(0))?;
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

    /// `lookup<T>(p, r) absent m` (FR-153). `r`'s own checked static type `S`
    /// (never `T`) is `reference.value_type` at evaluation time
    /// (`crate::value::expression::evaluate`), so [`NodeKind::Lookup`] does
    /// not restate it. FR-153 also refuses `ill_typed`/`type-mismatch` at
    /// check time when `S` does not conform to `T` (TC-198 L03's last case,
    /// "before any charge") -- this checker cannot decide that here without
    /// the model's own generalization graph, which `TypeEnvironment` does not
    /// carry (the "TypeEnvironment island", tracked at
    /// <https://github.com/agent-ix/quire-spec-language/issues/164>), so that
    /// refusal is deferred to evaluation, inside
    /// `crate::model::population::lookup`'s own `type_conforms` call
    /// (`crate::value::evaluate_lookup`).
    fn lookup(
        &mut self,
        target: &qsl_forms::TypeForm,
        population: &Expression,
        reference: &Expression,
        absence: AbsenceMode,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let target = self.resolve_type(target, location)?;
        let target = &target;
        self.check_declared_type(target, location)?;
        if !matches!(target, ValueType::Reference(_)) {
            return Err(mismatch(location));
        }
        let population = self.infer(population, None, &location.child(0))?;
        // FR-153 requires `p` to be a population binding with a declared
        // maximum, otherwise `ill_typed`/`operator-ineligible` (TC-198 L06):
        // the operand is the wrong kind, not merely the wrong type name.
        if !matches!(population.value_type, ValueType::Population(_)) {
            return Err(ineligible(&population.location));
        }
        let reference = self.infer(reference, None, &location.child(1))?;
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

    fn query(
        &mut self,
        query: BinderQuery,
        binder: &str,
        source: &Expression,
        body: &Expression,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let source = self.infer(source, None, &location.child(0))?;
        let ValueType::Collection(source_type) = source.value_type.clone() else {
            return Err(mismatch(location));
        };
        let slot = self.bind(
            binder,
            source_type.element().clone(),
            location,
            LocalKind::Bound,
        )?;
        let body_location = location.child(1);
        let typed = match query {
            BinderQuery::Map | BinderQuery::FlatMap => {
                let body = self.infer(body, None, &body_location)?;
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
                let body = self.check_as(body, &ValueType::Boolean, &body_location)?;
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
                let body = self.check_as(body, &ValueType::Boolean, &body_location)?;
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

    #[allow(clippy::too_many_arguments)] // One argument per syntax member.
    fn accumulate(
        &mut self,
        form: Accumulation,
        accumulator_type: &str,
        accumulator: &str,
        binder: &str,
        source: &Expression,
        step: &Expression,
        identity: Option<&Expression>,
        location: &Location,
    ) -> Result<Node, CheckRefusal> {
        let value_type = self.type_named(accumulator_type, location)?;
        let source = self.infer(source, None, &location.child(0))?;
        let ValueType::Collection(source_type) = source.value_type.clone() else {
            return Err(mismatch(location));
        };
        match (form, identity) {
            (Accumulation::Fold, None) | (Accumulation::Reduce, Some(_)) => {
                return Err(mismatch(location))
            }
            (Accumulation::Fold, Some(_)) | (Accumulation::Reduce, None) => {}
        }
        if form == Accumulation::Reduce && source_type.element() != &value_type {
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
        let raw = self.infer(step, Some(&value_type), &location.child(1))?;
        self.unbind(2);
        let catalogued = catalogued_step(&raw, accumulator_slot, &value_type);
        let step = coerce(raw, &value_type)?;
        let identity = match identity {
            Some(identity) => Some(Box::new(self.check_as(
                identity,
                &value_type,
                &location.child(2),
            )?)),
            None => None,
        };
        if !source_type.kind().is_ordered() && !catalogued {
            return Err(ineligible(location));
        }
        Ok(node(
            NodeKind::Fold {
                accumulator: accumulator_slot,
                binder: binder_slot,
                source: Box::new(source),
                step: Box::new(step),
                identity,
            },
            value_type,
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
