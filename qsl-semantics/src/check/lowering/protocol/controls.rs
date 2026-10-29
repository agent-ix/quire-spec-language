// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-466's step-control nodes: the `protocol`/`control` node each `sequence`,
//! `send`, `receive`, `attempt`, `effect`, `event`, `commit`, `check`,
//! `fence`, `await`, `parallel`, `choice` and `repeat` of a protocol's `run`
//! tree emits, with the argument
//! layout of QSpec FR-466 "Control layouts" (FR-383-AC-7).
//!
//! A control is its own node, declared at `P`, `run` and the control names
//! from the root down (QSpec FR-464 "Operator catalog"), and a control that
//! introduces a binder takes the number of binders readable at its entry as
//! the binder's level (QSpec FR-464 "Binders", FR-466 "Binder scope"). The
//! readable sets are the ones S3 checked every slot against
//! ([`CheckedSlots::entries`](crate::check::protocol_slots::CheckedSlots)),
//! so a body is lowered over exactly the binders it was checked over.
//!
//! A node is keyed after every node it names: its children, and the control
//! a `receive`, an `effect` or an `await` is anchored at, which may follow it
//! in the tree. A `parallel`, a `choice` and a `repeat` are written with
//! FR-466's layouts, except a `parallel` whose join is a predicate, which
//! names a function, and a post-test `repeat`, whose guard reads its body's
//! binders where FR-466 reads only the repeat's entry
//! ([`ProtocolConstruct`](crate::check::refusal::ProtocolConstruct)).
//! A `branch` or a `case` is no node: its body is held by its `parallel` or
//! `choice`, and is declared at that control's declaration followed by its
//! own name (QSpec FR-464 "Control tree").

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AnchorSite, JoinPolicy, Outstanding, ProtocolNodeKind};
use quire_exact::{Integer, NodeKey};

use super::super::{fault, Binder, Binders, Lowering};
use super::{
    empty_group, identifier, member, protocol_location, sorted, ProtocolInput, ProtocolSite,
};
use crate::check::ir::CheckedBody;
use crate::check::node_key::{
    BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm, Operation, OperationLaw,
    Operator, TupleMember, TupleTerm,
};
use crate::check::protocol_clause::{CheckedControl, CheckedControlTree, ProtocolNodeId};
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::value::member::Member;

/// The nodes one protocol `attempt` names, which are keyed after the
/// functions: its operation's `operation_anchor` (no execution point, QSpec
/// FR-464 "operation anchors") and the `state_clause` nodes of its
/// `contracts` list (FR-114 "Outputs").
#[derive(Clone, Debug)]
pub(crate) struct AttemptNodes {
    /// The index of the attempt's declaration among the protocol's
    /// declarations.
    pub(crate) declaration: usize,
    /// The operation's `operation_anchor` node.
    pub(crate) anchor: NodeKey,
    /// The state clauses the attempt lists.
    pub(crate) contracts: Vec<NodeKey>,
}

/// The operation anchor of a checked compensation template.
#[derive(Clone, Debug)]
pub(crate) struct CompensationNodes {
    /// The template's index among checked compensation declarations.
    pub(crate) declaration: usize,
    /// The operation's `operation_anchor` node.
    pub(crate) anchor: NodeKey,
}

/// What [`Lowering::run_nodes`] reads of the nodes written before the `run`
/// tree.
pub(super) struct RunInputs<'a> {
    /// The package's `protocol_profile` law.
    pub(super) law: &'a OperationLaw,
    /// Each role's name and node.
    pub(super) roles: &'a [(String, NodeKey)],
    /// Each channel's name and node.
    pub(super) channels: &'a [(String, NodeKey)],
    /// The nodes each `attempt` names.
    pub(super) attempts: &'a [AttemptNodes],
    /// The binders readable at the clause (`C0`).
    pub(super) clause_binders: &'a [Binder],
    /// The `Boolean` scalar type node.
    pub(super) boolean: NodeKey,
}

