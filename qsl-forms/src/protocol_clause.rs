// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `ProtocolClause` family's one S2 production (FR-102, ADR-012 §15.2,
//! §15.3 stage table row S2): `invariant`/`pre`/`post` state clauses.
//!
//! This module reads the CST of one `StateClause` declaration and builds
//! its [`StateClauseForm`]. It resolves no name and mints no identity, the
//! same contract `value`'s productions keep (FR-091): the `using` alias,
//! the model context and the operation member stay spelled and spanned,
//! unresolved, for the assembler and S3 (FR-104) to read. It shares the
//! `Value` family's own CST-walking helpers (`items`, `only`, `text`, ...)
//! and its one `Expression` builder, since the `Expression` enum and its
//! builder are defined once, in the `forms` core (ADR-012 §4.3): only the
//! declaration-level production is `ProtocolClause`'s own.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsFailure, FormsLimits};
use super::spans::DeclarationSpans;
use super::syntax::{
    AnchorForm, AnchorSegment, AnchorSite, DeclarationForm, DeclaredName, ProtocolDeclarationForm,
    ProtocolNodeDeclaration, ProtocolNodeKind, ScopeName, ScopedAnchorForm, StateClauseForm,
    StateClauseKind, UsingAlias,
};
use super::value::{
    declared_name, expression, items, name_form, nodes_of, only, production_node, text, tokens_of,
    unexpected, Item,
};

/// `invariant N using p on M::T at current { e }`, `pre N using p on
/// M::T::op { e }` or `post N using p on M::T::op { e }` (FR-102 "Behavior"):
/// the kind is read from the leading token alone, and the body's nesting
/// depth is bound exactly as a function body's is (`value::expression`
/// applies the same [`super::dispatch::FormsLimits`]).
pub(crate) fn state_clause(construct: Construct<'_>) -> Result<DeclarationForm, FormsFailure> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let kind = match clause_items.first() {
        Some(Item::Token(token)) if token.spelling() == b"invariant" => StateClauseKind::Invariant,
        Some(Item::Token(token)) if token.spelling() == b"pre" => StateClauseKind::Precondition,
        Some(Item::Token(token)) if token.spelling() == b"post" => StateClauseKind::Postcondition,
        Some(Item::Token(_) | Item::Node(_)) | None => return Err(unexpected(node)),
    };
    let identifiers = tokens_of(&clause_items, TokenKind::Identifier);
    let [name, alias] = identifiers.as_slice() else {
        return Err(unexpected(node));
    };
    let (context, operation) = match kind {
        StateClauseKind::Invariant => {
            let type_name = only(&clause_items, Production::TypeName, node)?;
            (name_form(cst, type_name)?, None)
        }
        StateClauseKind::Precondition | StateClauseKind::Postcondition => {
            let operation_name = only(&clause_items, Production::OperationName, node)?;
            let operation_items = items(cst, operation_name);
            let type_name = only(&operation_items, Production::TypeName, operation_name)?;
            let context = name_form(cst, type_name)?;
            let member = tokens_of(&operation_items, TokenKind::Identifier)
                .first()
                .copied()
                .ok_or_else(|| unexpected(operation_name))?;
            let operation = DeclaredName {
                name: text(member, operation_name)?,
                span: member.span(),
            };
            (context, Some(operation))
        }
    };
    let block = only(&clause_items, Production::Block, node)?;
    let block_items = items(cst, block);
    let body_node = only(&block_items, Production::Expression, block)?;
    let (body, body_spans) = expression(cst, body_node, construct.limits)?;
    let form = StateClauseForm {
        kind,
        name: DeclaredName {
            name: text(name, node)?,
            span: name.span(),
        },
        profile: UsingAlias {
            alias: text(alias, node)?,
            span: alias.span(),
        },
        context,
        operation,
        body,
        spans: DeclarationSpans {
            declaration: construct.node.span(),
            body: body_spans,
            measure: None,
        },
    };
    Ok(DeclarationForm::StateClause(Box::new(form)))
}

/// `protocol Name using p over (params) activation { ... run Control Finish
/// }` (FR-112, ADR-012 §12.2 Form row). S2 builds only the declaration's
/// scoped anchors, in source order of their references: the `for` and
/// `commit` references of every top-level `compensate` declaration (empty
/// scope), then the references the `run` control tree holds, each with the
/// names of its enclosing named controls. Every other part of the
/// declaration (roles, channels, requirements, the control tree's own
/// shape) is read only to walk past it: FR-112 gives this stage no reason
/// to keep it.
pub(crate) fn protocol_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsFailure> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let clause_items = items(cst, node);
    let name = declared_name(&clause_items, node)?;

    let mut scoped_anchors = Vec::new();
    let mut declarations = Vec::new();
    for compensation in nodes_of(&clause_items, Production::Compensation) {
        compensation_anchors(cst, compensation, &mut scoped_anchors, &mut declarations)?;
    }
    let run = only(&clause_items, Production::Control, node)?;
    control_anchors(
        cst,
        run,
        &mut Vec::new(),
        &mut scoped_anchors,
        &mut declarations,
        construct.limits,
        1,
    )?;
    let finish = only(&clause_items, Production::Finish, node)?;
    let finish_items = items(cst, finish);
    declarations.push(ProtocolNodeDeclaration {
        kind: ProtocolNodeKind::Finish,
        name: declared_name(&finish_items, finish)?,
        scope: Vec::new(),
        channel: None,
    });

    Ok(DeclarationForm::Protocol(ProtocolDeclarationForm {
        name,
        scoped_anchors,
        declarations,
    }))
}

