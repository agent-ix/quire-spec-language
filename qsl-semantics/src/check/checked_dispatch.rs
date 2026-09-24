// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bridges one FR-151 dispatch family from its normalized model into the
//! value checker (TC-196 D06-D08).
//!
//! [`crate::model::dispatch::link_dispatch`] links a dispatch family by
//! [`DeclarationKey`] alone; it has no `Expression` to check and builds no
//! call-graph index (its own module docs record both as out of scope: it is
//! link-time linking only). This module is the other half: for one
//! dispatch-eligible root operation, it types every linked candidate's
//! effective precondition and body against
//! [`crate::check::PackageDeclarations`], and translates
//! `link_dispatch`'s [`DeclarationKey`]-keyed
//! [`crate::model::dispatch::DispatchTable`] into the checked layer's
//! [`EffectiveId`]- and function-index-keyed
//! [`crate::check::DispatchTable`], ready for
//! [`PackageDeclarations::check`](crate::check::PackageDeclarations)
//! and the evaluator.
//!
//! Pure: no intake, no I/O. It takes a caller-supplied [`DomainPackage`] and an
//! [`OperationClauses`] side table naming each candidate's own clause
//! `Expression` and signature. `DomainPackage` carries no `Expression` payload —
//! adding one would perturb the
//! `model-effective-declaration.schema.json`/`model-complete.md`
//! correspondence every other rung of this crate keys against — so this is where a real
//! intake (`agent-ix/quire-specification#131`) will eventually plug in its
//! own clause source in place of a test-built side table; nothing else here
//! needs to change when it does.
//!
//! Scope, kept to exactly what TC-196 D06-D08 need (re-read this note before
//! extending it):
//!
//! - One dispatch-eligible operation at a time, mirroring `link_dispatch`'s
//!   own per-operation signature. A package with several dispatch call sites
//!   calls this once per distinct receiver-type/member pair and merges the
//!   resulting `functions`/`dispatch_operations`/`dispatch_tables`.
//! - The receiver's admitted static types are derived from
//!   `table.entries()` and the model's effective view, not supplied by the
//!   caller: each subtype's [`DeclarationKey`] resolves to its
//!   [`EffectiveId`] through [`EffectiveView::type_identities`], the identity
//!   a `Reference<T>` value's type component carries (ADR-013 O-05). One
//!   [`DispatchOperation`] entry is built per distinct [`EffectiveId`]
//!   a `table.entries()` subtype resolves to, wherever that
//!   subtype's own winning candidate is `root.key` itself (#176, tightened
//!   by #204 round 1's M5) -- the operation's own declared owner, plus every
//!   conforming subtype that inherits the operation unredefined. A subtype
//!   that redefines it is exposed by that redefiner's own, separate
//!   `checked_dispatch_operation` call instead, so no two calls ever expose
//!   the same `(receiver_type, member)` pair.
//! - Parameter and result types come from [`OperationClauses`], not from a
//!   translation of `OperationMemberRecord`'s own producer-interface
//!   parameter/result records: turning a model-layer type reference into a
//!   checker [`ValueType`] is its own, separately-scoped piece of work
//!   (needed well beyond dispatch), so this bridge accepts it pre-translated
//!   exactly as `link_dispatch` accepts `domain_package`/`view` pre-normalized.
//! - Every distinct linked candidate must itself be a query -- a declared
//!   result and an empty effect set (#174, FR-151's own restriction on what
//!   may be a dispatch target) -- checked directly against each candidate's
//!   own [`crate::model::domain_package::OperationMemberRecord`], not the
//!   pre-translated `OperationClauses`: `checked_dispatch_operation` refuses
//!   [`DispatchBridgeRefusal::NotAQuery`] before typing any candidate's
//!   clauses when one is not, since nothing earlier in `src/`'s own pipeline
//!   enforces it (see that variant's own doc for the FR-272 cause choice).
//!
//! Effective-precondition semantics (FR-151
//! `quire.model.conformance.refinement/v1`: "the disjunction of its own
//! precondition clauses with the effective preconditions of the members it
//! redefines... An absent precondition is `true`") are computed twice, for
//! two different consumers, over the same redefinition ancestry:
//!
//! - `effective_terms` is the *runtime* computation: Boolean absorption
//!   (`true ∨ X = true`) means the walk stops the instant any node in the
//!   ancestry (starting at the candidate itself) has an absent own clause,
//!   yielding [`DispatchCandidate::precondition`] — `None` exactly when the
//!   effective precondition is unconditionally `true` (TC-196 D06: B.size's
//!   own precondition is absent, so its guard is skipped even though A.size
//!   still declares one).
//! - `ancestor_closure` is the *static* FR-146 call-graph computation: it
//!   walks the full ancestry unconditionally, regardless of runtime
//!   short-circuiting, because the call-graph edge FR-151 requires ("an
//!   edge... to every precondition clause of every candidate's effective
//!   precondition, which the call evaluates") is a *reachability* fact, not
//!   a runtime-value fact. TC-196 D08 depends on this: B.size's runtime
//!   guard is skipped, but its static ancestry still reaches PS, so the
//!   self-recursive call through `self.size()` inside PS still closes a
//!   cycle back to PS.
//!
//! One shared checked [`FunctionDeclaration`] is built per *authored*
//! (non-absent) precondition clause, reused by every candidate that inherits
//! it by redefinition — never duplicated per candidate. A candidate's own
//! [`DispatchCandidate::precondition`] reuses that shared function directly
//! when its own clause is the only contributing term; a combinator function
//! is synthesized only when a genuine multi-term disjunction is needed (the
//! candidate's own clause and at least one ancestor both contribute a real,
//! non-absent term), splicing each ancestor term inline via parameter
//! substitution rather than a runtime call by name (every synthesized
//! function here is built with [`FunctionDeclaration::clause`], so none of
//! them is reachable through an ordinary named call in the first place —
//! TC-196 D07's bypass).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use super::check::{DispatchOperation, PackageDeclarations};
use super::ir::{DispatchCandidate, DispatchTable};
use crate::model::accounting::Meter;
use crate::model::dispatch::{
    link_dispatch, DispatchLinkOutcome, GeneralizationClosure, LinkCheckOutcome,
};
use crate::model::domain_package::{DomainPackage, DomainPackageRecord, OperationMemberRecord};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{EffectiveView, ModelRefusal, ModelRefusalCause};
use qsl_forms::{
    BinaryOperator, DeclaredClauseKind, Expression, FieldInitializer, FunctionDeclaration,
};
use qsl_foundation::diagnostic::Code;
use quire_exact::EffectiveId;
use quire_exact::ValueType;

