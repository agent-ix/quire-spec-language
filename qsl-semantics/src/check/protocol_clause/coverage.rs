//! The part of a protocol's content [`check`](super::check) does not read:
//! which node kinds and constructs are not yet checked, and the grammar's
//! required `using`, role and binder-type resolution.

use std::collections::BTreeMap;

use qsl_forms::{ProtocolConstructKind, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal;
use crate::check::protocol_obligation::role_model_refusal;
use crate::check::refusal::{AliasKind, CheckRefusal, ProtocolAnchorCause};
use crate::check::Scope;

/// Whether this checker covers every part of a static node of `kind`:
/// a `sequence` (a name and its children), an `attempt` (its
/// operation and `contracts`, FR-114; its role, binder and body are checked
/// by [`content`]), the `finish` node (its binder and body, the same) and a
/// `compensate` template (its stages, FR-381, are checked by
/// `protocol_obligation`). Every other kind carries content -- a channel, a
/// guard, a join policy, an anchor's target semantics -- that nothing checks
/// yet. Exhaustive, so a new node kind must be classified here.
fn covered_kind(kind: ProtocolNodeKind) -> bool {
    use ProtocolNodeKind as K;
    match kind {
        K::Sequence | K::Attempt | K::Finish | K::CompensateTemplate => true,
        K::Send
        | K::Receive
        | K::Effect
        | K::Event
        | K::Commit
        | K::Choice
        | K::Parallel
        | K::Repeat
        | K::Branch
        | K::Case
        | K::Await
        | K::Check => false,
    }
}

/// Whether a protocol construct of `kind` is checked: a protocol-level
/// `capture` and an `activation on each` are (FR-381, `protocol_obligation`).
/// Exhaustive, so a new construct kind must be classified here.
fn covered_construct(kind: ProtocolConstructKind) -> bool {
    use ProtocolConstructKind as K;
    match kind {
        K::Capture | K::ActivationEach => true,
        K::ReplicatedRole | K::Relationship | K::Channel | K::Requirement | K::Related => false,
    }
}

/// The rest of a protocol's content, once
/// [`check`](super::check) has resolved its anchors, binders and attempt bindings.
///
/// FR-113 and FR-114 check only those three parts. A protocol holding
/// anything else -- a node kind [`covered_kind`] rejects, any
/// [`qsl_forms::ProtocolConstructKind`] ([`covered_construct`] says which are
/// checked), or a body
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
                .filter(|construct| !covered_construct(construct.kind))
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
            // A name that binds another kind of type is a model that is not
            // an object type (FR-218-AC-24); one that binds nothing is
            // missing.
            let refused = if scope.named_types(&role.context.name).is_empty() {
                refusal(ProtocolAnchorCause::MissingRoleType {
                    role: role.name.name.clone(),
                    context: role.context.name.clone(),
                    span: role.context.span,
                })
            } else {
                role_model_refusal(role)
            };
            refusals.push((role.context.span, refused));
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
        if let Err(error) = crate::check::type_form::resolve_form(scope, &binder.value_type) {
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
