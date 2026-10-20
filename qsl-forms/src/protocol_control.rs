// SPDX-License-Identifier: AGPL-3.0-or-later
//! The control-construct and channel members of a protocol declaration's S2
//! forms (FR-308): channels with their premises, `parallel` with its join
//! policy, `repeat` with its optional maximum and loop proof, `choice` with
//! its owner, visible facts and case guards, `await`, roles static and
//! replicated, and the protocol's `terminal` and `scheduling` members.
//!
//! Every builder reads one CST node and records what was written, each part
//! with its span. Nothing is resolved here: a role, a label or a type stays
//! spelled, and the checks that decide whether it is admitted are S3's
//! (FR-218).

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};
use qsl_foundation::{Span, Spanned};

use super::dispatch::FormsRefusal;
use super::syntax::{
    ChoiceCaseForm, ChoiceForm, ClockForm, DeclaredName, ExpressionForm, IntervalForm, NameForm,
    ParameterForm, ReplicatedRoleForm, RoleForm, RoleLifetime, TypeForm, UsingAlias,
};
use super::temporal_clause::{expression_form, interval_form, parameter_form};
use super::value::{
    declared_name, has_token, items, name_form, nodes_of, only, text, tokens_of, type_form,
    unexpected, Item,
};

/// How a channel orders its messages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChannelOrdering {
    /// `ordering unordered`.
    Unordered,
    /// `ordering fifo by (binder: T) { key }`: messages with equal keys keep
    /// their send order.
    Fifo {
        /// The `by (m: T)` binder of one message.
        binder: Box<ParameterForm>,
        /// The key expression over the binder.
        key: ExpressionForm,
    },
}

/// A channel's delivery policy, an authored premise.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelDelivery {
    /// `delivery unknown`.
    Unknown,
    /// `delivery at-most-once`.
    AtMostOnce,
    /// `delivery at-least-once`.
    AtLeastOnce,
    /// `delivery exactly-once-premise`.
    ExactlyOncePremise,
}

/// A channel's capacity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChannelCapacity {
    /// `capacity 4`.
    Literal(u64),
    /// `capacity symbolic(Bound)`: the name stays unresolved.
    Symbolic(String),
}

/// Which queued message a channel drops under `overflow loss`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LossSelector {
    /// `head`: the oldest queued message is dropped.
    Head,
    /// `tail`: the newest queued message is dropped.
    Tail,
    /// `new`: the arriving message is dropped and the queue is unchanged.
    New,
}

/// A channel's overflow policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelOverflow {
    /// `overflow reject`.
    Reject,
    /// `overflow block`.
    Block,
    /// `overflow loss head`, `overflow loss tail` or `overflow loss new`.
    Loss(LossSelector),
}

/// One `channel` member of a protocol, as written (FR-308). The roles, the
/// payload type and the capacity name stay spelled; S3 resolves them
/// (FR-218).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelForm {
    /// The declared channel name.
    pub name: DeclaredName,
    /// The `from` role, as written.
    pub from: DeclaredName,
    /// The `to` role, as written.
    pub to: DeclaredName,
    /// The `carries` type, as spelled.
    pub carries: TypeForm,
    /// The ordering, with the span of its text.
    pub ordering: Spanned<ChannelOrdering>,
    /// The delivery policy, with the span of its text.
    pub delivery: Spanned<ChannelDelivery>,
    /// The capacity, with the span of its text.
    pub capacity: Spanned<ChannelCapacity>,
    /// The overflow policy, with the span of its text.
    pub overflow: Spanned<ChannelOverflow>,
    /// The span of the whole member, `;` included.
    pub span: Span,
}

/// How a `parallel` converges its branches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum JoinPolicy {
    /// `join all`.
    All,
    /// `join any`.
    Any,
    /// `join quorum(n)`.
    Quorum(u64),
    /// `join predicate(name)`: the name stays unresolved.
    Predicate(String),
}

/// What happens to the branches a join leaves outstanding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outstanding {
    /// `outstanding continue`.
    Continue,
    /// `outstanding cancel`.
    Cancel,
}