/// The `run` tree as written: the root's key and the binders readable after
/// it, which `finish` reads.
pub(super) struct Run {
    pub(super) root: NodeKey,
    pub(super) readable: Vec<Binder>,
    /// Every control's node, by its declaration.
    pub(super) nodes: BTreeMap<ProtocolNodeId, NodeKey>,
}

/// What the lowering of one control reads of its protocol.
struct Controls<'a, 'p> {
    protocol: &'a ProtocolInput<'p>,
    inputs: &'a RunInputs<'a>,
    /// Every binder introduced so far, by its slot.
    binders: BTreeMap<usize, Binder>,
    /// The binder each control introduces.
    own: BTreeMap<ProtocolNodeId, Binder>,
    /// The control each `receive`, `effect` and `await` is anchored at.
    anchored: BTreeMap<ProtocolNodeId, ProtocolNodeId>,
    /// The checked body of each control that has one.
    bodies: BTreeMap<ProtocolNodeId, &'a CheckedBody>,
    /// The name each slot was bound under.
    slot_names: Vec<String>,
}

impl Lowering<'_> {
    /// The control nodes of `protocol`'s `run` tree (FR-466 "Control
    /// layouts"), each declared at `P`, `run` and the names from the root
    /// down, with the root's key and the binders readable after it.
    pub(super) fn run_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        inputs: &RunInputs<'_>,
    ) -> Result<Run, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let checked = protocol.checked;
        let tree = &checked.controls;
        let slots = &checked.slots;
        let unchecked = || fault(&protocol.location(), KeyFault::UncheckedProtocolFact);
        let root = tree.root().ok_or_else(unchecked)?;

        let mut controls = Controls {
            protocol,
            inputs,
            binders: inputs
                .clause_binders
                .iter()
                .map(|binder| (binder.slot, *binder))
                .collect(),
            own: BTreeMap::new(),
            anchored: BTreeMap::new(),
            bodies: slots
                .bodies
                .iter()
                .map(|body| (body.node, &body.body))
                .collect(),
            slot_names: slots
                .binders
                .iter()
                .map(|binder| binder.name.clone())
                .collect(),
        };
        for (anchor, target) in protocol
            .form
            .scoped_anchors
            .iter()
            .zip(&checked.resolved_anchors)
        {
            match anchor.site {
                AnchorSite::ReceiveOf | AnchorSite::EffectOf | AnchorSite::AwaitAfter => {
                    controls
                        .anchored
                        .insert(ProtocolNodeId(anchor.owner), *target);
                }
                // A compensation is not written yet
                // (`compensate` templates are carried by the protocol clause).
                AnchorSite::EventFor | AnchorSite::CompensateFor | AnchorSite::CompensateCommit => {
                }
            }
        }

        // Each introducing construct's binder, in source order (FR-383-AC-9:
        // a binder node has one `expression` occurrence for each).
        let introduced: BTreeMap<ProtocolNodeId, usize> = slots
            .binders
            .iter()
            .enumerate()
            .filter_map(|(slot, binder)| binder.node.map(|node| (node, slot)))
            .collect();
        for control in &tree.controls {
            let Some(&slot) = introduced.get(&control.node) else {
                continue;
            };
            let location = protocol_location(name, ProtocolSite::Control(&control.path));
            let level = slots
                .entries
                .get(&control.node)
                .ok_or_else(unchecked)?
                .len();
            let declared = slots.binders.get(slot).ok_or_else(unchecked)?;
            let binder = self.protocol_binder(declared, slot, level, &location)?;
            controls.binders.insert(slot, binder);
            controls.own.insert(control.node, binder);
        }

        let declarations = control_declarations(name, tree);
        let mut keys: BTreeMap<ProtocolNodeId, NodeKey> = BTreeMap::new();
        for node in controls.build_order()? {
            let control = tree.control(node).ok_or_else(unchecked)?;
            // A branch's or a case's body is held by its `parallel` or its
            // `choice`, which reads it.
            if matches!(
                control.kind,
                ProtocolNodeKind::Branch | ProtocolNodeKind::Case
            ) {
                continue;
            }
            let location = protocol_location(name, ProtocolSite::Control(&control.path));
            let declaration = declarations
                .get(&node)
                .ok_or_else(unchecked)?
                .iter()
                .map(|segment| identifier(segment, &location))
                .collect::<Result<Vec<_>, _>>()?;
            let (operator, arguments) =
                self.control_arguments(&controls, node, &keys, &location)?;
            let key = self.protocol_member(
                &location,
                "control",
                inputs.boolean,
                declaration,
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    Operation {
                        laws: vec![inputs.law.clone()],
                        member: Some(Member::ProfileOperator {
                            operator: identifier(operator, &location)?,
                        }),
                        ..Operation::plain("quire.op.protocol.control")
                    },
                    inputs.boolean,
                    arguments,
                ),
            )?;
            keys.insert(node, key);
        }

        let readable = slots
            .root_out
            .iter()
            .map(|slot| controls.binders.get(slot).copied().ok_or_else(unchecked))
            .collect::<Result<Vec<_>, _>>()?;
        let root = keys.get(&root.node).copied().ok_or_else(unchecked)?;
        Ok(Run {
            root,
            readable,
            nodes: keys,
        })
    }

    /// The operator of the control `node` and its arguments, in the order of
    /// FR-466 "Control layouts". `keys` holds every control keyed so far.
    fn control_arguments(
        &mut self,
        controls: &Controls<'_, '_>,
        node: ProtocolNodeId,
        keys: &BTreeMap<ProtocolNodeId, NodeKey>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<(&'static str, Vec<MemberTerm>), CheckRefusal> {
        let protocol = controls.protocol;
        let form = protocol.form;
        let checked = protocol.checked;
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let control = checked.controls.control(node).ok_or_else(unchecked)?;
        let declaration = form.declarations.get(node.0).ok_or_else(unchecked)?;
        let key_of = |target: &ProtocolNodeId| keys.get(target).copied().ok_or_else(unchecked);
        let anchor_of = || {
            controls
                .anchored
                .get(&node)
                .ok_or_else(unchecked)
                .and_then(key_of)
        };
        let binder = || {
            controls
                .own
                .get(&node)
                .map(|binder| MemberTerm::bound("binder", LeafTerm::reference(binder.parameter)))
                .ok_or_else(unchecked)
        };
        let role = |name: Option<&str>| {
            let name = name.ok_or_else(unchecked)?;
            controls
                .inputs
                .roles
                .iter()
                .find(|(declared, _)| declared == name)
                .map(|(_, key)| MemberTerm::bound("role", LeafTerm::reference(*key)))
                .ok_or_else(unchecked)
        };
        let channel = || {
            let via = checked
                .vias
                .iter()
                .find(|via| via.node == node)
                .ok_or_else(unchecked)?;
            controls
                .inputs
                .channels
                .iter()
                .find(|(declared, _)| *declared == via.channel)
                .map(|(_, key)| MemberTerm::bound("channel", LeafTerm::reference(*key)))
                .ok_or_else(unchecked)
        };
        // A `related by` clause names a relationship, which is not written
        // yet ([`ProtocolConstruct::Relationships`](crate::check::refusal::ProtocolConstruct)).
        let related = || {
            if checked
                .slots
                .related
                .iter()
                .any(|related| related.node == node)
            {
                return Err(unchecked());
            }
            Ok(MemberTerm::binding(
                "related",
                BindingValue::Tuple(TupleTerm::new(Vec::new())),
            ))
        };
        let entry = checked.slots.entries.get(&node).ok_or_else(unchecked)?;

        Ok(match control.kind {
            ProtocolNodeKind::Sequence => {
                let children = control
                    .children
                    .iter()
                    .map(key_of)
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    "sequence",
                    vec![MemberTerm::binding(
                        "children",
                        BindingValue::Group(GroupTerm::references(children)),
                    )],
                )
            }
            ProtocolNodeKind::Check => {
                let condition = self.control_body(controls, node, entry, false, location)?;
                ("check", vec![MemberTerm::bound("condition", condition)])
            }
            ProtocolNodeKind::Fence => {
                let fence = form
                    .fences
                    .iter()
                    .find(|fence| fence.declaration == node.0)
                    .ok_or_else(unchecked)?;
                let ordering = self.text_literal(fence.ordering.value.label(), location)?;
                ("fence", vec![MemberTerm::bound("ordering", ordering)])
            }
            ProtocolNodeKind::Send => {
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "send",
                    vec![
                        channel()?,
                        binder()?,
                        related()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Receive => {
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "receive",
                    vec![
                        channel()?,
                        MemberTerm::bound("of", LeafTerm::reference(anchor_of()?)),
                        binder()?,
                        related()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Attempt => {
                let attempt = form
                    .attempts
                    .iter()
                    .find(|attempt| attempt.declaration == node.0)
                    .ok_or_else(unchecked)?;
                let nodes = controls
                    .inputs
                    .attempts
                    .iter()
                    .find(|nodes| nodes.declaration == node.0)
                    .ok_or_else(unchecked)?;
                let ordering = match &attempt.ordering {
                    None => empty_group(),
                    Some(ordering) => BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                        self.text_literal(ordering.value.label(), location)?,
                    )])),
                };
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "attempt",
                    vec![
                        role(Some(&attempt.role.name))?,
                        MemberTerm::bound("operation", LeafTerm::reference(nodes.anchor)),
                        MemberTerm::binding("ordering", ordering),
                        MemberTerm::binding(
                            "contracts",
                            BindingValue::Group(GroupTerm::references(sorted(
                                nodes.contracts.iter().copied(),
                            ))),
                        ),
                        binder()?,
                        related()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Effect => {
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "effect",
                    vec![
                        MemberTerm::bound("of", LeafTerm::reference(anchor_of()?)),
                        binder()?,
                        related()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Event => {
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "event",
                    vec![
                        role(declaration.role.as_ref().map(|role| role.name.as_str()))?,
                        MemberTerm::binding("compensation", empty_group()),
                        binder()?,
                        related()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Commit => {
                let constraint = self.control_body(controls, node, entry, true, location)?;
                (
                    "commit",
                    vec![
                        role(declaration.role.as_ref().map(|role| role.name.as_str()))?,
                        binder()?,
                        MemberTerm::bound("constraint", constraint),
                    ],
                )
            }
            ProtocolNodeKind::Await => {
                let parts = AwaitParts::of(form, control).ok_or_else(unchecked)?;
                let awaited = checked
                    .awaits
                    .iter()
                    .find(|awaited| awaited.node == node)
                    .ok_or_else(unchecked)?;
                let target = anchor_of()?;
                let after = GroupTerm::new(vec![
                    member("kind", self.text_literal("event", location)?),
                    member("target", LeafTerm::reference(target)),
                ]);
                let clock = self.text_literal(&awaited.clock, location)?;
                let within = GroupTerm::new(vec![
                    member(
                        "lower",
                        self.integer_literal(Integer::from(awaited.within.lower), location)?,
                    ),
                    member(
                        "upper",
                        self.integer_literal(Integer::from(awaited.within.upper), location)?,
                    ),
                ]);
                (
                    "await",
                    vec![
                        MemberTerm::binding("after", BindingValue::Group(after)),
                        MemberTerm::bound("clock", clock),
                        MemberTerm::binding("within", BindingValue::Group(within)),
                        MemberTerm::bound("event", LeafTerm::reference(key_of(&parts.event)?)),
                        MemberTerm::bound("then", LeafTerm::reference(key_of(&parts.then)?)),
                        MemberTerm::bound("timeout", LeafTerm::reference(key_of(&parts.timeout)?)),
                    ],
                )
            }
            ProtocolNodeKind::Parallel => (
                "parallel",
                self.parallel_arguments(protocol, control, keys, location)?,
            ),
            ProtocolNodeKind::Choice => (
                "choice",
                self.choice_arguments(controls, control, entry, keys, location)?,
            ),
            ProtocolNodeKind::Repeat => (
                "repeat",
                self.repeat_arguments(controls, control, entry, keys, location)?,
            ),
            // Not written yet: refused before any node is built.
            ProtocolNodeKind::Branch
            | ProtocolNodeKind::Case
            | ProtocolNodeKind::Finish
            | ProtocolNodeKind::CompensateTemplate => return Err(unchecked()),
        })
    }

    /// The arguments of the `parallel` `control` (FR-466 "Control layouts"):
    /// the `memory` the source writes, the `branches` sorted by label as
    /// UTF-8 bytes, each with its body's key in `keys`, the `join` Group,
    /// the `joined` labels in ascending order, and the written
    /// `outstanding` rule. An absent optional is the empty Group.
    fn parallel_arguments(
        &mut self,
        protocol: &ProtocolInput<'_>,
        control: &CheckedControl,
        keys: &BTreeMap<ProtocolNodeId, NodeKey>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<Vec<MemberTerm>, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let form = protocol
            .form
            .parallels
            .iter()
            .find(|parallel| parallel.declaration == control.node.0)
            .ok_or_else(unchecked)?;
        let mut branches: Vec<(&str, NodeKey)> = Vec::with_capacity(control.children.len());
        for branch in &control.children {
            let branch = protocol
                .checked
                .controls
                .control(*branch)
                .filter(|branch| branch.kind == ProtocolNodeKind::Branch)
                .ok_or_else(unchecked)?;
            let [body] = branch.children[..] else {
                return Err(unchecked());
            };
            let body = keys.get(&body).copied().ok_or_else(unchecked)?;
            branches.push((branch.name.as_str(), body));
        }
        branches.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
        let mut records = Vec::with_capacity(branches.len());
        for (label, body) in branches {
            records.push(TupleMember::Group(GroupTerm::new(vec![
                member("label", self.text_literal(label, location)?),
                member("body", LeafTerm::reference(body)),
            ])));
        }
        let mut join = vec![member(
            "policy",
            self.text_literal(join_text(&form.join.value), location)?,
        )];
        match &form.join.value {
            JoinPolicy::All | JoinPolicy::Any => {}
            JoinPolicy::Quorum(count) => join.push(member(
                "count",
                self.integer_literal(Integer::from(*count), location)?,
            )),
            // FR-466 "Control layouts": a reference to the `function` node,
            // keyed before the protocol (FR-093). S3 checked its arity and
            // types: one Boolean parameter per joined label, a Boolean
            // result.
            JoinPolicy::Predicate(_) => {
                let function = protocol
                    .checked
                    .pending_predicate_joins
                    .iter()
                    .find(|join| join.node == control.node)
                    .and_then(|join| self.functions.get(join.function).copied().flatten())
                    .ok_or_else(unchecked)?;
                join.push(member("predicate", LeafTerm::reference(function)));
            }
        }
        let mut joined: Vec<&str> = form
            .labels
            .iter()
            .map(|label| label.name.as_str())
            .collect();
        joined.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        joined.dedup();
        let joined = joined
            .into_iter()
            .map(|label| self.text_literal(label, location).map(GroupMember::Leaf))
            .collect::<Result<Vec<_>, _>>()?;
        let memory = match &form.memory {
            None => empty_group(),
            Some(memory) => BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                self.text_literal(memory.value.label(), location)?,
            )])),
        };
        let outstanding = match &form.outstanding {
            None => empty_group(),
            Some(outstanding) => BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                self.text_literal(outstanding_text(outstanding.value), location)?,
            )])),
        };
        Ok(vec![
            MemberTerm::binding("memory", memory),
            MemberTerm::binding("branches", BindingValue::Tuple(TupleTerm::new(records))),
            MemberTerm::binding("join", BindingValue::Group(GroupTerm::new(join))),
            MemberTerm::binding("joined", BindingValue::Group(GroupTerm::new(joined))),
            MemberTerm::binding("outstanding", outstanding),
        ])
    }

    /// The arguments of the `choice` `control` (FR-466 "Control layouts"):
    /// its owning role, its `visible` entries as a set of nodes, and its
    /// `cases` sorted by label as UTF-8 bytes, each with its guard and its
    /// body's key in `keys`. The entries and guards are lowered over the
    /// binders readable at the choice's entry, so a guard that reads an
    /// entry names that entry's node.
    fn choice_arguments(
        &mut self,
        controls: &Controls<'_, '_>,
        control: &CheckedControl,
        entry: &BTreeSet<usize>,
        keys: &BTreeMap<ProtocolNodeId, NodeKey>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<Vec<MemberTerm>, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let protocol = controls.protocol;
        let form = protocol
            .form
            .choices
            .iter()
            .find(|choice| choice.declaration == control.node.0)
            .ok_or_else(unchecked)?;
        let checked = protocol
            .checked
            .slots
            .choices
            .iter()
            .find(|choice| choice.node == control.node)
            .ok_or_else(unchecked)?;
        let owner = controls.role(&form.owner.name).ok_or_else(unchecked)?;
        let visible = self.visible_entries(controls, &checked.visible, entry, location)?;
        let mut cases: Vec<(&str, LeafTerm, NodeKey)> = Vec::with_capacity(control.children.len());
        for case in &control.children {
            let case = protocol
                .checked
                .controls
                .control(*case)
                .filter(|case| case.kind == ProtocolNodeKind::Case)
                .ok_or_else(unchecked)?;
            let [body] = case.children[..] else {
                return Err(unchecked());
            };
            let body = keys.get(&body).copied().ok_or_else(unchecked)?;
            let guard = checked
                .cases
                .iter()
                .find(|checked| checked.name == case.name)
                .ok_or_else(unchecked)?;
            let guard = self.scoped_body(controls, &guard.guard, entry, location)?;
            cases.push((case.name.as_str(), guard, body));
        }
        cases.sort_by(|left, right| left.0.as_bytes().cmp(right.0.as_bytes()));
        let mut records = Vec::with_capacity(cases.len());
        for (label, guard, body) in cases {
            records.push(TupleMember::Group(GroupTerm::new(vec![
                member("label", self.text_literal(label, location)?),
                member("guard", guard),
                member("body", LeafTerm::reference(body)),
            ])));
        }
        Ok(vec![
            MemberTerm::bound("owner", LeafTerm::reference(owner)),
            MemberTerm::binding("visible", visible),
            MemberTerm::binding("cases", BindingValue::Tuple(TupleTerm::new(records))),
        ])
    }

    /// The arguments of the `repeat` `control` (FR-466 "Control layouts"):
    /// its owning role, its `visible` entries, the written `maximum`, the
    /// `proof` Group (empty, or `invariant` and `variant`), the `while`
    /// guard, the body's key and the `exhausted` control's key, present
    /// exactly when a maximum is. The entries, guard and proof are lowered
    /// over the binders readable at the repeat's entry; a post-test repeat,
    /// whose guard reads its body's binders, is refused before this.
    fn repeat_arguments(
        &mut self,
        controls: &Controls<'_, '_>,
        control: &CheckedControl,
        entry: &BTreeSet<usize>,
        keys: &BTreeMap<ProtocolNodeId, NodeKey>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<Vec<MemberTerm>, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let protocol = controls.protocol;
        let form = protocol
            .form
            .repeats
            .iter()
            .find(|repeat| repeat.declaration == control.node.0)
            .ok_or_else(unchecked)?;
        let checked = protocol
            .checked
            .slots
            .repeats
            .iter()
            .find(|repeat| repeat.node == control.node)
            .filter(|repeat| !repeat.post_test)
            .ok_or_else(unchecked)?;
        let within = |span: qsl_foundation::Span| {
            control
                .children
                .iter()
                .copied()
                .find(|child| {
                    protocol
                        .form
                        .declarations
                        .get(child.0)
                        .is_some_and(|declaration| {
                            span.start <= declaration.name.span.start
                                && declaration.name.span.end <= span.end
                        })
                })
                .and_then(|child| keys.get(&child).copied())
                .ok_or_else(unchecked)
        };
        let body = within(form.body)?;
        let exhausted = match form.exhausted {
            None => empty_group(),
            Some(span) => BindingValue::Group(GroupTerm::references([within(span)?])),
        };
        let owner = controls.role(&form.owner.name).ok_or_else(unchecked)?;
        let visible = self.visible_entries(controls, &checked.visible, entry, location)?;
        let maximum = match checked.max {
            None => empty_group(),
            Some(max) => BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                self.integer_literal(Integer::from(max), location)?,
            )])),
        };
        let proof = match &checked.loop_proof {
            None => Vec::new(),
            Some(proof) => vec![
                member(
                    "invariant",
                    self.scoped_body(controls, &proof.invariant, entry, location)?,
                ),
                member(
                    "variant",
                    self.scoped_body(controls, &proof.variant, entry, location)?,
                ),
            ],
        };
        let guard = self.scoped_body(controls, &checked.guard, entry, location)?;
        Ok(vec![
            MemberTerm::bound("owner", LeafTerm::reference(owner)),
            MemberTerm::binding("visible", visible),
            MemberTerm::binding("maximum", maximum),
            MemberTerm::binding("proof", BindingValue::Group(GroupTerm::new(proof))),
            MemberTerm::bound("guard", guard),
            MemberTerm::bound("body", LeafTerm::reference(body)),
            MemberTerm::binding("exhausted", exhausted),
        ])
    }

    /// A `choice`'s or `repeat`'s `visible` entries as the set of their
    /// nodes (FR-466 "Decision facts"), each lowered over the binders
    /// readable at the control's entry. An entry that lowers to no node, a
    /// bare literal, is refused before this.
    fn visible_entries(
        &mut self,
        controls: &Controls<'_, '_>,
        visible: &[CheckedBody],
        entry: &BTreeSet<usize>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<BindingValue, CheckRefusal> {
        let mut nodes = Vec::with_capacity(visible.len());
        for body in visible {
            match self.scoped_body(controls, body, entry, location)? {
                LeafTerm::Reference { target } => nodes.push(target.0),
                LeafTerm::Literal { .. } | LeafTerm::DependencyReference { .. } => {
                    return Err(fault(location, KeyFault::UncheckedProtocolFact));
                }
            }
        }
        Ok(BindingValue::Group(GroupTerm::references(sorted(nodes))))
    }

    /// `body`, lowered over the binders readable at a control's entry
    /// `entry`.
    fn scoped_body(
        &mut self,
        controls: &Controls<'_, '_>,
        body: &CheckedBody,
        entry: &BTreeSet<usize>,
        location: &quire_semantic_value::location::Location,
    ) -> Result<LeafTerm, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let scope = entry
            .iter()
            .map(|slot| controls.binders.get(slot).copied().ok_or_else(unchecked))
            .collect::<Result<Vec<_>, _>>()?;
        let mut binders = Binders::new(&controls.slot_names, &scope);
        self.expression(body.root(), &mut binders)
    }

    /// The constraint or condition of the control `node`, lowered over the
    /// binders readable at its entry `entry`, and its own binder when
    /// `own`.
    fn control_body(
        &mut self,
        controls: &Controls<'_, '_>,
        node: ProtocolNodeId,
        entry: &BTreeSet<usize>,
        own: bool,
        location: &quire_semantic_value::location::Location,
    ) -> Result<LeafTerm, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let body = controls.bodies.get(&node).ok_or_else(unchecked)?;
        let mut scope = entry
            .iter()
            .map(|slot| controls.binders.get(slot).copied().ok_or_else(unchecked))
            .collect::<Result<Vec<_>, _>>()?;
        if own {
            scope.push(controls.own.get(&node).copied().ok_or_else(unchecked)?);
        }
        let mut binders = Binders::new(&controls.slot_names, &scope);
        self.expression(body.root(), &mut binders)
    }
}

