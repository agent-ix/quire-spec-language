// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-466's step-control nodes: the `protocol`/`control` node each `sequence`,
//! `send`, `receive`, `attempt`, `effect`, `event`, `commit`, `check`,
//! `fence` and `await` of a protocol's `run` tree emits, with the argument
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
//! in the tree. `choice`, `parallel` and `repeat` are not written here
//! ([`ProtocolConstruct::Control`](crate::check::refusal::ProtocolConstruct)).

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{AnchorSite, ProtocolNodeKind};
use quire_exact::{Integer, NodeKey};

use super::super::{fault, Binder, Binders, Lowering};
use super::{
    empty_group, identifier, member, protocol_location, sorted, ProtocolInput, ProtocolSite,
};
use crate::check::ir::CheckedBody;
use crate::check::node_key::{
    BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm, Operation, OperationLaw,
    Operator, TupleTerm,
};
use crate::check::protocol_clause::ProtocolNodeId;
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
                // ([`ProtocolConstruct::Compensations`](crate::check::refusal::ProtocolConstruct)).
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

        let mut keys: BTreeMap<ProtocolNodeId, NodeKey> = BTreeMap::new();
        for node in controls.build_order()? {
            let control = tree.control(node).ok_or_else(unchecked)?;
            let location = protocol_location(name, ProtocolSite::Control(&control.path));
            let mut declaration = vec![identifier(name, &location)?, identifier("run", &location)?];
            for segment in &control.path {
                declaration.push(identifier(segment, &location)?);
            }
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
            // Not written yet: refused before any node is built.
            ProtocolNodeKind::Choice
            | ProtocolNodeKind::Parallel
            | ProtocolNodeKind::Repeat
            | ProtocolNodeKind::Branch
            | ProtocolNodeKind::Case
            | ProtocolNodeKind::Finish
            | ProtocolNodeKind::CompensateTemplate => return Err(unchecked()),
        })
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
                order.push(node);
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