/// `compensate N for R as (p) by role on Op using alias clock "c" { ...
/// commit (R2 | never); ... }`: the template's own declaration (FR-113
/// Inputs: the top level declares the protocol's `compensate` templates),
/// the `for` reference (`compensate-for`, FR-112 site list) and, when
/// `commit` names a reference rather than `never`, the `commit` reference
/// (`compensate-commit`). All three are protocol-level: neither is written
/// inside `run`'s control tree, so all carry the empty scope.
fn compensation_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
) -> Result<(), FormsFailure> {
    let compensation_items = items(cst, node);
    out_decls.push(ProtocolNodeDeclaration {
        kind: ProtocolNodeKind::CompensateTemplate,
        name: declared_name(&compensation_items, node)?,
        scope: Vec::new(),
        channel: None,
    });
    let references = nodes_of(&compensation_items, Production::NodeReference);
    let for_reference = references.first().ok_or_else(|| unexpected(node))?;
    out.push(scoped_anchor(
        cst,
        for_reference,
        AnchorSite::CompensateFor,
        &[],
        None,
    )?);
    // `commit never;` names no reference: `references` then holds only the
    // `for` target, and this site builds no anchor (FR-112 "Behavior").
    if let Some(commit_reference) = references.get(1) {
        out.push(scoped_anchor(
            cst,
            commit_reference,
            AnchorSite::CompensateCommit,
            &[],
            None,
        )?);
    }
    Ok(())
}

/// Runs `body` with `name` pushed onto `scope` as one more enclosing named
/// control, popping it again afterward -- even when `body` refuses -- so
/// every one of this walk's named-control shapes (`sequence`, `choice`,
/// `parallel`, `repeat`, `case`, `branch`, `await`) pushes and pops through
/// this single place instead of its own copy of the pattern (a missed push
/// in one copy is exactly how FND-001/SR-753 left `await`'s own name out
/// of its children's scope).
fn with_scope<F>(
    scope: &mut Vec<ScopeName>,
    name: DeclaredName,
    body: F,
) -> Result<(), FormsFailure>
where
    F: FnOnce(&mut Vec<ScopeName>) -> Result<(), FormsFailure>,
{
    scope.push(ScopeName {
        name: name.name,
        span: name.span,
    });
    let result = body(scope);
    scope.pop();
    result
}