/// Each control's declaration path (FR-464 "Control tree"): the root's is
/// `P`, `run` and its name, and every other control's is its holder's
/// followed by its own name. A `branch` or a `case` is no node: its body's
/// holder is the `parallel` or `choice` that holds it.
fn control_declarations(
    protocol: &str,
    tree: &CheckedControlTree,
) -> BTreeMap<ProtocolNodeId, Vec<String>> {
    let mut holder: BTreeMap<ProtocolNodeId, Vec<String>> = BTreeMap::new();
    let mut declarations = BTreeMap::new();
    for (index, control) in tree.controls.iter().enumerate() {
        let base = match holder.remove(&control.node) {
            Some(base) => base,
            None if index == 0 => vec![protocol.to_owned(), "run".to_owned()],
            None => continue,
        };
        let mut own = base.clone();
        own.push(control.name.clone());
        let held = if matches!(
            control.kind,
            ProtocolNodeKind::Branch | ProtocolNodeKind::Case
        ) {
            base
        } else {
            own.clone()
        };
        for child in &control.children {
            holder.insert(*child, held.clone());
        }
        declarations.insert(control.node, own);
    }
    declarations
}

/// A join's `policy` text (FR-466 "Control layouts").
fn join_text(policy: &JoinPolicy) -> &'static str {
    match policy {
        JoinPolicy::All => "all",
        JoinPolicy::Any => "any",
        JoinPolicy::Quorum(_) => "quorum",
        JoinPolicy::Predicate(_) => "predicate",
    }
}

