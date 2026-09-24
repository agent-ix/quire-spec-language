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
//! Pure: no intake, no I/O. It takes a caller-supplied [`EffectiveView`],
//! which carries the [`DomainPackage`] it was normalized from, and an
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
//!   exactly as `link_dispatch` accepts `view` pre-normalized.
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
use super::lowering::{AdmittedModel, ModelClause};
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
    /// The effective-precondition ancestor walk followed more `redefines`
    /// edges than the caller's `family_steps` ceiling
    /// ([`crate::model::accounting::ModelNormalizationLimits::family_steps`]),
    /// built from [`ModelRefusalCause::FamilySteps`] exactly as
    /// `crate::model::dispatch::build_family`'s own refusal is, rather than a
    /// bridge-only cause. Boxed: `ModelRefusal` is far larger than the other
    /// variants.
    FamilyStepsExceeded(Box<ModelRefusal>),
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
    /// declared operation member in the view's `domain_package.records`.
    /// `link_dispatch` reads the view's own index over the same records, so
    /// this should not arise from a table it linked; kept as a typed refusal
    /// rather than an `.expect()` so a malformed `root` supplied directly to
    /// this bridge (bypassing `link_dispatch`'s own lookups) is
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

fn family_steps_exceeded(candidate: &DeclarationKey, max_steps: u64) -> DispatchBridgeRefusal {
    DispatchBridgeRefusal::FamilyStepsExceeded(Box::new(ModelRefusal {
        code: Code::ResourceExhausted,
        cause: ModelRefusalCause::FamilySteps {
            original: candidate.clone(),
            limit: max_steps,
        },
        detail: format!(
            "effective-precondition ancestry for {} exceeded the family_steps limit of {max_steps}",
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
        .map(copy_expression)
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
/// translated from the records of `view`'s own domain package
/// ([`EffectiveView::domain_package`], QSL-217) from [`DeclarationKey`]s into
/// their [`EffectiveId`]s through `view`'s [`EffectiveView::type_identities`] --
/// the exact shape [`crate::value::declaration::ObjectTypeDeclaration::with_supertypes`]
/// needs (ADR-013 O-05). No production code builds a
/// [`crate::value::declaration::TypeEnvironment`] yet (only test scaffolding does), so
/// this is test-support infrastructure today; #131's own real intake can
/// call it exactly as tests do.
pub fn object_type_supertypes(
    view: &EffectiveView,
) -> Result<BTreeMap<EffectiveId, Vec<EffectiveId>>, DispatchBridgeRefusal> {
    let domain_package = view.domain_package();
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
/// over an explicit queue, never native recursion. `max_steps` is the
/// caller's `family_steps` ceiling, used as given: an ancestor `n`
/// `redefines` edges above `candidate` is admitted at `max_steps == n`, and
/// one edge further refuses instead of silently truncating the closure.
fn ancestor_closure(
    redefinition_parents: &BTreeMap<DeclarationKey, DeclarationKey>,
    candidate: &DeclarationKey,
    max_steps: u64,
) -> Result<BTreeSet<DeclarationKey>, DispatchBridgeRefusal> {
    let mut closure = BTreeSet::new();
    // `depth` is `current`'s own redefinition-chain distance from
    // `candidate` (0 for `candidate` itself), tracked per queue entry, not a
    // running count of every node the whole closure has visited so far: a
    // family where one member has many direct redefiners (wide, shallow
    // fan-out) must not exhaust the same bound a genuinely deep single chain
    // would.
    let mut pending: VecDeque<(DeclarationKey, u64)> = VecDeque::from([(candidate.clone(), 0)]);
    while let Some((current, depth)) = pending.pop_front() {
        if !closure.insert(current.clone()) {
            continue;
        }
        if let Some(parent) = redefinition_parents.get(&current) {
            if depth >= max_steps {
                return Err(family_steps_exceeded(candidate, max_steps));
            }
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
/// parameter. Memoized per candidate, since a diamond of redefinitions can
/// reach the same ancestor from several paths.
///
/// Iterative, never native recursion, so a caller-raised `max_steps` cannot
/// overflow the host stack: the `redefines` chain above `candidate` is first
/// collected up to the first memoized, clause-less or parentless ancestor,
/// then folded back down. `max_steps` is the caller's `family_steps`
/// ceiling, used as given and counted from *this* call's `candidate` (never
/// a counter shared across every candidate [`checked_dispatch_operation`]
/// computes this for): a chain of `n` `redefines` edges is admitted at
/// `max_steps == n`, so a wide family with only shallow chains is never
/// refused, and a memoized hit consumes none of the budget. A `redefines`
/// cycle is an unbounded chain, so it refuses with the same cause whatever
/// `max_steps` is, rather than looping.
fn effective_terms(
    candidate: &DeclarationKey,
    clauses: &OperationClauses,
    redefinition_parents: &BTreeMap<DeclarationKey, DeclarationKey>,
    memo: &mut BTreeMap<DeclarationKey, Option<Vec<Expression>>>,
    max_steps: u64,
) -> Result<Option<Vec<Expression>>, DispatchBridgeRefusal> {
    // `chain[0]` is `candidate`; `chain[i + 1]` is `chain[i]`'s redefinition
    // parent. Every element declares its own precondition and a signature.
    let mut chain: Vec<DeclarationKey> = Vec::new();
    let mut on_chain: BTreeSet<DeclarationKey> = BTreeSet::new();
    let mut current = candidate.clone();
    // `current`'s own `redefines`-edge distance from `candidate`.
    let mut depth: u64 = 0;
    // What the last chain element's parent contributes: `None` for "no
    // parent", `Some(result)` for a parent whose own effective terms are
    // `result`.
    let parent_result: Option<Option<Vec<Expression>>> = loop {
        if let Some(cached) = memo.get(&current) {
            if chain.is_empty() {
                return Ok(copy_terms(cached));
            }
            break Some(copy_terms(cached));
        }
        if depth > max_steps || !on_chain.insert(current.clone()) {
            return Err(family_steps_exceeded(candidate, max_steps));
        }
        if !clauses.own_precondition.contains_key(&current) {
            memo.insert(current.clone(), None);
            if chain.is_empty() {
                return Ok(None);
            }
            break Some(None);
        }
        require_signature(clauses, &current)?;
        chain.push(current.clone());
        let Some(parent) = redefinition_parents.get(&current) else {
            break None;
        };
        current = parent.clone();
        depth = depth.saturating_add(1);
    };

    let mut below = parent_result;
    for member in chain.iter().rev() {
        let own = require_expression(
            &clauses.own_precondition,
            member,
            MissingClauseField::OwnPrecondition,
        )?;
        let terms =
            match below {
                None => Some(vec![own]),
                Some(None) => None,
                Some(Some(parent_terms)) => {
                    let parent = redefinition_parents
                        .get(member)
                        .ok_or_else(|| missing(member, MissingClauseField::OwnPrecondition))?;
                    let (member_parameters, _) = require_signature(clauses, member)?;
                    let (parent_parameters, _) = require_signature(clauses, parent)?;
                    let mut terms = vec![own];
                    terms.extend(parent_terms.iter().map(|term| {
                        rename_parameters(term, &parent_parameters, &member_parameters)
                    }));
                    Some(terms)
                }
            };
        memo.insert(member.clone(), copy_terms(&terms));
        below = Some(terms);
    }
    // `chain` is never empty here: each exit above with an empty chain
    // returns early, so `below` holds the candidate's own terms.
    Ok(below.flatten())
}

/// `expression` with every name bound positionally in `from` rewritten to
/// its counterpart in `to`, leaving every shadowed occurrence (a `let`,
/// query or accumulate/count/sum binder reusing a `from` name) untouched.
/// Capture-avoiding: a binder whose own name coincides with a `to` name is
/// itself alpha-renamed to a fresh name before its scope is entered, so a
/// name introduced by the rename never falls under a binder that merely
/// happens to share that spelling in the source expression (`let b = 5 in
/// a = a`, renamed `a -> b`, must not become `let b = 5 in b = b`).
/// Short-circuits to a plain [`copy_expression`] when every name already
/// matches.
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
        return copy_expression(expression);
    }
    substitute_names(expression, &rename)
}

/// The spelling [`Scope::bind`] gives the `suffix`-th alpha-name of a binder
/// named `name`.
fn alpha_name(name: &str, suffix: usize) -> String {
    format!("{name}{ALPHA}{suffix}")
}

/// The separator of an alpha-name: `{name}#dispatch-alpha{suffix}`.
const ALPHA: &str = "#dispatch-alpha";

/// `(name, suffix)` when `spelling` is exactly [`alpha_name`]`(name,
/// suffix)`.
fn alpha_parts(spelling: &str) -> Option<(&str, usize)> {
    let (name, digits) = spelling.rsplit_once(ALPHA)?;
    let suffix = digits.parse().ok()?;
    (alpha_name(name, suffix) == spelling).then_some((name, suffix))
}

/// The binders in scope at one point of [`substitute_names`]'s rewrite,
/// indexed by name so that reading a name, binding and unbinding each cost
/// a map lookup however many binders enclose them.
///
/// A binder whose name is one of `to_names` -- a name the rename introduces
/// -- is alpha-renamed to the first `{name}#dispatch-alpha{k}` that neither
/// a `to_names` name nor a binder in scope uses, so a renamed name never
/// falls under it.
struct Scope<'r> {
    /// The names the rename introduces.
    to_names: &'r BTreeSet<String>,
    /// Every binder in scope, innermost last: its source name and the name
    /// the rewritten binder uses.
    binders: Vec<(String, String)>,
    /// For each source name bound in scope, the names its binders use,
    /// innermost last.
    shadows: BTreeMap<String, Vec<String>>,
    /// How many binders in scope use each name.
    in_use: BTreeMap<String, usize>,
    /// For each binder name, a suffix below which every alpha-name of that
    /// binder is occupied, so [`Scope::bind`] starts its search there.
    first_free: BTreeMap<String, usize>,
}

impl<'r> Scope<'r> {
    fn new(to_names: &'r BTreeSet<String>) -> Self {
        Self {
            to_names,
            binders: Vec::new(),
            shadows: BTreeMap::new(),
            in_use: BTreeMap::new(),
            first_free: BTreeMap::new(),
        }
    }

    /// Whether a `to_names` name or a binder in scope uses `spelling`.
    fn occupied(&self, spelling: &str) -> bool {
        self.to_names.contains(spelling) || self.in_use.contains_key(spelling)
    }

    /// `name` as read here: the innermost binder of `name` in scope
    /// shadows `rename`, and reads as the name that binder uses; a name no
    /// binder in scope binds is renamed by `rename`, or kept.
    fn resolve(&self, name: &str, rename: &BTreeMap<String, String>) -> String {
        if let Some(bound) = self.shadows.get(name).and_then(|uses| uses.last()) {
            return bound.clone();
        }
        rename.get(name).cloned().unwrap_or_else(|| name.to_owned())
    }

    /// Bind `name` for the scope about to be entered, returning the name the
    /// rewritten binder uses. [`Scope::unbind`] leaves that scope.
    fn bind(&mut self, name: &str) -> String {
        let used = if self.to_names.contains(name) {
            let mut suffix = self.first_free.get(name).copied().unwrap_or(0);
            let mut fresh = alpha_name(name, suffix);
            while self.occupied(&fresh) {
                suffix += 1;
                fresh = alpha_name(name, suffix);
            }
            self.first_free.insert(name.to_owned(), suffix + 1);
            fresh
        } else {
            name.to_owned()
        };
        self.binders.push((name.to_owned(), used.clone()));
        self.shadows
            .entry(name.to_owned())
            .or_default()
            .push(used.clone());
        *self.in_use.entry(used.clone()).or_default() += 1;
        used
    }

    /// Leave the innermost binder's scope. A name it frees that is some
    /// binder's alpha-name lowers that binder's [`Scope::first_free`].
    fn unbind(&mut self) {
        let popped = self.binders.pop();
        debug_assert!(popped.is_some(), "every Unbind follows its own Bind");
        let Some((name, used)) = popped else {
            return;
        };
        if let Some(uses) = self.shadows.get_mut(&name) {
            uses.pop();
            if uses.is_empty() {
                self.shadows.remove(&name);
            }
        }
        match self.in_use.get_mut(&used) {
            Some(count) if *count > 1 => *count -= 1,
            _ => {
                self.in_use.remove(&used);
                if !self.to_names.contains(&used) {
                    if let Some((binder, suffix)) = alpha_parts(&used) {
                        if let Some(first) = self.first_free.get_mut(binder) {
                            *first = (*first).min(suffix);
                        }
                    }
                }
            }
        }
    }
}

/// One step of [`substitute_names`]'s explicit stack.
enum Rewrite<'s, 't> {
    /// Rewrite `source` into `target`, a placeholder in the rewritten tree.
    Node(&'s Expression, &'t mut Expression),
    /// Bind the binder named `source` for the scope about to be entered
    /// ([`Scope::bind`]), writing the name the rewritten binder uses into
    /// `target`.
    Bind(&'s str, &'t mut String),
    /// Leave the innermost binder's scope.
    Unbind,
}

/// The placeholder an operand holds until the rewrite fills it.
fn hole() -> Box<Expression> {
    Box::new(Expression::Boolean(false))
}

/// `count` placeholders for a list of operands.
fn holes(count: usize) -> Vec<Expression> {
    vec![Expression::Boolean(false); count]
}

/// A source operand and the placeholder its rewrite fills.
type Operand<'s, 't> = (&'s Expression, &'t mut Box<Expression>);

/// Push the rewrite of each of `operands`, to run in order.
fn operands<'s, 't>(
    pending: &mut Vec<Rewrite<'s, 't>>,
    operands: impl DoubleEndedIterator<Item = (&'s Expression, &'t mut Expression)>,
) {
    pending.extend(
        operands
            .rev()
            .map(|(source, target)| Rewrite::Node(source, target)),
    );
}

/// Push a binder form's rewrite, to run in order: `outer` outside its
/// binders, then each of `binders` bound, then `body` under them, then each
/// binder unbound.
fn scoped<'s, 't>(
    pending: &mut Vec<Rewrite<'s, 't>>,
    outer: Vec<Operand<'s, 't>>,
    binders: Vec<(&'s str, &'t mut String)>,
    body: Operand<'s, 't>,
) {
    pending.extend(binders.iter().map(|_| Rewrite::Unbind));
    pending.push(Rewrite::Node(body.0, body.1));
    pending.extend(
        binders
            .into_iter()
            .rev()
            .map(|(source, target)| Rewrite::Bind(source, target)),
    );
    operands(
        pending,
        outer
            .into_iter()
            .map(|(source, target)| (source, target.as_mut())),
    );
}

/// Write `shell` into `target`, then bind `pattern` over the fields just
/// written. `pattern` is the shell's own variant, so it always matches.
macro_rules! write_shell {
    ($target:ident, $shell:expr, $pattern:pat) => {
        *$target = $shell;
        let $pattern = $target else {
            unreachable!("the pattern names the shell just written")
        };
    };
}

/// Rewrite `source` into `target`: a literal is copied and a name resolved
/// in `scope`; any other form is written as its shell over placeholder
/// operands and binder names, and the rewrite of each operand into its own
/// placeholder is pushed onto `pending`. One exhaustive match pairs every
/// form's operands with its placeholders, so a new form does not compile
/// until it is rewritten here.
#[deny(clippy::wildcard_enum_match_arm)]
fn open<'s, 't>(
    source: &'s Expression,
    target: &'t mut Expression,
    rename: &BTreeMap<String, String>,
    scope: &Scope<'_>,
    pending: &mut Vec<Rewrite<'s, 't>>,
) {
    match source {
        Expression::Boolean(_) | Expression::Integer(_) | Expression::Rational(..) => {
            *target = source.clone();
        }
        Expression::Name(name) => *target = Expression::Name(scope.resolve(name, rename)),
        Expression::Let { name, value, body } => {
            write_shell!(
                target,
                Expression::Let {
                    name: String::new(),
                    value: hole(),
                    body: hole(),
                },
                Expression::Let {
                    name: bound,
                    value: value_hole,
                    body: body_hole,
                }
            );
            scoped(
                pending,
                vec![(value, value_hole)],
                vec![(name, bound)],
                (body, body_hole),
            );
        }
        Expression::If {
            condition,
            then,
            otherwise,
        } => {
            write_shell!(
                target,
                Expression::If {
                    condition: hole(),
                    then: hole(),
                    otherwise: hole(),
                },
                Expression::If {
                    condition: condition_hole,
                    then: then_hole,
                    otherwise: otherwise_hole,
                }
            );
            operands(
                pending,
                [
                    (&**condition, &mut **condition_hole),
                    (then, then_hole),
                    (otherwise, otherwise_hole),
                ]
                .into_iter(),
            );
        }
        Expression::Binary {
            operator,
            left,
            right,
        } => {
            write_shell!(
                target,
                Expression::Binary {
                    operator: *operator,
                    left: hole(),
                    right: hole(),
                },
                Expression::Binary {
                    left: left_hole,
                    right: right_hole,
                    ..
                }
            );
            operands(
                pending,
                [(&**left, &mut **left_hole), (right, right_hole)].into_iter(),
            );
        }
        Expression::Negate(operand) => {
            write_shell!(target, Expression::Negate(hole()), Expression::Negate(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Not(operand) => {
            write_shell!(target, Expression::Not(hole()), Expression::Not(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Field { operand, field } => {
            write_shell!(
                target,
                Expression::Field {
                    operand: hole(),
                    field: field.clone(),
                },
                Expression::Field { operand: hole, .. }
            );
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Present(operand) => {
            write_shell!(
                target,
                Expression::Present(hole()),
                Expression::Present(hole)
            );
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Value(operand) => {
            write_shell!(target, Expression::Value(hole()), Expression::Value(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Deref(operand) => {
            write_shell!(target, Expression::Deref(hole()), Expression::Deref(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Call { name, arguments } => {
            write_shell!(
                target,
                Expression::Call {
                    name: name.clone(),
                    arguments: holes(arguments.len()),
                },
                Expression::Call {
                    arguments: argument_holes,
                    ..
                }
            );
            operands(pending, arguments.iter().zip(argument_holes.iter_mut()));
        }
        Expression::Record { name, fields } => {
            write_shell!(
                target,
                Expression::Record {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|(field, initializer)| {
                            let initializer = match initializer {
                                FieldInitializer::Value(_) => {
                                    FieldInitializer::Value(Expression::Boolean(false))
                                }
                                FieldInitializer::Null => FieldInitializer::Null,
                            };
                            (field.clone(), initializer)
                        })
                        .collect(),
                },
                Expression::Record {
                    fields: field_holes,
                    ..
                }
            );
            let pairs: Vec<_> = fields
                .iter()
                .zip(field_holes.iter_mut())
                .filter_map(|((_, initializer), (_, hole))| match (initializer, hole) {
                    (FieldInitializer::Value(value), FieldInitializer::Value(hole)) => {
                        Some((value, hole))
                    }
                    _ => None,
                })
                .collect();
            operands(pending, pairs.into_iter());
        }
        Expression::Collection { kind, elements } => {
            write_shell!(
                target,
                Expression::Collection {
                    kind: *kind,
                    elements: holes(elements.len()),
                },
                Expression::Collection {
                    elements: element_holes,
                    ..
                }
            );
            operands(pending, elements.iter().zip(element_holes.iter_mut()));
        }
        Expression::Convert {
            target: converted,
            operand,
        } => {
            write_shell!(
                target,
                Expression::Convert {
                    target: converted.clone(),
                    operand: hole(),
                },
                Expression::Convert { operand: hole, .. }
            );
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Query {
            query,
            binder,
            source,
            body,
        } => {
            write_shell!(
                target,
                Expression::Query {
                    query: *query,
                    binder: String::new(),
                    source: hole(),
                    body: hole(),
                },
                Expression::Query {
                    binder: bound,
                    source: source_hole,
                    body: body_hole,
                    ..
                }
            );
            scoped(
                pending,
                vec![(source, source_hole)],
                vec![(binder, bound)],
                (body, body_hole),
            );
        }
        Expression::Flatten(operand) => {
            write_shell!(
                target,
                Expression::Flatten(hole()),
                Expression::Flatten(hole)
            );
            pending.push(Rewrite::Node(operand, hole));
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
            write_shell!(
                target,
                Expression::Accumulate {
                    form: *form,
                    accumulator_type: accumulator_type.clone(),
                    accumulator: String::new(),
                    binder: String::new(),
                    source: hole(),
                    step: hole(),
                    identity: identity.as_ref().map(|_| hole()),
                },
                Expression::Accumulate {
                    accumulator: accumulator_bound,
                    binder: binder_bound,
                    source: source_hole,
                    step: step_hole,
                    identity: identity_hole,
                    ..
                }
            );
            let mut outer = vec![(&**source, source_hole)];
            if let (Some(identity), Some(identity_hole)) = (identity, identity_hole) {
                outer.push((identity, identity_hole));
            }
            scoped(
                pending,
                outer,
                vec![(accumulator, accumulator_bound), (binder, binder_bound)],
                (step, step_hole),
            );
        }
        Expression::Count {
            result_type,
            binder,
            source,
            predicate,
        } => {
            write_shell!(
                target,
                Expression::Count {
                    result_type: result_type.clone(),
                    binder: String::new(),
                    source: hole(),
                    predicate: hole(),
                },
                Expression::Count {
                    binder: bound,
                    source: source_hole,
                    predicate: predicate_hole,
                    ..
                }
            );
            scoped(
                pending,
                vec![(source, source_hole)],
                vec![(binder, bound)],
                (predicate, predicate_hole),
            );
        }
        Expression::Sum {
            result_type,
            binder,
            source,
            summand,
        } => {
            write_shell!(
                target,
                Expression::Sum {
                    result_type: result_type.clone(),
                    binder: String::new(),
                    source: hole(),
                    summand: hole(),
                },
                Expression::Sum {
                    binder: bound,
                    source: source_hole,
                    summand: summand_hole,
                    ..
                }
            );
            scoped(
                pending,
                vec![(source, source_hole)],
                vec![(binder, bound)],
                (summand, summand_hole),
            );
        }
        Expression::Size(operand) => {
            write_shell!(target, Expression::Size(hole()), Expression::Size(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
        Expression::Contains { collection, item } => {
            write_shell!(
                target,
                Expression::Contains {
                    collection: hole(),
                    item: hole(),
                },
                Expression::Contains {
                    collection: collection_hole,
                    item: item_hole,
                }
            );
            operands(
                pending,
                [(&**collection, &mut **collection_hole), (item, item_hole)].into_iter(),
            );
        }
        Expression::AllInstances {
            target: queried,
            population,
        } => {
            write_shell!(
                target,
                Expression::AllInstances {
                    target: queried.clone(),
                    population: hole(),
                },
                Expression::AllInstances {
                    population: hole,
                    ..
                }
            );
            pending.push(Rewrite::Node(population, hole));
        }
        Expression::Lookup {
            target: queried,
            population,
            reference,
            absence,
        } => {
            write_shell!(
                target,
                Expression::Lookup {
                    target: queried.clone(),
                    population: hole(),
                    reference: hole(),
                    absence: *absence,
                },
                Expression::Lookup {
                    population: population_hole,
                    reference: reference_hole,
                    ..
                }
            );
            operands(
                pending,
                [
                    (&**population, &mut **population_hole),
                    (reference, reference_hole),
                ]
                .into_iter(),
            );
        }
        Expression::Dispatch {
            receiver,
            member,
            arguments,
        } => {
            write_shell!(
                target,
                Expression::Dispatch {
                    receiver: hole(),
                    member: member.clone(),
                    arguments: holes(arguments.len()),
                },
                Expression::Dispatch {
                    receiver: receiver_hole,
                    arguments: argument_holes,
                    ..
                }
            );
            operands(
                pending,
                std::iter::once((&**receiver, &mut **receiver_hole))
                    .chain(arguments.iter().zip(argument_holes.iter_mut())),
            );
        }
        Expression::Pre(operand) => {
            write_shell!(target, Expression::Pre(hole()), Expression::Pre(hole));
            pending.push(Rewrite::Node(operand, hole));
        }
    }
}

/// The rewrite [`rename_parameters`] applies: every [`Expression::Name`] is
/// read in the scope of the binders around it ([`Scope::resolve`]), and
/// every binder is bound with [`Scope::bind`] before its scope is entered
/// and unbound after it.
///
/// Runs over an explicit heap stack, never native recursion: a clause
/// nested past the check stage's depth limit is rewritten in constant host
/// stack and left for the check stage to refuse on that limit. Operands are
/// rewritten in source order, and a binder form's operands before its body
/// are read outside its binders.
fn substitute_names(expression: &Expression, rename: &BTreeMap<String, String>) -> Expression {
    let to_names: BTreeSet<String> = rename.values().cloned().collect();
    let mut scope = Scope::new(&to_names);
    let mut rewritten = Expression::Boolean(false);
    let mut pending = vec![Rewrite::Node(expression, &mut rewritten)];
    while let Some(step) = pending.pop() {
        match step {
            Rewrite::Node(source, target) => {
                open(source, target, rename, &scope, &mut pending);
            }
            Rewrite::Bind(name, target) => *target = scope.bind(name),
            Rewrite::Unbind => scope.unbind(),
        }
    }
    rewritten
}

/// A copy of `expression`, built by [`substitute_names`]'s explicit stack
/// rather than the derived `Clone`, which recurses once per nesting level:
/// with nothing to rename, every name reads as itself and no binder is
/// alpha-renamed.
fn copy_expression(expression: &Expression) -> Expression {
    substitute_names(expression, &BTreeMap::new())
}

/// [`copy_expression`] over a memoized effective-precondition result.
fn copy_terms(terms: &Option<Vec<Expression>>) -> Option<Vec<Expression>> {
    terms
        .as_ref()
        .map(|terms| terms.iter().map(copy_expression).collect())
}

/// Types every linked candidate's effective precondition and body for one
/// FR-151 dispatch-eligible operation, and assembles the checked-layer
/// [`DispatchTable`] the evaluator needs. See the module docs for this
/// bridge's exact scope. `source` is the checked package's
/// [`PackageDeclarations::source`], the source unit's `RawSourceRef`. Each synthesized
/// clause function is keyed with its own `ModelOwner` instead (FR-094): the
/// authoring operation member for an authored precondition, the candidate
/// for an effective precondition and a body. The returned package admits
/// `view`'s own domain package with `view` ([`AdmittedModel::from_view`]),
/// whose selection those owners name. The package is read from the view
/// (QSL-217), so it always corresponds to the view.
pub fn checked_dispatch_operation(
    view: &EffectiveView,
    root: &DispatchRoot,
    clauses: &OperationClauses,
    source: qsl_foundation::source::provenance::RawSourceRef,
    meter: &mut Meter,
) -> Result<PackageDeclarations, DispatchBridgeRefusal> {
    let family_steps = meter.limits().family_steps;
    let domain_package = view.domain_package();
    let model = AdmittedModel::from_view(view);
    let outcome = link_dispatch(view, &root.key, root.closure, meter);
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
        let closure = ancestor_closure(&redefinitions, candidate, family_steps)?;
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
    // FR-094: each synthesized clause function's owner and clause kind.
    let mut model_clauses: BTreeMap<usize, ModelClause> = BTreeMap::new();
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
        model_clauses.insert(
            index,
            ModelClause {
                declaration: member.clone(),
                kind: DeclaredClauseKind::Precondition,
            },
        );
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

        let terms = effective_terms(candidate, clauses, &redefinitions, &mut memo, family_steps)?;
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
                model_clauses.insert(
                    index,
                    ModelClause {
                        declaration: candidate.clone(),
                        kind: DeclaredClauseKind::Precondition,
                    },
                );
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
        model_clauses.insert(
            index,
            ModelClause {
                declaration: candidate.clone(),
                kind: DeclaredClauseKind::Body,
            },
        );
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
    // them, since conformance is `true` for `s == t` -- see
    // `ModelIndex::conforms`). Feeds every exposing static type's own
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
        models: vec![model],
        model_clauses,
        ..PackageDeclarations::new(source)
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;

    use super::{
        ancestor_closure, copy_expression, effective_terms, rename_parameters,
        DispatchBridgeRefusal, OperationClauses,
    };
    use crate::model::key::DeclarationKey;
    use crate::model::normalize::ModelRefusalCause;
    use qsl_forms::{
        Accumulation, BinaryOperator, BinderQuery, Expression, FieldInitializer, TypeForm,
    };
    use qsl_foundation::absence::AbsenceMode;
    use quire_exact::{CollectionKind, Integer, ValueType};

    /// A caller-configured `family_steps` ceiling small enough to build a
    /// chain one past it cheaply.
    const FAMILY_STEPS: u64 = 8;

    /// `op[0]` .. `op[edges]`, where `op[i]` redefines `op[i - 1]`: a chain
    /// of exactly `edges` `redefines` edges. Returns the keys and the
    /// child-to-parent map.
    fn chain(
        prefix: &str,
        edges: u64,
    ) -> (
        Vec<DeclarationKey>,
        BTreeMap<DeclarationKey, DeclarationKey>,
    ) {
        let keys: Vec<DeclarationKey> = (0..=edges)
            .map(|index| DeclarationKey::fixture(format!("model.{prefix}.op{index}")))
            .collect();
        let redefinitions = keys
            .windows(2)
            .map(|pair| (pair[1].clone(), pair[0].clone()))
            .collect();
        (keys, redefinitions)
    }

    /// Every key in `keys` declares a `self`-only signature and its own
    /// `true` precondition.
    fn clauses_for(keys: &[DeclarationKey]) -> OperationClauses {
        let self_parameters = vec![("self".to_owned(), ValueType::Boolean)];
        let mut clauses = OperationClauses::default();
        for key in keys {
            clauses
                .parameters
                .insert(key.clone(), self_parameters.clone());
            clauses.result.insert(key.clone(), ValueType::Boolean);
            clauses
                .own_precondition
                .insert(key.clone(), Expression::Boolean(true));
        }
        clauses
    }

    fn assert_family_steps_refusal(refusal: DispatchBridgeRefusal, original: &DeclarationKey) {
        match refusal {
            DispatchBridgeRefusal::FamilyStepsExceeded(model_refusal) => assert_eq!(
                model_refusal.cause,
                ModelRefusalCause::FamilySteps {
                    original: original.clone(),
                    limit: FAMILY_STEPS,
                }
            ),
            other => panic!("expected FamilyStepsExceeded, got {other:?}"),
        }
    }

    /// `ancestor_closure` admits a chain of exactly `family_steps` edges with
    /// its full closure, and refuses one edge more instead of truncating.
    #[test]
    fn ancestor_closure_admits_the_bound_and_refuses_one_past_it() {
        let (keys, redefinitions) = chain("at", FAMILY_STEPS);
        let deepest = keys.last().unwrap();
        let closure = ancestor_closure(&redefinitions, deepest, FAMILY_STEPS)
            .expect("a chain exactly at family_steps must not be refused");
        assert_eq!(closure.len(), keys.len());

        let (keys, redefinitions) = chain("past", FAMILY_STEPS + 1);
        let deepest = keys.last().unwrap();
        let refusal = ancestor_closure(&redefinitions, deepest, FAMILY_STEPS)
            .expect_err("a chain one edge past family_steps must be refused");
        assert_family_steps_refusal(refusal, deepest);
    }

    /// `effective_terms` admits a chain of exactly `family_steps` edges with
    /// every ancestor's term, and refuses one edge more.
    #[test]
    fn effective_terms_admits_the_bound_and_refuses_one_past_it() {
        let (keys, redefinitions) = chain("at", FAMILY_STEPS);
        let deepest = keys.last().unwrap();
        let terms = effective_terms(
            deepest,
            &clauses_for(&keys),
            &redefinitions,
            &mut BTreeMap::new(),
            FAMILY_STEPS,
        )
        .expect("a chain exactly at family_steps must not be refused");
        assert_eq!(terms.map(|terms| terms.len()), Some(keys.len()));

        let (keys, redefinitions) = chain("past", FAMILY_STEPS + 1);
        let deepest = keys.last().unwrap();
        let refusal = effective_terms(
            deepest,
            &clauses_for(&keys),
            &redefinitions,
            &mut BTreeMap::new(),
            FAMILY_STEPS,
        )
        .expect_err("a chain one edge past family_steps must be refused");
        assert_family_steps_refusal(refusal, deepest);
    }

    /// A caller-raised `family_steps` over a long chain completes on a small
    /// thread stack: the walk is iterative, so raising the ceiling can never
    /// turn into a stack overflow.
    #[test]
    fn effective_terms_walks_a_long_chain_without_native_recursion() {
        const EDGES: u64 = 2_000;
        let outcome = std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let (keys, redefinitions) = chain("long", EDGES);
                effective_terms(
                    keys.last().unwrap(),
                    &clauses_for(&keys),
                    &redefinitions,
                    &mut BTreeMap::new(),
                    u64::MAX,
                )
                .map(|terms| terms.map(|terms| terms.len()))
            })
            .unwrap()
            .join()
            .expect("the walk must not overflow a 128 KiB stack");
        assert_eq!(outcome.unwrap(), Some(usize::try_from(EDGES).unwrap() + 1));
    }

    /// A `redefines` cycle is an unbounded chain: it refuses even under an
    /// unlimited `family_steps` rather than looping.
    #[test]
    fn effective_terms_refuses_a_redefinition_cycle_under_an_unlimited_ceiling() {
        let a = DeclarationKey::fixture("model.cycle.a");
        let b = DeclarationKey::fixture("model.cycle.b");
        let redefinitions = BTreeMap::from([(a.clone(), b.clone()), (b.clone(), a.clone())]);
        let refusal = effective_terms(
            &a,
            &clauses_for(&[a.clone(), b]),
            &redefinitions,
            &mut BTreeMap::new(),
            u64::MAX,
        )
        .expect_err("a cyclic redefinition chain must be refused");
        assert!(matches!(
            refusal,
            DispatchBridgeRefusal::FamilyStepsExceeded(model_refusal)
                if matches!(model_refusal.cause, ModelRefusalCause::FamilySteps { .. })
        ));
    }

    /// A family with more precondition-bearing members than `family_steps`
    /// is not refused when no single chain is deep: one root plus
    /// `family_steps + 1` direct children, each one edge deep. `memo` is
    /// shared across every top-level [`effective_terms`] call, mirroring
    /// [`super::checked_dispatch_operation`]'s own loop, but each call counts
    /// only its own chain.
    #[test]
    fn effective_terms_does_not_refuse_a_wide_family_with_a_shallow_chain() {
        let root = DeclarationKey::fixture("model.wide.root");
        let children: Vec<DeclarationKey> = (0..=FAMILY_STEPS)
            .map(|index| DeclarationKey::fixture(format!("model.wide.child{index}")))
            .collect();
        let mut members = children.clone();
        members.push(root.clone());
        let clauses = clauses_for(&members);
        let redefinitions: BTreeMap<DeclarationKey, DeclarationKey> = children
            .iter()
            .map(|child| (child.clone(), root.clone()))
            .collect();

        let mut memo = BTreeMap::new();
        for child in &children {
            let terms = effective_terms(child, &clauses, &redefinitions, &mut memo, FAMILY_STEPS)
                .unwrap_or_else(|refusal| {
                    panic!("a one-edge walk must not exhaust family_steps, got {refusal:?}")
                });
            // Each child's own clause plus the root's: exactly two terms,
            // never truncated by the shared `memo`.
            assert_eq!(terms.map(|terms| terms.len()), Some(2));
        }
    }

    /// `rename_parameters` renames `x` to `y` in `let y = x in x + y`:
    /// the `let` value reads outside the binder, and the binder, which a
    /// renamed `x` would otherwise fall under, is alpha-renamed in its body.
    /// A fold binds its accumulator before its element binder; its source
    /// and identity read outside both, each in its own place.
    #[trace("FR-151-AC-7")]
    #[test]
    fn rename_parameters_reads_each_operand_in_its_binders_scope() {
        let name = |name: &str| Expression::Name(name.to_owned());
        let parameters = |name: &str| vec![(name.to_owned(), ValueType::Boolean)];
        let add = |left, right| Expression::Binary {
            operator: BinaryOperator::Add,
            left: Box::new(left),
            right: Box::new(right),
        };
        let binding = Expression::Let {
            name: "y".to_owned(),
            value: Box::new(name("x")),
            body: Box::new(add(name("x"), name("y"))),
        };
        let fold = Expression::Accumulate {
            form: Accumulation::Fold,
            accumulator_type: "A".to_owned(),
            accumulator: "y".to_owned(),
            binder: "x".to_owned(),
            source: Box::new(name("x")),
            step: Box::new(add(name("x"), name("y"))),
            identity: Some(Box::new(name("z"))),
        };
        let renamed = rename_parameters(
            &Expression::Collection {
                kind: CollectionKind::Sequence,
                elements: vec![binding, fold, name("x")],
            },
            &parameters("x"),
            &parameters("y"),
        );
        let Expression::Collection { elements, .. } = &renamed else {
            panic!("a collection renames to a collection: {renamed:?}");
        };
        let rendered: Vec<String> = elements.iter().map(|e| format!("{e:?}")).collect();
        let alpha = "y#dispatch-alpha0";
        let expected = [
            Expression::Let {
                name: alpha.to_owned(),
                value: Box::new(name("y")),
                body: Box::new(add(name("y"), name(alpha))),
            },
            Expression::Accumulate {
                form: Accumulation::Fold,
                accumulator_type: "A".to_owned(),
                accumulator: alpha.to_owned(),
                binder: "x".to_owned(),
                source: Box::new(name("y")),
                step: Box::new(add(name("x"), name(alpha))),
                identity: Some(Box::new(name("z"))),
            },
            name("y"),
        ]
        .map(|e| format!("{e:?}"));
        assert_eq!(rendered, expected);
    }

    /// The renaming and copying of an inherited precondition nested 10,000
    /// levels -- far past the check stage's depth limit, which refuses it
    /// afterwards -- completes on a 2 MiB thread and renames every level.
    #[trace("FR-093-AC-14", "TC-415")]
    #[test]
    fn rename_and_copy_walk_a_deep_clause_on_a_small_stack() {
        const LEVELS: usize = 10_000;
        let renamed = std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                let mut clause = Expression::Name("x".to_owned());
                for _ in 0..LEVELS {
                    clause = Expression::Binary {
                        operator: BinaryOperator::And,
                        left: Box::new(Expression::Name("x".to_owned())),
                        right: Box::new(clause),
                    };
                }
                let copied = copy_expression(&clause);
                let renamed = rename_parameters(
                    &copied,
                    &[("x".to_owned(), ValueType::Boolean)],
                    &[("y".to_owned(), ValueType::Boolean)],
                );
                let mut names = Vec::new();
                let mut spine = &renamed;
                while let Expression::Binary { left, right, .. } = spine {
                    names.push(format!("{left:?}"));
                    spine = right;
                }
                names.push(format!("{spine:?}"));
                names
            })
            .expect("the rename thread spawns")
            .join()
            .expect("the rename completes on a 2 MiB stack");
        assert_eq!(renamed.len(), LEVELS + 1);
        assert!(renamed.iter().all(|name| name == r#"Name("y")"#));
    }

    /// One expression of every form, each operand a distinct name `o0`,
    /// `o1`, … and each binder a distinct name `b0`, `b1`, …, so an operand
    /// or binder rewritten into another's place changes the result.
    fn one_of_each_form() -> Vec<Expression> {
        let mut next = 0;
        let mut fresh = |prefix: &str| {
            next += 1;
            format!("{prefix}{next}")
        };
        let mut operand = || Box::new(Expression::Name(fresh("o")));
        let mut o = || *operand();
        let form = || TypeForm::name("T", qsl_foundation::Span { start: 0, end: 0 });
        let mut forms = vec![
            Expression::Boolean(true),
            Expression::Integer(Integer::from(3_i64)),
            Expression::Rational(Integer::from(1_i64), Integer::from(3_i64)),
            o(),
        ];
        let mut boxed = || Box::new(o());
        let mut binder = 0;
        let mut bound = || {
            binder += 1;
            format!("b{binder}")
        };
        forms.extend([
            Expression::Let {
                name: bound(),
                value: boxed(),
                body: boxed(),
            },
            Expression::If {
                condition: boxed(),
                then: boxed(),
                otherwise: boxed(),
            },
            Expression::Binary {
                operator: BinaryOperator::Add,
                left: boxed(),
                right: boxed(),
            },
            Expression::Negate(boxed()),
            Expression::Not(boxed()),
            Expression::Field {
                operand: boxed(),
                field: "f".to_owned(),
            },
            Expression::Present(boxed()),
            Expression::Value(boxed()),
            Expression::Deref(boxed()),
            Expression::Call {
                name: "g".to_owned(),
                arguments: vec![*boxed(), *boxed(), *boxed()],
            },
            Expression::Record {
                name: "R".to_owned(),
                fields: vec![
                    ("f".to_owned(), FieldInitializer::Value(*boxed())),
                    ("g".to_owned(), FieldInitializer::Null),
                    ("h".to_owned(), FieldInitializer::Value(*boxed())),
                ],
            },
            Expression::Collection {
                kind: CollectionKind::Sequence,
                elements: vec![*boxed(), *boxed(), *boxed()],
            },
            Expression::Convert {
                target: form(),
                operand: boxed(),
            },
            Expression::Query {
                query: BinderQuery::Map,
                binder: bound(),
                source: boxed(),
                body: boxed(),
            },
            Expression::Flatten(boxed()),
            Expression::Accumulate {
                form: Accumulation::Fold,
                accumulator_type: "A".to_owned(),
                accumulator: bound(),
                binder: bound(),
                source: boxed(),
                step: boxed(),
                identity: Some(boxed()),
            },
            Expression::Accumulate {
                form: Accumulation::Reduce,
                accumulator_type: "A".to_owned(),
                accumulator: bound(),
                binder: bound(),
                source: boxed(),
                step: boxed(),
                identity: None,
            },
            Expression::Count {
                result_type: "N".to_owned(),
                binder: bound(),
                source: boxed(),
                predicate: boxed(),
            },
            Expression::Sum {
                result_type: "N".to_owned(),
                binder: bound(),
                source: boxed(),
                summand: boxed(),
            },
            Expression::Size(boxed()),
            Expression::Contains {
                collection: boxed(),
                item: boxed(),
            },
            Expression::AllInstances {
                target: form(),
                population: boxed(),
            },
            Expression::Lookup {
                target: form(),
                population: boxed(),
                reference: boxed(),
                absence: AbsenceMode::Empty,
            },
            Expression::Dispatch {
                receiver: boxed(),
                member: "m".to_owned(),
                arguments: vec![*boxed(), *boxed()],
            },
            Expression::Pre(boxed()),
        ]);
        forms
    }

    /// Every name [`form_name`] gives, one per `Expression` form.
    const FORM_NAMES: [&str; 28] = [
        "Boolean",
        "Integer",
        "Rational",
        "Name",
        "Let",
        "If",
        "Binary",
        "Negate",
        "Not",
        "Field",
        "Present",
        "Value",
        "Deref",
        "Call",
        "Record",
        "Collection",
        "Convert",
        "Query",
        "Flatten",
        "Accumulate",
        "Count",
        "Sum",
        "Size",
        "Contains",
        "AllInstances",
        "Lookup",
        "Dispatch",
        "Pre",
    ];

    /// `expression`'s form, as named in [`FORM_NAMES`]. The match is
    /// exhaustive, so a new form does not compile until it is named here;
    /// its name then belongs in [`FORM_NAMES`] beside this match.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn form_name(expression: &Expression) -> &'static str {
        match expression {
            Expression::Boolean(_) => "Boolean",
            Expression::Integer(_) => "Integer",
            Expression::Rational(..) => "Rational",
            Expression::Name(_) => "Name",
            Expression::Let { .. } => "Let",
            Expression::If { .. } => "If",
            Expression::Binary { .. } => "Binary",
            Expression::Negate(_) => "Negate",
            Expression::Not(_) => "Not",
            Expression::Field { .. } => "Field",
            Expression::Present(_) => "Present",
            Expression::Value(_) => "Value",
            Expression::Deref(_) => "Deref",
            Expression::Call { .. } => "Call",
            Expression::Record { .. } => "Record",
            Expression::Collection { .. } => "Collection",
            Expression::Convert { .. } => "Convert",
            Expression::Query { .. } => "Query",
            Expression::Flatten(_) => "Flatten",
            Expression::Accumulate { .. } => "Accumulate",
            Expression::Count { .. } => "Count",
            Expression::Sum { .. } => "Sum",
            Expression::Size(_) => "Size",
            Expression::Contains { .. } => "Contains",
            Expression::AllInstances { .. } => "AllInstances",
            Expression::Lookup { .. } => "Lookup",
            Expression::Dispatch { .. } => "Dispatch",
            Expression::Pre(_) => "Pre",
        }
    }

    /// `copy_expression` rewrites every form into exactly what the derived
    /// `Clone` gives: each operand, binder and attribute in its own place.
    /// The forms [`one_of_each_form`] builds are exactly [`FORM_NAMES`], so
    /// a name added there without an expression here fails the test.
    #[trace("FR-151-AC-7")]
    #[test]
    fn copy_expression_matches_clone_on_every_form() {
        let forms = one_of_each_form();
        let covered: std::collections::BTreeSet<&str> = forms.iter().map(form_name).collect();
        let named: std::collections::BTreeSet<&str> = FORM_NAMES.into_iter().collect();
        assert_eq!(covered, named, "one expression per Expression form");
        for expression in &forms {
            assert_eq!(
                format!("{:?}", copy_expression(expression)),
                format!("{:?}", expression.clone())
            );
        }
        let nested = Expression::Collection {
            kind: CollectionKind::Set,
            elements: forms,
        };
        assert_eq!(
            format!("{:?}", copy_expression(&nested)),
            format!("{nested:?}")
        );
    }
}
