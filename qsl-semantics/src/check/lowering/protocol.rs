// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092's protocol nodes: the `protocol`/`protocol_clause` node each
//! checked protocol declaration emits, and the `protocol`/`protocol_reference`
//! carrier a protocol reference expression emits (FR-308, FR-218, QSpec
//! FR-322-AC-66).
//!
//! A protocol's node is a `quire.op.protocol.control` application of the
//! operator `protocol` with the thirteen arguments of QSpec FR-464 "Clause
//! body", carrying the package's selected `protocol_profile` law. It is keyed
//! by `quire.application-node/v1` with the declaration's qualified name `P`
//! and the unit's source owner. Every composite subterm of that body is its
//! own node, reached by reference (FR-464): the `over` and activation binders
//! (`value`/`parameter`), the `run` tree's controls (declared at `P`, `run`
//! and the control names from the root down, FR-466; see `controls`) and the
//! `finish` obligation (declared at `P`, `finish` and its name, FR-465).
//!
//! What this writes is what QSL can prove IR admits. A protocol that names a
//! construct whose node is not written yet (a role, a channel, a `choice`,
//! `parallel` or `repeat`, a `terminal` member, a requirement, a
//! compensation, a capture, guard or control body that names a function) is refused with that construct named
//! ([`ProtocolConstruct`]) rather than written without it: its package would
//! lack part of the graph its `package_id` stands for. A reference binds the
//! clause node's key and is itself a `{term: "reference", target}` leaf keyed
//! by `quire.structural-node/v1`.

use qsl_forms::{
    ActivationForm, ChannelCapacity, ChannelDelivery, ChannelOverflow, LossSelector,
    ProtocolDeclarationForm, ProtocolNodeKind,
};
use quire_exact::{EffectiveId, Identifier, Integer, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

mod controls;

pub(crate) use controls::AttemptNodes;
use controls::RunInputs;

use super::{fault, refuse, Binder, Binders, Lowering, NodeContent};
use crate::check::family::OccurrenceRole;
use crate::check::ir::{CheckedNode, NodeKind};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, MemberTerm,
    NodeTag, Operation, OperationLaw, Operator, TupleMember, TupleTerm,
};
use crate::check::protocol_clause::CheckedProtocol;
use crate::check::protocol_controls::{CheckedChannel, CheckedOrdering};
use crate::check::protocol_obligation::{CaptureOwner, CheckedActivation};
use crate::check::protocol_slots::{
    CheckedLifetime, CheckedScheduling, CheckedSlotBinder, CheckedTerminalMember,
};
use crate::check::refusal::{
    CheckCause, CheckRefusal, KeyFault, ProtocolConstruct, ProtocolReferenceCause,
};
use crate::value::member::Member;

/// The `Origin::TypeDeclaration` name a protocol's declaration is located
/// under. Spaced, so it names no declared type.
pub(crate) fn declaration_origin(protocol: &str) -> String {
    format!("protocol {protocol}")
}

/// The `Origin::TypeDeclaration` name a protocol's `compensate` template is
/// located under, where its operation's `anchor` occurrence is recorded
/// (FR-381). Spaced, so it names no declared type.
pub(crate) fn compensation_origin(protocol: &str, template: &str) -> String {
    format!("protocol {protocol} compensate {template}")
}

