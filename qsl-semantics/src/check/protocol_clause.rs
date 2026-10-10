// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-113: the `ProtocolClause` family's S3 scope check over one protocol's
//! `ScopedAnchorForm`s (ADR-012 §12.2 Check row).
//!
//! Each anchor's first segment resolves in the innermost scope of its
//! `scope`, then in each enclosing scope outward, then at the top level; a
//! later segment resolves among the names its previous target declares
//! directly. A scope that declares two static nodes of one name refuses
//! `ambiguous_declaration`/`ambiguous-name` at each declaration, and an
//! anchor whose resolution decides on that name refuses the same way at the
//! anchor. A resolved anchor is checked against the admitted target kinds
//! of its site (FR-113 "Behavior" table); a mismatch refuses
//! `ill_typed`/`type-mismatch`, naming the site, the admitted kinds and the
//! actual kind. A `receive` whose channel disagrees with the `send` its
//! `receive-of` anchor names refuses the same way, naming both channels.
//!
//! FR-113's binder no-shadowing rule (a record binder, a capture, a
//! compensation trigger or a retry or recovery parameter shadowing a
//! model/profile alias, a native declaration of the package or another
//! visible binder) is checked here too, over the `BinderForm`s
//! the S2 extension (`qsl_forms::protocol_clause::BinderForm`)
//! now builds for every binder position. QSpec `shared-grammar.md` makes
//! every binder "unique in their enclosing declaration": that declaration
//! is the checked protocol as a whole, so every binder name in it shares
//! one flat namespace, independent of the lexical scope it is written in
//! (SR-765 FND-001) -- a `finish` binder can shadow a `run`-tree binder,
//! two different `compensate` declarations' binders of one name collide,
//! and so do sibling `case`/`branch` binders of one name, matching the
//! composed lane's own `DuplicateBinder` checker
//! (`src/linking/composed/scopes.rs`). Only a *different* protocol
//! declaration is a separate enclosing declaration ("Two separate
//! protocols may reuse a binder name"), which this function's own
//! per-protocol call already gives for free. Binders are still checked
//! against a package-level alias or native declaration the same way (by
//! name alone, package-wide). Only one refusal is raised, at the shadowing
//! binder, naming what it shadows -- unlike a duplicate declaration,
//! shadowing is directional, so the shadowed declaration is not itself
//! refused.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal::{
    AliasKind, CheckCause, CheckRefusal, ProtocolAnchorCause, ShadowedDeclaration,
};
use super::state_clause::{
    AttemptDeclaration, ClauseOperation, PopulationDomain, StateClauseDeclaration,
};
use super::{Scope, Signatures};
use quire_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};
use quire_semantic_value::location::Origin;

/// The identity of one static protocol node: its index into the
/// declaration's own [`ProtocolDeclarationForm::declarations`] (FR-113
/// Outputs: "recorded on the checked protocol by that node's identity"). No
/// later stage recovers a target from a name.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProtocolNodeId(pub usize);

/// One protocol `attempt` FR-114's own S3 binding checked: its resolved
/// operation (already resolved by the FR-091 assembler, `AttemptDeclaration`
/// -- this only checks its `contracts` list against it) and each `contracts`
/// entry's own index into the package's `state_clauses`, in list order.
/// Everything S4 ([`super::lowering::Lowering::protocol_attempt`]) needs to
/// mint the operation's anchor and frame node identity, and to name each
/// contract's own checked identity.
#[derive(Clone, Debug)]
pub struct BoundAttempt {
    /// The index into the owning `ProtocolDeclarationForm::declarations` of
    /// this attempt's own static declaration.
    pub declaration: usize,
    /// The resolved operation `on M::T::op` names.
    pub operation: ClauseOperation,
    /// The population domain of the type declaring the operation.
    pub frame_population: Option<PopulationDomain>,
    /// Each `contracts` entry, resolved to the index of the unit's own
    /// `state_clauses` it names, in source order.
    pub contracts: Vec<usize>,
}

/// One protocol `attempt`, fully checked (FR-114 "Outputs"): the
/// identity of its operation's `state`/`operation_anchor` node and
/// `state`/`frame` node (FR-105), and the identity of each state clause its
/// `contracts` list names, in list order. The frame's own entries are never
/// copied here (FR-114: "SHALL record the attempt's frame only by the
/// frame node's identity, and SHALL NOT copy the frame's entries into the
/// attempt"). `check::mod`'s own pipeline fills this in once S4 (lowering)
/// mints the identities; `check` itself returns an empty list here (see
/// its own `Vec<BoundAttempt>` return value for the S3-level binding).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedAttempt {
    /// The index into the owning `ProtocolDeclarationForm::declarations` of
    /// this attempt's own static declaration.
    pub declaration: usize,
    /// The operation's `operation_anchor` node identity.
    pub anchor: quire_exact::NodeKey,
    /// The operation's `frame` node identity.
    pub frame: quire_exact::NodeKey,
    /// Each `contracts` entry's own checked state-clause identity, in list
    /// order.
    pub contracts: Vec<quire_exact::NodeKey>,
}

/// One protocol declaration FR-113 has checked: every scoped anchor's
/// resolved target, in the same order as the form's own `scoped_anchors`.
/// `attempts` starts empty: `check` checks FR-114's `contracts` binding
/// (its own `Vec<BoundAttempt>` return value) but mints no node identity,
/// since that needs S4 (lowering); `check::mod`'s own pipeline fills
/// `attempts` in once it does.
#[derive(Clone, Debug)]
pub struct CheckedProtocol {
    /// The declared protocol name.
    pub name: String,
    /// `resolved_anchors[i]` is the identity `scoped_anchors[i]` resolved
    /// to.
    pub resolved_anchors: Vec<ProtocolNodeId>,
    /// Every `attempt`'s own fully checked binding, filled in after S4.
    pub attempts: Vec<CheckedAttempt>,
}

/// A declaration scope, keyed the way FR-113 resolves a segment: the
/// enclosing named controls' spelled names, outermost first.
type ScopeKey = Vec<String>;