/// A `parallel`'s memory model name (FR-219): `sc`, `tso` or `ra`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryModelName {
    /// `memory sc`, the interleaving meaning and the default.
    Sc,
    /// `memory tso`.
    Tso,
    /// `memory ra`.
    Ra,
}

impl MemoryModelName {
    /// The model's spelling in source.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sc => "sc",
            Self::Tso => "tso",
            Self::Ra => "ra",
        }
    }

    /// Whether the model is `tso` or `ra`.
    pub fn is_weak(self) -> bool {
        self != Self::Sc
    }
}

/// An attempt's `ordering o` or a `fence`'s ordering (FR-220).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AccessOrdering {
    /// `relaxed`.
    Relaxed,
    /// `acquire`.
    Acquire,
    /// `release`.
    Release,
    /// `acq_rel`.
    AcqRel,
    /// `seq_cst`.
    SeqCst,
}

impl AccessOrdering {
    /// The ordering's spelling in source.
    pub fn label(self) -> &'static str {
        match self {
            Self::Relaxed => "relaxed",
            Self::Acquire => "acquire",
            Self::Release => "release",
            Self::AcqRel => "acq_rel",
            Self::SeqCst => "seq_cst",
        }
    }
}

/// One `fence Name ordering o;` control as written (FR-220).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FenceForm {
    /// The index into `ProtocolDeclarationForm::declarations` of the node
    /// this control declares.
    pub declaration: usize,
    /// The declared control name.
    pub name: DeclaredName,
    /// The ordering, with the span of its text.
    pub ordering: Spanned<AccessOrdering>,
}

/// One `parallel` control as written (FR-308).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParallelForm {
    /// The index into `ProtocolDeclarationForm::declarations` of the node
    /// this control declares.
    pub declaration: usize,
    /// The declared control name.
    pub name: DeclaredName,
    /// The branch labels, in source order.
    pub branches: Vec<DeclaredName>,
    /// The `memory` clause's model with the span of the clause, when
    /// written (FR-219).
    pub memory: Option<Spanned<MemoryModelName>>,
    /// The join policy, with the span of its text.
    pub join: Spanned<JoinPolicy>,
    /// The span of the `quorum` count, written exactly for `Quorum`.
    pub quorum_count: Option<Span>,
    /// The labels the join lists, in source order, repeats kept.
    pub labels: Vec<DeclaredName>,
    /// The span of the bracketed label list, brackets included.
    pub labels_span: Span,
    /// The `outstanding` rule with the span of its text, when written.
    pub outstanding: Option<Spanned<Outstanding>>,
    /// The span of the whole control.
    pub span: Span,
}

/// A `repeat`'s `invariant { } variant { }` pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopProofForm {
    /// The invariant body expression.
    pub invariant: ExpressionForm,
    /// The variant body expression.
    pub variant: ExpressionForm,
    /// The span of the pair, from `invariant` to the end of the variant.
    pub span: Span,
}

/// One `repeat` control as written (FR-308). The body and the `exhausted`
/// control are controls of the declaration's own scopes; their spans locate
/// them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepeatForm {
    /// The index into `ProtocolDeclarationForm::declarations` of the node
    /// this control declares.
    pub declaration: usize,
    /// The declared control name.
    pub name: DeclaredName,
    /// The owning role, written `by R`.
    pub owner: DeclaredName,
    /// The `visible(...)` entries, in source order.
    pub visible: Vec<ExpressionForm>,
    /// The span of the whole `visible(...)` clause.
    pub visible_span: Span,
    /// The iteration maximum with the span of its literal, when written.
    pub max: Option<Spanned<u64>>,
    /// The loop proof, when written.
    pub loop_proof: Option<LoopProofForm>,
    /// The `while { }` guard.
    pub guard: ExpressionForm,
    /// The span of the body control.
    pub body: Span,
    /// The span of the `exhausted` control, written exactly when a maximum
    /// is.
    pub exhausted: Option<Span>,
    /// The span of the whole control.
    pub span: Span,
}

