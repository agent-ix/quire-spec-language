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
    AnchorForm, AnchorSegment, AnchorSite, BinderForm, BinderKind, DeclarationForm, DeclaredName,
    ProtocolDeclarationForm, ProtocolNodeDeclaration, ProtocolNodeKind, ScopeName,
    ScopedAnchorForm, StateClauseForm, StateClauseKind, UsingAlias,
};
use super::value::{
    declared_name, expression, items, name_form, nodes_of, only, production_node, text, tokens_of,
    unexpected, Item,
};

/// The protocol form's three output collections and its nesting-depth
/// limit, threaded together through every S2 control-tree walk function
/// (FR-113 Inputs/Outputs): one struct rather than three `&mut Vec`
/// parameters plus `FormsLimits`, so a new output needs one field added
/// here, not a new parameter at every call site (SR-765 FND-008).
struct Collector<'a> {
    /// FR-112's scoped anchors, in source order of their references.
    anchors: &'a mut Vec<ScopedAnchorForm>,
    /// FR-113's declaration collection: every static node a scope declares
    /// directly, in source order.
    declarations: &'a mut Vec<ProtocolNodeDeclaration>,
    /// FR-113's binder collection, in source order.
    binders: &'a mut Vec<BinderForm>,
    /// The S2 nesting-depth bound `control_anchors` charges `depth`
    /// against.
    limits: FormsLimits,
}

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
    let mut binders = Vec::new();
    // The protocol's own `over (p)` input parameter (FR-113 "Refusals"
    // binder no-shadowing rule): visible everywhere in the protocol, at the
    // empty top-level scope (composed lane: `BinderKind::Input`).
    let input = only(&clause_items, Production::Parameter, node)?;
    binders.push(parameter_binder(cst, input, BinderKind::Input, &[])?);
    // The protocol's own `activation on each (p) [when (...)]` parameter,
    // when written: `on origin` binds nothing, so this is only sometimes
    // present.
    let activation = only(&clause_items, Production::Activation, node)?;
    let activation_items = items(cst, activation);
    if let Some(&activation_parameter) = nodes_of(&activation_items, Production::Parameter).first()
    {
        binders.push(parameter_binder(
            cst,
            activation_parameter,
            BinderKind::ActivationParameter,
            &[],
        )?);
    }
    // The protocol's own top-level captures (FR-113 "Refusals" binder
    // no-shadowing rule): written before the declaration's roles, at the
    // empty top-level scope every other top-level binder shares.
    for capture in nodes_of(&clause_items, Production::Capture) {
        binders.push(capture_binder(cst, capture, &[])?);
    }
    {
        let mut collector = Collector {
            anchors: &mut scoped_anchors,
            declarations: &mut declarations,
            binders: &mut binders,
            limits: construct.limits,
        };
        for compensation in nodes_of(&clause_items, Production::Compensation) {
            compensation_anchors(cst, compensation, &mut collector)?;
        }
        let run = only(&clause_items, Production::Control, node)?;
        control_anchors(cst, run, &mut Vec::new(), &mut collector, 1)?;
    }
    let finish = only(&clause_items, Production::Finish, node)?;
    let finish_items = items(cst, finish);
    declarations.push(ProtocolNodeDeclaration {
        kind: ProtocolNodeKind::Finish,
        name: declared_name(&finish_items, finish)?,
        scope: Vec::new(),
        channel: None,
    });
    binders.push(record_binder(cst, &finish_items, finish, &[])?);

    Ok(DeclarationForm::Protocol(ProtocolDeclarationForm {
        name,
        scoped_anchors,
        declarations,
        binders,
    }))
}