/// Bounds the effective-precondition ancestor walk. Declared separately here
/// since this is this bridge's own walk, independent of
/// `crate::model::dispatch`'s (ADR-011 §7.3, QSL-199 replaced that module's
/// own former `MAX_DISPATCH_DEPTH` with a caller-configured `Meter` ceiling;
/// out of this ticket's scope).
const MAX_ANCESTOR_DEPTH: usize = 128;

/// One dispatch candidate's own clause `Expression`s and signature, supplied
/// by the caller since [`DomainPackage`] carries no `Expression` payload. Every
/// operation this bridge is asked to check — the root operation and every
/// one of its linked candidates — needs an entry.
#[derive(Clone, Debug, Default)]
pub struct OperationClauses {
    /// The unqualified member name. Only the *root* operation's entry is
    /// consulted for this field.
    pub member: BTreeMap<DeclarationKey, String>,
    /// Declared parameters, receiver first: element 0 is always the
    /// receiver (`self`) parameter every candidate's own
    /// [`FunctionDeclaration`] needs at slot 0, matching how the evaluator's
    /// `NodeKind::Dispatch` binds `[receiver, ..arguments]` positionally into
    /// a linked candidate's frame. [`DispatchOperation::parameters`] (the
    /// call site's own argument types) is everything after that receiver
    /// element; this bridge strips it when building the call site.
    pub parameters: BTreeMap<DeclarationKey, Vec<(String, ValueType)>>,
    /// The declared result type.
    pub result: BTreeMap<DeclarationKey, ValueType>,
    /// This operation's own precondition clause, when it declares one.
    pub own_precondition: BTreeMap<DeclarationKey, Expression>,
    /// This operation's own body clause. Required for every linked
    /// candidate (an operation without a body is never a candidate;
    /// `link_dispatch` already filters those out).
    pub own_body: BTreeMap<DeclarationKey, Expression>,
}

/// Which [`OperationClauses`] field [`DispatchBridgeRefusal::MissingClauseData`]
/// found no entry under, as a closed set the compiler checks rather than a
/// free-text field name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissingClauseField {
    /// [`OperationClauses::member`].
    Member,
    /// [`OperationClauses::parameters`].
    Parameters,
    /// [`OperationClauses::result`].
    Result,
    /// [`OperationClauses::own_precondition`].
    OwnPrecondition,
    /// [`OperationClauses::own_body`].
    OwnBody,
}

/// Why [`checked_dispatch_operation`] could not produce a checked
/// [`PackageDeclarations`].
#[derive(Clone, Debug)]
pub enum DispatchBridgeRefusal {
    /// `link_dispatch` did not produce a linked table: ambiguous, refused,
    /// incomplete, or an open generalization closure. Boxed: `LinkCheckOutcome`
    /// is far larger than the other variants, and this refusal is returned by
    /// value from every fallible step below.
    Unlinked(Box<LinkCheckOutcome>),
    /// `clauses` has no entry for this operation under the named field.
    /// `operation` is boxed: [`DeclarationKey`] itself is larger than the rest
    /// of this refusal put together.
    MissingClauseData {
        /// The operation missing an entry.
        operation: Box<DeclarationKey>,
        /// The field that was missing it.
        field: MissingClauseField,
    },
    /// The effective view has no top-level declaration for a linked table's
    /// subtype or a declared supertype, so it has no [`EffectiveId`]. A
    /// distinct variant from [`Self::MissingClauseData`]: the key missing an
    /// entry here is a *subtype*, not an operation, so it earns its own field
    /// name instead of overloading `operation`.
    MissingObjectKey {
        /// The subtype missing an entry.
        subtype: Box<DeclarationKey>,
    },
    /// The effective-precondition ancestor walk exceeded
    /// `MAX_ANCESTOR_DEPTH` redefinition steps, built from
    /// [`ModelRefusalCause::DispatchFamilyDepth`] exactly as
    /// `crate::model::dispatch::build_family`'s own depth-exceeded refusal
    /// is, rather than a bridge-only cause. Boxed: `ModelRefusal` is far
    /// larger than the other variants.
    AncestorDepthExceeded(Box<ModelRefusal>),
    /// A linked candidate reachable as this dispatch's own target has no
    /// declared result, or a non-empty effect set (#174): FR-151
    /// (`quire.model.dispatch.single/v1`) admits only a query -- "whose
    /// result is present and whose effect set is empty" -- as a dispatch
    /// target. This is a valid declaration typed wrong for its role, not a
    /// malformed one, so it is [`Code::IllTyped`] /
    /// [`ModelRefusalCause::OperatorIneligible`], the same pairing
    /// `crate::model::population::all_instances` already uses for a binding
    /// ineligible for the operator applied to it, rather than a bridge-only
    /// cause. Boxed: `ModelRefusal` is far larger than the other variants.
    NotAQuery(Box<ModelRefusal>),
    /// A candidate reached from `root.key` or `table.entries()` names no
    /// declared operation member in `domain_package.records`. `link_dispatch`
    /// builds its own `DispatchIndex` from the same records, so this should
    /// not arise from a table it linked; kept as a typed refusal rather than
    /// an `.expect()` so a malformed `domain_package`/`root` pairing supplied
    /// directly to this bridge (bypassing `link_dispatch`'s own index) is
    /// reported, not panicked, exactly as `crate::model::dispatch::link_dispatch`
    /// itself reports every other missing-candidate lookup. Boxed:
    /// `ModelRefusal` is far larger than the other variants.
    UnknownCandidate(Box<ModelRefusal>),
}

fn missing(operation: &DeclarationKey, field: MissingClauseField) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::MissingClauseData {
        operation: Box::new(operation.clone()),
        field,
    }
}