/// A `parallel`'s `outstanding` text.
const fn outstanding_text(outstanding: Outstanding) -> &'static str {
    match outstanding {
        Outstanding::Continue => "continue",
        Outstanding::Cancel => "cancel",
    }
}

/// The three controls an `await` holds.
struct AwaitParts {
    event: ProtocolNodeId,
    then: ProtocolNodeId,
    timeout: ProtocolNodeId,
}

impl AwaitParts {
    /// The awaited event, `then` and `timeout` controls of `control`, told
    /// apart by the spans S2 records for them.
    fn of(
        protocol: &qsl_forms::ProtocolDeclarationForm,
        control: &crate::check::protocol_clause::CheckedControl,
    ) -> Option<Self> {
        let form = protocol
            .awaits
            .iter()
            .find(|form| form.declaration == control.node.0)?;
        let mut event = None;
        let mut then = None;
        let mut timeout = None;
        for &child in &control.children {
            let span = protocol.declarations.get(child.0)?.name.span;
            if span == form.matched.span {
                event = Some(child);
            } else if form.then.start <= span.start && span.end <= form.then.end {
                then = Some(child);
            } else if form.timeout.start <= span.start && span.end <= form.timeout.end {
                timeout = Some(child);
            }
        }
        Some(Self {
            event: event?,
            then: then?,
            timeout: timeout?,
        })
    }
}