/// One `await` control as written (FR-308).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AwaitForm {
    /// The index into `ProtocolDeclarationForm::declarations` of the node
    /// this control declares.
    pub declaration: usize,
    /// The declared control name.
    pub name: DeclaredName,
    /// The `after` node reference, as spelled.
    pub after: NameForm,
    /// The `using` profile alias.
    pub using: UsingAlias,
    /// The `clock "<string>"`.
    pub clock: ClockForm,
    /// The `within` interval, as written.
    pub within: IntervalForm,
    /// The name of the event node the `match` clause holds.
    pub matched: DeclaredName,
    /// The span of the `then` control.
    pub then: Span,
    /// The span of the `timeout` control.
    pub timeout: Span,
    /// The span of the whole control.
    pub span: Span,
}

/// A protocol's `terminal` member (FR-308).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalForm {
    /// `terminal when { predicate };`.
    When {
        /// The predicate body expression.
        predicate: ExpressionForm,
        /// The span of the whole member, `;` included.
        span: Span,
    },
    /// `terminal any;`.
    Any {
        /// The span of the whole member, `;` included.
        span: Span,
    },
}

/// A protocol's `scheduling` member (FR-308).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchedulingForm {
    /// `scheduling adversarial;`.
    Adversarial {
        /// The span of the whole member, `;` included.
        span: Span,
    },
}

/// One `terminal` or `scheduling` member of a protocol clause. S2 keeps
/// every one it finds, in source order, so S3 can refuse a second of either
/// kind at its span (FR-211, FR-212).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolMemberForm {
    /// A `terminal` member.
    Terminal(TerminalForm),
    /// A `scheduling` member.
    Scheduling(SchedulingForm),
}

impl ProtocolMemberForm {
    /// The span of the whole member.
    pub fn span(&self) -> Span {
        match self {
            Self::Terminal(TerminalForm::When { span, .. } | TerminalForm::Any { span }) => *span,
            Self::Scheduling(SchedulingForm::Adversarial { span }) => *span,
        }
    }
}

/// `node`'s own tokens, in order.
fn own_tokens<'c>(items: &[Item<'c>]) -> Vec<&'c CstToken> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Token(token) => Some(*token),
            Item::Node(_) => None,
        })
        .collect()
}

/// The first own token spelled `spelling`.
fn token_spelled<'c>(items: &[Item<'c>], spelling: &[u8]) -> Option<&'c CstToken> {
    own_tokens(items)
        .into_iter()
        .find(|token| token.spelling() == spelling)
}

