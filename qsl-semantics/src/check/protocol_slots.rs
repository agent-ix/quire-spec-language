// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-218 "Scopes" and "Choice and repeat decisions": the S3 check of every
//! expression slot of a protocol's `run` tree and `finish` -- each node's
//! body, each `check` condition, each `choice`'s and `repeat`'s `visible`
//! entries and guards and each `repeat`'s loop proof -- over the binders
//! readable at it, and of each `choice`'s partition. The clause's own
//! `terminal when` predicate (FR-211) is checked the same way, over the
//! `over` input and, under `on origin`, the protocol captures; so is each
//! end of an event node's `related by` clauses, against its relationship
//! end's type, and each replicated role's `until` predicate, over `C0` and
//! the role's own binder (QSpec FR-464 "Readable binders").
//!
//! The readable binders follow QSpec FR-466 "Binder scope": `In(n)` is
//! what a control reads at its entry and `Out(n)` what is readable after it
//! completes on every path. A sequence threads them through its children,
//! a `parallel` joined by `all` adds every branch's, a step adds its own
//! binder, and a `choice`, a `repeat` and an `await` add nothing past
//! themselves (the matched event is readable on `then` only). The root
//! reads the protocol's `over` input, its activation binder and its
//! captures, and `finish` reads `Out(root)` and its own binder.
//!
//! Each slot is typed through the one clause checker (FR-104) over every
//! binder of the protocol, then refused in this order: a read of model state
//! or of `over` in a slot that bars it (a `check`, a guard, a `visible`
//! entry), `ill_typed`/`operator-ineligible`; a read of a binder that is not
//! readable at the slot, `missing_declaration`/`missing-name` at every such
//! read, as FR-113 refuses an unresolved name; and, for a decision, a read
//! of a fact that is not an eligible, individually listed fact of the owner,
//! `wrong_snapshot`/`wrong-observation`. A `choice`'s guards are then proved
//! exhaustive and exclusive over its visible entries, and every other
//! Boolean subterm of a guard, as independent Booleans.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    ActivationForm, BinaryOperator, BinderKind, ExprNode, ExprRef, ExpressionForm,
    ProtocolDeclarationForm, ProtocolMemberForm, ProtocolNodeKind, RoleLifetime, TerminalForm,
};
use qsl_foundation::Span;
use quire_exact::ValueType;
use quire_semantic_value::location::{Location, Origin};

use super::check::Scope;
use super::ir::{CheckedBody, NodeKind};
use super::protocol_clause::{record_binder, CheckedProtocol, ProtocolNodeId};
use super::protocol_obligation::{model_state_reads, span_of, type_body, wraps, Goal};
use super::refusal::{
    ArmFault, CheckCause, CheckRefusal, ObservationReason, ProtocolAnchorCause, RelatedEnd,
    SlotKind,
};
use super::state_clause::ClauseDeclarations;
use super::type_form::resolve_form;
use crate::family::CheckContext;

/// One `case` of a checked `choice`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedCase {
    /// The case label.
    pub name: String,
    /// The checked guard.
    pub guard: CheckedBody,
}

/// One checked `choice` (FR-218 "Outputs": the owner and visible facts of
/// each `choice`).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedChoice {
    /// The `choice` node.
    pub node: ProtocolNodeId,
    /// The index of the owning role among the protocol's roles.
    pub owner: usize,
    /// The checked `visible` entries, in source order.
    pub visible: Vec<CheckedBody>,
    /// The cases, in source order.
    pub cases: Vec<CheckedCase>,
}

/// A `repeat`'s checked `invariant` and `variant`.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedLoopProof {
    /// The invariant.
    pub invariant: CheckedBody,
    /// The variant; only a termination claim reads it.
    pub variant: CheckedBody,
}

/// One checked `repeat` (FR-218 "Constructs": the maximum, invariant and
/// variant it has).
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedRepeat {
    /// The `repeat` node.
    pub node: ProtocolNodeId,
    /// The index of the owning role among the protocol's roles.
    pub owner: usize,
    /// The checked `visible` entries, in source order.
    pub visible: Vec<CheckedBody>,
    /// The iteration maximum, when one is written.
    pub max: Option<u64>,
    /// The loop proof, when one is written.
    pub loop_proof: Option<CheckedLoopProof>,
    /// The checked `while` guard.
    pub guard: CheckedBody,
}

/// The checked body of one node: an event node's constraint, a `check`'s
/// condition or the `finish` constraint.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedNodeBody {
    /// The node.
    pub node: ProtocolNodeId,
    /// The checked body.
    pub body: CheckedBody,
}

/// A checked replicated role's lifetime (FR-218 "Scopes", QSpec FR-428).
#[derive(Clone, Debug, PartialEq)]
pub enum CheckedLifetime {
    /// `lifetime workflow`: the instance retires when its protocol
    /// instance finishes.
    Workflow,
    /// `lifetime scope`: the smallest control of the `run` tree that holds
    /// every node `by` the role, strictly (a node is not its own region);
    /// `None` when no node of the tree is `by` the role.
    Scope(Option<ProtocolNodeId>),
    /// `lifetime until(p)`: the checked Boolean `p`, over `C0` and the
    /// role's binder.
    Until(CheckedBody),
}

/// One checked replicated role `role r each T from pop max n lifetime L`
/// (FR-218 "Outputs", QSpec FR-465 "Role").
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedReplicatedRole {
    /// The index of the role among the protocol's roles; its binder is
    /// named by the role and typed `Reference` to its object type.
    pub role: usize,
    /// The population the instances are drawn from, as written.
    pub population: String,
    /// The authored limit on live instances at once, at least 1.
    pub max: u64,
    /// The lifetime of one instance.
    pub lifetime: CheckedLifetime,
}

/// One checked `related by r (a, b)` clause of an event node (FR-218
/// "Relationships").
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedRelated {
    /// The event node.
    pub node: ProtocolNodeId,
    /// The index of the relationship among the checked protocol's
    /// relationships.
    pub relationship: usize,
    /// The first and second end, each of its relationship end's type.
    pub ends: [CheckedBody; 2],
}

/// The nodes that read one binder (FR-218 "Scopes").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedReaders {
    /// The binder's name.
    pub binder: String,
    /// The nodes whose slots read it, in declaration order.
    pub readers: Vec<ProtocolNodeId>,
}

