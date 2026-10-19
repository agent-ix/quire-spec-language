// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218: the S3 checks of a protocol's channels and of its `parallel`
//! joins and `terminal` and `scheduling` members, over the S2 forms of
//! FR-308 (ADR-012 §12.2 Check row).
//!
//! A channel declares its two roles, its payload type, its ordering, its
//! delivery policy, its capacity and its overflow policy. Each premise is a
//! separate fact: this check records every one exactly as written and
//! derives none from another, so two channels that differ in exactly one
//! premise, the overflow policy's loss selector included, are different
//! [`CheckedChannel`]s. A `parallel` join must name branches of its own
//! control and must be sound for the branches it lists; a protocol holds at
//! most one `terminal` and one `scheduling` member.
//!
//! Each `send` and `receive` names a declared channel and binds its
//! `carries` type. A `fifo` key is checked through the one clause checker
//! (FR-104) over its binder and constants only: it reads no model state and
//! its type admits `=`. A `predicate(name)` join names a function no checker
//! reads yet; `protocol_clause::content` refuses it as unimplemented through
//! the construct S2 records for it, so a protocol holding one never compiles
//! unchecked.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    BinaryOperator, ChannelCapacity, ChannelDelivery, ChannelForm, ChannelOrdering,
    ChannelOverflow, Expression, ExpressionForm, JoinPolicy, ParallelForm, ParameterForm,
    ProtocolDeclarationForm, ProtocolMemberForm, TypeFormHead,
};
use qsl_foundation::Span;
use quire_exact::ValueType;
use quire_semantic_value::location::{Location, Origin};

use super::ir::CheckedBody;
use super::protocol_obligation::{
    model_state_reads, span_of, type_body, wraps, CheckedBinder, Goal,
};
use super::refusal::{ChannelEnd, CheckCause, CheckRefusal, JoinFault, ProtocolAnchorCause};
use super::state_clause::ClauseDeclarations;
use super::type_form::resolve_form;
use super::Scope;
use crate::family::CheckContext;

/// A checked channel's ordering premise (FR-218 "Channels").
#[derive(Clone, Debug, PartialEq)]
pub enum CheckedOrdering {
    /// `ordering unordered`: no binder and no key.
    Unordered,
    /// `ordering fifo by (binder: T) { key }`.
    Fifo {
        /// The message binder, of the channel's `carries` type.
        binder: CheckedBinder,
        /// The checked key, whose type admits `=`.
        key: CheckedBody,
    },
}

/// One channel FR-218 has checked: its roles, its payload type and its four
/// premises, each exactly as written (FR-218 "Channels"). The capacity of a
/// `symbolic(name)` channel keeps the name unresolved.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedChannel {
    /// The declared channel name.
    pub name: String,
    /// The declared `from` role.
    pub from: String,
    /// The declared `to` role.
    pub to: String,
    /// The `carries` type, resolved.
    pub carries: ValueType,
    /// The ordering premise.
    pub ordering: CheckedOrdering,
    /// The delivery premise.
    pub delivery: ChannelDelivery,
    /// The capacity premise.
    pub capacity: ChannelCapacity,
    /// The overflow premise, loss selector included.
    pub overflow: ChannelOverflow,
    /// The span of the whole declaration, where S4 locates the channel and
    /// its queue (FR-383).
    pub span: Span,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads a
        // `ProtocolAnchor` cause's own span directly instead.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// The clause checker's inputs a FIFO key is checked with, and the
/// package's running node total.
struct Keys<'k, 'a, 'c> {
    declarations: &'k ClauseDeclarations<'a>,
    cx: &'k mut CheckContext<'c, Scope>,
    nodes: u64,
}

/// FR-218: the channels, joins and `terminal`/`scheduling` members of
/// `protocol`, with the package's running node total after the FIFO keys,
/// or the refusals in source position order. The channels are returned in
/// source order.
pub(super) fn check(
    protocol: &ProtocolDeclarationForm,
    declarations: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
) -> Result<(Vec<CheckedChannel>, u64), Vec<CheckRefusal>> {
    let scope = declarations.scope;
    let mut cx = cx.reborrow(scope);
    let mut keys = Keys {
        declarations,
        cx: &mut cx,
        nodes: declarations.nodes_used,
    };
    let mut refusals: Vec<(Span, CheckRefusal)> = Vec::new();
    let roles: BTreeSet<&str> = protocol
        .roles
        .iter()
        .map(|role| role.name.name.as_str())
        .collect();

    for role in &protocol.roles {
        let name = role.context.name.as_str();
        // A name that is no type at all is the missing-type refusal of
        // `protocol_clause::content`.
        if scope.object_type_named(name).is_none() && !scope.named_types(name).is_empty() {
            refusals.push((
                role.context.span,
                refusal(ProtocolAnchorCause::RoleModel {
                    role: role.name.name.clone(),
                    context: role.context.name.clone(),
                    span: role.context.span,
                }),
            ));
        }
    }

    let mut checked = Vec::with_capacity(protocol.channels.len());
    for channel in &protocol.channels {
        if let Some(channel) = check_channel(channel, &roles, &mut keys, &mut refusals) {
            checked.push(channel);
        }
    }
    refuse_duplicate_channels(&protocol.channels, &mut refusals);
    check_messages(protocol, scope, &mut refusals);

    for parallel in &protocol.parallels {
        check_join(protocol, parallel, &mut refusals);
    }
    refuse_repeated_members(protocol, &mut refusals);

    if refusals.is_empty() {
        return Ok((checked, keys.nodes));
    }
    // Source position order; the sort is stable, so refusals at one span
    // keep the order they were found in.
    refusals.sort_by_key(|(span, _)| span.start);
    Err(refusals.into_iter().map(|(_, refusal)| refusal).collect())
}