fn depth_exceeded(candidate: &DeclarationKey) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::AncestorDepthExceeded(Box::new(ModelRefusal {
        code: Code::ResourceExhausted,
        cause: ModelRefusalCause::DispatchFamilyDepth {
            original: candidate.clone(),
        },
        detail: format!(
            "effective-precondition ancestry for {} exceeded {MAX_ANCESTOR_DEPTH} redefinition steps",
            candidate.node
        ),
    }))
}

/// `operation` is a linked dispatch target with no declared result, or a
/// non-empty effect set (#174, see [`DispatchBridgeRefusal::NotAQuery`]'s
/// own doc for the cause choice).
fn not_a_query(operation: &DeclarationKey) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::NotAQuery(Box::new(ModelRefusal {
        code: Code::IllTyped,
        cause: ModelRefusalCause::OperatorIneligible,
        detail: format!(
            "{} is a dispatch target but is not a query: FR-151 \
             (quire.model.dispatch.single/v1) admits only an operation \
             whose result is present and whose effect set is empty",
            operation.node
        ),
    }))
}

/// `candidate` is reached from `root.key`/`table.entries()` but names no
/// declared operation member (see [`DispatchBridgeRefusal::UnknownCandidate`]'s
/// own doc for why this is typed rather than an `.expect()`).
fn unknown_candidate(candidate: &DeclarationKey) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::UnknownCandidate(Box::new(ModelRefusal {
        code: Code::DanglingReference,
        cause: ModelRefusalCause::UnknownCandidate {
            candidate: candidate.clone(),
        },
        detail: format!("{} is not a declared operation member", candidate.node),
    }))
}

fn require_expression(
    map: &BTreeMap<DeclarationKey, Expression>,
    operation: &DeclarationKey,
    field: MissingClauseField,
) -> Result<Expression, DispatchBridgeRefusal> {
    map.get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, field))
}

/// The `TypeForm` every synthesized `FunctionDeclaration` this module builds
/// carries in place of a declared type. `check` never resolves it:
/// `checked_dispatch_operation`'s `PackageDeclarations::resolved_signatures`
/// entry for the same index supplies the resolved signature
/// `OperationClauses` gave, and identity is minted over that resolved
/// signature. A synthesized clause is not parsed source, so layer 3 does not
/// render its resolved types back into syntax to parse them again.
fn opaque_type_form() -> qsl_forms::TypeForm {
    qsl_forms::TypeForm::name(
        "check::checked_dispatch synthesized (never resolved; see resolved_signatures)",
        qsl_foundation::Span { start: 0, end: 0 },
    )
}

/// [`opaque_type_form`] for every one of `parameters`, keeping each
/// parameter's own declared name (`bind_parameters`/`Definedness::new` still
/// need the right arity and names; only the type is opaque).
fn opaque_parameters(parameters: &[(String, ValueType)]) -> Vec<(String, qsl_forms::TypeForm)> {
    parameters
        .iter()
        .map(|(name, _)| (name.clone(), opaque_type_form()))
        .collect()
}

fn require_signature(
    clauses: &OperationClauses,
    operation: &DeclarationKey,
) -> Result<super::check::ResolvedSignature, DispatchBridgeRefusal> {
    let parameters = clauses
        .parameters
        .get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, MissingClauseField::Parameters))?;
    let result = clauses
        .result
        .get(operation)
        .cloned()
        .ok_or_else(|| missing(operation, MissingClauseField::Result))?;
    Ok((parameters, result))
}

/// The FR-151 dispatch-eligible operation [`checked_dispatch_operation`] is
/// asked to check: its own key, and whether the generalization closure is
/// known closed. Grouped into one value to keep that function's own
/// argument count small.
pub struct DispatchRoot {
    /// The root operation's own original producer key.
    pub key: DeclarationKey,
    /// Whether the generalization closure relevant to this operation is
    /// known closed.
    pub closure: GeneralizationClosure,
}

/// Every declared object type's own `supertypes` (H1, #204 round 1),
/// translated from `domain_package.records`' [`DeclarationKey`]s into their
/// [`EffectiveId`]s through `view`'s [`EffectiveView::type_identities`] --
/// the exact shape [`crate::value::declaration::ObjectTypeDeclaration::with_supertypes`]
/// needs (ADR-013 O-05). No production code builds a
/// [`crate::value::declaration::TypeEnvironment`] yet (only test scaffolding does), so
/// this is test-support infrastructure today; #131's own real intake can
/// call it exactly as tests do.
pub fn object_type_supertypes(
    domain_package: &DomainPackage,
    view: &EffectiveView,
) -> Result<BTreeMap<EffectiveId, Vec<EffectiveId>>, DispatchBridgeRefusal> {
    let type_identities = view.type_identities();
    let mut supertypes: BTreeMap<EffectiveId, Vec<EffectiveId>> = BTreeMap::new();
    for record in &domain_package.records {
        if let DomainPackageRecord::ObjectType(object_type) = record {
            let subtype_key = type_identities
                .get(&object_type.key)
                .copied()
                .ok_or_else(|| DispatchBridgeRefusal::MissingObjectKey {
                    subtype: Box::new(object_type.key.clone()),
                })?;
            let mut mapped = Vec::with_capacity(object_type.supertypes.len());
            for supertype in &object_type.supertypes {
                let supertype_key = type_identities.get(supertype).copied().ok_or_else(|| {
                    DispatchBridgeRefusal::MissingObjectKey {
                        subtype: Box::new(supertype.clone()),
                    }
                })?;
                mapped.push(supertype_key);
            }
            supertypes.insert(subtype_key, mapped);
        }
    }
    Ok(supertypes)
}

/// `domain_package.records`'s own first-appearance index of every declared
/// operation, keyed by its [`DeclarationKey`] (`domain_package.rs`: `records`
/// is "in the producer's declared order"). [`DeclarationKey`]'s own `Ord` sorts by
/// `package, node`, unrelated to source order, so this — not a
/// `BTreeSet<DeclarationKey>` iteration — is FR-151's "source declaration
/// order" for the D08 call-graph edge listing.
fn declaration_order(domain_package: &DomainPackage) -> BTreeMap<DeclarationKey, usize> {
    let mut order = BTreeMap::new();
    for (index, record) in domain_package.records.iter().enumerate() {
        if let DomainPackageRecord::OperationMember(operation) = record {
            order.entry(operation.key.clone()).or_insert(index);
        }
    }
    order
}