/// Walks one `Control` node: unwraps it to the alternative it actually
/// matched (`Sequence`, `Parallel`, ..., `EventNode`) and dispatches on
/// that alternative's own production.
///
/// `depth` is charged against `limits.nesting_depth`, the same S2 bound
/// `value::expression` applies to an expression tree (a root is at depth
/// 1, each nested `Control` one deeper): `await ... then <Control>` and
/// `repeat ... exhausted <Control>` nest with no bracket, so the CST's own
/// nesting ceiling does not bound them, and unbounded native recursion on
/// untrusted source would abort the process rather than refuse.
fn control_anchors(
    cst: &LosslessCst,
    control: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
    limits: FormsLimits,
    depth: u64,
) -> Result<(), FormsFailure> {
    let matched = items(cst, control)
        .into_iter()
        .find_map(|item| match item {
            Item::Node(node) => Some(node),
            Item::Token(_) => None,
        })
        .ok_or_else(|| unexpected(control))?;
    if depth > limits.nesting_depth {
        return Err(FormsFailure::depth(limits, matched.span()));
    }
    // Every arm below declares `matched` itself, directly in `scope` (the
    // scope enclosing it, before its own name -- if any -- is pushed): this
    // is the single place a control becomes a name other scopes and
    // anchors can resolve (FR-113 Inputs).
    match matched.production() {
        Production::Sequence => {
            let sequence_items = items(cst, matched);
            let name = declared_name(&sequence_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Sequence, name.clone(), scope);
            with_scope(scope, name, |scope| {
                for child in nodes_of(&sequence_items, Production::Control) {
                    control_anchors(cst, child, scope, out, out_decls, limits, depth + 1)?;
                }
                Ok(())
            })
        }
        Production::Choice => {
            let choice_items = items(cst, matched);
            let name = declared_name(&choice_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Choice, name.clone(), scope);
            with_scope(scope, name, |scope| {
                for case in nodes_of(&choice_items, Production::Case) {
                    case_anchors(cst, case, scope, out, out_decls, limits, depth)?;
                }
                Ok(())
            })
        }
        Production::Parallel => {
            let parallel_items = items(cst, matched);
            let name = declared_name(&parallel_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Parallel, name.clone(), scope);
            with_scope(scope, name, |scope| {
                for branch in nodes_of(&parallel_items, Production::Branch) {
                    branch_anchors(cst, branch, scope, out, out_decls, limits, depth)?;
                }
                Ok(())
            })
        }
        Production::Repetition => {
            let repetition_items = items(cst, matched);
            let name = declared_name(&repetition_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Repeat, name.clone(), scope);
            with_scope(scope, name, |scope| {
                for child in nodes_of(&repetition_items, Production::Control) {
                    control_anchors(cst, child, scope, out, out_decls, limits, depth + 1)?;
                }
                Ok(())
            })
        }
        Production::AwaitControl => {
            await_control_anchors(cst, matched, scope, out, out_decls, limits, depth)
        }
        Production::EventNode => event_node_anchors(cst, matched, scope, out, out_decls),
        Production::Check => {
            let check_items = items(cst, matched);
            let name = declared_name(&check_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Check, name, scope);
            // `check` holds no `NodeReference` and encloses no further
            // control (FR-112 "Behavior": "No other position yields one").
            Ok(())
        }
        Production::Commit => {
            let commit_items = items(cst, matched);
            let name = declared_name(&commit_items, matched)?;
            declare(out_decls, ProtocolNodeKind::Commit, name, scope);
            Ok(())
        }
        _ => Err(unexpected(matched)),
    }
}

/// Records `name` as a static node `kind` declares directly in `scope`
/// (FR-113 Inputs), with no channel: every kind but `send` and `receive`
/// carries none.
fn declare(
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
    kind: ProtocolNodeKind,
    name: DeclaredName,
    scope: &[ScopeName],
) {
    out_decls.push(ProtocolNodeDeclaration {
        kind,
        name,
        scope: scope.to_vec(),
        channel: None,
    });
}

/// `case N when { c } Control`: the case's own name encloses its one
/// control, the same shape a `branch` gives a `parallel` (FR-112
/// "Outputs": "the named controls that enclose the reference"). `depth` is
/// the enclosing `choice`'s own depth: the case's inner `Control` is one
/// level deeper, charged where [`control_anchors`] recurses into it.
fn case_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
    limits: FormsLimits,
    depth: u64,
) -> Result<(), FormsFailure> {
    let case_items = items(cst, node);
    let name = declared_name(&case_items, node)?;
    declare(out_decls, ProtocolNodeKind::Case, name.clone(), scope);
    with_scope(scope, name, |scope| {
        let control = only(&case_items, Production::Control, node)?;
        control_anchors(cst, control, scope, out, out_decls, limits, depth + 1)
    })
}

/// `branch N Control`: the branch's own name encloses its one control
/// (FR-112-AC-2: `branch left await Wait after Sent` records scope
/// `[..., left]`). `depth` is the enclosing `parallel`'s own depth, the
/// same convention [`case_anchors`] follows.
fn branch_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
    limits: FormsLimits,
    depth: u64,
) -> Result<(), FormsFailure> {
    let branch_items = items(cst, node);
    let name = declared_name(&branch_items, node)?;
    declare(out_decls, ProtocolNodeKind::Branch, name.clone(), scope);
    with_scope(scope, name, |scope| {
        let control = only(&branch_items, Production::Control, node)?;
        control_anchors(cst, control, scope, out, out_decls, limits, depth + 1)
    })
}

/// `await N after R using alias clock "c" within i match Event then Control
/// timeout Control`.
///
/// The `after` reference (`await-after`, FR-112-AC-2) is built under the
/// scope enclosing the await, before the await's own name is pushed: an
/// await is a leaf reference target for its own `after` anchor, the same
/// way an `effect`'s own name never encloses its own `of` reference.
///
/// The matched event template and the `then`/`timeout` branches are
/// different: FR-113 Inputs says a control declares the names of its
/// direct child controls and event nodes, and lists `await` among the
/// structural controls, so these three are the await's own children --
/// the legacy checker FR-113 replaces puts them under the await's own
/// symbol (`src/linking/composed/scopes/protocol.rs` `ControlKind::Await`).
/// A reference inside any of them therefore resolves as if nested one
/// level inside this await.
fn await_control_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
    limits: FormsLimits,
    depth: u64,
) -> Result<(), FormsFailure> {
    let await_items = items(cst, node);
    let after_reference = only(&await_items, Production::NodeReference, node)?;
    out.push(scoped_anchor(
        cst,
        after_reference,
        AnchorSite::AwaitAfter,
        scope,
        None,
    )?);
    let name = declared_name(&await_items, node)?;
    declare(out_decls, ProtocolNodeKind::Await, name.clone(), scope);
    with_scope(scope, name, |scope| {
        let matched_event = only(&await_items, Production::EventNode, node)?;
        event_node_anchors(cst, matched_event, scope, out, out_decls)?;
        for branch in nodes_of(&await_items, Production::Control) {
            control_anchors(cst, branch, scope, out, out_decls, limits, depth + 1)?;
        }
        Ok(())
    })
}