/// A protocol's intended terminal states (FR-211 "Outputs", the three
/// cases of FR-124's declaration).
#[derive(Clone, Debug, Default, PartialEq)]
pub enum CheckedTerminalMember {
    /// The protocol writes no `terminal` member.
    #[default]
    None,
    /// `terminal when { P }`: the checked Boolean predicate.
    When(CheckedBody),
    /// `terminal any`: every terminal state is intended.
    Any,
}

/// A protocol's scheduling premise (FR-212 "Targets at S3"): whether the
/// model checker adds the derived scheduler fairness constraints.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckedScheduling {
    /// The protocol writes no `scheduling` member: every runnable thread is
    /// served.
    #[default]
    Fair,
    /// `scheduling adversarial;`: the fairness set is the authored
    /// constraints alone.
    Adversarial,
}

/// Every checked slot of a protocol's `run` tree and `finish`, each list in
/// source order, and each binder's readers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CheckedSlots {
    /// The protocol's `terminal` member (FR-211).
    pub terminal: CheckedTerminalMember,
    /// The protocol's `scheduling` member (FR-212).
    pub scheduling: CheckedScheduling,
    /// The node bodies.
    pub bodies: Vec<CheckedNodeBody>,
    /// The choices.
    pub choices: Vec<CheckedChoice>,
    /// The repeats.
    pub repeats: Vec<CheckedRepeat>,
    /// Every replicated role, in source order.
    pub roles: Vec<CheckedReplicatedRole>,
    /// Every `related by` clause, by node in declaration order and each
    /// node's clauses in source order.
    pub related: Vec<CheckedRelated>,
    /// The readers of every binder the `run` tree and `finish` can read:
    /// the `over` input, the activation binder, the protocol captures and
    /// each node's binder, in that order.
    pub readers: Vec<CheckedReaders>,
    /// The same binders, index-aligned with `readers`: binder `i` is local
    /// slot `i` of every node body, guard and `terminal when` predicate the
    /// slots hold.
    pub binders: Vec<CheckedSlotBinder>,
}

/// One binder a protocol's slots read (FR-218 "Scopes"), at the local slot
/// of its index in [`CheckedSlots::binders`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedSlotBinder {
    /// The binder's name.
    pub name: String,
    /// Its declared type.
    pub value_type: ValueType,
    /// The node whose record binder it is; `None` for the `over` input, the
    /// activation binder and a protocol capture.
    pub node: Option<ProtocolNodeId>,
    /// Whether the binder ranges over its record type's values: the binder
    /// of an `event`, `effect`, `commit`, `send`, `receive` or `finish` step
    /// (ADR-027 TS-5), which a model check classifies as a root.
    pub ranged: bool,
}

/// What [`check`] found.
pub(crate) enum Slots {
    /// Every slot checked, with the package's running node total.
    Checked(CheckedSlots, u64),
    /// Refusals, in source order.
    Refused(Vec<CheckRefusal>),
    /// A binder's declared type did not resolve: `protocol_clause::content`
    /// refuses it, so there is nothing to add.
    Unresolved,
}

/// Who declares a binder the `run` tree can read.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BinderOrigin {
    /// The protocol's `over` input.
    Input,
    /// The `activation on each` binder.
    Activation,
    /// A protocol capture.
    Capture,
    /// The record binder of this node.
    Node(usize),
}

struct Binder {
    name: String,
    value_type: ValueType,
    origin: BinderOrigin,
}

/// Whether a binder is a decision fact of a role (FR-218 "Choice and
/// repeat decisions", the eligibility table).
enum Eligibility {
    /// A protocol capture: every role holds it.
    Capture,
    /// The `over` input: model state, refused before this table.
    Input,
    /// Eligible for exactly this role.
    Role(String),
    /// Never eligible.
    Never,
}

fn refusal(cause: ProtocolAnchorCause) -> CheckRefusal {
    CheckRefusal {
        // A protocol has no expression tree for a `Location` path to walk;
        // `region::DeclarationRegions::refusal_region` reads the cause's own
        // span.
        location: Location::root(Origin::Expression),
        cause: CheckCause::ProtocolAnchor(Box::new(cause)),
    }
}

/// What holds a slot: a node of the `run` tree or `finish`, whose reads
/// are recorded as that node's, or the protocol clause itself (its
/// `terminal when` predicate), which no node reads for.
#[derive(Clone, Copy)]
enum SlotOwner {
    /// The node of this index.
    Node(usize),
    /// The protocol clause.
    Clause,
    /// The role of this index: its `until` predicate, read over `C0` and
    /// the role's own binder only (QSpec FR-464 stage `role-lifetime`).
    Role(usize),
}

/// One slot to check: what holds it, which slot, its form, its goal, the
/// binders readable at it and whether it bars model state.
struct Slot<'s> {
    owner: SlotOwner,
    kind: SlotKind,
    form: &'s ExpressionForm,
    goal: Goal<'s>,
    readable: &'s BTreeSet<usize>,
    bars_model_state: bool,
}

struct Checker<'c, 'a> {
    protocol: &'c ProtocolDeclarationForm,
    declarations: &'c ClauseDeclarations<'a>,
    binders: Vec<Binder>,
    bound: Vec<(String, ValueType)>,
    by_name: BTreeMap<String, usize>,
    nodes: u64,
    readers: Vec<BTreeSet<usize>>,
    refusals: Vec<(Span, CheckRefusal)>,
    /// The number of `C0` binders: the `over` input, the activation binder
    /// and the protocol captures, the first entries of `binders`.
    roots: usize,
    /// Each replicated role's own binder, by role index: its name and
    /// `Reference` to the role's object type.
    role_binders: BTreeMap<usize, (String, ValueType)>,
}

impl Checker<'_, '_> {
    fn name(&self, node: usize) -> String {
        self.protocol
            .declarations
            .get(node)
            .map(|declaration| declaration.name.name.clone())
            .unwrap_or_default()
    }

    fn refuse(&mut self, cause: ProtocolAnchorCause) {
        self.refusals.push((cause.span(), refusal(cause)));
    }