/// `candidate` together with every operation reaching it by any chain of
/// members' own inline `redefines` property (`model-complete.md`:162): the
/// full static FR-146 reachability set [`ancestor_closure`] needs for
/// [`DispatchCandidate::precondition_clauses`]. Bounded breadth-first walk
/// over an explicit queue, never native recursion; refuses at the depth
/// bound instead of silently truncating the closure.
fn ancestor_closure(
    redefinition_parents: &BTreeMap<DeclarationKey, DeclarationKey>,
    candidate: &DeclarationKey,
) -> Result<BTreeSet<DeclarationKey>, DispatchBridgeRefusal> {
    let mut closure = BTreeSet::new();
    // `depth` is `current`'s own redefinition-chain distance from
    // `candidate` (0 for `candidate` itself), tracked per queue entry, not a
    // running count of every node the whole closure has visited so far: a
    // family where one member has many direct redefiners (wide, shallow
    // fan-out) must not exhaust the same bound a genuinely deep single chain
    // would.
    let mut pending: VecDeque<(DeclarationKey, usize)> = VecDeque::from([(candidate.clone(), 0)]);
    while let Some((current, depth)) = pending.pop_front() {
        if !closure.insert(current.clone()) {
            continue;
        }
        if depth > MAX_ANCESTOR_DEPTH {
            return Err(depth_exceeded(candidate));
        }
        if let Some(parent) = redefinition_parents.get(&current) {
            pending.push_back((parent.clone(), depth + 1));
        }
    }
    Ok(closure)
}

/// The runtime effective-precondition terms of `candidate`, in `candidate`'s
/// own parameter names: `None` where the effective precondition is
/// unconditionally `true` (its own clause is absent, or absorption through
/// any ancestor makes it so); `Some(terms)` where every term is a real,
/// non-absent clause still contributing — `terms[0]` is always `candidate`'s
/// own clause, and every later element is an ancestor's contributing clause,
/// substituted from that ancestor's parameter names into `candidate`'s own —
/// the receiver is `parameters[0]`, substituted the same way as every other
/// parameter. Memoized per candidate, since a diamond of
/// redefinitions can reach the same ancestor from several paths; bounded by
/// `depth`, the current call's own redefinition-chain distance from the
/// *top-level* candidate this walk started at (0 there, incremented once per
/// recursive step into a parent) — never a counter shared across every
/// candidate [`checked_dispatch_operation`] computes this for. A family with
/// many precondition-bearing members but no chain longer than
/// [`MAX_ANCESTOR_DEPTH`] must not be refused just because the family is
/// wide; a memoized hit returns immediately without consuming any of the
/// caller's own depth budget, since its own walk already passed the bound
/// when it was first computed.
fn effective_terms(
    candidate: &DeclarationKey,
    clauses: &OperationClauses,
    redefinition_parents: &BTreeMap<DeclarationKey, DeclarationKey>,
    memo: &mut BTreeMap<DeclarationKey, Option<Vec<Expression>>>,
    depth: usize,
) -> Result<Option<Vec<Expression>>, DispatchBridgeRefusal> {
    if let Some(cached) = memo.get(candidate) {
        return Ok(cached.clone());
    }
    if depth > MAX_ANCESTOR_DEPTH {
        return Err(depth_exceeded(candidate));
    }
    let Some(own_expression) = clauses.own_precondition.get(candidate).cloned() else {
        memo.insert(candidate.clone(), None);
        return Ok(None);
    };
    let (candidate_parameters, _) = require_signature(clauses, candidate)?;
    let mut terms = vec![own_expression];
    if let Some(parent) = redefinition_parents.get(candidate) {
        match effective_terms(parent, clauses, redefinition_parents, memo, depth + 1)? {
            None => {
                memo.insert(candidate.clone(), None);
                return Ok(None);
            }
            Some(parent_terms) => {
                let (parent_parameters, _) = require_signature(clauses, parent)?;
                for term in parent_terms {
                    terms.push(rename_parameters(
                        &term,
                        &parent_parameters,
                        &candidate_parameters,
                    ));
                }
            }
        }
    }
    memo.insert(candidate.clone(), Some(terms.clone()));
    Ok(Some(terms))
}

/// `expression` with every name bound positionally in `from` rewritten to
/// its counterpart in `to`, leaving every shadowed occurrence (a `let`,
/// query or accumulate/count/sum binder reusing a `from` name) untouched.
/// Capture-avoiding: a binder whose own name coincides with a `to` name is
/// itself alpha-renamed to a fresh name before its scope is entered, so a
/// name introduced by the rename never falls under a binder that merely
/// happens to share that spelling in the source expression (`let b = 5 in
/// a = a`, renamed `a -> b`, must not become `let b = 5 in b = b`).
/// Short-circuits to a plain clone when every name already matches.
fn rename_parameters(
    expression: &Expression,
    from: &[(String, ValueType)],
    to: &[(String, ValueType)],
) -> Expression {
    let rename: BTreeMap<String, String> = from
        .iter()
        .zip(to)
        .filter(|((from_name, _), (to_name, _))| from_name != to_name)
        .map(|((from_name, _), (to_name, _))| (from_name.clone(), to_name.clone()))
        .collect();
    if rename.is_empty() {
        return expression.clone();
    }
    let to_names: BTreeSet<String> = rename.values().cloned().collect();
    let mut scope: Vec<(String, Option<String>)> = Vec::new();
    substitute_names(expression, &rename, &to_names, &mut scope)
}

/// Binds `name` for the scope about to be entered: `scope` records, for
/// every binder currently in view, its original name and — when that name
/// collides with a `to_names` name a rename could introduce — the fresh
/// name it was alpha-renamed to, avoiding capture. Returns the name the
/// rewritten binder must actually use. The caller pops `scope` once the
/// binder's own scope (its body/step/predicate) has been rewritten.
fn bind_name(
    name: &str,
    to_names: &BTreeSet<String>,
    scope: &mut Vec<(String, Option<String>)>,
) -> String {
    if !to_names.contains(name) {
        scope.push((name.to_owned(), None));
        return name.to_owned();
    }
    let occupied: BTreeSet<String> = to_names
        .iter()
        .cloned()
        .chain(
            scope
                .iter()
                .map(|(original, alpha)| alpha.clone().unwrap_or_else(|| original.clone())),
        )
        .collect();
    let mut suffix = 0usize;
    let mut fresh = format!("{name}#dispatch-alpha{suffix}");
    while occupied.contains(&fresh) {
        suffix += 1;
        fresh = format!("{name}#dispatch-alpha{suffix}");
    }
    scope.push((name.to_owned(), Some(fresh.clone())));
    fresh
}