fn declared(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// The `Expression` inside one `Block` node.
fn block_expression(cst: &LosslessCst, block: &CstNode) -> Result<ExpressionForm, FormsRefusal> {
    let block_items = items(cst, block);
    expression_form(cst, only(&block_items, Production::Expression, block)?)
}

/// An unsigned decimal `u64` token, which S1 admits only in that range.
fn u64_token(token: &CstToken, node: &CstNode) -> Result<u64, FormsRefusal> {
    text(token, node)?
        .parse::<u64>()
        .map_err(|_| unexpected(node))
}

fn ordering_form(cst: &LosslessCst, ordering: &CstNode) -> Result<ChannelOrdering, FormsRefusal> {
    let ordering_items = items(cst, ordering);
    if has_token(&ordering_items, b"unordered") {
        return Ok(ChannelOrdering::Unordered);
    }
    if has_token(&ordering_items, b"fifo") {
        return Ok(ChannelOrdering::Fifo {
            binder: Box::new(parameter_form(
                cst,
                only(&ordering_items, Production::Parameter, ordering)?,
            )?),
            key: block_expression(cst, only(&ordering_items, Production::Block, ordering)?)?,
        });
    }
    Err(unexpected(ordering))
}

fn delivery_form(cst: &LosslessCst, delivery: &CstNode) -> Result<ChannelDelivery, FormsRefusal> {
    let delivery_items = items(cst, delivery);
    if has_token(&delivery_items, b"unknown") {
        Ok(ChannelDelivery::Unknown)
    } else if has_token(&delivery_items, b"at-most-once") {
        Ok(ChannelDelivery::AtMostOnce)
    } else if has_token(&delivery_items, b"at-least-once") {
        Ok(ChannelDelivery::AtLeastOnce)
    } else if has_token(&delivery_items, b"exactly-once-premise") {
        Ok(ChannelDelivery::ExactlyOncePremise)
    } else {
        Err(unexpected(delivery))
    }
}

fn capacity_form(cst: &LosslessCst, capacity: &CstNode) -> Result<ChannelCapacity, FormsRefusal> {
    let capacity_items = items(cst, capacity);
    let numbers = tokens_of(&capacity_items, TokenKind::Integer);
    let names = tokens_of(&capacity_items, TokenKind::Identifier);
    match (numbers.as_slice(), names.as_slice()) {
        ([number], []) => Ok(ChannelCapacity::Literal(u64_token(number, capacity)?)),
        ([], [name]) => Ok(ChannelCapacity::Symbolic(text(name, capacity)?)),
        _ => Err(unexpected(capacity)),
    }
}

fn overflow_form(cst: &LosslessCst, overflow: &CstNode) -> Result<ChannelOverflow, FormsRefusal> {
    let overflow_items = items(cst, overflow);
    if has_token(&overflow_items, b"reject") {
        Ok(ChannelOverflow::Reject)
    } else if has_token(&overflow_items, b"block") {
        Ok(ChannelOverflow::Block)
    } else if has_token(&overflow_items, b"loss") {
        if has_token(&overflow_items, b"head") {
            Ok(ChannelOverflow::Loss(LossSelector::Head))
        } else if has_token(&overflow_items, b"tail") {
            Ok(ChannelOverflow::Loss(LossSelector::Tail))
        } else if has_token(&overflow_items, b"new") {
            Ok(ChannelOverflow::Loss(LossSelector::New))
        } else {
            Err(unexpected(overflow))
        }
    } else {
        Err(unexpected(overflow))
    }
}

/// `channel C from R to R carries T ordering o delivery d capacity c
/// overflow p;` (FR-308): every premise as written, with its span.
pub(crate) fn channel_form(
    cst: &LosslessCst,
    channel: &CstNode,
) -> Result<ChannelForm, FormsRefusal> {
    let channel_items = items(cst, channel);
    let [name, from, to] = tokens_of(&channel_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(channel));
    };
    let ordering = only(&channel_items, Production::Ordering, channel)?;
    let delivery = only(&channel_items, Production::DeliveryPolicy, channel)?;
    let capacity = only(&channel_items, Production::Capacity, channel)?;
    let overflow = only(&channel_items, Production::OverflowPolicy, channel)?;
    Ok(ChannelForm {
        name: declared(name, channel)?,
        from: declared(from, channel)?,
        to: declared(to, channel)?,
        carries: type_form(
            cst,
            only(&channel_items, Production::TypeReference, channel)?,
        )?,
        ordering: Spanned {
            value: ordering_form(cst, ordering)?,
            span: ordering.span(),
        },
        delivery: Spanned {
            value: delivery_form(cst, delivery)?,
            span: delivery.span(),
        },
        capacity: Spanned {
            value: capacity_form(cst, capacity)?,
            span: capacity.span(),
        },
        overflow: Spanned {
            value: overflow_form(cst, overflow)?,
            span: overflow.span(),
        },
        span: channel.span(),
    })
}

/// A `JoinPolicy` node as its policy and, for `quorum`, the count's span.
fn join_policy(
    cst: &LosslessCst,
    join: &CstNode,
) -> Result<(JoinPolicy, Option<Span>), FormsRefusal> {
    let join_items = items(cst, join);
    if has_token(&join_items, b"all") {
        Ok((JoinPolicy::All, None))
    } else if has_token(&join_items, b"any") {
        Ok((JoinPolicy::Any, None))
    } else if has_token(&join_items, b"quorum") {
        let [count] = tokens_of(&join_items, TokenKind::Integer)[..] else {
            return Err(unexpected(join));
        };
        Ok((
            JoinPolicy::Quorum(u64_token(count, join)?),
            Some(count.span()),
        ))
    } else if has_token(&join_items, b"predicate") {
        let [name] = tokens_of(&join_items, TokenKind::Identifier)[..] else {
            return Err(unexpected(join));
        };
        Ok((JoinPolicy::Predicate(text(name, join)?), None))
    } else {
        Err(unexpected(join))
    }
}