impl Controls<'_, '_> {
    /// The node of the role named `name`.
    fn role(&self, name: &str) -> Option<NodeKey> {
        self.inputs
            .roles
            .iter()
            .find(|(declared, _)| declared == name)
            .map(|(_, key)| *key)
    }

    /// The controls of the tree in an order that keys each after every node
    /// it names: its children, and the control it is anchored at. A cycle is
    /// a fact S3 refused (QSpec FR-464 "Derived causal relation").
    fn build_order(&self) -> Result<Vec<ProtocolNodeId>, CheckRefusal> {
        let tree = &self.protocol.checked.controls;
        let unchecked = || fault(&self.protocol.location(), KeyFault::UncheckedProtocolFact);
        let root = tree.root().ok_or_else(unchecked)?;
        let mut order = Vec::with_capacity(tree.controls.len());
        // `false`: its names are being keyed; `true`: it is keyed.
        let mut state: BTreeMap<ProtocolNodeId, bool> = BTreeMap::new();
        let mut stack = vec![(root.node, false)];
        while let Some((node, expanded)) = stack.pop() {
            if expanded {
                state.insert(node, true);
                if tree
                    .control(node)
                    .is_some_and(|control| control.kind != ProtocolNodeKind::Branch)
                {
                    order.push(node);
                }
                continue;
            }
            match state.get(&node) {
                Some(true) => continue,
                Some(false) => return Err(unchecked()),
                None => {}
            }
            state.insert(node, false);
            stack.push((node, true));
            let control = tree.control(node).ok_or_else(unchecked)?;
            let names = control.children.iter().chain(self.anchored.get(&node));
            for &name in names.collect::<Vec<_>>().into_iter().rev() {
                if tree.control(name).is_none() {
                    return Err(unchecked());
                }
                match state.get(&name) {
                    Some(true) => {}
                    Some(false) => return Err(unchecked()),
                    None => stack.push((name, false)),
                }
            }
        }
        Ok(order)
    }
}