/// The full recursive rewrite [`rename_parameters`] applies: every
/// [`Expression::Name`] is looked up in `scope` (innermost binder first);
/// a hit shadows the top-level `rename` map, using the binder's alpha-name
/// when it has one and the original name otherwise. A name `scope` does not
/// mention falls through to `rename`. Every binder site pushes its own
/// [`bind_name`] result onto `scope` before recursing into its own scope and
/// pops it after — see [`bind_name`] for how binder capture is avoided.
fn substitute_names(
    expression: &Expression,
    rename: &BTreeMap<String, String>,
    to_names: &BTreeSet<String>,
    scope: &mut Vec<(String, Option<String>)>,
) -> Expression {
    match expression {
        Expression::Boolean(_) | Expression::Integer(_) | Expression::Rational(..) => {
            expression.clone()
        }
        Expression::Name(name) => {
            for (original, alpha) in scope.iter().rev() {
                if original == name {
                    return match alpha {
                        Some(fresh) => Expression::Name(fresh.clone()),
                        None => expression.clone(),
                    };
                }
            }
            match rename.get(name) {
                Some(renamed) => Expression::Name(renamed.clone()),
                None => expression.clone(),
            }
        }
        Expression::Let { name, value, body } => {
            let value = Box::new(substitute_names(value, rename, to_names, scope));
            let name = bind_name(name, to_names, scope);
            let body = Box::new(substitute_names(body, rename, to_names, scope));
            scope.pop();
            Expression::Let { name, value, body }
        }
        Expression::If {
            condition,
            then,
            otherwise,
        } => Expression::If {
            condition: Box::new(substitute_names(condition, rename, to_names, scope)),
            then: Box::new(substitute_names(then, rename, to_names, scope)),
            otherwise: Box::new(substitute_names(otherwise, rename, to_names, scope)),
        },
        Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            operator: *operator,
            left: Box::new(substitute_names(left, rename, to_names, scope)),
            right: Box::new(substitute_names(right, rename, to_names, scope)),
        },
        Expression::Negate(operand) => {
            Expression::Negate(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Not(operand) => {
            Expression::Not(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Field { operand, field } => Expression::Field {
            operand: Box::new(substitute_names(operand, rename, to_names, scope)),
            field: field.clone(),
        },
        Expression::Present(operand) => {
            Expression::Present(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Value(operand) => {
            Expression::Value(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Deref(operand) => {
            Expression::Deref(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Call { name, arguments } => Expression::Call {
            name: name.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_names(argument, rename, to_names, scope))
                .collect(),
        },
        Expression::Record { name, fields } => Expression::Record {
            name: name.clone(),
            fields: fields
                .iter()
                .map(|(field, initializer)| {
                    let initializer = match initializer {
                        FieldInitializer::Value(value) => FieldInitializer::Value(
                            substitute_names(value, rename, to_names, scope),
                        ),
                        FieldInitializer::Null => FieldInitializer::Null,
                    };
                    (field.clone(), initializer)
                })
                .collect(),
        },
        Expression::Collection { kind, elements } => Expression::Collection {
            kind: *kind,
            elements: elements
                .iter()
                .map(|element| substitute_names(element, rename, to_names, scope))
                .collect(),
        },
        Expression::Convert { target, operand } => Expression::Convert {
            target: target.clone(),
            operand: Box::new(substitute_names(operand, rename, to_names, scope)),
        },
        Expression::Query {
            query,
            binder,
            source,
            body,
        } => {
            let source = Box::new(substitute_names(source, rename, to_names, scope));
            let binder = bind_name(binder, to_names, scope);
            let body = Box::new(substitute_names(body, rename, to_names, scope));
            scope.pop();
            Expression::Query {
                query: *query,
                binder,
                source,
                body,
            }
        }
        Expression::Flatten(operand) => {
            Expression::Flatten(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Accumulate {
            form,
            accumulator_type,
            accumulator,
            binder,
            source,
            step,
            identity,
        } => {
            let source = Box::new(substitute_names(source, rename, to_names, scope));
            let identity = identity
                .as_ref()
                .map(|identity| Box::new(substitute_names(identity, rename, to_names, scope)));
            let accumulator = bind_name(accumulator, to_names, scope);
            let binder = bind_name(binder, to_names, scope);
            let step = Box::new(substitute_names(step, rename, to_names, scope));
            scope.pop();
            scope.pop();
            Expression::Accumulate {
                form: *form,
                accumulator_type: accumulator_type.clone(),
                accumulator,
                binder,
                source,
                step,
                identity,
            }
        }
        Expression::Count {
            result_type,
            binder,
            source,
            predicate,
        } => {
            let source = Box::new(substitute_names(source, rename, to_names, scope));
            let binder = bind_name(binder, to_names, scope);
            let predicate = Box::new(substitute_names(predicate, rename, to_names, scope));
            scope.pop();
            Expression::Count {
                result_type: result_type.clone(),
                binder,
                source,
                predicate,
            }
        }
        Expression::Sum {
            result_type,
            binder,
            source,
            summand,
        } => {
            let source = Box::new(substitute_names(source, rename, to_names, scope));
            let binder = bind_name(binder, to_names, scope);
            let summand = Box::new(substitute_names(summand, rename, to_names, scope));
            scope.pop();
            Expression::Sum {
                result_type: result_type.clone(),
                binder,
                source,
                summand,
            }
        }
        Expression::Size(operand) => {
            Expression::Size(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
        Expression::Contains { collection, item } => Expression::Contains {
            collection: Box::new(substitute_names(collection, rename, to_names, scope)),
            item: Box::new(substitute_names(item, rename, to_names, scope)),
        },
        Expression::AllInstances { target, population } => Expression::AllInstances {
            target: target.clone(),
            population: Box::new(substitute_names(population, rename, to_names, scope)),
        },
        Expression::Lookup {
            target,
            population,
            reference,
            absence,
        } => Expression::Lookup {
            target: target.clone(),
            population: Box::new(substitute_names(population, rename, to_names, scope)),
            reference: Box::new(substitute_names(reference, rename, to_names, scope)),
            absence: *absence,
        },
        Expression::Dispatch {
            receiver,
            member,
            arguments,
        } => Expression::Dispatch {
            receiver: Box::new(substitute_names(receiver, rename, to_names, scope)),
            member: member.clone(),
            arguments: arguments
                .iter()
                .map(|argument| substitute_names(argument, rename, to_names, scope))
                .collect(),
        },
        Expression::Pre(operand) => {
            Expression::Pre(Box::new(substitute_names(operand, rename, to_names, scope)))
        }
    }
}

/// Types every linked candidate's effective precondition and body for one
/// FR-151 dispatch-eligible operation, and assembles the checked-layer
/// [`DispatchTable`] the evaluator needs. See the module docs for this
/// bridge's exact scope.
pub fn checked_dispatch_operation(
    domain_package: &DomainPackage,
    view: &EffectiveView,
    root: &DispatchRoot,
    clauses: &OperationClauses,
    meter: &mut Meter,
) -> Result<PackageDeclarations, DispatchBridgeRefusal> {
    let outcome = link_dispatch(domain_package, view, &root.key, root.closure, meter);
    let type_identities = view.type_identities();
    let table = match outcome {
        LinkCheckOutcome::Completed(DispatchLinkOutcome::Linked(table)) => table,
        other => return Err(DispatchBridgeRefusal::Unlinked(Box::new(other))),
    };

    let mut redefinitions: BTreeMap<DeclarationKey, DeclarationKey> = BTreeMap::new();
    let mut operation_records: BTreeMap<DeclarationKey, &OperationMemberRecord> = BTreeMap::new();
    for record in &domain_package.records {
        if let DomainPackageRecord::OperationMember(operation) = record {
            operation_records.insert(operation.key.clone(), operation);
            if let Some(redefined) = &operation.redefines {
                redefinitions.insert(operation.key.clone(), redefined.clone());
            }
        }
    }
    let order = declaration_order(domain_package);
    let by_order = |key: &DeclarationKey| order.get(key).copied().unwrap_or(usize::MAX);

    let mut distinct: BTreeSet<DeclarationKey> = BTreeSet::new();
    for (_, candidate) in table.entries() {
        distinct.insert(candidate.clone());
    }
    let mut ordered_candidates: Vec<DeclarationKey> = distinct.iter().cloned().collect();
    ordered_candidates.sort_by_key(|candidate| (by_order(candidate), candidate.clone()));

    // FR-151 (`quire.model.dispatch.single/v1`): "Only query operations,
    // whose result is present and whose effect set is empty, may be
    // called" (#174). `root.key` is checked first, even when it declares no
    // body and so is never itself a `table.entries()` candidate: FR-151's
    // query-only restriction binds the operation a dispatch call resolves
    // to by member name, not only the operations that happened to end up
    // runtime-eligible under it. Every distinct linked candidate follows,
    // since any of them can be "the operation actually dispatched" at
    // runtime for some conforming subtype; FR-151's own conformance
    // variance already forces every redefiner's result presence and effect
    // frame to agree with what it redefines wherever that checking runs,
    // but nothing in `src/`'s own build pipeline runs it before this
    // bridge, so this is the one real gate today. Checked in plain
    // `DeclarationKey` order (`distinct`'s own `BTreeSet` order), not
    // `ordered_candidates`' record order, so a refusal with more than one
    // disqualified candidate is deterministic regardless of
    // `domain_package.records`' declaration order; `ordered_candidates`
    // itself is left untouched since `functions` below still needs its
    // record order for FR-151 D08's declaration-order call-graph edges.
    let mut query_checked: BTreeSet<DeclarationKey> = BTreeSet::new();
    let mut query_check_order: Vec<DeclarationKey> = Vec::with_capacity(distinct.len() + 1);
    if query_checked.insert(root.key.clone()) {
        query_check_order.push(root.key.clone());
    }
    for candidate in &distinct {
        if query_checked.insert(candidate.clone()) {
            query_check_order.push(candidate.clone());
        }
    }
    for candidate in &query_check_order {
        let record = operation_records
            .get(candidate)
            .ok_or_else(|| unknown_candidate(candidate))?;
        let is_query = record.result.is_some()
            && record.effect.modifies.is_empty()
            && record.effect.creates.is_empty()
            && record.effect.deletes.is_empty();
        if !is_query {
            return Err(not_a_query(candidate));
        }
    }

    // Every member (candidate or redefinition ancestor) whose own
    // precondition clause some candidate's static FR-146 call-graph ancestry
    // reaches, regardless of runtime short-circuiting (TC-196 D08).
    let mut closures: BTreeMap<DeclarationKey, BTreeSet<DeclarationKey>> = BTreeMap::new();
    let mut every_member: BTreeSet<DeclarationKey> = BTreeSet::new();
    for candidate in &ordered_candidates {
        let closure = ancestor_closure(&redefinitions, candidate)?;
        every_member.extend(closure.iter().cloned());
        closures.insert(candidate.clone(), closure);
    }
    let mut authored: Vec<DeclarationKey> = every_member
        .into_iter()
        .filter(|member| clauses.own_precondition.contains_key(member))
        .collect();
    authored.sort_by_key(|member| (by_order(member), member.clone()));

    // One shared checked function per authored clause, in source declaration
    // order, built before any candidate body or combinator.
    let mut functions: Vec<FunctionDeclaration> = Vec::new();
    // Every synthesized `FunctionDeclaration` pushed below carries
    // `opaque_type_form`; this records its resolved signature by index into
    // `functions`.
    let mut resolved_signatures = super::check::ResolvedSignatures::default();
    let mut authored_index: BTreeMap<DeclarationKey, usize> = BTreeMap::new();
    for member in &authored {
        let (parameters, _) = require_signature(clauses, member)?;
        let body = require_expression(
            &clauses.own_precondition,
            member,
            MissingClauseField::OwnPrecondition,
        )?;
        let index = functions.len();
        let declared_parameters = opaque_parameters(&parameters);
        resolved_signatures.insert(index, (parameters, ValueType::Boolean));
        functions.push(FunctionDeclaration::clause(
            format!("{}.precondition", member.node),
            declared_parameters,
            opaque_type_form(),
            None,
            body,
            DeclaredClauseKind::Precondition,
        ));
        authored_index.insert(member.clone(), index);
    }

    let mut memo: BTreeMap<DeclarationKey, Option<Vec<Expression>>> = BTreeMap::new();
    let mut body_index: BTreeMap<DeclarationKey, usize> = BTreeMap::new();
    let mut precondition_index: BTreeMap<DeclarationKey, usize> = BTreeMap::new();
    let mut precondition_clauses_index: BTreeMap<DeclarationKey, Vec<usize>> = BTreeMap::new();

    for candidate in &ordered_candidates {
        let (parameters, result) = require_signature(clauses, candidate)?;

        let terms = effective_terms(candidate, clauses, &redefinitions, &mut memo, 0)?;
        if let Some(terms) = terms {
            let precondition_function = if terms.len() == 1 {
                // The candidate's own clause is the only contributing term:
                // reuse the shared authored function directly, no combinator.
                *authored_index
                    .get(candidate)
                    .ok_or_else(|| missing(candidate, MissingClauseField::OwnPrecondition))?
            } else {
                let combined = terms
                    .into_iter()
                    .reduce(|left, right| Expression::Binary {
                        operator: BinaryOperator::Or,
                        left: Box::new(left),
                        right: Box::new(right),
                    })
                    .ok_or_else(|| missing(candidate, MissingClauseField::OwnPrecondition))?;
                let index = functions.len();
                let declared_parameters = opaque_parameters(&parameters);
                resolved_signatures.insert(index, (parameters.clone(), ValueType::Boolean));
                functions.push(FunctionDeclaration::clause(
                    format!("{}.precondition.effective", candidate.node),
                    declared_parameters,
                    opaque_type_form(),
                    None,
                    combined,
                    DeclaredClauseKind::Precondition,
                ));
                index
            };
            precondition_index.insert(candidate.clone(), precondition_function);
        }

        let closure = closures
            .get(candidate)
            .cloned()
            .unwrap_or_else(|| BTreeSet::from([candidate.clone()]));
        let mut clause_functions: Vec<usize> = closure
            .into_iter()
            .filter_map(|member| authored_index.get(&member).copied())
            .collect();
        clause_functions.sort_unstable();
        clause_functions.dedup();
        precondition_clauses_index.insert(candidate.clone(), clause_functions);

        let own_body =
            require_expression(&clauses.own_body, candidate, MissingClauseField::OwnBody)?;
        let index = functions.len();
        let declared_parameters = opaque_parameters(&parameters);
        resolved_signatures.insert(index, (parameters, result));
        functions.push(FunctionDeclaration::clause(
            candidate.node.clone(),
            declared_parameters,
            opaque_type_form(),
            None,
            own_body,
            DeclaredClauseKind::Body,
        ));
        body_index.insert(candidate.clone(), index);
    }

    let mut entries = Vec::with_capacity(table.entries().len());
    // Every subtype's own [`EffectiveId`] that `table.entries()` links
    // *to `root.key` itself*, in the table's own subtype-enumeration order
    // (ascending effective identity), deduplicated (#176: `link_dispatch`
    // links one entry per conforming subtype in `view`; the
    // reflexive entry for the operation's own declared owner is one of
    // them, since `type_conforms(s, t)` is `true` for `s == t` -- see
    // `conformance::type_conforms`). Feeds every exposing static type's own
    // [`DispatchOperation`] entry below, so a call whose receiver's static
    // type is any type this dispatch is exposed through -- the operation's
    // own declared owner or any conforming subtype that exposes it by
    // inheriting it unredefined -- resolves (FR-151,
    // `quire.model.dispatch.single/v1`: "member-name resolves statically to
    // exactly one exposed effective operation... of the receiver's static
    // type `T`"), not only a call through the operation's own declared
    // owner type. A subtype whose own winning candidate is some *other*
    // redefiner is left out here (M5, #204 round 1): that subtype's own
    // effective member resolves to that redefiner, not to `root.key`, so it
    // is exposed by that redefiner's own separate `checked_dispatch_operation`
    // call instead -- exposing it here too would hand `check.rs`'s
    // `dispatch_call` two `DispatchOperation` entries for the same
    // `(receiver_type, member)` pair with different signatures, and which
    // one it found would depend on merge order. `table.entries()` itself
    // still gets a row for every subtype below, redefiner or not: that
    // feeds `checked_table`, which resolves a receiver's *runtime* type
    // regardless of which static type admitted the call.
    let mut exposing_receiver_types: Vec<EffectiveId> = Vec::new();
    let mut exposing_seen: BTreeSet<EffectiveId> = BTreeSet::new();
    for (subtype, candidate) in table.entries() {
        let subtype_key = type_identities.get(subtype).copied().ok_or_else(|| {
            DispatchBridgeRefusal::MissingObjectKey {
                subtype: Box::new(subtype.clone()),
            }
        })?;
        if candidate == &root.key && exposing_seen.insert(subtype_key) {
            exposing_receiver_types.push(subtype_key);
        }
        let &body = body_index
            .get(candidate)
            .ok_or_else(|| missing(candidate, MissingClauseField::OwnBody))?;
        let precondition = precondition_index.get(candidate).copied();
        let precondition_clauses = precondition_clauses_index
            .get(candidate)
            .cloned()
            .unwrap_or_default();
        entries.push((
            subtype_key,
            DispatchCandidate {
                body,
                precondition,
                precondition_clauses,
            },
        ));
    }
    // `distinct.len()` is a real candidate count, never a value near
    // `u64::MAX`; the fallback only avoids a lossy `as` cast and is not a
    // reachable refusal path.
    let candidate_count = u64::try_from(distinct.len()).unwrap_or(u64::MAX);
    let checked_table = DispatchTable::new(entries, candidate_count);

    let member = clauses
        .member
        .get(&root.key)
        .cloned()
        .ok_or_else(|| missing(&root.key, MissingClauseField::Member))?;
    let (parameters, result) = require_signature(clauses, &root.key)?;
    // `parameters` is receiver-first (see `OperationClauses::parameters`);
    // the call site's own argument types are everything after it.
    let call_arguments: Vec<ValueType> = parameters
        .get(1..)
        .unwrap_or_default()
        .iter()
        .map(|(_, value_type)| value_type.clone())
        .collect();
    // One `DispatchOperation` entry per exposing static type (#176), all
    // sharing this same `member`/`parameters`/`result` signature and
    // pointing at the identical `table: 0` -- the receiver's *runtime* type
    // still resolves through `checked_table` as before; only the set of
    // *static* types `check.rs`'s `dispatch_call` admits as a call site's
    // receiver grows from the operation's own declared owner alone to every
    // type this dispatch is exposed through.
    let dispatch_operations: Vec<DispatchOperation> = exposing_receiver_types
        .into_iter()
        .map(|receiver_type| DispatchOperation {
            receiver_type,
            member: member.clone(),
            parameters: call_arguments.clone(),
            result: result.clone(),
            table: 0,
        })
        .collect();

    Ok(PackageDeclarations {
        functions,
        dispatch_operations,
        dispatch_tables: vec![checked_table],
        resolved_signatures,
        ..PackageDeclarations::default()
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        ancestor_closure, effective_terms, DispatchBridgeRefusal, OperationClauses,
        MAX_ANCESTOR_DEPTH,
    };
    use crate::model::key::DeclarationKey;
    use crate::model::normalize::ModelRefusalCause;
    use qsl_forms::Expression;
    use quire_exact::ValueType;

    /// `ancestor_closure` refuses at [`MAX_ANCESTOR_DEPTH`]
    /// rather than silently truncating the closure. A straight redefinition
    /// chain one step past the bound (`op[N]` redefines `op[N-1]`, ...,
    /// `op[1]` redefines `op[0]`) must be refused
    /// `AncestorDepthExceeded`/`DispatchFamilyDepth`, not truncated to a
    /// partial, silently-wrong closure.
    #[test]
    fn ancestor_closure_refuses_past_the_depth_bound_instead_of_truncating() {
        let chain_length = MAX_ANCESTOR_DEPTH + 1;
        let keys: Vec<DeclarationKey> = (0..=chain_length)
            .map(|index| DeclarationKey::fixture(format!("model.chain.op{index}")))
            .collect();
        let redefinitions: BTreeMap<DeclarationKey, DeclarationKey> = (1..keys.len())
            .map(|index| (keys[index].clone(), keys[index - 1].clone()))
            .collect();
        let deepest = keys.last().unwrap();

        let refusal = ancestor_closure(&redefinitions, deepest)
            .expect_err("a redefinition chain past MAX_ANCESTOR_DEPTH must be refused");
        match refusal {
            DispatchBridgeRefusal::AncestorDepthExceeded(model_refusal) => {
                assert_eq!(
                    model_refusal.cause,
                    ModelRefusalCause::DispatchFamilyDepth {
                        original: deepest.clone(),
                    }
                );
            }
            other => panic!("expected AncestorDepthExceeded, got {other:?}"),
        }

        // A chain reaching exactly the bound must still succeed and return
        // the full closure — the guard (`depth > MAX_ANCESTOR_DEPTH`) fires
        // strictly past the bound, never at it.
        let shallow_chain = MAX_ANCESTOR_DEPTH;
        let shallow_keys: Vec<DeclarationKey> = (0..=shallow_chain)
            .map(|index| DeclarationKey::fixture(format!("model.shallow-chain.op{index}")))
            .collect();
        let shallow_redefinitions: BTreeMap<DeclarationKey, DeclarationKey> = (1..shallow_keys
            .len())
            .map(|index| (shallow_keys[index].clone(), shallow_keys[index - 1].clone()))
            .collect();
        let shallow_deepest = shallow_keys.last().unwrap();
        let closure = ancestor_closure(&shallow_redefinitions, shallow_deepest)
            .expect("a chain exactly at MAX_ANCESTOR_DEPTH must not be refused");
        assert_eq!(closure.len(), shallow_keys.len());
    }

    /// A family with more precondition-bearing members than
    /// [`MAX_ANCESTOR_DEPTH`] must not be refused when no single chain is
    /// deep: one root plus `MAX_ANCESTOR_DEPTH + 1` direct children, each
    /// redefining the root and each declaring its own precondition, so every
    /// child's own walk is exactly one step deep. `memo` is shared across
    /// every top-level [`effective_terms`] call in this loop, mirroring
    /// [`checked_dispatch_operation`]'s own loop, but each top-level call
    /// starts `depth` fresh at `0`: `depth` tracks only the current call's
    /// own recursion, so a far-side child is never charged for nodes a
    /// sibling already visited.
    #[test]
    fn effective_terms_does_not_refuse_a_wide_family_with_a_shallow_chain() {
        let root = DeclarationKey::fixture("model.wide.root");
        let self_parameters = vec![("self".to_owned(), ValueType::Boolean)];
        let mut clauses = OperationClauses::default();
        clauses
            .parameters
            .insert(root.clone(), self_parameters.clone());
        clauses.result.insert(root.clone(), ValueType::Boolean);
        clauses
            .own_precondition
            .insert(root.clone(), Expression::Boolean(true));

        let child_count = MAX_ANCESTOR_DEPTH + 1;
        let children: Vec<DeclarationKey> = (0..child_count)
            .map(|index| DeclarationKey::fixture(format!("model.wide.child{index}")))
            .collect();
        let mut redefinitions: BTreeMap<DeclarationKey, DeclarationKey> = BTreeMap::new();
        for child in &children {
            clauses
                .parameters
                .insert(child.clone(), self_parameters.clone());
            clauses.result.insert(child.clone(), ValueType::Boolean);
            clauses
                .own_precondition
                .insert(child.clone(), Expression::Boolean(true));
            redefinitions.insert(child.clone(), root.clone());
        }

        let mut memo = BTreeMap::new();
        for child in &children {
            let terms = effective_terms(child, &clauses, &redefinitions, &mut memo, 0)
                .unwrap_or_else(|refusal| {
                    panic!(
                        "a shallow one-hop walk must not exhaust the depth bound just \
                         because {child_count} other members share it, got {refusal:?}"
                    )
                });
            // Each child's own clause plus the root's: exactly two terms,
            // never truncated by the shared `memo`.
            assert_eq!(terms.map(|terms| terms.len()), Some(2));
        }
    }
}
