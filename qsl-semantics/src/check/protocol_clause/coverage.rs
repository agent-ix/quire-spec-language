//! The part of a protocol's content [`check`](super::check) does not read:
//! which constructs are not yet checked, and the grammar's required
//! `using`, role and binder-type resolution.

use std::collections::BTreeMap;

use qsl_forms::{ProtocolConstructKind, ProtocolDeclarationForm, ProtocolNodeKind};
use qsl_foundation::Span;

use super::refusal;
use crate::check::refusal::{AliasKind, CheckRefusal, ProtocolAnchorCause};
use crate::check::Scope;

/// Whether this checker covers a static node of `kind`: every node kind of
/// the `run` tree is checked (`graph`, `protocol_controls`, `protocol_slots`,
/// `protocol_obligation`) except a `fence`, whose memory semantics nothing
/// checks yet. Exhaustive, so a new node kind must be classified here.
fn covered_kind(kind: ProtocolNodeKind) -> bool {
    use ProtocolNodeKind as K;
    match kind {
        K::Sequence
        | K::Parallel
        | K::Branch
        | K::Attempt
        | K::Finish
        | K::CompensateTemplate
        | K::Send
        | K::Receive
        | K::Effect
        | K::Event
        | K::Commit
        | K::Choice
        | K::Repeat
        | K::Case
        | K::Await
        | K::Check => true,
        K::Fence => false,
    }
}

/// Whether a protocol construct of `kind` is checked: a protocol-level
/// `capture` and an `activation on each` are (FR-381, `protocol_obligation`),
/// a `channel` with its `fifo` key is (FR-218, `protocol_controls`), a
/// `terminal when` predicate is (FR-211, `protocol_slots`), a `requires
/// temporal` requirement is (FR-218, `protocol_requirement`), a
/// `relationship` is (FR-218, `protocol_relationship`) and a `related by`
/// clause is (FR-218, `protocol_slots`). Exhaustive, so a new construct kind
/// must be classified here.
fn covered_construct(kind: ProtocolConstructKind) -> bool {
    use ProtocolConstructKind as K;
    match kind {
        K::Capture
        | K::ActivationEach
        | K::Channel
        | K::FifoOrdering
        | K::TerminalWhen
        | K::Requirement
        | K::Relationship
        | K::Related => true,
        K::ReplicatedRole | K::JoinPredicate | K::MemoryClause | K::AccessOrdering => false,
    }
}

/// The rest of a protocol's content, once
/// [`check`](super::check) has resolved its anchors, binders and attempt
/// bindings and compiled its control tree.
///
/// Every node kind and expression slot of the `run` tree but a `fence` is
/// checked (`graph`, `protocol_controls`, `protocol_slots`,
/// `protocol_obligation`).
/// A protocol holding a node kind [`covered_kind`] rejects (a `fence`) or
/// a [`qsl_forms::ProtocolConstructKind`] that no checker reads yet
/// ([`covered_construct`] says which are checked) refuses
/// `unsupported_construct`/`not-yet-implemented` once, at the earliest such
/// construct, rather than compiling with unchecked content that the emitted
/// package then silently omits (SR-753 FND-002, SR-761 FND-001).
///
/// A protocol made only of covered parts still has content the grammar
/// requires of every protocol, which is checked here: its `using` alias,
/// and each `check`'s and `await`'s, names a profile selection; each role's
/// object type resolves and no two roles share a name; each attempt's,
/// event's and commit's `by`, and each `choice`'s and `repeat`'s owner,
/// names a declared role; and each binder's declared type resolves against
/// the package `scope`. Refusals are in source position order.
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
        if scope.object_type_named(&role.context.name).is_none()
            && scope.named_types(&role.context.name).is_empty()
        {
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
    // FR-218: an `event` or `commit` written `by`, and a `choice` or
    // `repeat` owned `by`, a role the protocol declares.
    let owners = protocol
        .declarations
        .iter()
        .filter(|declaration| declaration.kind != ProtocolNodeKind::Attempt)
        .filter_map(|declaration| Some((&declaration.name.name, declaration.role.as_ref()?)))
        .chain(
            protocol
                .choices
                .iter()
                .map(|choice| (&choice.name.name, &choice.owner)),
        )
        .chain(
            protocol
                .repeats
                .iter()
                .map(|repeat| (&repeat.name.name, &repeat.owner)),
        );
    for (node, owner) in owners {
        if !roles.contains_key(owner.name.as_str()) {
            refusals.push((
                owner.span,
                refusal(ProtocolAnchorCause::NodeRole {
                    node: node.clone(),
                    name: owner.name.clone(),
                    span: owner.span,
                }),
            ));
        }
    }
    // A `check` and an `await` are written `using` a profile selection, as
    // the protocol itself is.
    let usings = protocol
        .checks
        .iter()
        .map(|check| &check.using)
        .chain(protocol.awaits.iter().map(|await_form| &await_form.using));
    for using in usings {
        if alias_names.get(&using.alias) != Some(&AliasKind::Profile) {
            refusals.push((
                using.span,
                refusal(ProtocolAnchorCause::MissingProfile {
                    alias: using.alias.clone(),
                    span: using.span,
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
