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

use super::dispatch::{Construct, FormsFailure};
use super::spans::DeclarationSpans;
use super::syntax::{
    AnchorForm, AnchorSegment, AnchorSite, DeclarationForm, DeclaredName, ProtocolDeclarationForm,
    ScopeName, ScopedAnchorForm, StateClauseForm, StateClauseKind, UsingAlias,
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
    for compensation in nodes_of(&clause_items, Production::Compensation) {
        compensation_anchors(cst, compensation, &mut scoped_anchors)?;
    }
    let run = only(&clause_items, Production::Control, node)?;
    control_anchors(cst, run, &mut Vec::new(), &mut scoped_anchors)?;

    Ok(DeclarationForm::Protocol(ProtocolDeclarationForm {
        name,
        scoped_anchors,
    }))
}

/// `compensate N for R as (p) by role on Op using alias clock "c" { ...
/// commit (R2 | never); ... }`: the `for` reference (`compensate-for`,
/// FR-112 site list) and, when `commit` names a reference rather than
/// `never`, the `commit` reference (`compensate-commit`). Both are
/// protocol-level: neither is written inside `run`'s control tree, so both
/// carry the empty scope.
fn compensation_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    out: &mut Vec<ScopedAnchorForm>,
) -> Result<(), FormsFailure> {
    let compensation_items = items(cst, node);
    let references = nodes_of(&compensation_items, Production::NodeReference);
    let for_reference = references.first().ok_or_else(|| unexpected(node))?;
    out.push(scoped_anchor(
        cst,
        for_reference,
        AnchorSite::CompensateFor,
        &[],
    )?);
    // `commit never;` names no reference: `references` then holds only the
    // `for` target, and this site builds no anchor (FR-112 "Behavior").
    if let Some(commit_reference) = references.get(1) {
        out.push(scoped_anchor(
            cst,
            commit_reference,
            AnchorSite::CompensateCommit,
            &[],
        )?);
    }
    Ok(())
}

/// Walks one `Control` node: unwraps it to the alternative it actually
/// matched (`Sequence`, `Parallel`, ..., `EventNode`) and dispatches on
/// that alternative's own production.
fn control_anchors(
    cst: &LosslessCst,
    control: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
) -> Result<(), FormsFailure> {
    let matched = items(cst, control)
        .into_iter()
        .find_map(|item| match item {
            Item::Node(node) => Some(node),
            Item::Token(_) => None,
        })
        .ok_or_else(|| unexpected(control))?;
    match matched.production() {
        Production::Sequence => {
            let sequence_items = items(cst, matched);
            let name = declared_name(&sequence_items, matched)?;
            scope.push(ScopeName {
                name: name.name,
                span: name.span,
            });
            for child in nodes_of(&sequence_items, Production::Control) {
                control_anchors(cst, child, scope, out)?;
            }
            scope.pop();
            Ok(())
        }
        Production::Choice => {
            let choice_items = items(cst, matched);
            let name = declared_name(&choice_items, matched)?;
            scope.push(ScopeName {
                name: name.name,
                span: name.span,
            });
            for case in nodes_of(&choice_items, Production::Case) {
                case_anchors(cst, case, scope, out)?;
            }
            scope.pop();
            Ok(())
        }
        Production::Parallel => {
            let parallel_items = items(cst, matched);
            let name = declared_name(&parallel_items, matched)?;
            scope.push(ScopeName {
                name: name.name,
                span: name.span,
            });
            for branch in nodes_of(&parallel_items, Production::Branch) {
                branch_anchors(cst, branch, scope, out)?;
            }
            scope.pop();
            Ok(())
        }
        Production::Repetition => {
            let repetition_items = items(cst, matched);
            let name = declared_name(&repetition_items, matched)?;
            scope.push(ScopeName {
                name: name.name,
                span: name.span,
            });
            for child in nodes_of(&repetition_items, Production::Control) {
                control_anchors(cst, child, scope, out)?;
            }
            scope.pop();
            Ok(())
        }
        Production::AwaitControl => await_control_anchors(cst, matched, scope, out),
        Production::EventNode => event_node_anchors(cst, matched, scope, out),
        // `check` and `commit` (the bare control, distinct from a
        // compensation's `commit`) hold no `NodeReference` and enclose no
        // further control (FR-112 "Behavior": "No other position yields
        // one").
        Production::Check | Production::Commit => Ok(()),
        _ => Err(unexpected(matched)),
    }
}

/// `case N when { c } Control`: the case's own name encloses its one
/// control, the same shape a `branch` gives a `parallel` (FR-112
/// "Outputs": "the named controls that enclose the reference").
fn case_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
) -> Result<(), FormsFailure> {
    let case_items = items(cst, node);
    let name = declared_name(&case_items, node)?;
    scope.push(ScopeName {
        name: name.name,
        span: name.span,
    });
    let control = only(&case_items, Production::Control, node)?;
    control_anchors(cst, control, scope, out)?;
    scope.pop();
    Ok(())
}

/// `branch N Control`: the branch's own name encloses its one control
/// (FR-112-AC-2: `branch left await Wait after Sent` records scope
/// `[..., left]`).
fn branch_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
) -> Result<(), FormsFailure> {
    let branch_items = items(cst, node);
    let name = declared_name(&branch_items, node)?;
    scope.push(ScopeName {
        name: name.name,
        span: name.span,
    });
    let control = only(&branch_items, Production::Control, node)?;
    control_anchors(cst, control, scope, out)?;
    scope.pop();
    Ok(())
}

/// `await N after R using alias clock "c" within i match Event then Control
/// timeout Control`: the `after` reference (`await-after`, FR-112-AC-2),
/// the matched event template's own reference when it names one, and both
/// branches, all under the current scope (the await's own name `N` is a
/// leaf reference target, not an enclosing control, the same way an
/// `effect`'s own name never encloses its own `of` reference).
fn await_control_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    out: &mut Vec<ScopedAnchorForm>,
) -> Result<(), FormsFailure> {
    let await_items = items(cst, node);
    let after_reference = only(&await_items, Production::NodeReference, node)?;
    out.push(scoped_anchor(
        cst,
        after_reference,
        AnchorSite::AwaitAfter,
        scope,
    )?);
    let matched_event = only(&await_items, Production::EventNode, node)?;
    event_node_anchors(cst, matched_event, scope, out)?;
    for branch in nodes_of(&await_items, Production::Control) {
        control_anchors(cst, branch, scope, out)?;
    }
    Ok(())
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
) -> Result<(), FormsFailure> {
    let event_items = items(cst, node);
    let site = match event_items.first() {
        Some(Item::Token(token)) if token.spelling() == b"receive" => Some(AnchorSite::ReceiveOf),
        Some(Item::Token(token)) if token.spelling() == b"effect" => Some(AnchorSite::EffectOf),
        Some(Item::Token(token)) if token.spelling() == b"event" => Some(AnchorSite::EventFor),
        Some(Item::Token(token))
            if token.spelling() == b"send" || token.spelling() == b"attempt" =>
        {
            None
        }
        Some(Item::Token(_) | Item::Node(_)) | None => return Err(unexpected(node)),
    };
    let Some(site) = site else {
        return Ok(());
    };
    // `event`'s `for R` is optional (FR-112 "Behavior" names it as a site,
    // and the grammar admits an `event` with none); `receive`'s `of R` and
    // `effect`'s `of R` are both mandatory, so this is always exactly one
    // when present.
    if let Some(reference) = nodes_of(&event_items, Production::NodeReference).first() {
        out.push(scoped_anchor(cst, reference, site, scope)?);
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
    })
}