/// The `outstanding continue|cancel` rule among a `parallel`'s own tokens.
fn outstanding(
    parallel_items: &[Item<'_>],
    parallel: &CstNode,
) -> Result<Option<Spanned<Outstanding>>, FormsRefusal> {
    let tokens = own_tokens(parallel_items);
    let Some(at) = tokens
        .iter()
        .position(|token| token.spelling() == b"outstanding")
    else {
        return Ok(None);
    };
    let (Some(keyword), Some(mode)) = (tokens.get(at), tokens.get(at + 1)) else {
        return Err(unexpected(parallel));
    };
    let value = if mode.spelling() == b"continue" {
        Outstanding::Continue
    } else if mode.spelling() == b"cancel" {
        Outstanding::Cancel
    } else {
        return Err(unexpected(parallel));
    };
    Ok(Some(Spanned {
        value,
        span: Span {
            start: keyword.span().start,
            end: mode.span().end,
        },
    }))
}

/// A `memory sc | tso | ra` clause (FR-219): the model and the clause's
/// span.
fn memory_model(
    cst: &LosslessCst,
    clause: &CstNode,
) -> Result<Spanned<MemoryModelName>, FormsRefusal> {
    let clause_items = items(cst, clause);
    let model = own_tokens(&clause_items)
        .into_iter()
        .nth(1)
        .ok_or_else(|| unexpected(clause))?;
    let value = match model.spelling() {
        b"sc" => MemoryModelName::Sc,
        b"tso" => MemoryModelName::Tso,
        b"ra" => MemoryModelName::Ra,
        _ => return Err(unexpected(clause)),
    };
    Ok(Spanned {
        value,
        span: clause.span(),
    })
}

/// A `MemoryOrder` or `FenceOrder` node (FR-220): the ordering and the span
/// of its text.
pub(crate) fn access_ordering(
    cst: &LosslessCst,
    order: &CstNode,
) -> Result<Spanned<AccessOrdering>, FormsRefusal> {
    let order_items = items(cst, order);
    let token = own_tokens(&order_items)
        .into_iter()
        .next()
        .ok_or_else(|| unexpected(order))?;
    let value = match token.spelling() {
        b"relaxed" => AccessOrdering::Relaxed,
        b"acquire" => AccessOrdering::Acquire,
        b"release" => AccessOrdering::Release,
        b"acq_rel" => AccessOrdering::AcqRel,
        b"seq_cst" => AccessOrdering::SeqCst,
        _ => return Err(unexpected(order)),
    };
    Ok(Spanned {
        value,
        span: order.span(),
    })
}

/// `parallel P { branch a ..; branch b ..; } join policy [a, b]
/// [outstanding rule];` (FR-308). `declaration` is the index of the
/// control's own declaration.
pub(crate) fn parallel_form(
    cst: &LosslessCst,
    parallel: &CstNode,
    declaration: usize,
) -> Result<ParallelForm, FormsRefusal> {
    let parallel_items = items(cst, parallel);
    // The control's name and then the joined labels are the only direct
    // identifier tokens: the branches and the policy are nested nodes.
    let identifiers = tokens_of(&parallel_items, TokenKind::Identifier);
    let Some((name, labels)) = identifiers.split_first() else {
        return Err(unexpected(parallel));
    };
    let branches = nodes_of(&parallel_items, Production::Branch)
        .into_iter()
        .map(|branch| declared_name(&items(cst, branch), branch))
        .collect::<Result<Vec<_>, _>>()?;
    let labels = labels
        .iter()
        .map(|label| declared(label, parallel))
        .collect::<Result<Vec<_>, _>>()?;
    let join_node = only(&parallel_items, Production::JoinPolicy, parallel)?;
    let (policy, quorum_count) = join_policy(cst, join_node)?;
    let (Some(open), Some(close)) = (
        token_spelled(&parallel_items, b"["),
        token_spelled(&parallel_items, b"]"),
    ) else {
        return Err(unexpected(parallel));
    };
    let memory = nodes_of(&parallel_items, Production::MemoryClause)
        .first()
        .map(|clause| memory_model(cst, clause))
        .transpose()?;
    Ok(ParallelForm {
        declaration,
        name: declared(name, parallel)?,
        branches,
        memory,
        join: Spanned {
            value: policy,
            span: join_node.span(),
        },
        quorum_count,
        labels,
        labels_span: Span {
            start: open.span().start,
            end: close.span().end,
        },
        outstanding: outstanding(&parallel_items, parallel)?,
        span: parallel.span(),
    })
}

/// `repeat R by role visible (..) [max n] [invariant { } variant { }] while
/// { } body [exhausted c]` (FR-308): the maximum, the loop proof and the
/// `exhausted` control exactly as written. `declaration` is the index of the
/// control's own declaration.
pub(crate) fn repeat_form(
    cst: &LosslessCst,
    repeat: &CstNode,
    declaration: usize,
) -> Result<RepeatForm, FormsRefusal> {
    let repeat_items = items(cst, repeat);
    let [name, owner] = tokens_of(&repeat_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(repeat));
    };
    let visibility = only(&repeat_items, Production::Visibility, repeat)?;
    let visible = nodes_of(&items(cst, visibility), Production::Expression)
        .into_iter()
        .map(|entry| expression_form(cst, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let max = if has_token(&repeat_items, b"max") {
        let [bound] = tokens_of(&repeat_items, TokenKind::Integer)[..] else {
            return Err(unexpected(repeat));
        };
        Some(Spanned {
            value: u64_token(bound, repeat)?,
            span: bound.span(),
        })
    } else {
        None
    };
    let blocks = nodes_of(&repeat_items, Production::Block);
    let (loop_proof, guard) = match (
        token_spelled(&repeat_items, b"invariant"),
        blocks.as_slice(),
    ) {
        (Some(keyword), [invariant, variant, guard]) => (
            Some(LoopProofForm {
                invariant: block_expression(cst, invariant)?,
                variant: block_expression(cst, variant)?,
                span: Span {
                    start: keyword.span().start,
                    end: variant.span().end,
                },
            }),
            *guard,
        ),
        (None, [guard]) => (None, *guard),
        _ => return Err(unexpected(repeat)),
    };
    let controls = nodes_of(&repeat_items, Production::Control);
    let (body, exhausted) = match (max.is_some(), controls.as_slice()) {
        (false, [body]) => (*body, None),
        (true, [body, exhausted]) => (*body, Some(exhausted.span())),
        _ => return Err(unexpected(repeat)),
    };
    Ok(RepeatForm {
        declaration,
        name: declared(name, repeat)?,
        owner: declared(owner, repeat)?,
        visible,
        visible_span: visibility.span(),
        max,
        loop_proof,
        guard: block_expression(cst, guard)?,
        body: body.span(),
        exhausted,
        span: repeat.span(),
    })
}

/// `role R on M::T;` or `role R each M::T from population max n lifetime
/// l;` (FR-308): the name, the model and, for a replicated role, its
/// population, maximum and lifetime, as written.
pub(crate) fn role_form(cst: &LosslessCst, role: &CstNode) -> Result<RoleForm, FormsRefusal> {
    let role_items = items(cst, role);
    let context = name_form(cst, only(&role_items, Production::QualifiedName, role)?)?;
    let replicated = if has_token(&role_items, b"each") {
        // `role R each M::T from p max n lifetime l;`: the role's name and
        // the population are its only direct identifier tokens.
        let [_, population] = tokens_of(&role_items, TokenKind::Identifier)[..] else {
            return Err(unexpected(role));
        };
        let [max] = tokens_of(&role_items, TokenKind::Integer)[..] else {
            return Err(unexpected(role));
        };
        let lifetime = only(&role_items, Production::RoleLifetime, role)?;
        let lifetime_items = items(cst, lifetime);
        let lifetime = if let Some(token) = token_spelled(&lifetime_items, b"workflow") {
            RoleLifetime::Workflow(token.span())
        } else if let Some(token) = token_spelled(&lifetime_items, b"scope") {
            RoleLifetime::Scope(token.span())
        } else {
            RoleLifetime::Until(expression_form(
                cst,
                only(&lifetime_items, Production::Expression, lifetime)?,
            )?)
        };
        Some(ReplicatedRoleForm {
            population: declared(population, role)?,
            max: u64_token(max, role)?,
            max_span: max.span(),
            lifetime,
        })
    } else {
        None
    };
    Ok(RoleForm {
        name: declared_name(&role_items, role)?,
        context,
        replicated,
        span: role.span(),
    })
}

/// `choice N by R visible (..) { case a when { g } c ... }` (FR-308): the
/// owner, the visible entries and each case's label and guard, as written.
/// `declaration` is the index of the control's own declaration.
pub(crate) fn choice_form(
    cst: &LosslessCst,
    choice: &CstNode,
    declaration: usize,
) -> Result<ChoiceForm, FormsRefusal> {
    let choice_items = items(cst, choice);
    let [name, owner] = tokens_of(&choice_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(choice));
    };
    let visibility = only(&choice_items, Production::Visibility, choice)?;
    let visible = nodes_of(&items(cst, visibility), Production::Expression)
        .into_iter()
        .map(|entry| expression_form(cst, entry))
        .collect::<Result<Vec<_>, _>>()?;
    let cases = nodes_of(&choice_items, Production::Case)
        .into_iter()
        .map(|case| {
            let case_items = items(cst, case);
            Ok(ChoiceCaseForm {
                name: declared_name(&case_items, case)?,
                guard: block_expression(cst, only(&case_items, Production::Block, case)?)?,
            })
        })
        .collect::<Result<Vec<_>, FormsRefusal>>()?;
    Ok(ChoiceForm {
        declaration,
        name: declared(name, choice)?,
        owner: declared(owner, choice)?,
        visible,
        visible_span: visibility.span(),
        cases,
        span: choice.span(),
    })
}

/// `await N after R using alias clock "c" within i match Event then c1
/// timeout c2` (FR-308). `declaration` is the index of the control's own
/// declaration.
pub(crate) fn await_form(
    cst: &LosslessCst,
    await_node: &CstNode,
    declaration: usize,
) -> Result<AwaitForm, FormsRefusal> {
    let await_items = items(cst, await_node);
    let [name, alias] = tokens_of(&await_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(await_node));
    };
    let [clock] = tokens_of(&await_items, TokenKind::Text)[..] else {
        return Err(unexpected(await_node));
    };
    let controls = nodes_of(&await_items, Production::Control);
    let [then, timeout] = controls[..] else {
        return Err(unexpected(await_node));
    };
    let matched_event = only(&await_items, Production::EventNode, await_node)?;
    Ok(AwaitForm {
        declaration,
        name: declared(name, await_node)?,
        after: name_form(
            cst,
            only(&await_items, Production::NodeReference, await_node)?,
        )?,
        using: {
            let alias = declared(alias, await_node)?;
            UsingAlias {
                alias: alias.name,
                span: alias.span,
            }
        },
        clock: ClockForm {
            text: qsl_cst::token::decode_quoted(&text(clock, await_node)?)
                .ok_or_else(|| unexpected(await_node))?,
            span: clock.span(),
        },
        within: interval_form(cst, only(&await_items, Production::Interval, await_node)?)?,
        matched: declared_name(&items(cst, matched_event), matched_event)?,
        then: then.span(),
        timeout: timeout.span(),
        span: await_node.span(),
    })
}

/// The protocol clause's `terminal` and `scheduling` members among
/// `clause_items`, in source order.
pub(crate) fn member_forms(
    cst: &LosslessCst,
    clause_items: &[Item<'_>],
) -> Result<Vec<ProtocolMemberForm>, FormsRefusal> {
    let mut members = Vec::new();
    for item in clause_items {
        let Item::Node(node) = item else {
            continue;
        };
        match node.production() {
            Production::ProtocolTerminal => {
                let terminal_items = items(cst, node);
                let terminal = if has_token(&terminal_items, b"any") {
                    TerminalForm::Any { span: node.span() }
                } else if has_token(&terminal_items, b"when") {
                    TerminalForm::When {
                        predicate: block_expression(
                            cst,
                            only(&terminal_items, Production::Block, node)?,
                        )?,
                        span: node.span(),
                    }
                } else {
                    return Err(unexpected(node));
                };
                members.push(ProtocolMemberForm::Terminal(terminal));
            }
            Production::ProtocolScheduling => {
                members.push(ProtocolMemberForm::Scheduling(
                    SchedulingForm::Adversarial { span: node.span() },
                ));
            }
            _ => {}
        }
    }
    Ok(members)
}
