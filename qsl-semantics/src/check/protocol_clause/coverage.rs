//! The part of a protocol's content [`check`](super::check) does not read:
//! which constructs and expressions are not yet checked, and the grammar's
//! required `using`, role and binder-type resolution.

use std::collections::BTreeMap;

use qsl_forms::{
    BinaryOperator, ExprNode, ExpressionForm, ProtocolConstructKind, ProtocolDeclarationForm,
    ProtocolNodeKind,
};
use qsl_foundation::Span;

use super::refusal;
use crate::check::refusal::{AliasKind, CheckRefusal, ProtocolAnchorCause};
use crate::check::Scope;

/// Whether this checker covers every part of a static node of `kind`
/// other than its expressions (whose coverage [`uncovered_expressions`]
/// decides): its children and their causal edges (`graph`), its anchors
/// (FR-113), an `attempt`'s operation and `contracts` (FR-114), its role,
/// binder and channel ([`content`], `protocol_controls`), a `parallel`'s
/// join, a `choice`'s and a `repeat`'s owner and guards
/// (`protocol_decisions`) and a `compensate` template's stages (FR-381,
/// `protocol_obligation`). Every node kind is covered; the match stays
/// exhaustive, so a new node kind must be classified here.
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
    }
}

/// Whether `form` is a Boolean formula over literals alone: `true`,
/// `false`, `not`, `and`, `or` and `implies`. Such a guard reads no name,
/// so it is checked without the binder scopes and decision facts a guard
/// that reads one needs (FR-218 "Choice and repeat decisions"), which are
/// not checked yet.
pub(crate) fn constant_formula(form: &ExpressionForm) -> Option<bool> {
    // Children precede their parent in post-order, so one explicit stack
    // evaluates a formula of any depth.
    let mut values: BTreeMap<qsl_forms::ExprId, bool> = BTreeMap::new();
    let mut stack = vec![(form.expression.root(), false)];
    while let Some((node, expanded)) = stack.pop() {
        if !expanded {
            stack.push((node, true));
            for child in node.children() {
                stack.push((child, false));
            }
            continue;
        }
        let value = |id| values.get(&id).copied();
        let result = match node.node() {
            ExprNode::Boolean(value) => *value,
            ExprNode::Not(operand) => !value(*operand)?,
            ExprNode::Binary {
                operator,
                left,
                right,
            } => {
                let (left, right) = (value(*left)?, value(*right)?);
                match operator {
                    BinaryOperator::And => left && right,
                    BinaryOperator::Or => left || right,
                    BinaryOperator::Implies => !left || right,
                    _ => return None,
                }
            }
            _ => return None,
        };
        values.insert(node.id(), result);
    }
    values.get(&form.expression.root_id()).copied()
}

/// Whether `form` is an integer literal, negated or not: the one `variant`
/// that reads no name.
pub(crate) fn constant_integer(form: &ExpressionForm) -> bool {
    match form.expression.root_node() {
        ExprNode::Integer(_) => true,
        ExprNode::Negate(operand) => matches!(
            form.expression.get(*operand).map(|operand| operand.node()),
            Some(ExprNode::Integer(_))
        ),
        _ => false,
    }
}

/// The span of `form`'s whole expression.
fn span_of(form: &ExpressionForm) -> Span {
    form.spans.at(&[]).unwrap_or(Span { start: 0, end: 0 })
}

/// Every expression of a `choice` or `repeat` this checker does not check
/// yet, by span and keyword: a `visible` entry, and a guard, invariant or
/// variant that reads a name. Checking those needs the binder scopes and
/// the decision-fact rules of FR-218, which are not implemented.
fn uncovered_expressions(protocol: &ProtocolDeclarationForm) -> Vec<(Span, &'static str)> {
    let mut uncovered = Vec::new();
    for choice in &protocol.choices {
        if !choice.visible.is_empty() {
            uncovered.push((choice.visible_span, "visible"));
        }
        for case in &choice.cases {
            if constant_formula(&case.guard).is_none() {
                uncovered.push((span_of(&case.guard), "case guard"));
            }
        }
    }
    for repeat in &protocol.repeats {
        if !repeat.visible.is_empty() {
            uncovered.push((repeat.visible_span, "visible"));
        }
        if constant_formula(&repeat.guard).is_none() {
            uncovered.push((span_of(&repeat.guard), "repeat guard"));
        }
        if let Some(proof) = &repeat.loop_proof {
            if constant_formula(&proof.invariant).is_none() {
                uncovered.push((span_of(&proof.invariant), "invariant"));
            }
            if !constant_integer(&proof.variant) {
                uncovered.push((span_of(&proof.variant), "variant"));
            }
        }
    }
    uncovered
}

/// Whether a protocol construct of `kind` is checked: a protocol-level
/// `capture` and an `activation on each` are (FR-381, `protocol_obligation`),
/// and a `channel` with its `fifo` key is (FR-218, `protocol_controls`).
/// Exhaustive, so a new construct kind must be classified here.
fn covered_construct(kind: ProtocolConstructKind) -> bool {
    use ProtocolConstructKind as K;
    match kind {
        K::Capture | K::ActivationEach | K::Channel | K::FifoOrdering => true,
        K::ReplicatedRole
        | K::Relationship
        | K::Requirement
        | K::Related
        | K::JoinPredicate
        | K::TerminalWhen => false,
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
        .chain(uncovered_expressions(protocol))
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