/// One channel's roles and payload type. `None` when any part refuses; the
/// refusals are pushed to `refusals`, the `from` role first.
fn check_channel(
    channel: &ChannelForm,
    roles: &BTreeSet<&str>,
    keys: &mut Keys<'_, '_, '_>,
    refusals: &mut Vec<(Span, CheckRefusal)>,
) -> Option<CheckedChannel> {
    let scope = keys.declarations.scope;
    let before = refusals.len();
    for (end, role) in [
        (ChannelEnd::From, &channel.from),
        (ChannelEnd::To, &channel.to),
    ] {
        if !roles.contains(role.name.as_str()) {
            refusals.push((
                role.span,
                refusal(ProtocolAnchorCause::ChannelRole {
                    channel: channel.name.name.clone(),
                    end,
                    name: role.name.clone(),
                    span: role.span,
                }),
            ));
        }
    }
    let carries = match resolve_form(scope, &channel.carries) {
        Ok(ValueType::Reference(_) | ValueType::Population(_)) => {
            let name = match &channel.carries.head {
                TypeFormHead::Name(name) => name.clone(),
                TypeFormHead::Builtin(_)
                | TypeFormHead::Collection(_)
                | TypeFormHead::Population => String::new(),
            };
            refusals.push((
                channel.carries.span,
                refusal(ProtocolAnchorCause::ChannelCarriesObject {
                    channel: channel.name.name.clone(),
                    name,
                    span: channel.carries.span,
                }),
            ));
            None
        }
        Ok(carries) => Some(carries),
        Err(error) => {
            refusals.push((
                error.span,
                refusal(ProtocolAnchorCause::ChannelType {
                    channel: channel.name.name.clone(),
                    fault: error.fault,
                    span: error.span,
                }),
            ));
            None
        }
    };
    let ordering = match &channel.ordering.value {
        ChannelOrdering::Unordered => Some(CheckedOrdering::Unordered),
        ChannelOrdering::Fifo { binder, key } => {
            check_fifo(channel, binder, key, carries.as_ref(), keys, refusals)
        }
    };
    if refusals.len() != before {
        return None;
    }
    Some(CheckedChannel {
        name: channel.name.name.clone(),
        from: channel.from.name.clone(),
        to: channel.to.name.clone(),
        carries: carries?,
        ordering: ordering?,
        delivery: channel.delivery.value,
        capacity: channel.capacity.value.clone(),
        overflow: channel.overflow.value,
        span: channel.span,
    })
}

/// FR-218 "Channels": `ordering fifo by (m: T) { key }`. `T` resolves and
/// is the channel's `carries` type (not compared with a refused `carries`
/// type); `key` is checked through the clause checker over `m` and
/// constants only, reads no model state, and is of a type over which the
/// clause checker admits `=`.
fn check_fifo(
    channel: &ChannelForm,
    binder: &ParameterForm,
    key: &ExpressionForm,
    carries: Option<&ValueType>,
    keys: &mut Keys<'_, '_, '_>,
    refusals: &mut Vec<(Span, CheckRefusal)>,
) -> Option<CheckedOrdering> {
    let name = channel.name.name.as_str();
    let binder_type = match resolve_form(keys.declarations.scope, &binder.value_type) {
        Ok(binder_type) => binder_type,
        Err(error) => {
            refusals.push((
                error.span,
                refusal(ProtocolAnchorCause::BinderType {
                    binder: binder.name.name.clone(),
                    fault: error.fault,
                    span: error.span,
                }),
            ));
            return None;
        }
    };
    if let Some(carries) = carries {
        if *carries != binder_type {
            refusals.push((
                binder.value_type.span,
                refusal(ProtocolAnchorCause::CarriesMismatch {
                    channel: name.to_owned(),
                    expected: carries.clone(),
                    actual: binder_type.clone(),
                    span: binder.value_type.span,
                }),
            ));
            return None;
        }
    }
    let bound = [(binder.name.name.clone(), binder_type.clone())];
    let body = match keys.type_key(key, &bound, Goal::Infer) {
        Ok(body) => body,
        Err(CheckRefusal { location, cause }) => {
            let span = span_of(key, &location);
            let wrapped = if wraps(&cause) && cause.cause().is_some() {
                refusal(ProtocolAnchorCause::KeyClause {
                    channel: name.to_owned(),
                    cause: Box::new(cause),
                    span,
                })
            } else {
                // A reached limit, a cancellation or a fault keeps its own
                // cause.
                CheckRefusal { location, cause }
            };
            refusals.push((span, wrapped));
            return None;
        }
    };
    let reads = model_state_reads(&body, key, None);
    if !reads.is_empty() {
        for span in reads {
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::KeyModelState {
                    channel: name.to_owned(),
                    span,
                }),
            ));
        }
        return None;
    }
    let key_type = body.root().value_type().clone();
    // `=` over two values of the key's type, as the clause checker types it.
    let equal = ExpressionForm {
        expression: Expression::binary(
            BinaryOperator::Equal,
            Expression::name("left"),
            Expression::name("right"),
        ),
        spans: key.spans.clone(),
    };
    let operands = [
        ("left".to_owned(), key_type.clone()),
        ("right".to_owned(), key_type.clone()),
    ];
    if keys.type_key(&equal, &operands, Goal::Boolean).is_err() {
        let span = span_of(key, &Location::root(Origin::Expression));
        refusals.push((
            span,
            refusal(ProtocolAnchorCause::KeyEquality {
                channel: name.to_owned(),
                key_type,
                span,
            }),
        ));
        return None;
    }
    Some(CheckedOrdering::Fifo {
        binder: CheckedBinder {
            name: binder.name.name.clone(),
            value_type: binder_type,
        },
        key: body,
    })
}