/// The one `as (x: T)` bound parameter directly among `items` (FR-113
/// "Refusals": a record binder, "the `as (x: T)` of an event node", plus
/// the same-shaped binder of a `commit`, a `finish` or a `compensate`
/// declaration), scoped at `scope`.
fn record_binder(
    cst: &LosslessCst,
    items: &[Item<'_>],
    node: &CstNode,
    scope: &[ScopeName],
) -> Result<BinderForm, FormsFailure> {
    let parameter = only(items, Production::Parameter, node)?;
    parameter_binder(cst, parameter, BinderKind::RecordBinder, scope)
}

/// One `capture p = e;`'s own parameter, scoped at `scope` (FR-113
/// "Refusals").
fn capture_binder(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &[ScopeName],
) -> Result<BinderForm, FormsFailure> {
    let capture_items = items(cst, node);
    let parameter = only(&capture_items, Production::Parameter, node)?;
    parameter_binder(cst, parameter, BinderKind::Capture, scope)
}

/// One `Parameter` node's own declared name as a binder of `kind`, scoped
/// at `scope`: a `Parameter` (`ident : ParameterType`) names itself with
/// its one identifier token, the same shape [`declared_name`] reads.
fn parameter_binder(
    cst: &LosslessCst,
    parameter: &CstNode,
    kind: BinderKind,
    scope: &[ScopeName],
) -> Result<BinderForm, FormsFailure> {
    let parameter_items = items(cst, parameter);
    Ok(BinderForm {
        kind,
        name: declared_name(&parameter_items, parameter)?,
        scope: scope.to_vec(),
    })
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
    collector: &mut Collector<'_>,
) -> Result<(), FormsFailure> {
    let compensation_items = items(cst, node);
    let name = declared_name(&compensation_items, node)?;
    collector.declarations.push(ProtocolNodeDeclaration {
        kind: ProtocolNodeKind::CompensateTemplate,
        name: name.clone(),
        scope: Vec::new(),
        channel: None,
    });
    let references = nodes_of(&compensation_items, Production::NodeReference);
    let for_reference = references.first().ok_or_else(|| unexpected(node))?;
    collector.anchors.push(scoped_anchor(
        cst,
        for_reference,
        AnchorSite::CompensateFor,
        &[],
        None,
    )?);
    // `commit never;` names no reference: `references` then holds only the
    // `for` target, and this site builds no anchor (FR-112 "Behavior").
    if let Some(commit_reference) = references.get(1) {
        collector.anchors.push(scoped_anchor(
            cst,
            commit_reference,
            AnchorSite::CompensateCommit,
            &[],
            None,
        )?);
    }
    // The declaration's own binders (FR-113 "Refusals"): its record binder
    // (`as (p)`) at the empty top-level scope its own declaration lives in,
    // then the trigger, every capture, and the retry/recovery parameters,
    // one level inside it, scoped by the declaration's own name. This
    // production has no wrapping node for `activate`'s guard, so its own
    // `star(Capture)` and the declaration's other, earlier `star(Capture)`
    // (written directly under the declaration, before `activate`) are the
    // same flat child list -- a capture may legally appear on either side
    // of the trigger -- and the trigger, the two retry parameters and the
    // recovery parameter are the only `Parameter` nodes directly under it,
    // in that fixed grammar order. Binders are sorted by their own span
    // once collected, so they land in true source order regardless of
    // which side of the trigger a capture is written on (FR-113's
    // directionality rule needs the *later* binder to be the one refused;
    // a mutation that instead pushes them in a fixed grammar-position order
    // gets this backwards whenever a capture precedes the trigger).
    let parameters = nodes_of(&compensation_items, Production::Parameter);
    let [own, trigger, retry_first, retry_second, recover] = parameters.as_slice() else {
        return Err(unexpected(node));
    };
    collector
        .binders
        .push(parameter_binder(cst, own, BinderKind::RecordBinder, &[])?);
    let inner_scope = vec![ScopeName {
        name: name.name,
        span: name.span,
    }];
    let mut inner_binders = vec![parameter_binder(
        cst,
        trigger,
        BinderKind::Trigger,
        &inner_scope,
    )?];
    for capture in nodes_of(&compensation_items, Production::Capture) {
        inner_binders.push(capture_binder(cst, capture, &inner_scope)?);
    }
    inner_binders.push(parameter_binder(
        cst,
        retry_first,
        BinderKind::RetryParameter,
        &inner_scope,
    )?);
    inner_binders.push(parameter_binder(
        cst,
        retry_second,
        BinderKind::RetryParameter,
        &inner_scope,
    )?);
    inner_binders.push(parameter_binder(
        cst,
        recover,
        BinderKind::RecoveryParameter,
        &inner_scope,
    )?);
    inner_binders.sort_by_key(|binder| binder.name.span.start);
    collector.binders.extend(inner_binders);
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
    collector: &mut Collector<'_>,
    depth: u64,
) -> Result<(), FormsFailure> {
    let matched = items(cst, control)
        .into_iter()
        .find_map(|item| match item {
            Item::Node(node) => Some(node),
            Item::Token(_) => None,
        })
        .ok_or_else(|| unexpected(control))?;
    if depth > collector.limits.nesting_depth {
        return Err(FormsFailure::depth(collector.limits, matched.span()));
    }
    // Every arm below declares `matched` itself, directly in `scope` (the
    // scope enclosing it, before its own name -- if any -- is pushed): this
    // is the single place a control becomes a name other scopes and
    // anchors can resolve (FR-113 Inputs).
    match matched.production() {
        Production::Sequence => {
            let sequence_items = items(cst, matched);
            let name = declared_name(&sequence_items, matched)?;
            declare(
                collector.declarations,
                ProtocolNodeKind::Sequence,
                name.clone(),
                scope,
            );
            with_scope(scope, name, |scope| {
                for child in nodes_of(&sequence_items, Production::Control) {
                    control_anchors(cst, child, scope, collector, depth + 1)?;
                }
                Ok(())
            })
        }
        Production::Choice => {
            let choice_items = items(cst, matched);
            let name = declared_name(&choice_items, matched)?;
            declare(
                collector.declarations,
                ProtocolNodeKind::Choice,
                name.clone(),
                scope,
            );
            with_scope(scope, name, |scope| {
                for case in nodes_of(&choice_items, Production::Case) {
                    case_anchors(cst, case, scope, collector, depth)?;
                }
                Ok(())
            })
        }
        Production::Parallel => {
            let parallel_items = items(cst, matched);
            let name = declared_name(&parallel_items, matched)?;
            declare(
                collector.declarations,
                ProtocolNodeKind::Parallel,
                name.clone(),
                scope,
            );
            with_scope(scope, name, |scope| {
                for branch in nodes_of(&parallel_items, Production::Branch) {
                    branch_anchors(cst, branch, scope, collector, depth)?;
                }
                Ok(())
            })
        }
        Production::Repetition => {
            let repetition_items = items(cst, matched);
            let name = declared_name(&repetition_items, matched)?;
            declare(
                collector.declarations,
                ProtocolNodeKind::Repeat,
                name.clone(),
                scope,
            );
            with_scope(scope, name, |scope| {
                for child in nodes_of(&repetition_items, Production::Control) {
                    control_anchors(cst, child, scope, collector, depth + 1)?;
                }
                Ok(())
            })
        }
        Production::AwaitControl => await_control_anchors(cst, matched, scope, collector, depth),
        Production::EventNode => event_node_anchors(cst, matched, scope, collector),
        Production::Check => {
            let check_items = items(cst, matched);
            let name = declared_name(&check_items, matched)?;
            declare(collector.declarations, ProtocolNodeKind::Check, name, scope);
            // `check` holds no `NodeReference` and encloses no further
            // control (FR-112 "Behavior": "No other position yields one").
            Ok(())
        }
        Production::Commit => {
            let commit_items = items(cst, matched);
            let name = declared_name(&commit_items, matched)?;
            declare(
                collector.declarations,
                ProtocolNodeKind::Commit,
                name,
                scope,
            );
            collector
                .binders
                .push(record_binder(cst, &commit_items, matched, scope)?);
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
#[allow(clippy::too_many_arguments)]
fn case_anchors(
    cst: &LosslessCst,
    node: &CstNode,
    scope: &mut Vec<ScopeName>,
    collector: &mut Collector<'_>,
    depth: u64,
) -> Result<(), FormsFailure> {
    let case_items = items(cst, node);
    let name = declared_name(&case_items, node)?;
    declare(
        collector.declarations,
        ProtocolNodeKind::Case,
        name.clone(),
        scope,
    );
    with_scope(scope, name, |scope| {
        let control = only(&case_items, Production::Control, node)?;
        control_anchors(cst, control, scope, collector, depth + 1)
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
    collector: &mut Collector<'_>,
    depth: u64,
) -> Result<(), FormsFailure> {
    let branch_items = items(cst, node);
    let name = declared_name(&branch_items, node)?;
    declare(
        collector.declarations,
        ProtocolNodeKind::Branch,
        name.clone(),
        scope,
    );
    with_scope(scope, name, |scope| {
        let control = only(&branch_items, Production::Control, node)?;
        control_anchors(cst, control, scope, collector, depth + 1)
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
    collector: &mut Collector<'_>,
    depth: u64,
) -> Result<(), FormsFailure> {
    let await_items = items(cst, node);
    let after_reference = only(&await_items, Production::NodeReference, node)?;
    collector.anchors.push(scoped_anchor(
        cst,
        after_reference,
        AnchorSite::AwaitAfter,
        scope,
        None,
    )?);
    let name = declared_name(&await_items, node)?;
    declare(
        collector.declarations,
        ProtocolNodeKind::Await,
        name.clone(),
        scope,
    );
    with_scope(scope, name, |scope| {
        let matched_event = only(&await_items, Production::EventNode, node)?;
        event_node_anchors(cst, matched_event, scope, collector)?;
        for branch in nodes_of(&await_items, Production::Control) {
            control_anchors(cst, branch, scope, collector, depth + 1)?;
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
    collector: &mut Collector<'_>,
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
    collector.declarations.push(ProtocolNodeDeclaration {
        kind,
        name,
        scope: scope.to_vec(),
        channel: channel.clone(),
    });
    // Every event node kind (`send` and `attempt` included) carries its own
    // `as (x: T)` record binder (FR-113 "Refusals").
    collector
        .binders
        .push(record_binder(cst, &event_items, node, scope)?);
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
        collector
            .anchors
            .push(scoped_anchor(cst, reference, site, scope, owner_channel)?);
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