fn scope_key(protocol: &ProtocolDeclarationForm, scope: Option<qsl_forms::ScopeId>) -> ScopeKey {
    protocol
        .scope_names(scope)
        .into_iter()
        .map(|name| name.name.clone())
        .collect()
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a
        // `ProtocolAnchor` cause's own span directly instead, so this
        // `Location` is never resolved.
        location: quire_semantic_value::location::Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The kinds `site` admits, and the word FR-113 names `site` by.
fn admitted_kinds(site: AnchorSite) -> (&'static str, &'static [ProtocolNodeKind]) {
    use ProtocolNodeKind as K;
    match site {
        AnchorSite::ReceiveOf => ("receive-of", &[K::Send]),
        AnchorSite::EffectOf => ("effect-of", &[K::Attempt]),
        AnchorSite::EventFor => ("event-for", &[K::CompensateTemplate]),
        AnchorSite::CompensateFor => ("compensate-for", &[K::Effect]),
        AnchorSite::CompensateCommit => ("compensate-commit", &[K::Commit]),
        AnchorSite::AwaitAfter => (
            "await-after",
            &[
                K::Send,
                K::Receive,
                K::Attempt,
                K::Effect,
                K::Event,
                K::Commit,
                K::CompensateTemplate,
            ],
        ),
    }
}

/// FR-113: the protocol node `protocol`'s scoped anchors resolve to, and
/// FR-114: each `attempt`'s own `contracts` list checked against its
/// already-resolved operation -- or the refusals below, in source
/// position order (declaration-order duplicate refusals first at equal
/// span, then binder-shadow refusals, then per-anchor refusals, then
/// per-contract refusals). `alias_names` (the unit's own `profile`/`model`
/// selection aliases), `native_names` (its `dimension` and `unit`
/// declaration names), `scope` (the package's other native type
/// declarations: `type`, `record`, `tuple` and `enum`) and `signatures`
/// (its native function declarations) are the only package-wide state this
/// checker reads, for FR-113 "Refusals" binder no-shadowing rule;
/// `attempts` (the assembler's own per-attempt operation resolution) and
/// `state_clauses` (the unit's own checked state clauses, for FR-114's
/// `contracts` check) are read for FR-114 alone; every other check is over
/// the protocol's own form. Returns the checked protocol (its `attempts`
/// empty: S4/lowering fills it in) alongside the S3-level `BoundAttempt`s
/// `check::mod`'s own pipeline lowers next.
pub fn check(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
    attempts: &[AttemptDeclaration],
    state_clauses: &[StateClauseDeclaration],
) -> Result<(CheckedProtocol, Vec<BoundAttempt>), Vec<CheckRefusal>> {
    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();

    // Every static node by (its scope, its name): the declaration
    // collection FR-113 Inputs asks S2 for. A scope with two or more
    // entries of one name is refused up front, independent of whether any
    // anchor names it (FR-113 "Refusals").
    let mut by_scope_name: BTreeMap<(ScopeKey, String), Vec<usize>> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        by_scope_name
            .entry((
                scope_key(protocol, declaration.scope),
                declaration.name.name.clone(),
            ))
            .or_default()
            .push(index);
    }
    for ((_, name), indices) in &by_scope_name {
        if indices.len() < 2 {
            continue;
        }
        let loci: Vec<Span> = indices
            .iter()
            .map(|&index| protocol.declarations[index].name.span)
            .collect();
        for &index in indices {
            let span = protocol.declarations[index].name.span;
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: name.clone(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }

    refusals.extend(shadow_refusals(
        protocol,
        alias_names,
        native_names,
        scope,
        signatures,
    ));

    let mut resolved = Vec::with_capacity(protocol.scoped_anchors.len());
    for anchor in &protocol.scoped_anchors {
        match resolve(protocol, &by_scope_name, anchor) {
            Ok(target) => {
                if let Some(cause) = check_kind(protocol, anchor, target) {
                    refusals.push((anchor.anchor.span, refusal(cause)));
                } else {
                    resolved.push(target);
                }
            }
            Err(cause) => refusals.push((anchor.anchor.span, refusal(cause))),
        }
    }

    let (bound_attempts, contract_refusals) =
        bind_attempts(protocol, attempts, state_clauses, scope.types());
    refusals.extend(contract_refusals);

    if !refusals.is_empty() {
        // FR-113 "Outputs": refusals in source position order. `sort_by_key`
        // is stable, so refusals at equal spans (the up-front duplicate
        // pass, which emits one per declaration at that declaration's own
        // span) keep their relative order.
        refusals.sort_by_key(|(span, _)| span.start);
        return Err(refusals.into_iter().map(|(_, refusal)| refusal).collect());
    }

    Ok((
        CheckedProtocol {
            name: protocol.name.name.clone(),
            resolved_anchors: resolved,
            attempts: Vec::new(),
        },
        bound_attempts,
    ))
}

/// FR-114 "Behavior": each `attempt`'s already-resolved operation
/// (`attempts`, the assembler's own `AttemptDeclaration`s, index-aligned
/// with `protocol.attempts`) bound against its `contracts` list: an entry
/// naming no state clause of the unit refuses `missing_declaration`/
/// `missing-name` at the entry; one naming an invariant or a `pre`/`post`
/// clause anchored at a different operation refuses `wrong_snapshot`/
/// `wrong-anchor` at the entry, naming both anchors. An entry naming two
/// or more state clauses of one name refuses `ambiguous_declaration`/
/// `ambiguous-name` at the entry (SR-770 FND-005): no source order or first
/// match picks one, the same rule FR-113 applies to a protocol node.
fn bind_attempts(
    protocol: &ProtocolDeclarationForm,
    attempts: &[AttemptDeclaration],
    state_clauses: &[StateClauseDeclaration],
    types: &TypeEnvironment,
) -> (Vec<BoundAttempt>, Vec<(Span, CheckRefusal)>) {
    let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (index, clause) in state_clauses.iter().enumerate() {
        by_name.entry(clause.name.as_str()).or_default().push(index);
    }
    let anchor_label = |declaring, operation: &str| -> String {
        let context = types
            .object_type(declaring)
            .map(ObjectTypeDeclaration::name)
            .unwrap_or("?");
        format!("{context}::{operation}")
    };

    let mut refusals = Vec::new();
    let mut bound = Vec::with_capacity(attempts.len());
    for (index, attempt_decl) in attempts.iter().enumerate() {
        let form = &protocol.attempts[index];
        let attempt_anchor = anchor_label(
            attempt_decl.operation.declaring,
            attempt_decl.operation.declaration.name(),
        );
        let mut contracts = Vec::with_capacity(form.contracts.len());
        for entry in &form.contracts {
            let Some(indices) = by_name.get(entry.name.as_str()) else {
                refusals.push((
                    entry.span,
                    refusal(ProtocolAnchorCause::MissingContract {
                        entry: entry.name.clone(),
                        span: entry.span,
                    }),
                ));
                continue;
            };
            // SR-770 FND-005: two state clauses sharing the entry's name
            // leave no single clause for it to bind -- refused at the
            // entry, naming every clause of that name, the same
            // no-first-match rule FR-113 applies to a protocol node name.
            let [clause_index] = indices[..] else {
                refusals.push((
                    entry.span,
                    refusal(ProtocolAnchorCause::Ambiguous {
                        name: entry.name.clone(),
                        loci: indices
                            .iter()
                            .filter_map(|&index| state_clauses.get(index))
                            .map(|clause| clause.spans.declaration)
                            .collect(),
                        span: entry.span,
                    }),
                ));
                continue;
            };
            let Some(clause) = state_clauses.get(clause_index) else {
                continue;
            };
            let matches = clause
                .operation
                .as_ref()
                .is_some_and(|clause_operation| *clause_operation == attempt_decl.operation);
            if matches {
                contracts.push(clause_index);
                continue;
            }
            let clause_anchor = match &clause.operation {
                Some(clause_operation) => anchor_label(
                    clause_operation.declaring,
                    clause_operation.declaration.name(),
                ),
                None => types
                    .object_type(clause.context)
                    .map(ObjectTypeDeclaration::name)
                    .unwrap_or("?")
                    .to_owned(),
            };
            refusals.push((
                entry.span,
                refusal(ProtocolAnchorCause::WrongContractAnchor {
                    entry: entry.name.clone(),
                    attempt_anchor: attempt_anchor.clone(),
                    clause_anchor,
                    span: entry.span,
                }),
            ));
        }
        bound.push(BoundAttempt {
            declaration: attempt_decl.declaration,
            operation: attempt_decl.operation.clone(),
            frame_population: attempt_decl.frame_population,
            contracts,
        });
    }
    (bound, refusals)
}

/// Whether this checker covers every part of a static node of `kind`:
/// a `sequence` (a name and its children), an `attempt` (its
/// operation and `contracts`, FR-114; its role, binder and body are checked
/// by [`content`]) and the `finish` node (its binder and body, the same).
/// Every other kind carries content -- a channel, a guard, a join policy,
/// a compensation's blocks, an anchor's target semantics -- that nothing
/// checks yet. Exhaustive, so a new node kind must be classified here.
fn covered_kind(kind: ProtocolNodeKind) -> bool {
    use ProtocolNodeKind as K;
    match kind {
        K::Sequence | K::Attempt | K::Finish => true,
        K::Send
        | K::Receive
        | K::Effect
        | K::Event
        | K::Commit
        | K::CompensateTemplate
        | K::Choice
        | K::Parallel
        | K::Repeat
        | K::Branch
        | K::Case
        | K::Await
        | K::Check => false,
    }
}

/// The rest of a protocol's content, once
/// [`check`] has resolved its anchors, binders and attempt bindings.
///
/// FR-113 and FR-114 check only those three parts. A protocol holding
/// anything else -- a node kind [`covered_kind`] rejects, any
/// [`qsl_forms::ProtocolConstructKind`] (channel, relationship, requirement,
/// capture, `activation on each`, replicated role, `related by`), or a body
/// other than a bare Boolean literal -- refuses `unsupported_construct`/
/// `not-yet-implemented` once, at the earliest such construct, rather than
/// compiling with unchecked content that the emitted package then silently
/// omits (SR-753 FND-002, SR-761 FND-001).
///
/// A protocol made only of covered parts still has content the grammar
/// requires of every protocol, which is checked here: its `using` alias
/// names a profile selection, each role's object type resolves and no two
/// roles share a name, each attempt's `by` names a declared role, and each
/// binder's declared type resolves against the package `scope`. A bare
/// Boolean literal body names no binder and no declaration, so it has
/// nothing further to resolve. Refusals are in source position order.
pub fn content(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    scope: &Scope,
) -> Vec<CheckRefusal> {
    let uncovered = protocol
        .declarations
        .iter()
        .filter(|declaration| !covered_kind(declaration.kind))
        .map(|declaration| (declaration.name.span, declaration.kind.label()))
        .chain(
            protocol
                .constructs
                .iter()
                .map(|construct| (construct.span, construct.kind.label())),
        )
        .chain(
            protocol
                .bodies
                .iter()
                .filter(|body| body.constant.is_none())
                .map(|body| (body.span, "body")),
        )
        .min_by_key(|(span, _)| span.start);
    if let Some((span, construct)) = uncovered {
        return vec![refusal(ProtocolAnchorCause::Unimplemented {
            name: protocol.name.name.clone(),
            construct,
            span,
        })];
    }

    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    let using = &protocol.using;
    if alias_names.get(&using.alias) != Some(&AliasKind::Profile) {
        refusals.push((
            using.span,
            refusal(ProtocolAnchorCause::MissingProfile {
                alias: using.alias.clone(),
                span: using.span,
            }),
        ));
    }
    let mut roles: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    for role in &protocol.roles {
        roles
            .entry(role.name.name.as_str())
            .or_default()
            .push(role.name.span);
        if scope.object_type_named(&role.context.name).is_none() {
            refusals.push((
                role.context.span,
                refusal(ProtocolAnchorCause::MissingRoleType {
                    role: role.name.name.clone(),
                    context: role.context.name.clone(),
                    span: role.context.span,
                }),
            ));
        }
    }
    for (name, loci) in &roles {
        if loci.len() < 2 {
            continue;
        }
        for &span in loci {
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: (*name).to_owned(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }
    for attempt in &protocol.attempts {
        if !roles.contains_key(attempt.role.name.as_str()) {
            refusals.push((
                attempt.role.span,
                refusal(ProtocolAnchorCause::MissingRole {
                    role: attempt.role.name.clone(),
                    span: attempt.role.span,
                }),
            ));
        }
    }
    for binder in &protocol.binders {
        if let Err(error) = super::type_form::resolve_form(scope, &binder.value_type) {
            refusals.push((
                error.span,
                refusal(ProtocolAnchorCause::BinderType {
                    binder: binder.name.name.clone(),
                    fault: error.fault,
                    span: error.span,
                }),
            ));
        }
    }
    refusals.sort_by_key(|(span, _)| span.start);
    refusals.into_iter().map(|(_, refusal)| refusal).collect()
}

/// Resolves one anchor's segments through nested scopes (FR-113
/// "Resolution").
fn resolve(
    protocol: &ProtocolDeclarationForm,
    by_scope_name: &BTreeMap<(ScopeKey, String), Vec<usize>>,
    anchor: &qsl_forms::ScopedAnchorForm,
) -> Result<ProtocolNodeId, ProtocolAnchorCause> {
    let lexical_scope = scope_key(protocol, anchor.scope);
    // `ScopedAnchorForm`'s fields are public, so a hand-built form (not one
    // S2 built, which the grammar guarantees at least one segment for) can
    // hold no segments; refuse rather than index into an empty slice.
    let Some((first, rest)) = anchor.anchor.segments.split_first() else {
        return Err(ProtocolAnchorCause::Missing {
            segments: Vec::new(),
            segment: String::new(),
            scope: lexical_scope,
            span: anchor.anchor.span,
        });
    };
    let segments: Vec<String> = anchor
        .anchor
        .segments
        .iter()
        .map(|segment| segment.text.clone())
        .collect();

    let mut target_index = None;
    for depth in (0..=lexical_scope.len()).rev() {
        let key = (lexical_scope[..depth].to_vec(), first.text.clone());
        let Some(indices) = by_scope_name.get(&key) else {
            continue;
        };
        if indices.len() > 1 {
            return Err(ambiguous(
                protocol,
                indices,
                &first.text,
                anchor.anchor.span,
            ));
        }
        target_index = Some(indices[0]);
        break;
    }
    let Some(mut target_index) = target_index else {
        return Err(ProtocolAnchorCause::Missing {
            segments: segments.clone(),
            segment: first.text.clone(),
            scope: lexical_scope,
            span: anchor.anchor.span,
        });
    };

    for segment in rest {
        let target = &protocol.declarations[target_index];
        let mut child_scope = scope_key(protocol, target.scope);
        child_scope.push(target.name.name.clone());
        let key = (child_scope, segment.text.clone());
        match by_scope_name.get(&key) {
            Some(indices) if indices.len() > 1 => {
                return Err(ambiguous(
                    protocol,
                    indices,
                    &segment.text,
                    anchor.anchor.span,
                ));
            }
            Some(indices) => target_index = indices[0],
            None => {
                return Err(ProtocolAnchorCause::Missing {
                    segments: segments.clone(),
                    segment: segment.text.clone(),
                    scope: scope_key(protocol, anchor.scope),
                    span: anchor.anchor.span,
                });
            }
        }
    }
    Ok(ProtocolNodeId(target_index))
}

fn ambiguous(
    protocol: &ProtocolDeclarationForm,
    indices: &[usize],
    name: &str,
    span: Span,
) -> ProtocolAnchorCause {
    ProtocolAnchorCause::Ambiguous {
        name: name.to_owned(),
        loci: indices
            .iter()
            .map(|&index| protocol.declarations[index].name.span)
            .collect(),
        span,
    }
}

/// FR-113's wrong-kind and channel-mismatch checks over an anchor already
/// resolved to `target`.
fn check_kind(
    protocol: &ProtocolDeclarationForm,
    anchor: &qsl_forms::ScopedAnchorForm,
    target: ProtocolNodeId,
) -> Option<ProtocolAnchorCause> {
    let declaration = &protocol.declarations[target.0];
    let (site, admitted) = admitted_kinds(anchor.site);
    if !admitted.contains(&declaration.kind) {
        return Some(ProtocolAnchorCause::WrongKind {
            site,
            admitted: admitted.iter().map(|kind| kind.label()).collect(),
            actual: declaration.kind.label(),
            span: anchor.anchor.span,
        });
    }
    if anchor.site == AnchorSite::ReceiveOf {
        if let (Some(receive_channel), Some(send_channel)) =
            (anchor.channel.as_ref(), declaration.channel.as_ref())
        {
            if receive_channel != send_channel {
                return Some(ProtocolAnchorCause::ChannelMismatch {
                    receive_channel: receive_channel.clone(),
                    send_channel: send_channel.clone(),
                    span: anchor.anchor.span,
                });
            }
        }
    }
    None
}

/// FR-113 "Refusals" binder no-shadowing rule: every binder that shadows a
/// model/profile alias, a native declaration of the package or another
/// binder visible where it is declared, in source order.
///
/// Binders are checked in `protocol.binders`' own order (source order,
/// the S2 walk), each against every binder built before it anywhere
/// in the protocol: a later binder shadows an earlier one, never the other
/// way round, so only the later binder is refused (FR-113: "refuse ... at
/// that binder, naming the declaration it would shadow"). QSpec
/// `shared-grammar.md` makes binders "unique in their enclosing
/// declaration" -- the whole protocol, not the lexical scope a binder's
/// own `BinderForm::scope` names -- so this check ignores scope entirely
/// when comparing binder against binder; `scope` still locates a binder for
/// FR-114's later binding pass. A package-level alias or native
/// declaration is checked only when no earlier binder in the protocol
/// already shares the name.
fn shadow_refusals(
    protocol: &ProtocolDeclarationForm,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Vec<(Span, CheckRefusal)> {
    let mut refusals = Vec::new();
    // Every earlier binder's own span, by name, flat across the whole
    // protocol (SR-765 FND-001).
    let mut seen: BTreeMap<String, Span> = BTreeMap::new();
    for binder in &protocol.binders {
        let shadowed = seen
            .get(binder.name.name.as_str())
            .map(|&span| ShadowedDeclaration::Binder(span))
            .or_else(|| {
                package_shadow(
                    &binder.name.name,
                    alias_names,
                    native_names,
                    scope,
                    signatures,
                )
            });
        if let Some(shadowed) = shadowed {
            refusals.push((
                binder.name.span,
                refusal(ProtocolAnchorCause::Shadow {
                    name: binder.name.name.clone(),
                    shadowed,
                    span: binder.name.span,
                }),
            ));
        }
        seen.insert(binder.name.name.clone(), binder.name.span);
    }
    refusals
}

/// Whether `name` is a model/profile alias or a native declaration of the
/// package (SR-765 FND-003: dimensions and units too), and which kind, for
/// [`shadow_refusals`]'s package-wide half of FR-113's no-shadowing rule.
fn package_shadow(
    name: &str,
    alias_names: &BTreeMap<String, AliasKind>,
    native_names: &BTreeSet<String>,
    scope: &Scope,
    signatures: &Signatures,
) -> Option<ShadowedDeclaration> {
    if let Some(alias) = alias_names.get(name) {
        return Some(match alias {
            AliasKind::Profile => ShadowedDeclaration::ProfileAlias,
            AliasKind::Model => ShadowedDeclaration::ModelAlias,
        });
    }
    if !scope.named_types(name).is_empty() {
        return Some(ShadowedDeclaration::Type);
    }
    if signatures.declares(name) {
        return Some(ShadowedDeclaration::Function);
    }
    if native_names.contains(name) {
        return Some(ShadowedDeclaration::Quantity);
    }
    None
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{build_unit, AnchorSite, ProtocolDeclarationForm, ProtocolNodeKind};
    use qsl_foundation::SourceIdentity;

    use super::super::{
        CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations, Scope, Signatures,
    };
    use super::{CheckedProtocol, ProtocolAnchorCause, ProtocolNodeId, ShadowedDeclaration};
    use quire_semantic_value::checking::CheckingLimits;
    use quire_semantic_value::declaration::TypeEnvironment;

    mod attempt_fixture {
        use crate as semantics;
        include!("../../tests/fixtures/protocol_attempt.rs");
    }
    use attempt_fixture::{attempt_flow, m_actor_model, shared_binder_protocols, HEADER};

    /// S1, S2 and the assembler over `declarations`, against
    /// [`m_actor_model`]'s own `M::Actor::op`: FR-113's checker
    /// itself resolves protocol node references only, never a model type,
    /// but FR-114's own assembler resolution now runs for every attempt
    /// regardless, so this module's fixtures need a real admitted model
    /// even though FR-113's own checks do not read it.
    fn assemble(declarations: &str) -> (String, PackageDeclarations) {
        let text = format!("{HEADER}{declarations}\n");
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = build_unit(&parsed).expect("S2 builds the unit");
        let assembled = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            vec![m_actor_model()],
            Vec::new(),
        )
        .unwrap_or_else(|refusal| panic!("{declarations}: the assembler refuses: {refusal:?}"));
        (text, assembled)
    }

    /// An empty `Scope` and `Signatures`: no alias, type or function names
    /// for a binder to shadow, so [`resolve_protocol`]'s fixtures (which
    /// carry no shadowing binder) resolve exactly as the anchor-resolution
    /// algorithm alone would.
    fn empty_scope_and_signatures() -> (Scope, Signatures) {
        (
            Scope::new(
                TypeEnvironment::default(),
                crate::model::operation::OperationTable::default(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                None,
                Vec::new(),
            ),
            Signatures::new(Vec::new()),
        )
    }

    /// `super::check` called directly on the one protocol `declarations`
    /// holds, bypassing `PackageDeclarations::check`'s whole-package pass:
    /// this is the anchor-resolution algorithm's own oracle, not the whole
    /// pipeline's. FR-114's own binding is bypassed too (empty `attempts`
    /// and `state_clauses`), since this helper isolates FR-113 alone.
    /// [`super::check`]'s own result, for [`resolve_protocol`]'s return type
    /// (clippy's `type_complexity`: three levels of nesting is easier read
    /// as one name than inline).
    type CheckResult = Result<(CheckedProtocol, Vec<super::BoundAttempt>), Vec<CheckRefusal>>;

    fn resolve_protocol(declarations: &str) -> (String, ProtocolDeclarationForm, CheckResult) {
        let (text, assembled) = assemble(declarations);
        let protocol = assembled.protocols[0].clone();
        let (scope, signatures) = empty_scope_and_signatures();
        let result = super::check(
            &protocol,
            &std::collections::BTreeMap::new(),
            &std::collections::BTreeSet::new(),
            &scope,
            &signatures,
            &[],
            &[],
        );
        (text, protocol, result)
    }

    /// The full `PackageDeclarations::check` pipeline over `declarations`,
    /// expecting it to refuse.
    fn refusals(declarations: &str) -> (String, Vec<CheckRefusal>) {
        let (text, assembled) = assemble(declarations);
        match assembled.check(CheckingLimits::default()) {
            Err(refusals) => (text, refusals),
            Ok(_) => panic!("{declarations}: checking refuses"),
        }
    }

    /// The full `PackageDeclarations::check` pipeline over `declarations`,
    /// expecting it to succeed.
    fn checks(declarations: &str) -> CheckedGraph {
        let (_, assembled) = assemble(declarations);
        assembled
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
    }

    /// S1, S2 and the assembler over `declarations` (against
    /// [`m_actor_model`]), expecting the assembler to refuse.
    fn assembly_errors(declarations: &str) -> Vec<crate::check::AssemblyError> {
        let text = format!("{HEADER}{declarations}\n");
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        let unit = build_unit(&parsed).expect("S2 builds the unit");
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            vec![m_actor_model()],
            Vec::new(),
        )
        .map(|_| ())
        .expect_err("the assembler refuses")
        .errors
    }

    /// The one refusal `declarations` checks to, expecting exactly one.
    fn only_cause(declarations: &str) -> (CheckRefusal, ProtocolAnchorCause) {
        let (_, found) = refusals(declarations);
        assert_eq!(found.len(), 1, "{declarations}: {found:?}");
        let cause = protocol_causes(&found)[0].clone();
        (found[0].clone(), cause)
    }

    fn protocol_causes(refusals: &[CheckRefusal]) -> Vec<&ProtocolAnchorCause> {
        refusals
            .iter()
            .map(|refusal| match &refusal.cause {
                CheckCause::ProtocolAnchor(cause) => cause.as_ref(),
                other => panic!("a ProtocolAnchor cause, got {other:?}"),
            })
            .collect()
    }

    /// The one declaration named `name` of kind `kind`, in scope `scope`
    /// (outermost first): the algorithm-level oracle `resolved_anchors`
    /// assertions read against, so a test names the expected *target*, not
    /// just "no refusal".
    fn declaration_id(
        protocol: &ProtocolDeclarationForm,
        name: &str,
        kind: ProtocolNodeKind,
        scope: &[&str],
    ) -> ProtocolNodeId {
        let index = protocol
            .declarations
            .iter()
            .position(|declaration| {
                declaration.name.name == name
                    && declaration.kind == kind
                    && protocol
                        .scope_names(declaration.scope)
                        .into_iter()
                        .map(|name| name.name.as_str())
                        .eq(scope.iter().copied())
            })
            .unwrap_or_else(|| {
                panic!(
                    "no {kind:?} named {name} in scope {scope:?}: {:?}",
                    protocol.declarations
                )
            });
        ProtocolNodeId(index)
    }

    /// The indices of every scoped anchor at `site`, in source order.
    fn anchors_at(protocol: &ProtocolDeclarationForm, site: AnchorSite) -> Vec<usize> {
        protocol
            .scoped_anchors
            .iter()
            .enumerate()
            .filter(|(_, anchor)| anchor.site == site)
            .map(|(index, _)| index)
            .collect()
    }

    /// The `RecoveryFlow` shape TC-510/511's own fixture uses (matching
    /// `qsl-forms`'s `recovery_flow`), with `Main::Applied`/`Main::Committed`
    /// resolved through the `compensate` template and `Tried`/`Undo`
    /// resolved from inside `Main`. Declares channel `C` (FR-113-AC-7's
    /// `receive-of` cases route a `receive` through it).
    fn recovery_flow(commit: &str, main_extra: &str) -> String {
        format!(
            "protocol RecoveryFlow using v over (input: Boolean) on origin {{\n\
             role R on M::Actor;\n\
             channel C from R to R carries Boolean ordering unordered delivery at-most-once \
             capacity 1 overflow reject;\n\
             compensate Undo for Main::Applied as (failure: Boolean) by R \
             on M::Actor::op using v clock \"ticks\" {{\n\
             activate first (trigger: Boolean) when {{ true }} {{ }}\n\
             within [0,10]; attempts 2 of M::Actor;\n\
             retry (current_attempt: Boolean, prior: Boolean) {{ true }};\n\
             commit {commit}; recover (recovery: Boolean) {{ true }}; }}\n\
             run sequence Main {{\n\
             attempt Tried by R on M::Actor::op contracts [] as (tried: Boolean) {{ true }};\n\
             {main_extra}\
             effect Applied of Tried as (applied: Boolean) {{ true }};\n\
             event Recovered by R for Undo as (notice: Boolean) {{ true }};\n\
             commit Committed by R as (committed: Boolean) {{ true }};\n\
             }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}"
        )
    }

    /// A minimal `send`/`receive` protocol, its `receive` routed via
    /// `channel`, with both `C` and `D` declared (FR-113-AC-7's channel
    /// cases).
    fn handoff_protocol(channel: &str) -> String {
        format!(
            "protocol Handoff using v over (input: Boolean) on origin {{\n\
             role R on M::Actor;\n\
             channel C from R to R carries Boolean ordering unordered delivery at-most-once \
             capacity 1 overflow reject;\n\
             channel D from R to R carries Boolean ordering unordered delivery at-most-once \
             capacity 1 overflow reject;\n\
             run sequence Main {{\n\
             send Ping via C as (ping: Boolean) {{ true }};\n\
             receive Got via {channel} of Ping as (got: Boolean) {{ true }};\n\
             }}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}"
        )
    }

    /// FR-113-AC-1 (TC-511): every anchor of the `RecoveryFlow` fixture
    /// resolves to the exact node FR-113-AC-1 names, by identity.
    #[trace("TC-511", "FR-113-AC-1")]
    #[test]
    fn every_anchor_of_the_recovery_flow_fixture_resolves_to_its_named_node() {
        let (_, protocol, result) = resolve_protocol(&recovery_flow("Main::Committed", ""));
        let (checked, _) = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        assert_eq!(
            checked.resolved_anchors.len(),
            protocol.scoped_anchors.len()
        );
        // Source order: compensate-for, compensate-commit, effect-of,
        // event-for (FR-112-AC-1).
        assert_eq!(
            checked.resolved_anchors[0],
            declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[1],
            declaration_id(&protocol, "Committed", ProtocolNodeKind::Commit, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[2],
            declaration_id(&protocol, "Tried", ProtocolNodeKind::Attempt, &["Main"])
        );
        assert_eq!(
            checked.resolved_anchors[3],
            declaration_id(&protocol, "Undo", ProtocolNodeKind::CompensateTemplate, &[])
        );
    }

    /// FR-113-AC-2 (TC-511): a nested `sequence Inner` inside `Main`, after
    /// the outer `effect Applied`, declares its own `Applied`. An `await`
    /// inside `Inner` naming `Applied` resolves to the *inner* node; one
    /// naming `Main::Applied` resolves to the outer node from inside
    /// `Inner`; and one in `Main` (outside `Inner`) naming `Applied`
    /// resolves to the outer node -- asserted against the two nodes'
    /// distinct identities, not just "no refusal" (a mutation to
    /// outermost-first resolution passes every other test in this module
    /// but fails this one).
    #[trace("TC-511", "FR-113-AC-2")]
    #[test]
    fn a_nested_scope_resolves_its_own_name_over_an_outer_one_of_the_same_spelling() {
        // Every record binder here is uniquely named (`applied_inner`,
        // `seen_inner`/`seen_qualified`/`seen_outer`), distinct from the
        // outer fixture's own `applied`: FR-113's binder no-shadowing rule
        // is now protocol-wide (SR-765 FND-001), and this test's
        // own assertions are about anchor *resolution* through nested and
        // sibling named-control scopes, not about binder shadowing --
        // reusing a binder name here would make the fixture itself refuse.
        let inner = "sequence Inner {\n\
             effect Applied of Tried as (applied_inner: Boolean) { true };\n\
             await WaitInner after Applied using v clock \"ticks\" within [0,1] \
             match event Seen by R for Undo as (seen_inner: Boolean) { true }; \
             then sequence Then { } timeout sequence Timeout { }\n\
             }\n\
             await WaitQualified after Main::Applied using v clock \"ticks\" within [0,1] \
             match event SeenQ by R for Undo as (seen_qualified: Boolean) { true }; \
             then sequence ThenQ { } timeout sequence TimeoutQ { }\n\
             await WaitOuter after Applied using v clock \"ticks\" within [0,1] \
             match event SeenO by R for Undo as (seen_outer: Boolean) { true }; \
             then sequence ThenO { } timeout sequence TimeoutO { }\n";
        let (_, protocol, result) = resolve_protocol(&recovery_flow("Main::Committed", inner));
        let (checked, _) = result.unwrap_or_else(|refusals| panic!("{refusals:?}"));

        let outer_applied =
            declaration_id(&protocol, "Applied", ProtocolNodeKind::Effect, &["Main"]);
        let inner_applied = declaration_id(
            &protocol,
            "Applied",
            ProtocolNodeKind::Effect,
            &["Main", "Inner"],
        );
        assert_ne!(outer_applied, inner_applied);

        // Built in this order: `WaitInner` (inside `Inner`), `WaitQualified`
        // and `WaitOuter` (both directly in `Main`).
        let await_afters = anchors_at(&protocol, AnchorSite::AwaitAfter);
        assert_eq!(await_afters.len(), 3, "{await_afters:?}");
        let [wait_inner, wait_qualified, wait_outer] = await_afters[..] else {
            unreachable!()
        };
        assert_eq!(
            checked.resolved_anchors[wait_inner], inner_applied,
            "an unqualified reference inside Inner must resolve to Inner's own Applied"
        );
        assert_eq!(
            checked.resolved_anchors[wait_qualified], outer_applied,
            "Main::Applied must reach the outer node from inside Inner"
        );
        assert_eq!(
            checked.resolved_anchors[wait_outer], outer_applied,
            "an unqualified reference in Main (outside Inner) must resolve to the outer node"
        );
    }

    /// FR-113-AC-3 (TC-511): a missing anchor or member refuses
    /// `missing_declaration`/`missing-name`, naming the failing segment,
    /// the whole anchor's segments and the anchor's own (empty, at the top
    /// level) lexical scope.
    #[trace("TC-511", "FR-113-AC-3")]
    #[test]
    fn a_missing_anchor_or_member_refuses_naming_the_failing_segment() {
        let cases = [
            ("Main::Missing", vec!["Main", "Missing"], "Missing"),
            ("Other::Applied", vec!["Other", "Applied"], "Other"),
        ];
        for (target, segments, segment) in cases {
            let source = recovery_flow("Main::Committed", "").replacen("Main::Applied", target, 1);
            let (_, refusals) = refusals(&source);
            assert_eq!(refusals.len(), 1, "{target}: {refusals:?}");
            let [cause] = protocol_causes(&refusals)[..] else {
                panic!("one cause");
            };
            match cause {
                ProtocolAnchorCause::Missing {
                    segment: found,
                    segments: found_segments,
                    scope,
                    ..
                } => {
                    assert_eq!(found, segment, "{target}");
                    assert_eq!(found_segments, &segments, "{target}");
                    assert!(scope.is_empty(), "{target}: {scope:?}");
                }
                other => panic!("{target}: a Missing cause, got {other:?}"),
            }
        }

        let (_, refusals) = refusals(&recovery_flow("Main::Committed", "").replacen(
            "effect Applied of Tried",
            "effect Applied of Absent",
            1,
        ));
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        let [cause] = protocol_causes(&refusals)[..] else {
            panic!("one cause");
        };
        match cause {
            ProtocolAnchorCause::Missing {
                segment,
                segments,
                scope,
                ..
            } => {
                assert_eq!(segment, "Absent");
                assert_eq!(segments, &["Absent".to_owned()]);
                assert_eq!(scope, &["Main".to_owned()]);
            }
            other => panic!("a Missing cause, got {other:?}"),
        }
    }

    /// The two `effect Applied` declarations' own name spans, in the order
    /// they appear in `text` (not the earlier `compensate ... for
    /// Main::Applied` reference).
    fn duplicate_applied_spans(text: &str) -> [usize; 2] {
        let mut effect_applied = text.match_indices("effect Applied");
        let first = effect_applied.next().unwrap().0 + "effect ".len();
        let second = effect_applied.next().unwrap().0 + "effect ".len();
        [first, second]
    }

    /// Every `Ambiguous` cause of `refusals` names `Applied` and lists its
    /// two loci in exactly `expected` order (FR-113-AC-4: "naming both in
    /// source order").
    fn assert_ambiguous_loci_order(refusals: &[CheckRefusal], expected: [usize; 2]) {
        for cause in protocol_causes(refusals) {
            match cause {
                ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
                    assert_eq!(name, "Applied");
                    let starts: Vec<usize> = loci.iter().map(|span| span.start).collect();
                    assert_eq!(starts, expected, "{cause:?}");
                }
                other => panic!("an Ambiguous cause, got {other:?}"),
            }
        }
    }

    /// FR-113-AC-4 (TC-512): two event nodes named `Applied` in `Main`
    /// refuse `ambiguous_declaration`/`ambiguous-name` at both
    /// declarations, and `Main::Applied` refuses at the anchor, naming
    /// both in source order. Swapping the two declarations swaps nothing
    /// but their order in the refusal.
    #[trace("TC-512", "FR-113-AC-4")]
    #[test]
    fn two_declarations_of_one_name_refuse_at_both_and_at_a_qualified_anchor() {
        let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
        let source = recovery_flow("Main::Committed", extra);
        let (text, found) = refusals(&source);
        // Two duplicate-declaration refusals (one per `Applied`) plus one
        // at the `compensate ... for Main::Applied` anchor, which decides
        // on the ambiguous name.
        assert_eq!(found.len(), 3, "{found:?}");
        for refusal in &found {
            assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
        }
        // One refusal at each `effect Applied` declaration's own name, and
        // one at the `compensate ... for Main::Applied` anchor's whole
        // reference -- in source position order, the anchor first, since it
        // is written before `run sequence Main`.
        let anchor_span = text.find("Main::Applied").unwrap();
        let [first_decl, second_decl] = duplicate_applied_spans(&text);
        let mut expected = [anchor_span, first_decl, second_decl];
        expected.sort_unstable();
        let starts: Vec<usize> = protocol_causes(&found)
            .iter()
            .map(|cause| cause.span().start)
            .collect();
        assert_eq!(starts, expected);
        assert_ambiguous_loci_order(&found, [first_decl, second_decl]);

        // Swap the two declarations' physical order (the `extra` copy,
        // marked `again`, now comes after the fixture's own `effect
        // Applied`, marked `applied`, instead of before it): the same
        // three refusals occur, and the loci list simply swaps to match
        // the new source order.
        assert!(
            text.find("again").unwrap() < text.find("applied").unwrap(),
            "fixture sanity: `again` (extra) must be declared before `applied` (outer)"
        );
        let without_extra = recovery_flow("Main::Committed", "");
        let outer_line = "effect Applied of Tried as (applied: Boolean) { true };\n";
        assert!(without_extra.contains(outer_line));
        let swapped = without_extra.replacen(outer_line, &format!("{outer_line}{extra}"), 1);
        assert_ne!(source, swapped);
        let (swapped_text, swapped_refusals) = refusals(&swapped);
        assert_eq!(swapped_refusals.len(), 3, "{swapped_refusals:?}");
        assert!(
            swapped_text.find("applied").unwrap() < swapped_text.find("again").unwrap(),
            "swap sanity: `applied` (outer) must now be declared before `again` (extra)"
        );
        let [swapped_first, swapped_second] = duplicate_applied_spans(&swapped_text);
        assert_ambiguous_loci_order(&swapped_refusals, [swapped_first, swapped_second]);
    }

    /// A regression fixture distinct from FR-113-AC-6's own (a duplicate
    /// declaration, not a shadowing binder, paired with a missing anchor):
    /// a protocol with a duplicate declaration (physically inside `run`,
    /// so later in source) and an unrelated missing anchor (physically
    /// before `run`) reports both, ordered strictly by source position --
    /// not by the order the checker happens to build them in, which is the
    /// opposite: the up-front duplicate-declaration pass runs before the
    /// per-anchor pass, so without the sort this test's own refusals would
    /// come back in exactly the wrong order. Checking it twice gives the
    /// same refusals in the same order.
    /// [`a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order`]
    /// carries AC-6's own trace tag, over its own fixture (a missing anchor
    /// paired with a shadowing binder).
    #[trace("TC-512")]
    #[test]
    fn a_missing_anchor_and_a_later_duplicate_declaration_report_in_source_order() {
        let extra = "effect Applied of Tried as (again: Boolean) { true };\n";
        let source = recovery_flow("Main::Committed", extra).replacen(
            "for Main::Applied",
            "for Main::Missing",
            1,
        );
        let (text, refusals_a) = refusals(&source);
        let (_, refusals_b) = refusals(&source);
        assert_eq!(refusals_a, refusals_b, "checking twice must agree");
        assert_eq!(refusals_a.len(), 3, "{refusals_a:?}");

        let missing_span = text.find("Main::Missing").unwrap();
        let [first_decl, second_decl] = duplicate_applied_spans(&text);
        // The missing-anchor refusal is textually first (before `run`), so
        // it must sort first even though the checker builds the two
        // duplicate-declaration refusals before it.
        let mut expected = [missing_span, first_decl, second_decl];
        expected.sort_unstable();
        assert_eq!(
            expected[0], missing_span,
            "fixture sanity: missing must be earliest"
        );
        let starts: Vec<usize> = protocol_causes(&refusals_a)
            .iter()
            .map(|cause| cause.span().start)
            .collect();
        assert_eq!(
            starts, expected,
            "refusals are not in source position order"
        );
        assert!(
            matches!(
                protocol_causes(&refusals_a)[0],
                ProtocolAnchorCause::Missing { .. }
            ),
            "the earliest refusal must be the Missing one, {:?}",
            refusals_a[0]
        );
    }

    /// FR-113-AC-7 (TC-512): a wrong-kind target refuses
    /// `ill_typed`/`type-mismatch` naming the site, the admitted kinds and
    /// the actual kind, for each of the six reference positions; a
    /// mismatched channel refuses the same way, naming both channels, and
    /// a matching channel resolves.
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_resolved_anchor_of_the_wrong_kind_refuses_naming_the_actual_kind() {
        // effect-of must admit only `attempt`; naming `Recovered` (an
        // `event`, FR-113's own example) instead of `Tried` (the
        // `attempt`) is wrong-kind.
        let wrong_effect_of = recovery_flow("Main::Committed", "").replacen(
            "effect Applied of Tried",
            "effect Applied of Recovered",
            1,
        );
        let (_, found) = refusals(&wrong_effect_of);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "effect-of");
                assert_eq!(*actual, "event");
                assert_eq!(admitted.as_slice(), ["attempt"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // compensate-for must admit only `effect`; naming `Tried` (an
        // `attempt`) is wrong-kind.
        let wrong_compensate_for =
            recovery_flow("Main::Committed", "").replacen("Main::Applied", "Main::Tried", 1);
        let (_, found) = refusals(&wrong_compensate_for);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "compensate-for");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["effect"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // compensate-commit must admit only `commit`; naming `Tried` (an
        // `attempt`) is wrong-kind.
        let wrong_compensate_commit = recovery_flow("Main::Tried", "");
        let (_, found) = refusals(&wrong_compensate_commit);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "compensate-commit");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["commit"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // event-for must admit only a `compensate` template; naming
        // `Tried` (an `attempt`) is wrong-kind.
        let wrong_event_for =
            recovery_flow("Main::Committed", "").replacen("for Undo", "for Tried", 1);
        let (_, found) = refusals(&wrong_event_for);
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "event-for");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["compensate"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // await-after admits every event-like kind and a compensate
        // template, but never a structural control: naming `Main` (a
        // `sequence`) is wrong-kind.
        let awaiting_a_sequence = "await Wait after Main using v clock \"ticks\" within [0,1] \
             match event Seen by R for Undo as (seen: Boolean) { true }; \
             then sequence Then { } timeout sequence Timeout { }\n";
        let (_, found) = refusals(&recovery_flow("Main::Committed", awaiting_a_sequence));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "await-after");
                assert_eq!(*actual, "sequence");
                assert!(admitted.contains(&"send"));
                assert!(!admitted.contains(&"sequence"));
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // receive-of must admit only `send`; naming `Tried` (an `attempt`)
        // is wrong-kind.
        let wrong_receive_of = "receive Got via C of Tried as (got: Boolean) { true };\n";
        let (_, found) = refusals(&recovery_flow("Main::Committed", wrong_receive_of));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongKind {
                site,
                actual,
                admitted,
                ..
            } => {
                assert_eq!(*site, "receive-of");
                assert_eq!(*actual, "attempt");
                assert_eq!(admitted.as_slice(), ["send"]);
            }
            other => panic!("a WrongKind cause, got {other:?}"),
        }

        // A channel mismatch between a matching-kind `receive` and `send`
        // refuses `type-mismatch` naming both channels; the same `receive`
        // over the matching channel resolves.
        let (_, _, result) = resolve_protocol(&handoff_protocol("C"));
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let (_, found) = refusals(&handoff_protocol("D"));
        assert_eq!(found.len(), 1, "{found:?}");
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::ChannelMismatch {
                receive_channel,
                send_channel,
                ..
            } => {
                assert_eq!(receive_channel, "D");
                assert_eq!(send_channel, "C");
            }
            other => panic!("a ChannelMismatch cause, got {other:?}"),
        }
    }

    /// SR-753/SR-754-style mutation check, over a path
    /// `a_resolved_anchor_of_the_wrong_kind_...` does not itself exercise:
    /// flipping the channel comparison to always agree (as an unresolved
    /// check would) must turn this test red, and reverting must turn it
    /// green again.
    #[trace("TC-512", "FR-113-AC-7")]
    #[test]
    fn a_channel_mismatch_refusal_is_not_vacuous_over_an_otherwise_valid_protocol() {
        let matching = handoff_protocol("C");
        let (_, _, result) = resolve_protocol(&matching);
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let mutated = handoff_protocol("D");
        assert_ne!(matching, mutated);
        let (_, refusals) = refusals(&mutated);
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        assert!(matches!(
            protocol_causes(&refusals)[0],
            ProtocolAnchorCause::ChannelMismatch { .. }
        ));
    }

    /// SR-770 FND-001: a protocol whose anchors (FR-113) and
    /// attempt bindings (FR-114) all resolve, but which holds content no
    /// checker reads yet (here a channel, a `compensate` template, an
    /// `effect`, an `event` and a `commit`), still refuses
    /// `unsupported_construct`/`not-yet-implemented` once, at the earliest
    /// such construct -- the `channel` -- rather than compiling with that
    /// content unchecked and silently missing from the emitted package.
    #[trace("TC-511", "FR-113")]
    #[test]
    fn a_resolving_protocol_with_unchecked_content_still_refuses() {
        let source = recovery_flow("Main::Committed", "");
        let (_, _, result) = resolve_protocol(&source);
        result.unwrap_or_else(|refusals| panic!("{refusals:?}"));
        let (refusal, cause) = only_cause(&source);
        assert_eq!(refusal.cause.code().as_str(), "unsupported_construct");
        assert_eq!(refusal.cause.cause(), Some("not-yet-implemented"));
        match cause {
            ProtocolAnchorCause::Unimplemented {
                name,
                construct,
                span,
            } => {
                assert_eq!(name, "RecoveryFlow");
                assert_eq!(construct, "channel");
                let (text, _) = assemble(&source);
                assert!(text[span.start..].starts_with("channel C"), "{span:?}");
            }
            other => panic!("an Unimplemented cause, got {other:?}"),
        }
    }

    /// A protocol made only of parts the checker covers in full
    /// (a role, a `sequence`, an `attempt` and a `finish`, each body a bare
    /// Boolean literal) checks through the whole pipeline, and its checked
    /// attempt names the resolved operation's own anchor and frame nodes.
    #[trace("TC-513", "FR-114")]
    #[test]
    fn a_fully_covered_attempt_protocol_checks() {
        let graph = checks(&attempt_flow("Flow", ""));
        let [protocol] = graph.protocols() else {
            panic!("one protocol: {:?}", graph.protocols());
        };
        assert_eq!(protocol.name, "Flow");
        let [attempt] = protocol.attempts.as_slice() else {
            panic!("one attempt: {:?}", protocol.attempts);
        };
        assert_ne!(attempt.anchor, attempt.frame);
    }

    /// One `content` refusal per unchecked or ill-formed part of an
    /// otherwise fully covered protocol (SR-770 FND-001), each checked
    /// against its own catalog code, tag and cause.
    #[trace("TC-513", "FR-114")]
    #[test]
    fn each_unchecked_part_of_an_attempt_protocol_refuses() {
        let base = attempt_flow("Flow", "");
        let cases: [(&str, &str, &str, &str); 7] = [
            // An ill-typed body is not checked yet: refused, not accepted.
            (
                "as (tried: Boolean) { true }",
                "as (tried: Boolean) { 1 + true }",
                "unsupported_construct",
                "not-yet-implemented",
            ),
            // A node kind other than `sequence`/`attempt`/`finish`.
            (
                "contracts [] as (tried: Boolean) { true };\n",
                "contracts [] as (tried: Boolean) { true };\n\
                 send Ping via R as (ping: Boolean) { true };\n",
                "unsupported_construct",
                "not-yet-implemented",
            ),
            (
                "role R on M::Actor;",
                "role R on Nope::Actor;",
                "missing_declaration",
                "missing-name",
            ),
            (
                "attempt Tried by R",
                "attempt Tried by Q",
                "missing_declaration",
                "missing-name",
            ),
            (
                "(tried: Boolean)",
                "(tried: Undeclared)",
                "missing_declaration",
                "missing-name",
            ),
            (
                "using v over",
                "using M over",
                "missing_declaration",
                "missing-name",
            ),
            (
                "role R on M::Actor;",
                "role R on M::Actor;\nrole R on M::Actor;",
                "ambiguous_declaration",
                "ambiguous-name",
            ),
        ];
        for (from, to, code, tag) in cases {
            let source = base.replacen(from, to, 1);
            assert_ne!(source, base, "{from}");
            let (_, found) = refusals(&source);
            assert!(!found.is_empty(), "{source}");
            for refusal in &found {
                assert_eq!(refusal.cause.code().as_str(), code, "{source}: {found:?}");
                assert_eq!(refusal.cause.cause(), Some(tag), "{source}: {found:?}");
            }
        }
    }

    /// SR-770 FND-005: a `contracts` entry naming two state clauses of one
    /// name never binds the first silently. The whole pipeline already
    /// refuses the duplicate clause names themselves (`AmbiguousName` at
    /// both clauses, before any protocol is checked); `check`'s own binding
    /// refuses `ambiguous_declaration`/`ambiguous-name` at the entry too,
    /// naming both clauses, so it never relies on that earlier stage.
    #[trace("TC-513", "FR-114-AC-3")]
    #[test]
    fn a_contract_entry_naming_two_clauses_of_one_name_refuses() {
        let source = format!(
            "post Dup using v on M::Actor::op {{ true }}\n\
             post Dup using v on M::Actor::op {{ false }}\n{}",
            attempt_flow("Flow", "Dup")
        );
        let (_, found) = refusals(&source);
        assert!(
            !found.is_empty()
                && found
                    .iter()
                    .all(|refusal| matches!(&refusal.cause, CheckCause::AmbiguousName { name, .. } if name == "Dup")),
            "{found:?}"
        );

        let (_, assembled) = assemble(&source);
        let (scope, signatures) = empty_scope_and_signatures();
        let refused = super::check(
            &assembled.protocols[0],
            &std::collections::BTreeMap::new(),
            &std::collections::BTreeSet::new(),
            &scope,
            &signatures,
            &assembled.protocol_attempts[0],
            &assembled.state_clauses,
        )
        .map(|_| ())
        .expect_err("an ambiguous contracts entry refuses");
        let [cause] = protocol_causes(&refused)[..] else {
            panic!("one refusal: {refused:?}");
        };
        match cause {
            ProtocolAnchorCause::Ambiguous { name, loci, .. } => {
                assert_eq!(name, "Dup");
                assert_eq!(loci.len(), 2, "{loci:?}");
            }
            other => panic!("an Ambiguous cause, got {other:?}"),
        }
    }

    /// FR-114-AC-3 (TC-513): a `contracts` entry naming no state clause of
    /// the unit refuses `missing_declaration`/`missing-name` at the entry.
    #[trace("TC-513", "FR-114-AC-3")]
    #[test]
    fn a_missing_contract_entry_refuses_naming_the_entry() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "attempt Tried by R on M::Actor::op contracts []",
            "attempt Tried by R on M::Actor::op contracts [Absent]",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "missing_declaration");
        assert_eq!(found[0].cause.cause(), Some("missing-name"));
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::MissingContract { entry, .. } => assert_eq!(entry, "Absent"),
            other => panic!("a MissingContract cause, got {other:?}"),
        }
    }

    /// FR-114-AC-3 (TC-513): a `contracts` entry naming an invariant (which
    /// names no operation at all) refuses `wrong_snapshot`/`wrong-anchor`,
    /// naming both the attempt's own anchor and the invariant's.
    #[trace("TC-513", "FR-114-AC-3")]
    #[test]
    fn a_contract_entry_naming_an_invariant_refuses() {
        let source = format!(
            "invariant ParentOrder using v on M::Actor at current {{ true }}\n{}",
            recovery_flow("Main::Committed", "").replacen(
                "attempt Tried by R on M::Actor::op contracts []",
                "attempt Tried by R on M::Actor::op contracts [ParentOrder]",
                1,
            )
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "wrong_snapshot");
        assert_eq!(found[0].cause.cause(), Some("wrong-anchor"));
        match protocol_causes(&found)[0] {
            ProtocolAnchorCause::WrongContractAnchor {
                entry,
                attempt_anchor,
                clause_anchor,
                ..
            } => {
                assert_eq!(entry, "ParentOrder");
                assert_eq!(attempt_anchor, "M::Actor::op");
                assert_eq!(clause_anchor, "M::Actor");
            }
            other => panic!("a WrongContractAnchor cause, got {other:?}"),
        }
    }

    /// FR-114-AC-3 (TC-513, SR-771 FND-003): a `contracts` entry naming a
    /// `pre` clause of a *different* operation (`M::Actor::other`, not the
    /// attempt's own `M::Actor::op`) refuses `wrong_snapshot`/`wrong-anchor`,
    /// naming both operation anchors.
    #[trace("TC-513", "FR-114-AC-3")]
    #[test]
    fn a_contract_entry_anchored_at_a_different_operation_refuses() {
        let source = format!(
            "pre ProbePre using v on M::Actor::other {{ true }}\n{}",
            attempt_flow("Flow", "ProbePre")
        );
        let (refusal, cause) = only_cause(&source);
        assert_eq!(refusal.cause.code().as_str(), "wrong_snapshot");
        assert_eq!(refusal.cause.cause(), Some("wrong-anchor"));
        match cause {
            ProtocolAnchorCause::WrongContractAnchor {
                entry,
                attempt_anchor,
                clause_anchor,
                ..
            } => {
                assert_eq!(entry, "ProbePre");
                assert_eq!(attempt_anchor, "M::Actor::op");
                assert_eq!(clause_anchor, "M::Actor::other");
            }
            other => panic!("a WrongContractAnchor cause, got {other:?}"),
        }
    }

    /// FR-114-AC-2 (TC-513, SR-771 FND-002): `contracts []` still binds the
    /// attempt's operation -- its checked attempt carries an anchor and a
    /// frame -- with an empty `contracts` list and no refusal, while the
    /// same protocol naming a `post` clause of that operation binds exactly
    /// that clause's identity.
    #[trace("TC-513", "FR-114-AC-2")]
    #[test]
    fn an_empty_contracts_list_binds_with_no_refusal() {
        let empty = checks(&attempt_flow("Flow", ""));
        let attempt = &empty.protocols()[0].attempts[0];
        assert!(attempt.contracts.is_empty(), "{attempt:?}");

        let named = checks(&format!(
            "post Done using v on M::Actor::op {{ true }}\n{}",
            attempt_flow("Flow", "Done")
        ));
        let attempt = &named.protocols()[0].attempts[0];
        assert_eq!(attempt.contracts, [named.state_clauses()[0].identity()]);
    }

    /// The `Shadow` cause of `refusals`, expecting exactly one.
    fn shadow_cause(refusals: &[CheckRefusal]) -> (&str, &ShadowedDeclaration) {
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        match protocol_causes(refusals)[0] {
            ProtocolAnchorCause::Shadow { name, shadowed, .. } => (name, shadowed),
            other => panic!("a Shadow cause, got {other:?}"),
        }
    }

    /// FR-113-AC-5 (TC-512), first case: a `capture` named `forward` inside
    /// `Undo`, where `forward` is already the template's own bound
    /// parameter (`as (forward: Boolean)`, renamed from the fixture's own
    /// `failure`), refuses `ambiguous_declaration`/`ambiguous-name` at the
    /// capture, naming the parameter it shadows.
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn a_capture_shadowing_its_templates_own_bound_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
            .replacen(
                "clock \"ticks\" {\nactivate first",
                "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
                1,
            );
        assert!(
            source.contains("capture forward: Boolean = true;"),
            "fixture sanity: the capture must be inserted"
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
        assert_eq!(found[0].cause.cause(), Some("ambiguous-name"));
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "forward");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// FR-113-AC-5 (TC-512), second case: an event record binder named
    /// after the unit's own `v` profile alias (FR-113 "Refusals" reaches a
    /// model or profile alias the same way; this fixture declares no
    /// `model`, so its `v` profile alias stands in for a model alias)
    /// refuses at the binder, naming the alias. A second, unrelated
    /// protocol in the same unit whose own record binder is also `v`
    /// checks (FR-113 "Refusals": "Two separate protocols may reuse a
    /// binder name" -- this asserts the alias/native-declaration half is
    /// package-wide while a binder's own shadow check stays local to its
    /// protocol).
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn a_record_binder_shadowing_the_units_profile_alias_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (applied: Boolean)",
            "as (v: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].cause.code().as_str(), "ambiguous_declaration");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "v");
        assert!(
            matches!(shadowed, ShadowedDeclaration::ProfileAlias),
            "{shadowed:?}"
        );

        // A second protocol, naming its own `effect`'s record binder `v`
        // too: this protocol's own anchors and declarations are entirely
        // separate from the first, but `v` is still package-wide, so it
        // still refuses -- the "two separate protocols" carve-out is about
        // reusing a binder name *between* protocols, not about escaping the
        // package-level alias/native-declaration check.
        let second = recovery_flow("Main::Committed", "")
            .replacen("protocol RecoveryFlow", "protocol RecoveryFlowTwo", 1)
            .replacen("as (applied: Boolean)", "as (v: Boolean)", 1);
        let combined = format!("{source}\n{second}");
        let (_, found) = refusals(&combined);
        let shadow_causes: Vec<&ProtocolAnchorCause> = protocol_causes(&found)
            .into_iter()
            .filter(|cause| matches!(cause, ProtocolAnchorCause::Shadow { .. }))
            .collect();
        assert_eq!(shadow_causes.len(), 2, "{shadow_causes:?}");
        for cause in shadow_causes {
            match cause {
                ProtocolAnchorCause::Shadow { name, shadowed, .. } => {
                    assert_eq!(name, "v");
                    assert!(matches!(shadowed, ShadowedDeclaration::ProfileAlias));
                }
                other => panic!("a Shadow cause, got {other:?}"),
            }
        }
    }

    /// A `compensate` declaration's `activate first (p)` trigger parameter
    /// shadowing a native `predicate` declaration of the package (FR-113
    /// "Refusals": a native declaration is checked the same way an alias
    /// is, `ShadowedDeclaration::Function`).
    #[trace("TC-512")]
    #[test]
    fn a_trigger_parameter_shadowing_a_native_function_declaration_refuses() {
        let source = format!(
            "predicate forward using v(): Boolean {{ true }}\n{}",
            recovery_flow("Main::Committed", "").replacen(
                "(trigger: Boolean)",
                "(forward: Boolean)",
                1
            )
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "forward");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Function),
            "{shadowed:?}"
        );
    }

    /// A `retry (a, b)` parameter shadowing another binder visible where it
    /// is declared: `Undo`'s own record binder, renamed `shared`, and
    /// `retry`'s first parameter, also renamed `shared` -- both inside the
    /// same `compensate` declaration, the retry parameter one scope level
    /// inside the declaration's own record binder.
    #[trace("TC-512")]
    #[test]
    fn a_retry_parameter_shadowing_its_declarations_own_record_binder_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("as (failure: Boolean)", "as (shared: Boolean)", 1)
            .replacen("current_attempt: Boolean", "shared: Boolean", 1);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "shared");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// A bare `commit` control's own record binder shadowing the unit's `v`
    /// profile alias, spreading FR-113-AC-5's coverage across a binder
    /// position AC-5's own fixture does not exercise.
    #[trace("TC-512")]
    #[test]
    fn a_commit_controls_record_binder_shadowing_an_alias_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (committed: Boolean)",
            "as (v: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "v");
        assert!(
            matches!(shadowed, ShadowedDeclaration::ProfileAlias),
            "{shadowed:?}"
        );
    }

    /// FR-113-AC-6 (TC-512 step 3, exactly): `effect Applied of Absent` (a
    /// missing member, TC-512's own step-3 fixture, physically inside
    /// `run`) combined with the shadowing `forward` capture (TC-512 step
    /// 2's own fixture: `Undo` binds `forward` as its record binder, and a
    /// `capture forward = ...;` inside it shadows that binder), physically
    /// inside `compensate Undo`, so earlier in source. Both refusals
    /// report, ordered by source position (SR-766 FND-004: this fixture
    /// used to differ from TC-512 step 3's own procedure -- a
    /// `Main::Missing` anchor and a `v`-named record binder shadowing the
    /// unit's own alias, neither of which step 3 names -- so it is now the
    /// same fixture step 3 describes). Checking it twice gives the same
    /// refusals in the same order, and no checked protocol node emits (the
    /// full pipeline refuses).
    #[trace("TC-512", "FR-113-AC-6")]
    #[test]
    fn a_missing_anchor_and_a_later_shadowing_binder_report_in_source_order() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("effect Applied of Tried", "effect Applied of Absent", 1)
            .replacen("as (failure: Boolean)", "as (forward: Boolean)", 1)
            .replacen(
                "clock \"ticks\" {\nactivate first",
                "clock \"ticks\" {\ncapture forward: Boolean = true;\nactivate first",
                1,
            );
        let (text, refusals_a) = refusals(&source);
        let (_, refusals_b) = refusals(&source);
        assert_eq!(refusals_a, refusals_b, "checking twice must agree");
        assert_eq!(refusals_a.len(), 2, "{refusals_a:?}");

        let shadow_span = text.find("capture forward").unwrap();
        let missing_span = text.find("effect Applied of Absent").unwrap();
        assert!(
            shadow_span < missing_span,
            "fixture sanity: the shadowing capture must come first in source"
        );
        let causes = protocol_causes(&refusals_a);
        assert!(
            matches!(causes[0], ProtocolAnchorCause::Shadow { .. }),
            "the earliest refusal must be the Shadow one, {:?}",
            refusals_a[0]
        );
        assert!(
            matches!(causes[1], ProtocolAnchorCause::Missing { .. }),
            "the later refusal must be the Missing one, {:?}",
            refusals_a[1]
        );
    }

    /// SR-765 FND-001: FR-113's binder no-shadowing rule is protocol-wide
    /// (QSpec `shared-grammar.md`: binders are "unique in their enclosing
    /// declaration"), not merely within scopes that lexically nest one
    /// inside the other. Three cases the old lexical-prefix check missed,
    /// each a fresh `RecoveryFlow`-shaped protocol so one refusal per case
    /// stays unambiguous:
    ///
    /// - `finish`'s own record binder reusing `run`'s own `attempt`
    ///   binder's name (neither scope is a prefix of the other: `finish` is
    ///   top-level, `attempt` is scoped `["Main"]`).
    /// - A `sequence Inner` inside `Main`, whose own `effect`'s record
    ///   binder reuses a name already bound by `Main`'s own outer `effect`.
    /// - Two sibling `case`s of one `choice`, each binding an event record
    ///   binder of the same name (neither `case`'s scope is a prefix of the
    ///   other's).
    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_finish_binder_reusing_a_run_binders_name_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (outcome: Boolean)",
            "as (tried: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_nested_sequences_binder_reusing_an_outer_siblings_name_refuses() {
        let inner = "sequence Inner {\n\
             event Recovered2 by R for Undo as (tried: Boolean) { true };\n\
             }\n";
        let source = recovery_flow("Main::Committed", inner);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-512", "FR-113")]
    #[test]
    fn sibling_case_binders_of_one_name_refuse() {
        let choice = "choice Which by R visible () {\n\
             case left when { true } sequence Left {\n\
             event LeftSeen by R for Undo as (dup: Boolean) { true };\n\
             }\n\
             case right when { false } sequence Right {\n\
             event RightSeen by R for Undo as (dup: Boolean) { true };\n\
             }\n\
             }\n";
        let source = recovery_flow("Main::Committed", choice);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "dup");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// SR-765 FND-002: the protocol's own `over (p)` input parameter and
    /// `activation on each (p)` parameter are binders too, visible
    /// everywhere in the protocol (`BinderKind::Input` and
    /// `BinderKind::ActivationParameter`); a record binder reusing either
    /// name refuses.
    #[trace("TC-510", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_the_protocols_own_input_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "").replacen(
            "as (tried: Boolean)",
            "as (input: Boolean)",
            1,
        );
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "input");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    #[trace("TC-510", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_the_protocols_own_activation_parameter_refuses() {
        let source = recovery_flow("Main::Committed", "")
            .replacen("on origin", "on each (activated: Boolean) when (true)", 1)
            .replacen("as (tried: Boolean)", "as (activated: Boolean)", 1);
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "activated");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Binder(_)),
            "{shadowed:?}"
        );
    }

    /// SR-765 FND-003: a `dimension`/`unit` declaration participates in the
    /// package-wide half of FR-113's no-shadowing rule the same way an
    /// alias or a function does, not just `Scope::named_types`'s composites,
    /// enums and object types.
    #[trace("TC-512", "FR-113")]
    #[test]
    fn a_record_binder_shadowing_a_native_dimension_declaration_refuses() {
        let source = format!("dimension tried;\n{}", recovery_flow("Main::Committed", ""));
        let (_, found) = refusals(&source);
        assert_eq!(found.len(), 1, "{found:?}");
        let (name, shadowed) = shadow_cause(&found);
        assert_eq!(name, "tried");
        assert!(
            matches!(shadowed, ShadowedDeclaration::Quantity),
            "{shadowed:?}"
        );
    }

    // SR-766 FND-001's model-alias regression (the `.chain(selections.
    // models...)` half of `PackageDeclarations::alias_names`'s assembly)
    // lives in `qsl-semantics/tests/it/model_operations.rs`
    // (`a_models_own_alias_is_recorded_in_the_packages_alias_names`), whose
    // existing `admit_and_assemble` pipeline admits a real domain package;
    // this module's own `assemble` test helper has no such admission and
    // refuses `UnadmittedModel` for any `model` declaration.

    /// SR-766 FND-002: AC-5's own "two separate protocols" carve-out, tested
    /// directly rather than only implicitly (the pre-existing test's second
    /// protocol reused the alias `v` itself, so the carve-out was never
    /// actually exercised on an ordinary, non-alias binder name). Two
    /// protocols in one unit, each with a record binder named `shared` (not
    /// an alias or any native declaration): neither refuses, since a
    /// binder's own protocol-wide uniqueness check (SR-765 FND-001) stays
    /// scoped to its own protocol.
    #[trace("TC-512", "FR-113-AC-5")]
    #[test]
    fn two_protocols_sharing_an_ordinary_binder_name_both_check() {
        let combined = shared_binder_protocols();
        let checked = checks(&combined);
        let names: Vec<&str> = checked
            .protocols()
            .iter()
            .map(|protocol| protocol.name.as_str())
            .collect();
        assert_eq!(names, ["First", "Second"]);
    }

    /// FR-114-AC-3 (TC-513, SR-771 FND-003): an attempt `on
    /// M::Actor::missing`, an operation `M::Actor` does not declare, refuses
    /// `missing_declaration`/`missing-name` at `missing` at assembly, the
    /// same resolution (and refusal) a `pre`/`post` clause's own operation
    /// gets (FR-104).
    #[trace("TC-513", "FR-114-AC-3")]
    #[test]
    fn an_attempt_naming_a_missing_operation_refuses_at_its_name() {
        let source = attempt_flow("Flow", "").replacen("M::Actor::op", "M::Actor::missing", 1);
        let errors = assembly_errors(&source);
        let [error] = errors.as_slice() else {
            panic!("one error: {errors:?}");
        };
        match &error.cause {
            crate::check::AssemblyCause::UnresolvedOperation { context, operation } => {
                assert_eq!(context, "M::Actor");
                assert_eq!(operation, "missing");
            }
            other => panic!("an UnresolvedOperation cause, got {other:?}"),
        }
        let text = format!("{HEADER}{source}\n");
        assert_eq!(&text[error.span.start..error.span.end], "missing");
        assert_eq!(error.cause.code().as_str(), "missing_declaration");
    }

    /// SR-770 FND-006: a unit with a bad state clause and a bad attempt
    /// reports both in one assembly refusal, not only the clause.
    #[trace("TC-513", "FR-114")]
    #[test]
    fn a_bad_clause_and_a_bad_attempt_refuse_together_at_assembly() {
        let source = format!(
            "post Broken using v on M::Actor::absent {{ true }}\n{}",
            attempt_flow("Flow", "").replacen("M::Actor::op", "M::Actor::missing", 1)
        );
        let errors = assembly_errors(&source);
        let operations: Vec<&str> = errors
            .iter()
            .filter_map(|error| match &error.cause {
                crate::check::AssemblyCause::UnresolvedOperation { operation, .. } => {
                    Some(operation.as_str())
                }
                _ => None,
            })
            .collect();
        assert_eq!(operations, ["absent", "missing"], "{errors:?}");
    }
}