impl Keys<'_, '_, '_> {
    /// `form` checked through the clause checker over `bound` alone.
    fn type_key(
        &mut self,
        form: &ExpressionForm,
        bound: &[(String, ValueType)],
        goal: Goal<'_>,
    ) -> Result<CheckedBody, CheckRefusal> {
        let mut nodes = self.nodes;
        let typed = type_body(form, bound, goal, self.declarations, &mut nodes, self.cx);
        self.nodes = nodes;
        typed
    }
}

/// FR-218 "Channels": each `send` and `receive` names a declared channel
/// `via`, and its record binder declares the channel's `carries` type, by
/// type identity. A binder is not compared with a `carries` type that is
/// refused (an object type, or a name that does not resolve), nor with one
/// of two channels of one name; a binder type that does not resolve is
/// refused by `protocol_clause::content`.
fn check_messages(
    protocol: &ProtocolDeclarationForm,
    scope: &Scope,
    refusals: &mut Vec<(Span, CheckRefusal)>,
) {
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        let Some(via) = &declaration.channel else {
            continue;
        };
        let channels: Vec<&ChannelForm> = protocol
            .channels
            .iter()
            .filter(|channel| channel.name.name == via.name)
            .collect();
        let channel = match channels.as_slice() {
            [] => {
                refusals.push((
                    via.span,
                    refusal(ProtocolAnchorCause::MissingChannel {
                        node: declaration.name.name.clone(),
                        name: via.name.clone(),
                        span: via.span,
                    }),
                ));
                continue;
            }
            [channel] => *channel,
            // Two channels of one name refuse at their declarations.
            _ => continue,
        };
        let Ok(expected) = resolve_form(scope, &channel.carries) else {
            continue;
        };
        if matches!(expected, ValueType::Reference(_) | ValueType::Population(_)) {
            continue;
        }
        let Some(binder) = super::protocol_clause::record_binder(protocol, index) else {
            continue;
        };
        let Ok(actual) = resolve_form(scope, &binder.value_type) else {
            continue;
        };
        if actual != expected {
            refusals.push((
                binder.value_type.span,
                refusal(ProtocolAnchorCause::CarriesMismatch {
                    channel: channel.name.name.clone(),
                    expected,
                    actual,
                    span: binder.value_type.span,
                }),
            ));
        }
    }
}

/// FR-218 "Channels": two channels of one name refuse at each declaration,
/// naming both in source order. Channels and relationships have separate
/// namespaces, so only channels are compared.
fn refuse_duplicate_channels(channels: &[ChannelForm], refusals: &mut Vec<(Span, CheckRefusal)>) {
    let mut by_name: BTreeMap<&str, Vec<Span>> = BTreeMap::new();
    for channel in channels {
        by_name
            .entry(channel.name.name.as_str())
            .or_default()
            .push(channel.name.span);
    }
    for (name, loci) in by_name {
        if loci.len() < 2 {
            continue;
        }
        for &span in &loci {
            refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: name.to_owned(),
                    loci: loci.clone(),
                    span,
                }),
            ));
        }
    }
}