/// One place of a protocol that has a location of its own (FR-096): the
/// occurrence of the node it declares is recorded there, and a location that
/// roots an expression tree resolves its paths against that tree's spans.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ProtocolSite<'a> {
    /// The protocol's declaration: its `protocol_clause` node's `declaration`
    /// occurrence and the `over` and activation binders' `expression`
    /// occurrences.
    Declaration,
    /// The `finish` node of this name: its `declaration` occurrence and its
    /// closing record binder's `expression` occurrence.
    Finish(&'a str),
    /// The `finish` constraint's expression tree.
    FinishBody(&'a str),
    /// The `terminal` node: its `declaration` occurrence and its condition.
    Terminal,
    /// The `terminal when` condition's expression tree.
    TerminalBody,
    /// The `activation on each` guard's expression tree.
    ActivationGuard,
    /// The capture of this name: its binder's `expression` occurrence.
    Capture(&'a str),
    /// The initializer of the capture of this name: its expression tree.
    CaptureInitializer(&'a str),
    /// The role of this name: its `declaration` occurrence.
    Role(&'a str),
    /// The `until` term of the replicated role of this name.
    RoleUntil(&'a str),
    /// The channel of this name: its `declaration` occurrence and its
    /// queue's `generated` occurrence.
    Channel(&'a str),
    /// The FIFO key of the channel of this name.
    ChannelKey(&'a str),
    /// The control at this path from the `run` root, the root's name first:
    /// its `declaration` occurrence.
    Control(&'a [String]),
    /// The constraint or condition tree of the control at this path.
    ControlBody(&'a [String]),
}

/// The `Origin::TypeDeclaration` name of `site` of protocol `protocol`.
/// Spaced, so it names no declared type.
pub(crate) fn protocol_site(protocol: &str, site: ProtocolSite<'_>) -> String {
    match site {
        ProtocolSite::Declaration => declaration_origin(protocol),
        ProtocolSite::Finish(name) => format!("protocol {protocol} finish {name}"),
        ProtocolSite::Role(role) => format!("protocol {protocol} role {role}"),
        ProtocolSite::RoleUntil(role) => format!("protocol {protocol} role {role} until"),
        ProtocolSite::Channel(channel) => format!("protocol {protocol} channel {channel}"),
        ProtocolSite::ChannelKey(channel) => format!("protocol {protocol} channel {channel} key"),
        ProtocolSite::Terminal => format!("protocol {protocol} terminal"),
        ProtocolSite::TerminalBody => format!("protocol {protocol} terminal body"),
        ProtocolSite::FinishBody(name) => format!("protocol {protocol} finish {name} body"),
        ProtocolSite::ActivationGuard => format!("protocol {protocol} activation guard"),
        ProtocolSite::Capture(name) => format!("protocol {protocol} capture {name}"),
        ProtocolSite::CaptureInitializer(name) => {
            format!("protocol {protocol} capture {name} initializer")
        }
        ProtocolSite::Control(path) => format!("protocol {protocol} run {}", path.join(" ")),
        ProtocolSite::ControlBody(path) => {
            format!("protocol {protocol} run {} body", path.join(" "))
        }
    }
}

/// The root location of [`protocol_site`].
pub(crate) fn protocol_location(protocol: &str, site: ProtocolSite<'_>) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: protocol_site(protocol, site),
    })
}

/// One checked protocol declaration, as lowering reads it: its form and
/// every fact S3 checked of it.
pub(crate) struct ProtocolInput<'a> {
    /// The declaration as S2 built it.
    pub(crate) form: &'a ProtocolDeclarationForm,
    /// What S3 checked of it.
    pub(crate) checked: &'a CheckedProtocol,
}

impl ProtocolInput<'_> {
    /// Where the declaration's `declaration` occurrence is recorded.
    fn location(&self) -> Location {
        protocol_location(&self.form.name.name, ProtocolSite::Declaration)
    }

    /// The constructs of this protocol whose node is not written yet, each
    /// once, in a fixed order. Empty when the whole graph is written.
    fn unemitted(&self) -> Vec<ProtocolConstruct> {
        let form = self.form;
        let slots = &self.checked.slots;
        let finish_node = self.checked.finish.as_ref().map(|(node, _)| *node);
        let mut unemitted = Vec::new();
        if self
            .checked
            .roles
            .iter()
            .any(|role| !role.replicated && role.context.is_none())
        {
            unemitted.push(ProtocolConstruct::WholeModelRole);
        }
        if !form.relationships.is_empty() {
            unemitted.push(ProtocolConstruct::Relationships);
        }
        if self
            .checked
            .channels
            .iter()
            .any(|channel| bounded_carries(&channel.carries))
        {
            unemitted.push(ProtocolConstruct::ChannelCarriesBounded);
        }
        if !form.requirements.is_empty() {
            unemitted.push(ProtocolConstruct::Requirements);
        }
        if !form.compensations.is_empty() {
            unemitted.push(ProtocolConstruct::Compensations);
        }
        let mut kinds: Vec<ProtocolNodeKind> = Vec::new();
        for control in &self.checked.controls.controls {
            // A `branch` and a `case` are held by the `parallel` and the
            // `choice` that is refused for them.
            let refused = matches!(
                control.kind,
                ProtocolNodeKind::Choice | ProtocolNodeKind::Parallel | ProtocolNodeKind::Repeat
            );
            if refused && !kinds.contains(&control.kind) {
                kinds.push(control.kind);
                unemitted.push(ProtocolConstruct::Control(control.kind.label()));
            }
        }
        if self.checked.obligations.captures.iter().any(|capture| {
            capture.owner == CaptureOwner::Protocol && reads_declaration(capture.initializer.root())
        }) {
            unemitted.push(ProtocolConstruct::CaptureReadsDeclaration);
        }
        if matches!(
            &self.checked.obligations.activation,
            CheckedActivation::OnEach { guard: Some(guard), .. }
                if reads_declaration(guard.root())
        ) {
            unemitted.push(ProtocolConstruct::ActivationGuardReadsDeclaration);
        }
        if matches!(
            &slots.terminal,
            CheckedTerminalMember::When(body) if reads_declaration(body.root())
        ) {
            unemitted.push(ProtocolConstruct::TerminalReadsDeclaration);
        }
        if matches!(form.activation, ActivationForm::Each { .. })
            && matches!(&slots.terminal, CheckedTerminalMember::When(body) if body.reads_local(0))
        {
            unemitted.push(ProtocolConstruct::TerminalReadsOverUnderEach);
        }
        if slots
            .bodies
            .iter()
            .any(|body| reads_declaration(body.body.root()) && Some(body.node) != finish_node)
        {
            unemitted.push(ProtocolConstruct::ControlReadsDeclaration);
        }
        if self
            .finish_body()
            .is_some_and(|body| reads_declaration(body.root()))
        {
            unemitted.push(ProtocolConstruct::FinishReadsDeclaration);
        }
        unemitted
    }

    /// Whether the `run` tree holds an `attempt`, whose node names the
    /// state clauses and operation anchors written after the functions.
    fn has_attempt(&self) -> bool {
        self.checked
            .controls
            .controls
            .iter()
            .any(|control| control.kind == ProtocolNodeKind::Attempt)
    }

    /// The checked body of the `finish` constraint.
    fn finish_body(&self) -> Option<&crate::check::ir::CheckedBody> {
        let (finish, _) = self.checked.finish.as_ref()?;
        self.checked
            .slots
            .bodies
            .iter()
            .find(|body| body.node == *finish)
            .map(|body| &body.body)
    }
}

/// Whether `carries` lowers to a `bounded_domain` node, which IR's reader
/// does not admit as a channel's `carries`. An `option` is a `composite_type`
/// whatever its payload; a quantity is a `scalar_type` and a nominal enum a
/// `value` node, both of which the reader admits.
fn bounded_carries(carries: &ValueType) -> bool {
    match carries {
        ValueType::Int(_)
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Text(_) => true,
        ValueType::Collection(collection) => collection.bound().is_some(),
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Reference(_)
        | ValueType::Composite(_)
        | ValueType::Option(_)
        | ValueType::Enum(_)
        | ValueType::Quantity(_)
        | ValueType::BoundedQuantity(_)
        | ValueType::Map(_)
        | ValueType::Population(_) => false,
    }
}

/// Whether `body` reads a declaration whose node is written after the
/// protocols': a function it calls, a dispatch it makes or a protocol it
/// refers to.
fn reads_declaration(body: CheckedNode<'_>) -> bool {
    body.descendants().into_iter().any(|node| {
        matches!(
            node.kind(),
            NodeKind::Call { .. } | NodeKind::Dispatch { .. } | NodeKind::ProtocolReference { .. }
        )
    })
}

/// A protocol's channel nodes and queue nodes, each set ascending by digest,
/// and each channel's node by its name.
type ChannelNodes = (Vec<NodeKey>, Vec<NodeKey>, Vec<(String, NodeKey)>);

/// The operations of a protocol's `channel` and `queue` nodes.
struct ChannelOperations {
    channel: Operation,
    queue: Operation,
}

/// `keys` in ascending node-id digest order, each once (FR-464 "Sets").
fn sorted(keys: impl IntoIterator<Item = NodeKey>) -> Vec<NodeKey> {
    let mut keys: Vec<NodeKey> = keys.into_iter().collect();
    keys.sort();
    keys.dedup();
    keys
}

/// A channel's `delivery` text (FR-465 "Channel").
const fn delivery_text(delivery: ChannelDelivery) -> &'static str {
    match delivery {
        ChannelDelivery::Unknown => "unknown",
        ChannelDelivery::AtMostOnce => "at-most-once",
        ChannelDelivery::AtLeastOnce => "at-least-once",
        ChannelDelivery::ExactlyOncePremise => "exactly-once-premise",
    }
}

/// A queue's `loss` text (FR-465 "Queue").
const fn loss_text(selector: LossSelector) -> &'static str {
    match selector {
        LossSelector::Head => "head",
        LossSelector::Tail => "tail",
        LossSelector::New => "new",
    }
}

/// `text` as an identifier, or the refusal that it is empty.
fn identifier(text: &str, location: &Location) -> Result<Identifier, CheckRefusal> {
    Identifier::new(text.to_owned()).map_err(|_| {
        refuse(
            location,
            CheckCause::NodePreimage(crate::check::node_key::NodeKeyRefusal::EmptyQualifiedName),
        )
    })
}

/// A Group member `name` bound to `leaf`.
fn member(name: &str, leaf: LeafTerm) -> GroupMember {
    GroupMember::Binding(Binding::new(name, leaf))
}

/// An `aggregate` of no member, the encoding of an absent optional and of an
/// empty set (FR-464 "Term shapes").
fn empty_group() -> BindingValue {
    BindingValue::Group(GroupTerm::new(Vec::new()))
}

/// The refusal that `construct` of the protocol at `location` has no node
/// written yet.
fn not_emitted(construct: ProtocolConstruct, location: &Location) -> CheckRefusal {
    refuse(
        location,
        CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::NotEmitted { construct })),
    )
}

impl Lowering<'_> {
    /// Lower and key each protocol of `protocols`, in declaration order,
    /// recording its one `declaration` occurrence. A package that selects no
    /// protocol profile has no `protocol_profile` law to carry (ADR-011
    /// §2.4): no node is built and nothing is refused, so the unit is as it
    /// was before protocol nodes; a reference to a protocol refuses
    /// `missing-selection`. With a profile selected, a protocol that names a
    /// construct whose node is not written yet refuses at its declaration,
    /// once for each such construct
    /// (`unsupported_construct`/`not-yet-implemented`), and so does a
    /// reference to it.
    pub(crate) fn protocol_nodes(
        &mut self,
        protocols: &[ProtocolInput<'_>],
    ) -> Result<Vec<CheckRefusal>, CheckRefusal> {
        let mut keys = Vec::with_capacity(protocols.len());
        let mut refusals = Vec::new();
        for protocol in protocols {
            let location = protocol.location();
            let law = match self.law(LawRole::ProtocolProfile, &location) {
                Ok(law) => law,
                Err(CheckRefusal {
                    cause: CheckCause::MissingSelection { .. },
                    ..
                }) => {
                    keys.push(ProtocolKey::Unselected);
                    continue;
                }
                Err(other) => return Err(other),
            };
            let mut unemitted = protocol.unemitted();
            if self.fifo_key_unstable(protocol, &location)? {
                unemitted.push(ProtocolConstruct::ChannelKeyNotEqualityStable);
            }
            let owner = self.owner_model_object(protocol);
            if owner.is_none() {
                unemitted.push(ProtocolConstruct::NoOwnerModel);
            }
            if let Some(first) = unemitted.first() {
                refusals.extend(
                    unemitted
                        .iter()
                        .map(|construct| not_emitted(*construct, &location)),
                );
                keys.push(ProtocolKey::NotEmitted(*first));
                continue;
            }
            let Some(owner) = owner else {
                return Err(fault(&location, KeyFault::UncheckedProtocolFact));
            };
            if protocol.has_attempt() {
                keys.push(ProtocolKey::AfterClauses);
                continue;
            }
            let key = self.protocol_node(protocol, &law, owner, &[], &location)?;
            keys.push(ProtocolKey::Keyed(key));
        }
        self.protocol_keys = keys;
        Ok(refusals)
    }

    /// The nodes of the protocols that hold an `attempt`, written once the
    /// state clauses and operation anchors they name are (`attempts[i]` is
    /// the `i`-th protocol's). Each is refused or keyed as the rest are.
    pub(crate) fn protocol_nodes_with_attempts(
        &mut self,
        protocols: &[ProtocolInput<'_>],
        attempts: &[Vec<AttemptNodes>],
    ) -> Result<(), CheckRefusal> {
        for (index, protocol) in protocols.iter().enumerate() {
            if !matches!(
                self.protocol_keys.get(index),
                Some(ProtocolKey::AfterClauses)
            ) {
                continue;
            }
            let location = protocol.location();
            let law = self.law(LawRole::ProtocolProfile, &location)?;
            let owner = self
                .owner_model_object(protocol)
                .ok_or_else(|| fault(&location, KeyFault::UncheckedProtocolFact))?;
            let nodes = attempts.get(index).map_or(&[][..], Vec::as_slice);
            let key = self.protocol_node(protocol, &law, owner, nodes, &location)?;
            self.protocol_keys[index] = ProtocolKey::Keyed(key);
        }
        Ok(())
    }

    /// The object type `protocol` is declared over, whose model declaration
    /// node is the clause's `owner_model` (QSpec FR-464 "owner_model": the
    /// exact local model declaration node the clause's source `over` type
    /// represents; QSpec's published fixtures name the object type the
    /// `over` reference targets). `None` when `over` is not a reference to an
    /// object type of a selected model: no node can be named, so the clause
    /// is not emitted ([`ProtocolConstruct::NoOwnerModel`]).
    fn owner_model_object(&self, protocol: &ProtocolInput<'_>) -> Option<EffectiveId> {
        let ValueType::Reference(context) = &protocol.checked.slots.binders.first()?.value_type
        else {
            return None;
        };
        self.models
            .iter()
            .any(|model| model.object_types().any(|(_, object)| object == *context))
            .then_some(*context)
    }

    /// Whether a FIFO channel of `protocol` is keyed by a type IR's reader
    /// does not take as equality-stable (FR-465 "Channel": the key's type
    /// must be). The reader's rule, over the key type's node: every node
    /// reached is a Boolean, Integer, Rational, Decimal, Text, String, UUID,
    /// Timestamp or enum scalar, a `reference`, or a composite whose
    /// children are such; a bounded domain, a quantity, a float or a
    /// model-owned node is not.
    fn fifo_key_unstable(
        &mut self,
        protocol: &ProtocolInput<'_>,
        location: &Location,
    ) -> Result<bool, CheckRefusal> {
        for channel in &protocol.checked.channels {
            let CheckedOrdering::Fifo { key, .. } = &channel.ordering else {
                continue;
            };
            let node = self.type_node(key.root().value_type(), location)?;
            if !self.equality_stable(node) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The reader's `equality_stable` over the type node `key`.
    fn equality_stable(&self, key: NodeKey) -> bool {
        let mut pending = vec![key];
        let mut visited = std::collections::BTreeSet::new();
        while let Some(key) = pending.pop() {
            if !visited.insert(key) {
                continue;
            }
            let content = match self.graph.nodes.get(&key) {
                Some(node) => &node.content,
                None => match self.drafts.get(&key) {
                    Some(draft) => &draft.content,
                    // A nominal enum is keyed by its declaration node, which
                    // is written with the declarations and not by this
                    // type: a scalar `enum` the reader compares.
                    None => continue,
                },
            };
            match (content.node_tag, content.semantic_form) {
                (
                    NodeTag::ScalarType,
                    "boolean" | "integer" | "rational" | "decimal" | "text" | "string" | "uuid"
                    | "timestamp" | "enum",
                )
                | (NodeTag::CompositeType, "reference") => {}
                (NodeTag::CompositeType, _) => {
                    content.body.for_each_key(&mut |child| pending.push(child));
                }
                (
                    NodeTag::ScalarType
                    | NodeTag::BoundedDomain
                    | NodeTag::Value
                    | NodeTag::Expression
                    | NodeTag::Function
                    | NodeTag::Model
                    | NodeTag::Relation
                    | NodeTag::State
                    | NodeTag::Temporal
                    | NodeTag::Protocol
                    | NodeTag::Claim
                    | NodeTag::Correspondence
                    | NodeTag::Hybrid,
                    _,
                ) => return false,
            }
        }
        true
    }

    /// The `value`/`parameter` node of `binder`, at `level`, and the scope
    /// entry that binds its slot `slot`.
    fn protocol_binder(
        &mut self,
        binder: &CheckedSlotBinder,
        slot: usize,
        level: usize,
        location: &Location,
    ) -> Result<Binder, CheckRefusal> {
        let semantic_type = self.binder_type(&binder.value_type, None, location)?;
        let parameter = self.parameter(&binder.name, level, semantic_type, location)?;
        Ok(Binder {
            slot,
            parameter,
            semantic_type,
        })
    }

    /// A protocol member node: tagged `protocol`, of the form
    /// `semantic_form`, Boolean, declared at `declaration` and keyed with no
    /// owner (FR-464 "Operator catalog"), with its one `declaration`
    /// occurrence at `location`.
    fn protocol_member(
        &mut self,
        location: &Location,
        semantic_form: &'static str,
        boolean: NodeKey,
        declaration: Vec<Identifier>,
        owner_model: Option<NodeKey>,
        body: BodyTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        let key = self.insert_node(
            location,
            NodeContent {
                owner_model,
                node_tag: NodeTag::Protocol,
                semantic_form,
                semantic_type: Some(boolean),
                declaration: Some(declaration),
                owner: None,
                body,
            },
        )?;
        self.record(key, OccurrenceRole::Declaration, location.clone());
        Ok(key)
    }

    /// The nodes of one protocol whose every part is written, and its
    /// `protocol_clause` node's key.
    fn protocol_node(
        &mut self,
        protocol: &ProtocolInput<'_>,
        law: &OperationLaw,
        owner: EffectiveId,
        attempts: &[AttemptNodes],
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let slots = &protocol.checked.slots;
        let owner_model = self.object_node(owner, location)?;
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let operation = |operator: &'static str| -> Result<Operation, CheckRefusal> {
            Ok(Operation {
                laws: vec![law.clone()],
                member: Some(Member::ProfileOperator {
                    operator: identifier(operator, location)?,
                }),
                ..Operation::plain("quire.op.protocol.control")
            })
        };
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);

        // The binders readable at the clause: `over` at level 0 and the
        // `each` trigger at level 1 (FR-464 "Binders", "Level"). The `run`
        // tree binds none yet, so `finish`'s closing record follows them.
        let over =
            self.protocol_binder(slots.binders.first().ok_or_else(unchecked)?, 0, 0, location)?;
        let trigger = match protocol.form.activation {
            ActivationForm::Origin => None,
            ActivationForm::Each { .. } => Some(self.protocol_binder(
                slots.binders.get(1).ok_or_else(unchecked)?,
                1,
                1,
                location,
            )?),
        };
        let mut readable: Vec<Binder> = [Some(over), trigger].into_iter().flatten().collect();
        let guard = self.activation_guard(protocol, over, trigger)?;
        let captures = self.capture_records(protocol, &mut readable)?;

        let roles = self.role_nodes(protocol, &operation("role")?, &readable, boolean)?;
        let (channels, queues, named_channels) = self.channel_nodes(
            protocol,
            &ChannelOperations {
                channel: operation("channel")?,
                queue: operation("queue")?,
            },
            &roles,
            &readable,
            boolean,
        )?;
        let run = self.run_nodes(
            protocol,
            &RunInputs {
                law,
                roles: &roles,
                channels: &named_channels,
                attempts,
                clause_binders: &readable,
                boolean,
            },
        )?;
        let roles: Vec<NodeKey> = sorted(roles.into_iter().map(|(_, key)| key));
        let finish = self.finish_node(protocol, &operation("finish")?, &run.readable, boolean)?;
        let terminal = self.terminal_node(
            protocol,
            &operation("terminal")?,
            &readable,
            trigger,
            boolean,
        )?;
        let run = run.root;

        let activation = match &trigger {
            None => GroupTerm::new(vec![member("kind", self.text_literal("origin", location)?)]),
            Some(trigger) => {
                let mut members = vec![
                    member("kind", self.text_literal("each", location)?),
                    member("trigger", LeafTerm::reference(trigger.parameter)),
                ];
                members.extend(guard.map(|guard| member("guard", guard)));
                GroupTerm::new(members)
            }
        };
        let scheduling = match slots.scheduling {
            CheckedScheduling::Fair => empty_group(),
            CheckedScheduling::Adversarial => {
                BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                    self.text_literal("adversarial", location)?,
                )]))
            }
        };
        self.protocol_member(
            location,
            "protocol_clause",
            boolean,
            vec![identifier(name, location)?],
            Some(owner_model),
            BodyTerm::application(
                Operator::ProtocolControl,
                operation("protocol")?,
                boolean,
                vec![
                    MemberTerm::bound("over", LeafTerm::reference(over.parameter)),
                    MemberTerm::binding("activation", BindingValue::Group(activation)),
                    MemberTerm::binding("captures", BindingValue::Tuple(captures)),
                    MemberTerm::binding("roles", BindingValue::Group(GroupTerm::references(roles))),
                    MemberTerm::binding("relationships", empty_group()),
                    MemberTerm::binding(
                        "channels",
                        BindingValue::Group(GroupTerm::references(channels)),
                    ),
                    MemberTerm::binding(
                        "queues",
                        BindingValue::Group(GroupTerm::references(queues)),
                    ),
                    MemberTerm::binding("requirements", empty_group()),
                    MemberTerm::binding("compensations", empty_group()),
                    MemberTerm::binding(
                        "terminal",
                        terminal.map_or_else(empty_group, |terminal| {
                            BindingValue::Group(GroupTerm::references([terminal]))
                        }),
                    ),
                    MemberTerm::binding("scheduling", scheduling),
                    MemberTerm::bound("run", LeafTerm::reference(run)),
                    MemberTerm::bound("finish", LeafTerm::reference(finish)),
                ],
            ),
        )
    }

    /// The `guard` term of the `activation on each` guard of `protocol`
    /// (FR-464 "Clause body"), or `None` for an `origin` activation and an
    /// `each` without a guard. The guard is checked over the activation
    /// binder and `over`, in that order (QSpec FR-464 stage
    /// `protocol-activation`), so its local slot 0 is the trigger and slot 1
    /// is `over`; the checker bars a read of `over`.
    fn activation_guard(
        &mut self,
        protocol: &ProtocolInput<'_>,
        over: Binder,
        trigger: Option<Binder>,
    ) -> Result<Option<LeafTerm>, CheckRefusal> {
        let CheckedActivation::OnEach {
            guard: Some(guard), ..
        } = &protocol.checked.obligations.activation
        else {
            return Ok(None);
        };
        let unchecked = || fault(&protocol.location(), KeyFault::UncheckedProtocolFact);
        let trigger = trigger.ok_or_else(unchecked)?;
        let slots = &protocol.checked.slots;
        let slot_names: Vec<String> = [slots.binders.get(1), slots.binders.first()]
            .into_iter()
            .map(|binder| binder.map(|binder| binder.name.clone()))
            .collect::<Option<_>>()
            .ok_or_else(unchecked)?;
        let scope = [Binder { slot: 0, ..trigger }, Binder { slot: 1, ..over }];
        let mut binders = Binders::new(&slot_names, &scope);
        self.expression(guard.root(), &mut binders).map(Some)
    }

    /// The `captures` Tuple of `protocol` (FR-464 "Clause body"): one Group
    /// of `binder` and `initializer` per capture, in evaluation order. Each
    /// capture binder is a `parameter` at the level of the binders readable
    /// before it, `over` and the trigger first, and joins `readable`; its
    /// initializer is read over those binders alone (stage
    /// `protocol-capture`).
    fn capture_records(
        &mut self,
        protocol: &ProtocolInput<'_>,
        readable: &mut Vec<Binder>,
    ) -> Result<TupleTerm, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let slots = &protocol.checked.slots;
        let slot_names: Vec<String> = slots
            .binders
            .iter()
            .map(|binder| binder.name.clone())
            .collect();
        let checked: Vec<_> = protocol
            .checked
            .obligations
            .captures
            .iter()
            .filter(|capture| capture.owner == CaptureOwner::Protocol)
            .collect();
        if checked.len() != protocol.form.captures.len() {
            return Err(fault(&protocol.location(), KeyFault::UncheckedProtocolFact));
        }
        let mut records = Vec::with_capacity(checked.len());
        for capture in checked {
            let location = protocol_location(name, ProtocolSite::Capture(&capture.name));
            let slot = readable.len();
            let declared = slots
                .binders
                .get(slot)
                .ok_or_else(|| fault(&location, KeyFault::UncheckedProtocolFact))?;
            let binder = self.protocol_binder(declared, slot, slot, &location)?;
            let mut binders = Binders::new(&slot_names, readable);
            let initializer = self.expression(capture.initializer.root(), &mut binders)?;
            readable.push(binder);
            records.push(TupleMember::Group(GroupTerm::new(vec![
                member("binder", LeafTerm::reference(binder.parameter)),
                member("initializer", initializer),
            ])));
        }
        Ok(TupleTerm::new(records))
    }

    /// The `role` nodes of `protocol` (FR-465 "Role") with each role's name,
    /// in source order. A role on a model type refers to that type's model
    /// node. A static role's `replication` Group is empty; a replicated role's
    /// holds its population, `max`, binder, lifetime and, for `until`, the
    /// term. A role on a whole model is refused before this
    /// ([`ProtocolConstruct::WholeModelRole`]).
    fn role_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        readable: &[Binder],
        boolean: NodeKey,
    ) -> Result<Vec<(String, NodeKey)>, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let mut roles = Vec::with_capacity(protocol.checked.roles.len());
        for (index, role) in protocol.checked.roles.iter().enumerate() {
            let location = protocol_location(name, ProtocolSite::Role(&role.name));
            let unchecked = || fault(&location, KeyFault::UncheckedProtocolFact);
            let context = role.context.ok_or_else(unchecked)?;
            let model = LeafTerm::reference(self.object_node(context, &location)?);
            let replication = if role.replicated {
                self.replication(protocol, index, context, readable, &location)?
            } else {
                empty_group()
            };
            let declaration = vec![
                identifier(name, &location)?,
                identifier("role", &location)?,
                identifier(&role.name, &location)?,
            ];
            let key = self.protocol_member(
                &location,
                "role",
                boolean,
                declaration,
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operation.clone(),
                    boolean,
                    vec![
                        MemberTerm::bound("model", model),
                        MemberTerm::binding("replication", replication),
                    ],
                ),
            )?;
            roles.push((role.name.clone(), key));
        }
        Ok(roles)
    }

    /// The `replication` Group of the replicated role at `index` of
    /// `protocol`, over the object type `context` (FR-465 "Role"): its
    /// binder, named by the role, is at the level the `readable` binders'
    /// count fixes, and an `until` term is read over them and that binder.
    fn replication(
        &mut self,
        protocol: &ProtocolInput<'_>,
        index: usize,
        context: EffectiveId,
        readable: &[Binder],
        location: &Location,
    ) -> Result<BindingValue, CheckRefusal> {
        let unchecked = || fault(location, KeyFault::UncheckedProtocolFact);
        let role = protocol.checked.roles.get(index).ok_or_else(unchecked)?;
        let replicated = protocol
            .form
            .roles
            .get(index)
            .and_then(|role| role.replicated.as_ref())
            .ok_or_else(unchecked)?;
        let checked = protocol
            .checked
            .slots
            .roles
            .iter()
            .find(|checked| checked.role == index)
            .ok_or_else(unchecked)?;
        let semantic_type = self.binder_type(&ValueType::Reference(context), None, location)?;
        let parameter = self.parameter(&role.name, readable.len(), semantic_type, location)?;
        let mut members = vec![
            member(
                "population",
                self.text_literal(&replicated.population.name, location)?,
            ),
            member(
                "max",
                self.integer_literal(Integer::from(checked.max), location)?,
            ),
            member("binder", LeafTerm::reference(parameter)),
        ];
        match &checked.lifetime {
            CheckedLifetime::Workflow => {
                members.push(member("lifetime", self.text_literal("workflow", location)?));
            }
            CheckedLifetime::Scope(_) => {
                members.push(member("lifetime", self.text_literal("scope", location)?));
            }
            CheckedLifetime::Until(body) => {
                members.push(member("lifetime", self.text_literal("until", location)?));
                let mut slot_names: Vec<String> = protocol
                    .checked
                    .slots
                    .binders
                    .iter()
                    .take(readable.len())
                    .map(|binder| binder.name.clone())
                    .collect();
                slot_names.push(role.name.clone());
                let mut scope = readable.to_vec();
                scope.push(Binder {
                    slot: readable.len(),
                    parameter,
                    semantic_type,
                });
                let mut binders = Binders::new(&slot_names, &scope);
                members.push(member("until", self.expression(body.root(), &mut binders)?));
            }
        }
        Ok(BindingValue::Group(GroupTerm::new(members)))
    }

    /// The `channel` nodes of `protocol` (FR-465 "Channel") and the one
    /// `queue` node of each (FR-465 "Queue"), each set ascending by digest.
    /// A queue is declared by its channel's `capacity` and `overflow`, so it
    /// has no declaration and one `generated` occurrence at the channel's
    /// region.
    fn channel_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operations: &ChannelOperations,
        roles: &[(String, NodeKey)],
        readable: &[Binder],
        boolean: NodeKey,
    ) -> Result<ChannelNodes, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let mut named = Vec::with_capacity(protocol.checked.channels.len());
        let mut channels = Vec::with_capacity(protocol.checked.channels.len());
        let mut queues = Vec::with_capacity(protocol.checked.channels.len());
        for channel in &protocol.checked.channels {
            let location = protocol_location(name, ProtocolSite::Channel(&channel.name));
            let end = |role: &str| {
                roles
                    .iter()
                    .find(|(declared, _)| declared == role)
                    .map(|(_, key)| LeafTerm::reference(*key))
                    .ok_or_else(|| fault(&location, KeyFault::UncheckedProtocolFact))
            };
            let from = end(&channel.from)?;
            let to = end(&channel.to)?;
            let carries = self.type_node(&channel.carries, &location)?;
            let ordering = self.ordering(channel, carries, readable, &location)?;
            let delivery = self.text_literal(delivery_text(channel.delivery), &location)?;
            let declaration = vec![
                identifier(name, &location)?,
                identifier("channel", &location)?,
                identifier(&channel.name, &location)?,
            ];
            let key = self.protocol_member(
                &location,
                "channel",
                boolean,
                declaration,
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operations.channel.clone(),
                    boolean,
                    vec![
                        MemberTerm::bound("from", from),
                        MemberTerm::bound("to", to),
                        MemberTerm::bound("carries", LeafTerm::reference(carries)),
                        MemberTerm::binding("ordering", ordering),
                        MemberTerm::bound("delivery", delivery),
                    ],
                ),
            )?;
            channels.push(key);
            named.push((channel.name.clone(), key));
            queues.push(self.queue_node(operations, channel, key, boolean, &location)?);
        }
        Ok((sorted(channels), sorted(queues), named))
    }

    /// The `ordering` Group of `channel`: `unordered`, or `fifo` with the
    /// message binder, at the level the `readable` binders' count fixes, and
    /// the key read over that binder alone.
    fn ordering(
        &mut self,
        channel: &CheckedChannel,
        carries: NodeKey,
        readable: &[Binder],
        location: &Location,
    ) -> Result<BindingValue, CheckRefusal> {
        let kind = |this: &mut Self, text: &str| -> Result<GroupMember, CheckRefusal> {
            Ok(member("kind", this.text_literal(text, location)?))
        };
        match &channel.ordering {
            CheckedOrdering::Unordered => Ok(BindingValue::Group(GroupTerm::new(vec![kind(
                self,
                "unordered",
            )?]))),
            CheckedOrdering::Fifo { binder, key } => {
                let parameter = self.parameter(&binder.name, readable.len(), carries, location)?;
                let slot_names = [binder.name.clone()];
                let mut binders = Binders::new(
                    &slot_names,
                    &[Binder {
                        slot: 0,
                        parameter,
                        semantic_type: carries,
                    }],
                );
                let key = self.expression(key.root(), &mut binders)?;
                Ok(BindingValue::Group(GroupTerm::new(vec![
                    kind(self, "fifo")?,
                    member("binder", LeafTerm::reference(parameter)),
                    member("key", key),
                ])))
            }
        }
    }

    /// The `queue` node of `channel`, whose node is `key`.
    fn queue_node(
        &mut self,
        operations: &ChannelOperations,
        channel: &CheckedChannel,
        key: NodeKey,
        boolean: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let capacity = match &channel.capacity {
            ChannelCapacity::Literal(value) => GroupTerm::new(vec![
                member("kind", self.text_literal("literal", location)?),
                member(
                    "value",
                    self.integer_literal(Integer::from(*value), location)?,
                ),
            ]),
            ChannelCapacity::Symbolic(name) => GroupTerm::new(vec![
                member("kind", self.text_literal("symbolic", location)?),
                member("name", self.text_literal(name, location)?),
            ]),
        };
        let (overflow, loss) = match channel.overflow {
            ChannelOverflow::Reject => ("reject", None),
            ChannelOverflow::Block => ("block", None),
            ChannelOverflow::Loss(selector) => ("loss", Some(loss_text(selector))),
        };
        let loss = match loss {
            None => empty_group(),
            Some(selector) => BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(
                self.text_literal(selector, location)?,
            )])),
        };
        let overflow = self.text_literal(overflow, location)?;
        let queue = self.insert_node(
            location,
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Protocol,
                semantic_form: "queue",
                semantic_type: Some(boolean),
                declaration: None,
                owner: None,
                body: BodyTerm::application(
                    Operator::ProtocolControl,
                    operations.queue.clone(),
                    boolean,
                    vec![
                        MemberTerm::bound("channel", LeafTerm::reference(key)),
                        MemberTerm::binding("capacity", BindingValue::Group(capacity)),
                        MemberTerm::bound("overflow", overflow),
                        MemberTerm::binding("loss", loss),
                    ],
                ),
            },
        )?;
        self.record(queue, OccurrenceRole::Generated, location.clone());
        Ok(queue)
    }

    /// The `finish` obligation node of `protocol` (FR-465 "Finish"): its
    /// closing record binder, at the level `readable`'s length fixes, and its
    /// constraint, read over `readable` and that binder.
    fn finish_node(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        readable: &[Binder],
        boolean: NodeKey,
    ) -> Result<NodeKey, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let checked = protocol.checked;
        let (finish, finish_name) = checked
            .finish
            .as_ref()
            .ok_or_else(|| fault(&protocol.location(), KeyFault::UncheckedProtocolFact))?;
        let location = protocol_location(name, ProtocolSite::Finish(finish_name));
        let unchecked = || fault(&location, KeyFault::UncheckedProtocolFact);
        let (slot, closing) = checked
            .slots
            .binders
            .iter()
            .enumerate()
            .find(|(_, binder)| binder.node == Some(*finish))
            .ok_or_else(unchecked)?;
        let binder = self.protocol_binder(closing, slot, readable.len(), &location)?;
        let mut scope = readable.to_vec();
        scope.push(binder);
        let slot_names: Vec<String> = checked
            .slots
            .binders
            .iter()
            .map(|binder| binder.name.clone())
            .collect();
        let mut binders = Binders::new(&slot_names, &scope);
        let body = protocol.finish_body().ok_or_else(unchecked)?;
        let constraint = self.expression(body.root(), &mut binders)?;
        let declaration = vec![
            identifier(name, &location)?,
            identifier("finish", &location)?,
            identifier(finish_name, &location)?,
        ];
        self.protocol_member(
            &location,
            "obligation",
            boolean,
            declaration,
            None,
            BodyTerm::application(
                Operator::ProtocolControl,
                operation.clone(),
                boolean,
                vec![
                    MemberTerm::bound("binder", LeafTerm::reference(binder.parameter)),
                    MemberTerm::bound("constraint", constraint),
                ],
            ),
        )
    }

    /// The `terminal` obligation node of `protocol` (FR-465 "Terminal"), or
    /// `None` when it writes no `terminal` member: `mode` is `any` or
    /// `when`, and `condition` the checked predicate for `when` and an empty
    /// optional for `any`. The condition reads `over` and, under an `origin`
    /// activation, the protocol captures, and no other binder (QSpec FR-464
    /// stage `terminal`), so under `each` its scope is `over` alone.
    fn terminal_node(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        readable: &[Binder],
        trigger: Option<Binder>,
        boolean: NodeKey,
    ) -> Result<Option<NodeKey>, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let location = protocol_location(name, ProtocolSite::Terminal);
        let (mode, condition) = match &protocol.checked.slots.terminal {
            CheckedTerminalMember::None => return Ok(None),
            CheckedTerminalMember::Any => ("any", None),
            CheckedTerminalMember::When(body) => ("when", Some(body)),
        };
        let condition = match condition {
            None => MemberTerm::binding("condition", empty_group()),
            Some(body) => {
                let scope: Vec<Binder> = match trigger {
                    None => readable.to_vec(),
                    Some(_) => readable.iter().take(1).copied().collect(),
                };
                let slot_names: Vec<String> = protocol
                    .checked
                    .slots
                    .binders
                    .iter()
                    .map(|binder| binder.name.clone())
                    .collect();
                let mut binders = Binders::new(&slot_names, &scope);
                let term = self.expression(body.root(), &mut binders)?;
                MemberTerm::binding(
                    "condition",
                    BindingValue::Group(GroupTerm::new(vec![GroupMember::Leaf(term)])),
                )
            }
        };
        let declaration = vec![
            identifier(name, &location)?,
            identifier("terminal", &location)?,
        ];
        let mode = MemberTerm::bound("mode", self.text_literal(mode, &location)?);
        self.protocol_member(
            &location,
            "obligation",
            boolean,
            declaration,
            None,
            BodyTerm::application(
                Operator::ProtocolControl,
                operation.clone(),
                boolean,
                vec![mode, condition],
            ),
        )
        .map(Some)
    }

    /// The carrier of a reference to the `protocol`-th declaration: a
    /// `reference` leaf to its node, typed as that node is, with one
    /// `expression` occurrence at the reference.
    pub(super) fn protocol_reference(
        &mut self,
        node: CheckedNode<'_>,
        protocol: usize,
    ) -> Result<LeafTerm, CheckRefusal> {
        let target = self.protocol_key(protocol, node.location())?;
        let semantic_type = self.type_node(&ValueType::Boolean, node.location())?;
        let key = self.insert_node(
            node.location(),
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Protocol,
                semantic_form: "protocol_reference",
                semantic_type: Some(semantic_type),
                declaration: None,
                owner: None,
                body: BodyTerm::Leaf(LeafTerm::reference(target)),
            },
        )?;
        self.record(key, OccurrenceRole::Expression, node.location().clone());
        Ok(LeafTerm::reference(key))
    }

    /// The node key of the `protocol`-th declaration, or the refusal that
    /// the package selected no protocol profile for it to carry, or that it
    /// names a construct whose node is not written yet.
    pub(crate) fn protocol_key(
        &self,
        protocol: usize,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        match self.protocol_keys.get(protocol) {
            Some(ProtocolKey::Keyed(key)) => Ok(*key),
            Some(ProtocolKey::NotEmitted(construct)) => Err(not_emitted(*construct, location)),
            Some(ProtocolKey::AfterClauses) => Err(not_emitted(
                ProtocolConstruct::ReferenceToAttemptProtocol,
                location,
            )),
            Some(ProtocolKey::Unselected) | None => Err(refuse(
                location,
                CheckCause::MissingSelection {
                    role: LawRole::ProtocolProfile,
                },
            )),
        }
    }
}

/// What lowering holds for one protocol declaration.
#[derive(Clone, Copy, Debug)]
pub(super) enum ProtocolKey {
    /// The key of its `protocol_clause` node.
    Keyed(NodeKey),
    /// The package selects no protocol profile.
    Unselected,
    /// The protocol holds an `attempt`: its node is written once the state
    /// clauses it lists are ([`Lowering::protocol_nodes_with_attempts`]).
    AfterClauses,
    /// The profile is selected and the protocol names this construct, the
    /// first of those whose node is not written yet.
    NotEmitted(ProtocolConstruct),
}