    /// `slot` typed over every binder and checked against the slot's rules,
    /// in the order the module doc states; `None` when it refuses.
    fn slot(&mut self, slot: &Slot<'_>, cx: &mut CheckContext<'_, Scope>) -> Option<CheckedBody> {
        let node_name = match slot.owner {
            SlotOwner::Node(node) => self.name(node),
            SlotOwner::Clause => self.protocol.name.name.clone(),
            SlotOwner::Role(role) => self
                .protocol
                .roles
                .get(role)
                .map(|role| role.name.name.clone())
                .unwrap_or_default(),
        };
        // A role's `until` is typed over `C0` and the role's binder alone,
        // so every name it binds is readable there.
        let role_bound: Option<Vec<(String, ValueType)>> = match slot.owner {
            SlotOwner::Role(role) => Some(
                self.bound
                    .iter()
                    .take(self.roots)
                    .chain(self.role_binders.get(&role))
                    .cloned()
                    .collect(),
            ),
            SlotOwner::Node(_) | SlotOwner::Clause => None,
        };
        let mut nodes = self.nodes;
        let typed = type_body(
            slot.form,
            role_bound.as_deref().unwrap_or(&self.bound),
            slot.goal,
            self.declarations,
            &mut nodes,
            cx,
        );
        self.nodes = nodes;
        let body = match typed {
            Ok(body) => body,
            Err(CheckRefusal { location, cause }) => {
                let span = span_of(slot.form, &location);
                if wraps(&cause) && cause.cause().is_some() {
                    self.refuse(ProtocolAnchorCause::SlotClause {
                        node: node_name,
                        slot: slot.kind,
                        cause: Box::new(cause),
                        span,
                    });
                } else {
                    // A reached limit, a cancellation or a fault keeps its
                    // own cause.
                    self.refusals.push((span, CheckRefusal { location, cause }));
                }
                return None;
            }
        };
        if slot.bars_model_state {
            let input = self
                .binders
                .iter()
                .position(|binder| binder.origin == BinderOrigin::Input);
            let reads = model_state_reads(&body, slot.form, input);
            if !reads.is_empty() {
                for span in reads {
                    self.refuse(ProtocolAnchorCause::SlotModelState {
                        node: node_name.clone(),
                        slot: slot.kind,
                        span,
                    });
                }
                return None;
            }
        }
        if role_bound.is_some() {
            return Some(body);
        }
        let mut out_of_scope = Vec::new();
        let mut read = BTreeSet::new();
        for checked in body.root().descendants() {
            let NodeKind::Local(local) = checked.kind() else {
                continue;
            };
            let Some(binder) = self.binders.get(*local) else {
                // A `let` or quantifier local of the slot itself.
                continue;
            };
            if slot.readable.contains(local) {
                read.insert(*local);
            } else {
                out_of_scope.push((binder.name.clone(), span_of(slot.form, checked.location())));
            }
        }
        if !out_of_scope.is_empty() {
            for (name, span) in out_of_scope {
                self.refuse(ProtocolAnchorCause::SlotClause {
                    node: node_name.clone(),
                    slot: slot.kind,
                    cause: Box::new(CheckCause::MissingName(name)),
                    span,
                });
            }
            return None;
        }
        if let SlotOwner::Node(node) = slot.owner {
            for local in read {
                if let Some(readers) = self.readers.get_mut(local) {
                    readers.insert(node);
                }
            }
        }
        Some(body)
    }

    /// Whether the binder `name` is a decision fact of a role.
    fn eligibility(&self, name: &str) -> Option<Eligibility> {
        let binder = self.binders.get(*self.by_name.get(name)?)?;
        Some(match binder.origin {
            BinderOrigin::Input => Eligibility::Input,
            BinderOrigin::Capture => Eligibility::Capture,
            BinderOrigin::Activation => Eligibility::Never,
            BinderOrigin::Node(node) => {
                let declaration = self.protocol.declarations.get(node)?;
                match declaration.kind {
                    ProtocolNodeKind::Receive => {
                        let via = declaration.channel.as_ref()?;
                        let channel = self
                            .protocol
                            .channels
                            .iter()
                            .find(|channel| channel.name.name == via.name)?;
                        Eligibility::Role(channel.to.name.clone())
                    }
                    ProtocolNodeKind::Attempt | ProtocolNodeKind::Event => {
                        Eligibility::Role(declaration.role.as_ref()?.name.clone())
                    }
                    ProtocolNodeKind::Send
                    | ProtocolNodeKind::Effect
                    | ProtocolNodeKind::Commit
                    | ProtocolNodeKind::Finish
                    | ProtocolNodeKind::CompensateTemplate
                    | ProtocolNodeKind::Sequence
                    | ProtocolNodeKind::Choice
                    | ProtocolNodeKind::Parallel
                    | ProtocolNodeKind::Repeat
                    | ProtocolNodeKind::Branch
                    | ProtocolNodeKind::Case
                    | ProtocolNodeKind::Await
                    | ProtocolNodeKind::Check
                    | ProtocolNodeKind::Fence => Eligibility::Never,
                }
            }
        })
    }

    /// FR-218 "Choice and repeat decisions": every binder read of a
    /// `visible` entry is of an eligible binder of `owner`, and every fact
    /// a guard reads is an individually listed `visible` entry, the guard's
    /// closure stopping at the listed entries. Returns whether every read
    /// is admitted.
    fn facts(
        &mut self,
        node: usize,
        owner: &str,
        visible: &[ExpressionForm],
        guards: &[&ExpressionForm],
    ) -> bool {
        let before = self.refusals.len();
        for entry in visible {
            for (binder, field, span) in binder_reads(entry, |_| false) {
                let reason = match self.eligibility(&binder) {
                    Some(Eligibility::Role(role)) if role == owner => continue,
                    Some(Eligibility::Role(_)) => ObservationReason::OtherRole,
                    Some(Eligibility::Never) => ObservationReason::IneligibleKind,
                    Some(Eligibility::Capture | Eligibility::Input) | None => continue,
                };
                self.observation(node, owner, binder, field, reason, span);
            }
        }
        for guard in guards {
            let listed = |candidate: ExprRef<'_>| {
                visible
                    .iter()
                    .any(|entry| entry.expression.root().same_tree(candidate))
            };
            for (binder, field, span) in binder_reads(guard, listed) {
                let reason = match self.eligibility(&binder) {
                    Some(Eligibility::Role(role)) if role == owner => ObservationReason::Unlisted,
                    Some(Eligibility::Role(_)) => ObservationReason::OtherRole,
                    Some(Eligibility::Never) => ObservationReason::IneligibleKind,
                    Some(Eligibility::Capture | Eligibility::Input) | None => continue,
                };
                self.observation(node, owner, binder, field, reason, span);
            }
        }
        self.refusals.len() == before
    }

    fn observation(
        &mut self,
        node: usize,
        owner: &str,
        binder: String,
        field: Option<String>,
        reason: ObservationReason,
        span: Span,
    ) {
        self.refuse(ProtocolAnchorCause::WrongObservation {
            node: self.name(node),
            owner: owner.to_owned(),
            binder,
            field,
            reason,
            span,
        });
    }
}