/// FR-218 "Control constructs" and "Admission for emission": a join names
/// branches of its own `parallel`, and its policy is sound for the labels it
/// lists. A label listed twice is one joined label.
fn check_join(
    protocol: &ProtocolDeclarationForm,
    parallel: &ParallelForm,
    refusals: &mut Vec<(Span, CheckRefusal)>,
) {
    let branches: BTreeSet<&str> = parallel
        .branches
        .iter()
        .map(|branch| branch.name.as_str())
        .collect();
    let mut unknown = false;
    for label in &parallel.labels {
        if branches.contains(label.name.as_str()) {
            continue;
        }
        unknown = true;
        // The lexical scope the label is written in: the control's own
        // enclosing named controls, then the control itself.
        let mut scope: Vec<String> = protocol
            .declarations
            .get(parallel.declaration)
            .map(|declaration| {
                protocol
                    .scope_names(declaration.scope)
                    .into_iter()
                    .map(|name| name.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        scope.push(parallel.name.name.clone());
        refusals.push((
            label.span,
            refusal(ProtocolAnchorCause::Missing {
                segments: vec![label.name.clone()],
                segment: label.name.clone(),
                scope,
                span: label.span,
            }),
        ));
    }
    if unknown {
        return;
    }

    let joined: BTreeSet<&str> = parallel
        .labels
        .iter()
        .map(|label| label.name.as_str())
        .collect();
    let (fault, span) = match &parallel.join.value {
        JoinPolicy::All => {
            let omitted = parallel
                .branches
                .iter()
                .find(|branch| !joined.contains(branch.name.as_str()));
            (
                omitted.map(|branch| JoinFault::OmitsBranch {
                    branch: branch.name.clone(),
                }),
                parallel.labels_span,
            )
        }
        JoinPolicy::Quorum(count) => {
            let above = match usize::try_from(*count) {
                Ok(count) => count > joined.len(),
                Err(_) => true,
            };
            let fault = if *count < 1 {
                Some(JoinFault::QuorumBelowOne)
            } else if above {
                Some(JoinFault::QuorumAboveLabels {
                    labels: joined.len(),
                })
            } else {
                None
            };
            (fault, parallel.quorum_count.unwrap_or(parallel.labels_span))
        }
        JoinPolicy::Any | JoinPolicy::Predicate(_) => (
            joined.is_empty().then_some(JoinFault::NoLabel),
            parallel.labels_span,
        ),
    };
    if let Some(fault) = fault {
        refusals.push((
            span,
            refusal(ProtocolAnchorCause::JoinList {
                parallel: parallel.name.name.clone(),
                fault,
                span,
            }),
        ));
    }
}

/// FR-211, FR-212: a protocol clause holds at most one `terminal` member and
/// at most one `scheduling` member. A second of a kind refuses
/// `ambiguous_declaration`/`ambiguous-name` at its own span, naming the
/// first.
fn refuse_repeated_members(
    protocol: &ProtocolDeclarationForm,
    refusals: &mut Vec<(Span, CheckRefusal)>,
) {
    let mut first_terminal = None;
    let mut first_scheduling = None;
    for member in &protocol.members {
        let (name, first) = match member {
            ProtocolMemberForm::Terminal(_) => ("terminal", &mut first_terminal),
            ProtocolMemberForm::Scheduling(_) => ("scheduling", &mut first_scheduling),
        };
        let span = member.span();
        match *first {
            None => *first = Some(span),
            Some(earlier) => refusals.push((
                span,
                refusal(ProtocolAnchorCause::Ambiguous {
                    name: name.to_owned(),
                    loci: vec![earlier, span],
                    span,
                }),
            )),
        }
    }
}

#[cfg(test)]
mod model_state_tests;

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{build_unit, ChannelCapacity, ChannelDelivery, ChannelOverflow, LossSelector};
    use qsl_foundation::SourceIdentity;
    use quire_semantic_value::checking::CheckingLimits;

    use super::super::{
        ChannelEnd, CheckCause, CheckRefusal, CheckedGraph, JoinFault, PackageDeclarations,
        ProtocolAnchorCause,
    };
    use super::CheckedChannel;

    mod attempt_fixture {
        use crate as semantics;
        include!("../../tests/fixtures/protocol_attempt.rs");
    }
    use attempt_fixture::{attempt_flow, m_actor_model, shared_binder_protocols, HEADER};

    const RECORD: &str = "record Plain { ready: Boolean; }\n\
        enum Color { Red, Blue, }\n";

    const RUN: &str = "sequence Main { attempt Tried by Seller on M::Actor::op contracts [] \
        as (tried: Boolean) { true }; }";

    const ENDS: &str = "from Seller to Buyer";

    /// The base protocol of these tests: two roles on `M::Actor`, `members`
    /// between the roles and `run`, and the given run control.
    fn pipeline(members: &str, run: &str) -> String {
        format!(
            "{RECORD}protocol Pipeline using v over (input: Boolean) on origin {{\n\
             role Seller on M::Actor;\n\
             role Buyer on M::Actor;\n\
             {members}\n\
             run {run}\n\
             finish End as (outcome: Boolean) {{ true }};\n\
             }}"
        )
    }

    fn channel(
        name: &str,
        ends: &str,
        carries: &str,
        delivery: &str,
        capacity: &str,
        overflow: &str,
    ) -> String {
        format!(
            "channel {name} {ends} carries {carries} ordering unordered delivery {delivery} \
             capacity {capacity} overflow {overflow};"
        )
    }

    fn pay() -> String {
        channel("Pay", ENDS, "Plain", "at-least-once", "2", "block")
    }

    fn parallel(join: &str) -> String {
        format!(
            "parallel P {{ \
             branch east sequence E {{ attempt Tried by Seller on M::Actor::op contracts [] \
             as (tried: Boolean) {{ true }}; }} \
             branch west sequence W {{ }} }} \
             join {join};"
        )
    }

    fn assemble(declarations: &str) -> (String, PackageDeclarations) {
        let text = format!("{HEADER}{declarations}\n");
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
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

    fn refusals(declarations: &str) -> (String, Vec<CheckRefusal>) {
        let (text, assembled) = assemble(declarations);
        match assembled.check(CheckingLimits::default()) {
            Err(refusals) => (text, refusals),
            Ok(_) => panic!("{declarations}: checking refuses"),
        }
    }

    fn checks(declarations: &str) -> CheckedGraph {
        let (_, assembled) = assemble(declarations);
        assembled
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
    }

    fn causes(refusals: &[CheckRefusal]) -> Vec<&ProtocolAnchorCause> {
        refusals
            .iter()
            .map(|refusal| match &refusal.cause {
                CheckCause::ProtocolAnchor(cause) => cause.as_ref(),
                other => panic!("a ProtocolAnchor cause, got {other:?}"),
            })
            .collect()
    }

    fn located<'t>(text: &'t str, cause: &ProtocolAnchorCause) -> &'t str {
        let span = cause.span();
        &text[span.start..span.end]
    }

    /// The one refusal `declarations` checks to, as its text, cause, code
    /// and tag.
    fn only_refusal(declarations: &str) -> (String, ProtocolAnchorCause, &'static str, String) {
        let (text, found) = refusals(declarations);
        let [refusal] = &found[..] else {
            panic!("{declarations}: one refusal, got {found:?}");
        };
        let cause = causes(&found)[0].clone();
        (
            text,
            cause,
            refusal.cause.code().as_str(),
            refusal.cause.cause().unwrap_or_default().to_owned(),
        )
    }

    /// FR-218 "Channels" (TC-1056 rows 1 to 4): a `to` role that is no
    /// declared role, both ends undeclared, an unresolved payload type and
    /// two channels of one name each refuse at the name with the members the
    /// requirement retains.
    #[trace("TC-1056")]
    #[test]
    fn a_channel_naming_nothing_the_protocol_declares_refuses_at_the_name() {
        let base = pipeline(&pay(), RUN);
        checks(&base);

        let source = base.replacen("to Buyer", "to Absent", 1);
        let (text, cause, code, tag) = only_refusal(&source);
        assert_eq!(
            (code, tag.as_str()),
            ("missing_declaration", "missing-name")
        );
        let ProtocolAnchorCause::ChannelRole {
            channel, end, name, ..
        } = &cause
        else {
            panic!("a ChannelRole cause, got {cause:?}");
        };
        assert_eq!(
            (channel.as_str(), *end, name.as_str()),
            ("Pay", ChannelEnd::To, "Absent")
        );
        assert_eq!(located(&text, &cause), "Absent");

        let source = base.replacen("from Seller to Buyer", "from Nobody to Absent", 1);
        let (text, found) = refusals(&source);
        let found_causes = causes(&found);
        let ends: Vec<(ChannelEnd, &str)> = found_causes
            .iter()
            .map(|cause| match cause {
                ProtocolAnchorCause::ChannelRole { end, .. } => (*end, located(&text, cause)),
                other => panic!("a ChannelRole cause, got {other:?}"),
            })
            .collect();
        assert_eq!(
            ends,
            [(ChannelEnd::From, "Nobody"), (ChannelEnd::To, "Absent")]
        );

        let source = base.replacen("carries Plain", "carries M::Absent", 1);
        let (text, cause, code, tag) = only_refusal(&source);
        assert_eq!(
            (code, tag.as_str()),
            ("missing_declaration", "missing-name")
        );
        let ProtocolAnchorCause::ChannelType { channel, fault, .. } = &cause else {
            panic!("a ChannelType cause, got {cause:?}");
        };
        assert_eq!(channel, "Pay");
        assert_eq!(
            *fault,
            super::super::type_form::TypeFormFault::MissingName("M::Absent".to_owned())
        );
        assert_eq!(located(&text, &cause), "M::Absent");

        let source = pipeline(&format!("{}\n{}", pay(), pay()), RUN);
        let (text, found) = refusals(&source);
        let found_causes = causes(&found);
        assert_eq!(found_causes.len(), 2, "{found:?}");
        for cause in &found_causes {
            let ProtocolAnchorCause::Ambiguous { name, loci, .. } = cause else {
                panic!("an Ambiguous cause, got {cause:?}");
            };
            assert_eq!(name, "Pay");
            assert_eq!(loci.len(), 2);
            assert_eq!(located(&text, cause), "Pay");
        }
        assert!(found_causes[0].span().start < found_causes[1].span().start);
        for refusal in &found {
            assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
            assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
        }
    }

    fn recorded(graph: &CheckedGraph) -> &[CheckedChannel] {
        &graph
            .protocol("Pipeline")
            .expect("the checked protocol")
            .channels
    }

    /// FR-218-AC-21 and FR-218-AC-13 (TC-1065 rows 1 and 3, TC-1056 rows 13
    /// and 14): the five overflow policies are recorded as written whatever
    /// the delivery policy is, and two channels that differ in exactly one
    /// premise, the loss selector included, are different checked channels.
    #[trace("TC-1065", "FR-218-AC-21")]
    #[trace("TC-1056", "FR-218-AC-13")]
    #[test]
    fn every_premise_is_recorded_as_written_and_none_is_derived_from_another() {
        let channels: Vec<String> = [
            ("reject", "unknown"),
            ("block", "at-most-once"),
            ("loss head", "at-least-once"),
            ("loss tail", "exactly-once-premise"),
            ("loss new", "unknown"),
        ]
        .iter()
        .enumerate()
        .map(|(index, (overflow, delivery))| {
            channel(&format!("C{index}"), ENDS, "Plain", delivery, "2", overflow)
        })
        .collect();
        let graph = checks(&pipeline(&channels.join("\n"), RUN));
        let overflows: Vec<ChannelOverflow> = recorded(&graph)
            .iter()
            .map(|checked| checked.overflow)
            .collect();
        assert_eq!(
            overflows,
            [
                ChannelOverflow::Reject,
                ChannelOverflow::Block,
                ChannelOverflow::Loss(LossSelector::Head),
                ChannelOverflow::Loss(LossSelector::Tail),
                ChannelOverflow::Loss(LossSelector::New),
            ]
        );

        let variants = [
            ("exactly-once-premise", "loss head"),
            ("at-most-once", "loss head"),
            ("exactly-once-premise", "loss tail"),
            ("exactly-once-premise", "loss new"),
        ];
        let channels: Vec<String> = variants
            .iter()
            .enumerate()
            .map(|(index, (delivery, overflow))| {
                channel(
                    &format!("V{index}"),
                    ENDS,
                    "Plain",
                    delivery,
                    "symbolic(Bound)",
                    overflow,
                )
            })
            .collect();
        let graph = checks(&pipeline(&channels.join("\n"), RUN));
        let [base, delivery, tail, new] = recorded(&graph) else {
            panic!("four channels: {:?}", recorded(&graph));
        };
        assert_eq!(base.ordering, super::CheckedOrdering::Unordered);
        assert_eq!(base.delivery, ChannelDelivery::ExactlyOncePremise);
        assert_eq!(base.capacity, ChannelCapacity::Symbolic("Bound".to_owned()));
        assert_eq!(base.overflow, ChannelOverflow::Loss(LossSelector::Head));
        assert_eq!((base.from.as_str(), base.to.as_str()), ("Seller", "Buyer"));

        // Each variant differs from the base in its one premise alone.
        assert_eq!(delivery.delivery, ChannelDelivery::AtMostOnce);
        assert_eq!(delivery.overflow, base.overflow);
        assert_eq!(delivery.capacity, base.capacity);
        assert_eq!(tail.overflow, ChannelOverflow::Loss(LossSelector::Tail));
        assert_eq!(tail.delivery, base.delivery);
        assert_eq!(new.overflow, ChannelOverflow::Loss(LossSelector::New));
        assert_eq!(new.delivery, base.delivery);
        let same_but_place = |other: &CheckedChannel| CheckedChannel {
            name: base.name.clone(),
            span: base.span,
            ..other.clone()
        };
        assert_ne!(*base, same_but_place(delivery));
        assert_ne!(*base, same_but_place(tail));
        assert_ne!(*base, same_but_place(new));
        assert_ne!(same_but_place(tail), same_but_place(new));
    }

    /// FR-218-AC-22 (TC-1065 rows 4 and 5): two cases or two branches of
    /// one label refuse at both labels in source order, and a `choice` with
    /// one case and a `parallel` with one branch fail at the parser; two
    /// cases and two branches check.
    #[trace("TC-1065", "FR-218-AC-22")]
    #[test]
    fn equal_case_or_branch_labels_refuse_and_a_single_one_fails_at_the_parser() {
        let choice = "choice Decide by Buyer visible () { \
            case shipped when { true } sequence A { } \
            case shipped when { false } sequence B { } }";
        let parallel_run = "parallel P { branch east sequence E { } branch east sequence W { } } \
            join all [east];";
        for (run, label) in [(choice, "shipped"), (parallel_run, "east")] {
            let (text, found) = refusals(&pipeline("", run));
            let found_causes = causes(&found);
            assert_eq!(found_causes.len(), 2, "{found:?}");
            for cause in &found_causes {
                let ProtocolAnchorCause::Ambiguous { name, loci, .. } = cause else {
                    panic!("an Ambiguous cause, got {cause:?}");
                };
                assert_eq!(name, label);
                assert_eq!(loci.len(), 2);
                assert_eq!(located(&text, cause), label);
            }
            assert!(found_causes[0].span().start < found_causes[1].span().start);
            for refusal in &found {
                assert_eq!(refusal.cause.code().as_str(), "ambiguous_declaration");
                assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
            }
        }

        for run in [
            "choice Decide by Buyer visible () { case shipped when { true } sequence A { } }",
            "parallel P { branch east sequence E { } } join all [east];",
        ] {
            let text = format!("{HEADER}{}\n", pipeline("", run));
            let parsed = qsl_cst::parse(
                SourceIdentity::new("a", "u", "git", "1"),
                "unit.native",
                text.as_bytes(),
                qsl_cst::Limits::default(),
            )
            .expect("S1 reads the unit");
            assert!(!parsed.is_admissible(), "{run}");
        }
        checks(&pipeline("", &parallel("all [east, west]")));
        checks(&pipeline(
            "",
            "choice Decide by Buyer visible () { \
             case shipped when { true } sequence A { } \
             case held when { false } sequence B { } }",
        ));
    }

    /// FR-218-AC-23 (TC-1065 rows 6 and 7): an unsound join list refuses
    /// `invalid_package`/`invalid-value` at the list, or at the count for a
    /// quorum; sound ones check and a label listed twice joins once.
    #[trace("TC-1065", "FR-218-AC-23")]
    #[test]
    fn an_unsound_join_refuses_at_the_list_or_the_quorum_count() {
        for (join, located_at, expected) in [
            (
                "all [east]",
                "[east]",
                JoinFault::OmitsBranch {
                    branch: "west".to_owned(),
                },
            ),
            ("quorum(0) [east, west]", "0", JoinFault::QuorumBelowOne),
            (
                "quorum(3) [east, west]",
                "3",
                JoinFault::QuorumAboveLabels { labels: 2 },
            ),
            ("any []", "[]", JoinFault::NoLabel),
        ] {
            let (text, cause, code, tag) = only_refusal(&pipeline("", &parallel(join)));
            assert_eq!((code, tag.as_str()), ("invalid_package", "invalid-value"));
            let ProtocolAnchorCause::JoinList {
                parallel, fault, ..
            } = &cause
            else {
                panic!("a JoinList cause, got {cause:?}");
            };
            assert_eq!(parallel, "P");
            assert_eq!(*fault, expected, "{join}");
            assert_eq!(located(&text, &cause), located_at, "{join}");
        }
        for join in [
            "all [east, west]",
            "quorum(2) [east, west]",
            "any [east]",
            "all [east, west, east]",
        ] {
            checks(&pipeline("", &parallel(join)));
        }
        // A label listed twice is one joined label.
        let (_, cause, ..) = only_refusal(&pipeline("", &parallel("quorum(3) [east, west, east]")));
        assert!(matches!(
            cause,
            ProtocolAnchorCause::JoinList {
                fault: JoinFault::QuorumAboveLabels { labels: 2 },
                ..
            }
        ));
    }

    /// FR-218 "Control constructs": a join naming a label that is no branch
    /// of its `parallel` refuses `missing_declaration`/`missing-name` at the
    /// label, and only there.
    #[trace("TC-1065")]
    #[test]
    fn a_join_naming_no_branch_refuses_at_the_label() {
        let (text, cause, code, tag) = only_refusal(&pipeline("", &parallel("all [east, middle]")));
        assert_eq!(
            (code, tag.as_str()),
            ("missing_declaration", "missing-name")
        );
        let ProtocolAnchorCause::Missing {
            segments,
            segment,
            scope,
            ..
        } = &cause
        else {
            panic!("a Missing cause, got {cause:?}");
        };
        assert_eq!(segments, &["middle".to_owned()]);
        assert_eq!(segment, "middle");
        assert_eq!(scope, &["P".to_owned()]);
        assert_eq!(located(&text, &cause), "middle");
    }

    /// The record and object cases only (no criterion is claimed whole): a
    /// role on a record type and a channel carrying an object type refuse
    /// `invalid_model_binding`/`malformed-declaration` at the type name, and
    /// a role on an object type and channels carrying a record, an enum and
    /// a scalar check.
    #[trace("TC-1065")]
    #[test]
    fn a_role_on_a_record_and_a_channel_of_an_object_refuse_at_the_type_name() {
        let source =
            pipeline(&pay(), RUN).replacen("role Seller on M::Actor", "role Seller on Plain", 1);
        let (text, cause, code, tag) = only_refusal(&source);
        assert_eq!(
            (code, tag.as_str()),
            ("invalid_model_binding", "malformed-declaration")
        );
        let ProtocolAnchorCause::RoleModel { role, context, .. } = &cause else {
            panic!("a RoleModel cause, got {cause:?}");
        };
        assert_eq!((role.as_str(), context.as_str()), ("Seller", "Plain"));
        assert_eq!(located(&text, &cause), "Plain");

        let source = pipeline(&pay(), RUN).replacen("carries Plain", "carries M::Actor", 1);
        let (text, cause, code, tag) = only_refusal(&source);
        assert_eq!(
            (code, tag.as_str()),
            ("invalid_model_binding", "malformed-declaration")
        );
        let ProtocolAnchorCause::ChannelCarriesObject {
            channel: channel_name,
            name,
            ..
        } = &cause
        else {
            panic!("a ChannelCarriesObject cause, got {cause:?}");
        };
        assert_eq!((channel_name.as_str(), name.as_str()), ("Pay", "M::Actor"));
        assert_eq!(located(&text, &cause), "M::Actor");

        let channels: Vec<String> = ["Plain", "Color", "Boolean"]
            .iter()
            .enumerate()
            .map(|(index, carries)| {
                channel(
                    &format!("C{index}"),
                    ENDS,
                    carries,
                    "unknown",
                    "1",
                    "reject",
                )
            })
            .collect();
        let graph = checks(&pipeline(&channels.join("\n"), RUN));
        assert_eq!(recorded(&graph).len(), 3);
    }

    /// FR-211, FR-212: a second `terminal` or `scheduling` member refuses
    /// `ambiguous_declaration`/`ambiguous-name` at its own span, naming the
    /// first; one of each checks.
    #[trace("TC-656")]
    #[test]
    fn a_second_terminal_or_scheduling_member_refuses_at_its_span() {
        checks(&pipeline("terminal any;\nscheduling adversarial;", RUN));
        for (members, member, name) in [
            ("terminal any;\nterminal any;", "terminal any;", "terminal"),
            (
                "scheduling adversarial;\nscheduling adversarial;",
                "scheduling adversarial;",
                "scheduling",
            ),
        ] {
            let (text, cause, code, tag) = only_refusal(&pipeline(members, RUN));
            assert_eq!(
                (code, tag.as_str()),
                ("ambiguous_declaration", "ambiguous-name")
            );
            let ProtocolAnchorCause::Ambiguous {
                name: found, loci, ..
            } = &cause
            else {
                panic!("an Ambiguous cause, got {cause:?}");
            };
            assert_eq!(found, name);
            assert_eq!(located(&text, &cause), member);
            assert_eq!(loci.len(), 2);
            assert_eq!(&text[loci[0].start..loci[0].end], member);
            assert!(loci[0].start < loci[1].start);
            assert_eq!(loci[1], cause.span());
        }
    }

    /// The parts of a join and a protocol whose expression or function no
    /// checker reads yet refuse `unsupported_construct`/`not-yet-implemented`
    /// rather than compile unchecked.
    #[trace("TC-1065")]
    #[test]
    fn a_predicate_join_and_a_terminal_predicate_are_not_compiled_unchecked() {
        let predicate_join = parallel("predicate(done) [east, west]");
        for (members, run, construct) in [
            ("", predicate_join.as_str(), "join predicate"),
            ("terminal when { true };", RUN, "terminal when"),
        ] {
            let (_, cause, code, tag) = only_refusal(&pipeline(members, run));
            assert_eq!(
                (code, tag.as_str()),
                ("unsupported_construct", "not-yet-implemented")
            );
            let ProtocolAnchorCause::Unimplemented {
                construct: found, ..
            } = &cause
            else {
                panic!("an Unimplemented cause, got {cause:?}");
            };
            assert_eq!(*found, construct);
        }
    }

    /// A protocol with no channel, join or `terminal`/`scheduling` member
    /// checks as it did, and a channel beside an attempt flow is recorded.
    #[trace("TC-513")]
    #[test]
    fn protocols_without_controls_check_as_before_and_a_channel_beside_them_is_recorded() {
        checks(&shared_binder_protocols());
        let flow = attempt_flow("Flow", "");
        let graph = checks(&flow);
        assert!(graph
            .protocol("Flow")
            .expect("the checked protocol")
            .channels
            .is_empty());

        let with_channel = flow.replacen(
            "run sequence",
            "channel C from R to R carries Boolean ordering unordered delivery unknown \
             capacity 1 overflow loss new;\nrun sequence",
            1,
        );
        assert_ne!(with_channel, flow);
        let graph = checks(&with_channel);
        let [recorded] = &graph
            .protocol("Flow")
            .expect("the checked protocol")
            .channels[..]
        else {
            panic!("one channel");
        };
        assert_eq!(recorded.overflow, ChannelOverflow::Loss(LossSelector::New));
        assert_eq!(recorded.delivery, ChannelDelivery::Unknown);
    }
}