/// `send`, `receive ... of R`, `attempt`, `effect ... of R` or `event ...
/// [for R]`: the event node's own kind, read from its leading token
/// (mirrors [`state_clause`]'s kind read). Only `receive`, `effect` and
/// `event` name a scoped-anchor site (FR-112 "Behavior"); `send` and
/// `attempt` build no anchor.
fn event_node_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &[ScopeName],
    out: &mut Vec<ScopedAnchorForm>,
    out_decls: &mut Vec<ProtocolNodeDeclaration>,
) -> Result<(), FormsFailure> {
    let event_items = items(cst, node);
    let (kind, site) = match event_items.first() {
        Some(Item::Token(token)) if token.spelling() == b"send" => (ProtocolNodeKind::Send, None),
        Some(Item::Token(token)) if token.spelling() == b"receive" => {
            (ProtocolNodeKind::Receive, Some(AnchorSite::ReceiveOf))
        }
        Some(Item::Token(token)) if token.spelling() == b"attempt" => {
            (ProtocolNodeKind::Attempt, None)
        }
        Some(Item::Token(token)) if token.spelling() == b"effect" => {
            (ProtocolNodeKind::Effect, Some(AnchorSite::EffectOf))
        }
        Some(Item::Token(token)) if token.spelling() == b"event" => {
            (ProtocolNodeKind::Event, Some(AnchorSite::EventFor))
        }
        Some(Item::Token(_) | Item::Node(_)) | None => return Err(unexpected(node)),
    };
    let identifiers = tokens_of(&event_items, TokenKind::Identifier);
    let name_token = identifiers.first().ok_or_else(|| unexpected(node))?;
    let name = DeclaredName {
        name: text(name_token, node)?,
        span: name_token.span(),
    };
    // Only `send` and `receive` are written `via` a channel (FR-113's
    // channel-mismatch check); every other kind's second identifier, when
    // one exists, is a role or a `contracts` entry, never a channel.
    let channel = match kind {
        ProtocolNodeKind::Send | ProtocolNodeKind::Receive => {
            let channel_token = identifiers.get(1).ok_or_else(|| unexpected(node))?;
            Some(text(channel_token, node)?)
        }
        _ => None,
    };
    out_decls.push(ProtocolNodeDeclaration {
        kind,
        name,
        scope: scope.to_vec(),
        channel: channel.clone(),
    });
    let Some(site) = site else {
        return Ok(());
    };
    // `event`'s `for R` is optional (FR-112 "Behavior" names it as a site,
    // and the grammar admits an `event` with none); `receive`'s `of R` and
    // `effect`'s `of R` are both mandatory, so this is always exactly one
    // when present.
    if let Some(reference) = nodes_of(&event_items, Production::NodeReference).first() {
        // Only `receive-of` carries the owning event node's own channel
        // (FR-113's channel-mismatch check): `receive`'s `of` reference is
        // the only site whose site check compares a channel.
        let owner_channel = if site == AnchorSite::ReceiveOf {
            channel.clone()
        } else {
            None
        };
        out.push(scoped_anchor(cst, reference, site, scope, owner_channel)?);
    }
    Ok(())
}

/// One `ScopedAnchorForm` from a `NodeReference` CST node: its segments
/// exactly as written, with each segment's own span and the span of the
/// whole reference (FR-112 "Outputs" and "Behavior": "keep a reference's
/// segments exactly as written").
fn scoped_anchor(
    cst: &LosslessCst,
    reference: &CstNode,
    site: AnchorSite,
    scope: &[ScopeName],
    channel: Option<String>,
) -> Result<ScopedAnchorForm, FormsFailure> {
    let reference_items = items(cst, reference);
    let mut segments = Vec::new();
    for token in tokens_of(&reference_items, TokenKind::Identifier) {
        segments.push(AnchorSegment {
            text: text(token, reference)?,
            span: token.span(),
        });
    }
    if segments.is_empty() {
        return Err(unexpected(reference));
    }
    Ok(ScopedAnchorForm {
        site,
        anchor: AnchorForm {
            segments,
            span: reference.span(),
        },
        scope: scope.to_vec(),
        channel,
    })
}