/// Every read of a binder in `form`, outermost first, as (binder, field,
/// span): a `b.f` field read with its field, a read of `b` as a whole with
/// none. A subtree `stop` admits is not entered. Any name is reported;
/// the caller classifies it.
fn binder_reads(
    form: &ExpressionForm,
    stop: impl Fn(ExprRef<'_>) -> bool,
) -> Vec<(String, Option<String>, Span)> {
    let span = |path: &[usize]| form.spans.at(path).unwrap_or(Span { start: 0, end: 0 });
    let mut reads = Vec::new();
    let mut pending: Vec<(ExprRef<'_>, Vec<usize>)> = vec![(form.expression.root(), Vec::new())];
    while let Some((node, path)) = pending.pop() {
        if stop(node) {
            continue;
        }
        match node.node() {
            ExprNode::Name(name) => reads.push((name.clone(), None, span(&path))),
            ExprNode::Field { operand, field } => {
                if let ExprNode::Name(name) = node.at(*operand).node() {
                    reads.push((name.clone(), Some(field.clone()), span(&path)));
                    continue;
                }
                push_children(&mut pending, node, &path);
            }
            _ => push_children(&mut pending, node, &path),
        }
    }
    reads.sort_by_key(|(_, _, span)| span.start);
    reads
}

fn push_children<'e>(
    pending: &mut Vec<(ExprRef<'e>, Vec<usize>)>,
    node: ExprRef<'e>,
    path: &[usize],
) {
    for (index, child) in node.children().into_iter().enumerate().rev() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        pending.push((child, child_path));
    }
}

/// The Boolean value of `form` under one valuation of `atoms`: a subterm
/// that is one of `atoms` takes its value, `true`, `false`, `not`, `and`,
/// `or` and `implies` are evaluated, and any other subterm is looked up in
/// `atoms` too (the caller lists every one). `None` only for a subterm
/// missing from `atoms`.
fn evaluate(form: &ExpressionForm, atoms: &[ExprRef<'_>], valuation: u64) -> Option<bool> {
    // Post-order on an explicit stack.
    let mut values: Vec<(qsl_forms::ExprId, bool)> = Vec::new();
    let value_of = |values: &[(qsl_forms::ExprId, bool)], id| {
        values
            .iter()
            .rev()
            .find(|(seen, _)| *seen == id)
            .map(|(_, value)| *value)
    };
    let mut stack = vec![(form.expression.root(), false)];
    while let Some((node, expanded)) = stack.pop() {
        if let Some(atom) = atoms.iter().position(|atom| atom.same_tree(node)) {
            values.push((node.id(), valuation >> atom & 1 == 1));
            continue;
        }
        let connective = matches!(
            node.node(),
            ExprNode::Not(_)
                | ExprNode::Binary {
                    operator: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies,
                    ..
                }
        );
        if connective && !expanded {
            stack.push((node, true));
            for child in node.children() {
                stack.push((child, false));
            }
            continue;
        }
        let value = match node.node() {
            ExprNode::Boolean(value) => *value,
            ExprNode::Not(operand) => !value_of(&values, *operand)?,
            ExprNode::Binary {
                operator,
                left,
                right,
            } => {
                let (left, right) = (value_of(&values, *left)?, value_of(&values, *right)?);
                match operator {
                    BinaryOperator::And => left && right,
                    BinaryOperator::Or => left || right,
                    BinaryOperator::Implies => !left || right,
                    _ => return None,
                }
            }
            _ => return None,
        };
        values.push((node.id(), value));
    }
    value_of(&values, form.expression.root_id())
}

/// The atoms of `guards` over `visible`: each `visible` entry, then each
/// other maximal subterm of a guard that is not a Boolean literal or a
/// connective, each once.
fn atoms<'f>(visible: &'f [ExpressionForm], guards: &[&'f ExpressionForm]) -> Vec<ExprRef<'f>> {
    let mut atoms: Vec<ExprRef<'f>> = visible
        .iter()
        .map(|entry| entry.expression.root())
        .collect();
    for guard in guards {
        let mut stack = vec![guard.expression.root()];
        while let Some(node) = stack.pop() {
            if atoms.iter().any(|atom| atom.same_tree(node)) {
                continue;
            }
            match node.node() {
                ExprNode::Boolean(_) => {}
                ExprNode::Not(_)
                | ExprNode::Binary {
                    operator: BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies,
                    ..
                } => stack.extend(node.children()),
                _ => atoms.push(node),
            }
        }
    }
    atoms
}

/// Checks every slot of `protocol`'s `run` tree and `finish` over the
/// readable binders of `checked`'s control tree (FR-218 "Scopes", "Choice
/// and repeat decisions").
pub(crate) fn check(
    protocol: &ProtocolDeclarationForm,
    checked: &CheckedProtocol,
    declarations: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
) -> Slots {
    let scope = declarations.scope;
    let mut cx = cx.reborrow(scope);

    // Every binder the `run` tree and `finish` can read, in slot order.
    let mut binders = Vec::new();
    let mut unresolved = false;
    let mut push =
        |binders: &mut Vec<Binder>, name: &str, form, origin| match resolve_form(scope, form) {
            Ok(value_type) => binders.push(Binder {
                name: name.to_owned(),
                value_type,
                origin,
            }),
            Err(_) => unresolved = true,
        };
    for binder in &protocol.binders {
        match binder.kind {
            BinderKind::Input => {
                push(
                    &mut binders,
                    &binder.name.name,
                    &binder.value_type,
                    BinderOrigin::Input,
                );
            }
            BinderKind::ActivationParameter => push(
                &mut binders,
                &binder.name.name,
                &binder.value_type,
                BinderOrigin::Activation,
            ),
            _ => {}
        }
    }
    for capture in &protocol.captures {
        push(
            &mut binders,
            &capture.parameter.name.name,
            &capture.parameter.value_type,
            BinderOrigin::Capture,
        );
    }
    let roots = binders.len();
    let mut node_binder: BTreeMap<usize, usize> = BTreeMap::new();
    for (index, declaration) in protocol.declarations.iter().enumerate() {
        let has_binder = matches!(
            declaration.kind,
            ProtocolNodeKind::Send
                | ProtocolNodeKind::Receive
                | ProtocolNodeKind::Attempt
                | ProtocolNodeKind::Effect
                | ProtocolNodeKind::Event
                | ProtocolNodeKind::Commit
                | ProtocolNodeKind::Finish
        );
        if !has_binder {
            continue;
        }
        let Some(binder) = record_binder(protocol, index) else {
            continue;
        };
        let before = binders.len();
        push(
            &mut binders,
            &binder.name.name,
            &binder.value_type,
            BinderOrigin::Node(index),
        );
        if binders.len() > before {
            node_binder.insert(index, before);
        }
    }
    if unresolved {
        return Slots::Unresolved;
    }

    let flow = Flow::of(protocol, checked, &node_binder, (0..roots).collect());
    // A replicated role whose type is no object type is refused by
    // `protocol_clause::content` (no type) or `protocol_controls` (another
    // kind of type), so it has no binder here.
    let role_binders = protocol
        .roles
        .iter()
        .enumerate()
        .filter(|(_, role)| role.replicated.is_some())
        .filter_map(|(index, role)| {
            let object = scope.object_type_named(&role.context.name)?;
            Some((
                index,
                (role.name.name.clone(), ValueType::Reference(object)),
            ))
        })
        .collect();
    let mut checker = Checker {
        protocol,
        declarations,
        bound: binders
            .iter()
            .map(|binder| (binder.name.clone(), binder.value_type.clone()))
            .collect(),
        by_name: binders
            .iter()
            .enumerate()
            .map(|(index, binder)| (binder.name.clone(), index))
            .collect(),
        readers: vec![BTreeSet::new(); binders.len()],
        binders,
        nodes: declarations.nodes_used,
        refusals: Vec::new(),
        roots,
        role_binders,
    };
    let mut slots = CheckedSlots::default();

    // Node bodies: an event node's constraint, a `check`'s condition and
    // the `finish` constraint.
    for body in &protocol.bodies {
        let node = body.owner;
        let Some(declaration) = protocol.declarations.get(node) else {
            continue;
        };
        let readable = match declaration.kind {
            ProtocolNodeKind::Finish => {
                let mut readable = flow.root_out.clone();
                readable.extend(node_binder.get(&node));
                readable
            }
            _ => {
                let mut readable = flow.input(node);
                if declaration.kind != ProtocolNodeKind::Check {
                    readable.extend(node_binder.get(&node));
                }
                readable
            }
        };
        let (kind, bars_model_state) = match declaration.kind {
            ProtocolNodeKind::Check => (SlotKind::Condition, true),
            ProtocolNodeKind::Finish => (SlotKind::Finish, false),
            _ => (SlotKind::Constraint, false),
        };
        let slot = Slot {
            owner: SlotOwner::Node(node),
            kind,
            form: &body.expression,
            goal: Goal::Boolean,
            readable: &readable,
            bars_model_state,
        };
        if let Some(checked_body) = checker.slot(&slot, &mut cx) {
            slots.bodies.push(CheckedNodeBody {
                node: ProtocolNodeId(node),
                body: checked_body,
            });
        }
    }

    // FR-218 "Relationships": each `related by r (a, b)` clause of an
    // event node, checked independently. Each end is a value over the
    // binders readable at the node and the `over` input, of its
    // relationship end's type.
    for (node, declaration) in protocol.declarations.iter().enumerate() {
        for related in &declaration.related {
            let name = &related.relationship;
            if !protocol
                .relationships
                .iter()
                .any(|relationship| relationship.name.name == name.name)
            {
                checker.refuse(ProtocolAnchorCause::MissingRelated {
                    node: declaration.name.name.clone(),
                    name: name.name.clone(),
                    span: name.span,
                });
                continue;
            }
            // A declared relationship that did not resolve is refused by
            // `protocol_relationship`, so the package does not compile.
            let Some((relationship, checked_relationship)) = checked
                .relationships
                .iter()
                .enumerate()
                .find(|(_, relationship)| relationship.name == name.name)
            else {
                continue;
            };
            let mut readable = flow.input(node);
            readable.extend(node_binder.get(&node));
            let mut ends = Vec::with_capacity(2);
            for ((form, expected), end) in related
                .ends
                .iter()
                .zip(&checked_relationship.ends)
                .zip([RelatedEnd::First, RelatedEnd::Second])
            {
                let Some(body) = checker.slot(
                    &Slot {
                        owner: SlotOwner::Node(node),
                        kind: SlotKind::RelatedEnd,
                        form,
                        goal: Goal::Infer,
                        readable: &readable,
                        bars_model_state: false,
                    },
                    &mut cx,
                ) else {
                    continue;
                };
                let actual = body.root().value_type();
                if actual != expected {
                    checker.refuse(ProtocolAnchorCause::RelatedEnd {
                        relationship: name.name.clone(),
                        end,
                        expected: Box::new(expected.clone()),
                        actual: Box::new(actual.clone()),
                        span: span_of(form, &Location::root(Origin::Expression)),
                    });
                    continue;
                }
                ends.push(body);
            }
            if let Ok(ends) = <[CheckedBody; 2]>::try_from(ends) {
                slots.related.push(CheckedRelated {
                    node: ProtocolNodeId(node),
                    relationship,
                    ends,
                });
            }
        }
    }

    for choice in &protocol.choices {
        let node = choice.declaration;
        let readable = flow.input(node);
        let before = checker.refusals.len();
        let visible: Vec<Option<CheckedBody>> = choice
            .visible
            .iter()
            .map(|entry| {
                checker.slot(
                    &Slot {
                        owner: SlotOwner::Node(node),
                        kind: SlotKind::Visible,
                        form: entry,
                        goal: Goal::Infer,
                        readable: &readable,
                        bars_model_state: true,
                    },
                    &mut cx,
                )
            })
            .collect();
        let cases: Vec<Option<CheckedCase>> = choice
            .cases
            .iter()
            .map(|case| {
                checker
                    .slot(
                        &Slot {
                            owner: SlotOwner::Node(node),
                            kind: SlotKind::Guard,
                            form: &case.guard,
                            goal: Goal::Boolean,
                            readable: &readable,
                            bars_model_state: true,
                        },
                        &mut cx,
                    )
                    .map(|guard| CheckedCase {
                        name: case.name.name.clone(),
                        guard,
                    })
            })
            .collect();
        if checker.refusals.len() != before {
            continue;
        }
        let guards: Vec<&ExpressionForm> = choice.cases.iter().map(|case| &case.guard).collect();
        if !checker.facts(node, &choice.owner.name, &choice.visible, &guards) {
            continue;
        }
        match partition(choice, &guards, &mut cx) {
            Ok(None) => {}
            Ok(Some(refused)) => {
                checker.refusals.push((refused.span(), refusal(refused)));
                continue;
            }
            Err(limit) => {
                checker.refusals.push((choice.name.span, limit));
                continue;
            }
        }
        let (Some(owner), Some(visible), Some(cases)) = (
            role_index(protocol, &choice.owner.name),
            visible.into_iter().collect::<Option<Vec<_>>>(),
            cases.into_iter().collect::<Option<Vec<_>>>(),
        ) else {
            // `protocol_clause::content` refuses the owner.
            unresolved = true;
            continue;
        };
        slots.choices.push(CheckedChoice {
            node: ProtocolNodeId(node),
            owner,
            visible,
            cases,
        });
    }

    for repeat in &protocol.repeats {
        let node = repeat.declaration;
        let readable = flow.input(node);
        let before = checker.refusals.len();
        let mut decision = |checker: &mut Checker<'_, '_>,
                            form: &ExpressionForm,
                            kind: SlotKind,
                            goal: Goal<'_>,
                            bars_model_state: bool| {
            checker.slot(
                &Slot {
                    owner: SlotOwner::Node(node),
                    kind,
                    form,
                    goal,
                    readable: &readable,
                    bars_model_state,
                },
                &mut cx,
            )
        };
        let visible: Vec<Option<CheckedBody>> = repeat
            .visible
            .iter()
            .map(|entry| decision(&mut checker, entry, SlotKind::Visible, Goal::Infer, true))
            .collect();
        let guard = decision(
            &mut checker,
            &repeat.guard,
            SlotKind::Guard,
            Goal::Boolean,
            true,
        );
        let loop_proof = repeat.loop_proof.as_ref().map(|proof| {
            (
                decision(
                    &mut checker,
                    &proof.invariant,
                    SlotKind::Invariant,
                    Goal::Boolean,
                    false,
                ),
                decision(
                    &mut checker,
                    &proof.variant,
                    SlotKind::Variant,
                    Goal::Infer,
                    false,
                ),
            )
        });
        if checker.refusals.len() != before
            || !checker.facts(node, &repeat.owner.name, &repeat.visible, &[&repeat.guard])
        {
            continue;
        }
        let loop_proof = match loop_proof {
            None => Some(None),
            Some((Some(invariant), Some(variant))) => {
                Some(Some(CheckedLoopProof { invariant, variant }))
            }
            Some(_) => None,
        };
        let (Some(owner), Some(visible), Some(guard), Some(loop_proof)) = (
            role_index(protocol, &repeat.owner.name),
            visible.into_iter().collect::<Option<Vec<_>>>(),
            guard,
            loop_proof,
        ) else {
            // `protocol_clause::content` refuses the owner.
            unresolved = true;
            continue;
        };
        slots.repeats.push(CheckedRepeat {
            node: ProtocolNodeId(node),
            owner,
            visible,
            max: repeat.max.as_ref().map(|max| max.value),
            loop_proof,
            guard,
        });
    }

    // FR-218 "Scopes", QSpec FR-465 "Role": each replicated role's
    // authored `max` is at least 1, and its lifetime is recorded: the
    // `scope` region, or the `until` predicate over `C0` and the role's
    // binder.
    let no_reads = BTreeSet::new();
    for (index, role) in protocol.roles.iter().enumerate() {
        let Some(replicated) = &role.replicated else {
            continue;
        };
        if replicated.max == 0 {
            checker.refuse(ProtocolAnchorCause::RoleMax {
                role: role.name.name.clone(),
                minimum: 1,
                actual: replicated.max,
                span: replicated.max_span,
            });
        }
        let lifetime = match &replicated.lifetime {
            RoleLifetime::Workflow(_) => Some(CheckedLifetime::Workflow),
            RoleLifetime::Scope(_) => Some(CheckedLifetime::Scope(region(
                protocol,
                checked,
                &role.name.name,
            ))),
            RoleLifetime::Until(predicate) if checker.role_binders.contains_key(&index) => checker
                .slot(
                    &Slot {
                        owner: SlotOwner::Role(index),
                        kind: SlotKind::Lifetime,
                        form: predicate,
                        goal: Goal::Boolean,
                        readable: &no_reads,
                        bars_model_state: false,
                    },
                    &mut cx,
                )
                .map(CheckedLifetime::Until),
            RoleLifetime::Until(_) => None,
        };
        if let (Some(lifetime), true) = (lifetime, replicated.max > 0) {
            slots.roles.push(CheckedReplicatedRole {
                role: index,
                population: replicated.population.name.clone(),
                max: replicated.max,
                lifetime,
            });
        }
    }

    // FR-211 "The `terminal` member": `terminal when { P }` is a Boolean
    // predicate over the model observation (the `over` input) and, under
    // `on origin`, the protocol captures (QSpec FR-464 stage `terminal`,
    // FR-429). Any other binder, the `on each` binder included, is out of
    // scope. A second member is `protocol_controls`' refusal; the first is
    // the one recorded.
    let origin = matches!(protocol.activation, ActivationForm::Origin);
    let terminal_readable: BTreeSet<usize> = checker
        .binders
        .iter()
        .enumerate()
        .filter(|(_, binder)| match binder.origin {
            BinderOrigin::Input => true,
            BinderOrigin::Capture => origin,
            BinderOrigin::Activation | BinderOrigin::Node(_) => false,
        })
        .map(|(index, _)| index)
        .collect();
    let terminal = protocol.members.iter().find_map(|member| match member {
        ProtocolMemberForm::Terminal(terminal) => Some(terminal),
        ProtocolMemberForm::Scheduling(_) => None,
    });
    slots.scheduling = if protocol
        .members
        .iter()
        .any(|member| matches!(member, ProtocolMemberForm::Scheduling(_)))
    {
        CheckedScheduling::Adversarial
    } else {
        CheckedScheduling::Fair
    };
    slots.terminal = match terminal {
        None => CheckedTerminalMember::None,
        Some(TerminalForm::Any { .. }) => CheckedTerminalMember::Any,
        Some(TerminalForm::When { predicate, .. }) => checker
            .slot(
                &Slot {
                    owner: SlotOwner::Clause,
                    kind: SlotKind::Terminal,
                    form: predicate,
                    goal: Goal::Boolean,
                    readable: &terminal_readable,
                    bars_model_state: false,
                },
                &mut cx,
            )
            .map_or(CheckedTerminalMember::None, CheckedTerminalMember::When),
    };

    if !checker.refusals.is_empty() {
        checker.refusals.sort_by_key(|(span, _)| span.start);
        return Slots::Refused(
            checker
                .refusals
                .into_iter()
                .map(|(_, refusal)| refusal)
                .collect(),
        );
    }
    if unresolved {
        return Slots::Unresolved;
    }
    slots.readers = checker
        .binders
        .iter()
        .zip(&checker.readers)
        .map(|(binder, readers)| CheckedReaders {
            binder: binder.name.clone(),
            readers: readers.iter().copied().map(ProtocolNodeId).collect(),
        })
        .collect();
    slots.binders = checker
        .binders
        .iter()
        .map(|binder| CheckedSlotBinder {
            name: binder.name.clone(),
            value_type: binder.value_type.clone(),
            node: match binder.origin {
                BinderOrigin::Node(node) => Some(ProtocolNodeId(node)),
                BinderOrigin::Input | BinderOrigin::Activation | BinderOrigin::Capture => None,
            },
            ranged: match binder.origin {
                BinderOrigin::Node(node) => protocol.declarations.get(node).is_some_and(|declaration| {
                    matches!(
                        declaration.kind,
                        ProtocolNodeKind::Event
                            | ProtocolNodeKind::Effect
                            | ProtocolNodeKind::Commit
                            | ProtocolNodeKind::Send
                            | ProtocolNodeKind::Receive
                            | ProtocolNodeKind::Finish
                    )
                }),
                BinderOrigin::Input | BinderOrigin::Activation | BinderOrigin::Capture => false,
            },
        })
        .collect();
    Slots::Checked(slots, checker.nodes)
}

/// FR-218 "Scopes": the smallest control of `checked`'s `run` tree that
/// strictly holds every node `by` the role `role` (an `attempt`, `event` or
/// `commit` written `by` it, a `choice` or `repeat` it owns): the lowest
/// common ancestor of those nodes' parents, the root standing for itself.
fn region(
    protocol: &ProtocolDeclarationForm,
    checked: &CheckedProtocol,
    role: &str,
) -> Option<ProtocolNodeId> {
    let tree = &checked.controls;
    let mut parents: BTreeMap<ProtocolNodeId, ProtocolNodeId> = BTreeMap::new();
    for control in &tree.controls {
        for child in &control.children {
            parents.insert(*child, control.node);
        }
    }
    let members = protocol
        .declarations
        .iter()
        .enumerate()
        .filter(|(_, declaration)| {
            declaration
                .role
                .as_ref()
                .is_some_and(|written| written.name == role)
        })
        .map(|(index, _)| index)
        .chain(
            protocol
                .choices
                .iter()
                .filter(|choice| choice.owner.name == role)
                .map(|choice| choice.declaration),
        )
        .chain(
            protocol
                .repeats
                .iter()
                .filter(|repeat| repeat.owner.name == role)
                .map(|repeat| repeat.declaration),
        )
        .map(ProtocolNodeId)
        .filter(|node| tree.control(*node).is_some());
    // Each container's chain of ancestors, root first.
    let chain = |node: ProtocolNodeId| {
        let mut chain = vec![node];
        let mut at = node;
        while let Some(parent) = parents.get(&at) {
            chain.push(*parent);
            at = *parent;
        }
        chain.reverse();
        chain
    };
    let mut common: Option<Vec<ProtocolNodeId>> = None;
    for member in members {
        let container = parents.get(&member).copied().unwrap_or(member);
        let path = chain(container);
        common = Some(match common {
            None => path,
            Some(shared) => shared
                .into_iter()
                .zip(path)
                .take_while(|(left, right)| left == right)
                .map(|(left, _)| left)
                .collect(),
        });
    }
    common?.last().copied()
}

/// The index of the role `name` among `protocol`'s roles.
fn role_index(protocol: &ProtocolDeclarationForm, name: &str) -> Option<usize> {
    protocol
        .roles
        .iter()
        .position(|role| role.name.name == name)
}

/// FR-218 "Choice and repeat decisions": the `choice`'s guards are proved
/// exhaustive and exclusive over its atoms as independent Booleans. Returns
/// the refusal a gap or an overlap gives, or the refusal of a proof whose
/// enumeration (every guard under every valuation) exceeds the checking
/// stage's work budget.
fn partition(
    choice: &qsl_forms::ChoiceForm,
    guards: &[&ExpressionForm],
    cx: &mut CheckContext<'_, Scope>,
) -> Result<Option<ProtocolAnchorCause>, CheckRefusal> {
    let atoms = atoms(&choice.visible, guards);
    let cases = u64::try_from(guards.len()).unwrap_or(u64::MAX);
    // `2^atoms` valuations; past `u64` the work is not representable and is
    // charged as the greatest amount.
    let valuations = u32::try_from(atoms.len())
        .ok()
        .and_then(|count| 1u64.checked_shl(count));
    let work = valuations.map_or(u64::MAX, |valuations| valuations.saturating_mul(cases));
    let charged = cx.meter.charge(
        quire_exact::Charge::new(quire_exact::ChargePoint::DeclarationCheck)
            .work(quire_exact::Integer::from(work)),
    );
    let too_large = |limit: u64, consumed: u64| CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: super::limit_cause(&qsl_foundation::diagnostic::LimitExceeded::new(
            qsl_foundation::Setting::S3WorkUnits,
            limit,
            u128::from(consumed) + u128::from(work),
        )),
    };
    if let Err(incomplete) = charged {
        return Err(too_large(incomplete.limit, incomplete.consumed));
    }
    let Some(valuations) = valuations else {
        return Err(too_large(u64::MAX, 0));
    };
    let mut missing = false;
    let mut duplicate = false;
    let mut involved: BTreeSet<usize> = BTreeSet::new();
    for valuation in 0..valuations {
        let mut covering = Vec::new();
        for (index, guard) in guards.iter().enumerate() {
            // Every subterm of a guard is an atom, a Boolean literal or a
            // connective (`atoms`), so each guard has a value.
            let Some(value) = evaluate(guard, &atoms, valuation) else {
                return Err(CheckRefusal {
                    location: Location::root(Origin::Expression),
                    cause: CheckCause::InternalFault(Box::new(
                        super::refusal::KeyFault::StageFault(qsl_foundation::InternalFault::new(
                            "check.protocol",
                            "choice-guard-atoms",
                        )),
                    )),
                });
            };
            if value {
                covering.push(index);
            }
        }
        match covering.len() {
            0 => {
                missing = true;
                involved.extend(0..guards.len());
            }
            1 => {}
            _ => {
                duplicate = true;
                involved.extend(covering);
            }
        }
    }
    let failed: Vec<ArmFault> = [
        (missing, ArmFault::MissingArm),
        (duplicate, ArmFault::DuplicateArm),
    ]
    .into_iter()
    .filter_map(|(found, fault)| found.then_some(fault))
    .collect();
    if failed.is_empty() {
        return Ok(None);
    }
    Ok(Some(ProtocolAnchorCause::Exhaustiveness {
        choice: choice.name.name.clone(),
        cases: involved
            .into_iter()
            .filter_map(|index| choice.cases.get(index))
            .map(|case| case.name.name.clone())
            .collect(),
        failed,
        span: choice.name.span,
    }))
}

#[cfg(test)]
mod tests;

/// The binders readable at each control's entry, and after the root.
struct Flow {
    inputs: BTreeMap<usize, BTreeSet<usize>>,
    root_out: BTreeSet<usize>,
}

impl Flow {
    fn input(&self, node: usize) -> BTreeSet<usize> {
        self.inputs.get(&node).cloned().unwrap_or_default()
    }

    /// QSpec FR-466 "Binder scope" over `checked`'s control tree, from the
    /// root's `C0` (`roots`), on an explicit stack.
    fn of(
        protocol: &ProtocolDeclarationForm,
        checked: &CheckedProtocol,
        node_binder: &BTreeMap<usize, usize>,
        roots: BTreeSet<usize>,
    ) -> Self {
        let tree = &checked.controls;
        let mut inputs: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut outs: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let Some(root) = tree.root() else {
            return Self {
                inputs,
                root_out: roots,
            };
        };
        let children_of = |node: usize| -> Vec<usize> {
            tree.control(ProtocolNodeId(node))
                .map(|control| control.children.iter().map(|child| child.0).collect())
                .unwrap_or_default()
        };
        let kind_of = |node: usize| {
            protocol
                .declarations
                .get(node)
                .map(|declaration| declaration.kind)
        };
        // (node, its input, the index of the next child to enter)
        let mut stack: Vec<(usize, BTreeSet<usize>, usize)> = vec![(root.node.0, roots, 0)];
        while let Some(top) = stack.last_mut() {
            let (node, next) = (top.0, top.2);
            if next == 0 {
                inputs.insert(node, top.1.clone());
            }
            let children = children_of(node);
            if let Some(&child) = children.get(next) {
                top.2 += 1;
                let input = top.1.clone();
                let child_input = match kind_of(node) {
                    Some(ProtocolNodeKind::Sequence) if next > 0 => children
                        .get(next - 1)
                        .and_then(|previous| outs.get(previous))
                        .cloned()
                        .unwrap_or_default(),
                    Some(ProtocolNodeKind::Await) => match await_part(protocol, node, child) {
                        AwaitPart::Then => children
                            .iter()
                            .find(|&&sibling| {
                                matches!(await_part(protocol, node, sibling), AwaitPart::Event)
                            })
                            .and_then(|event| outs.get(event))
                            .cloned()
                            .unwrap_or_default(),
                        AwaitPart::Event | AwaitPart::Timeout => input,
                    },
                    _ => input,
                };
                stack.push((child, child_input, 0));
                continue;
            }
            let Some((node, input, _)) = stack.pop() else {
                break;
            };
            let out_of = |child: &usize| outs.get(child).cloned().unwrap_or_default();
            let out = match kind_of(node) {
                Some(
                    ProtocolNodeKind::Send
                    | ProtocolNodeKind::Receive
                    | ProtocolNodeKind::Attempt
                    | ProtocolNodeKind::Effect
                    | ProtocolNodeKind::Event
                    | ProtocolNodeKind::Commit,
                ) => {
                    let mut out = input;
                    out.extend(node_binder.get(&node));
                    out
                }
                Some(ProtocolNodeKind::Sequence) => children.last().map(out_of).unwrap_or(input),
                Some(ProtocolNodeKind::Branch | ProtocolNodeKind::Case) => {
                    children.first().map(out_of).unwrap_or(input)
                }
                Some(ProtocolNodeKind::Parallel) => {
                    let all = protocol
                        .parallels
                        .iter()
                        .find(|parallel| parallel.declaration == node)
                        .is_some_and(|parallel| {
                            matches!(parallel.join.value, qsl_forms::JoinPolicy::All)
                        });
                    let mut out = input;
                    if all {
                        for branch in &children {
                            out.extend(out_of(branch));
                        }
                    }
                    out
                }
                Some(
                    ProtocolNodeKind::Choice
                    | ProtocolNodeKind::Repeat
                    | ProtocolNodeKind::Await
                    | ProtocolNodeKind::Check
                    | ProtocolNodeKind::Fence
                    | ProtocolNodeKind::Finish
                    | ProtocolNodeKind::CompensateTemplate,
                )
                | None => input,
            };
            outs.insert(node, out);
        }
        let root_out = outs.get(&root.node.0).cloned().unwrap_or_default();
        Self { inputs, root_out }
    }
}

/// Which part of an `await` a child of it is.
enum AwaitPart {
    Event,
    Then,
    Timeout,
}

fn await_part(protocol: &ProtocolDeclarationForm, node: usize, child: usize) -> AwaitPart {
    let span = protocol
        .declarations
        .get(child)
        .map(|declaration| declaration.name.span);
    let form = protocol
        .awaits
        .iter()
        .find(|await_form| await_form.declaration == node);
    match (form, span) {
        (Some(form), Some(span)) if span == form.matched.span => AwaitPart::Event,
        (Some(form), Some(span)) if form.then.start <= span.start && span.end <= form.then.end => {
            AwaitPart::Then
        }
        _ => AwaitPart::Timeout,
    }
}
