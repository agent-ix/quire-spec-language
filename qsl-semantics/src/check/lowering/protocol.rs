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
//! construct whose node is not written yet (a role, a channel, a
//! predicate-joined `parallel`, a post-test `repeat`, a `terminal` member, a
//! protocol reference written directly in a capture, guard, terminal, control
//! or `finish` body) is refused with that construct named
//! ([`ProtocolConstruct`]) rather than written without it: its package would
//! lack part of the graph its `package_id` stands for. A reference binds the
//! clause node's key and is itself a `{term: "reference", target}` leaf keyed
//! by `quire.structural-node/v1`.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    ActivationForm, ChannelCapacity, ChannelDelivery, ChannelOverflow, LossSelector,
    ProtocolDeclarationForm, ProtocolNodeKind,
};
use qsl_foundation::MemoryModelName;
use quire_exact::{EffectiveId, Identifier, Integer, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

mod controls;

use controls::RunInputs;
pub(crate) use controls::{AttemptNodes, CompensationNodes};

use super::{fault, refuse, Binder, Binders, Lowering, NodeContent, NodeRef};
use crate::check::family::OccurrenceRole;
use crate::check::ir::{CheckedBody, CheckedNode, NodeKind};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, MemberTerm,
    NodeTag, Operation, OperationLaw, Operator, TupleMember, TupleTerm,
};
use crate::check::protocol_clause::{CheckedProtocol, ProtocolNodeId};
use crate::check::protocol_controls::{CheckedChannel, CheckedOrdering};
use crate::check::protocol_obligation::{
    CaptureOwner, CaptureStage, CheckedActivation, CheckedCommit, CheckedCompensation,
};
use crate::check::protocol_slots::{
    CheckedLifetime, CheckedScheduling, CheckedSlotBinder, CheckedTerminalMember,
};
use crate::check::refinement::{
    KeyedCatalogClass, KeyedClassCatalog, KeyedMemoryClass, MemoryStepKind, RefinementCatalogClass,
    RefinementCatalogNode, RefinementClassCatalog,
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
    /// A compensation template's `declaration` occurrence.
    Compensation(&'a str),
    /// One expression body owned by a compensation template.
    CompensationBody(&'a str, &'static str, usize),
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
    /// A `visible` entry, guard, invariant or variant of the `choice` or
    /// `repeat` at this path, told apart by its source span.
    Decision(&'a [String], qsl_foundation::Span),
}

/// The `Origin::TypeDeclaration` name of `site` of protocol `protocol`.
/// Spaced, so it names no declared type.
pub(crate) fn protocol_site(protocol: &str, site: ProtocolSite<'_>) -> String {
    match site {
        ProtocolSite::Declaration => declaration_origin(protocol),
        ProtocolSite::Finish(name) => format!("protocol {protocol} finish {name}"),
        ProtocolSite::Compensation(name) => format!("protocol {protocol} compensate {name}"),
        ProtocolSite::CompensationBody(name, stage, index) => {
            format!("protocol {protocol} compensate {name} {stage} {index}")
        }
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
        ProtocolSite::Decision(path, span) => format!(
            "protocol {protocol} run {} decision {}..{}",
            path.join(" "),
            span.start,
            span.end
        ),
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
        let mut kinds: Vec<ProtocolNodeKind> = Vec::new();
        for control in &self.checked.controls.controls {
            // A `branch` and a `case` are held by the `parallel` and the
            // `choice` that hold them. A `parallel` is written whatever its
            // join: a predicate join's function is keyed before the
            // protocol (`dependencies`).
            let refused = match control.kind {
                ProtocolNodeKind::Parallel => !form
                    .parallels
                    .iter()
                    .any(|parallel| parallel.declaration == control.node.0),
                _ => false,
            };
            if refused && !kinds.contains(&control.kind) {
                kinds.push(control.kind);
                unemitted.push(ProtocolConstruct::Control(control.kind.label()));
            }
        }
        if self
            .body_roots()
            .iter()
            .any(|root| !root.protocol_references().is_empty())
        {
            unemitted.push(ProtocolConstruct::ProtocolReferenceInBody);
        }
        if slots.repeats.iter().any(|repeat| repeat.post_test) {
            unemitted.push(ProtocolConstruct::PostTestRepeat);
        }
        if slots
            .repeats
            .iter()
            .any(|repeat| repeat.decisions_read_body && !repeat.post_test)
        {
            unemitted.push(ProtocolConstruct::RepeatDecisionReadsBody);
        }
        if slots
            .choices
            .iter()
            .flat_map(|choice| &choice.visible)
            .chain(slots.repeats.iter().flat_map(|repeat| &repeat.visible))
            .any(|entry| {
                matches!(
                    entry.root().kind(),
                    NodeKind::Literal(_) | NodeKind::Local(_)
                )
            })
        {
            unemitted.push(ProtocolConstruct::VisibleEntryNotAnExpression);
        }
        if self.repeat_steps_through_repeat() {
            unemitted.push(ProtocolConstruct::RepeatStepsThroughRepeat);
        }
        if matches!(form.activation, ActivationForm::Each { .. })
            && matches!(&slots.terminal, CheckedTerminalMember::When(body) if body.reads_local(0))
        {
            unemitted.push(ProtocolConstruct::TerminalReadsOverUnderEach);
        }
        unemitted
    }

    /// Whether some `repeat`'s body does not necessarily step when no
    /// `repeat` counts as stepping: S3 admitted it through a nested repeat
    /// whose exit is infeasible (QSpec FR-464 "Nested `repeat` rule",
    /// STD-539). It is withheld until IR's reader decides that rule
    /// (IR-1100).
    fn repeat_steps_through_repeat(&self) -> bool {
        use ProtocolNodeKind as K;
        let tree = &self.checked.controls;
        let mut steps: BTreeMap<ProtocolNodeId, bool> = BTreeMap::new();
        // Children follow their holder in `controls`, so in reverse each
        // child is decided before its holder.
        for control in tree.controls.iter().rev() {
            let child = |index: usize| {
                control
                    .children
                    .get(index)
                    .and_then(|child| steps.get(child))
                    .copied()
                    .unwrap_or(false)
            };
            let necessarily = match control.kind {
                K::Send
                | K::Receive
                | K::Attempt
                | K::Effect
                | K::Event
                | K::Commit
                | K::Fence
                | K::Await
                | K::Parallel => true,
                K::Check | K::Repeat | K::Finish | K::CompensateTemplate => false,
                K::Branch | K::Case => child(0),
                K::Sequence => (0..control.children.len()).any(child),
                K::Choice => (0..control.children.len()).all(child),
            };
            steps.insert(control.node, necessarily);
        }
        self.form.repeats.iter().any(|repeat| {
            tree.control(ProtocolNodeId(repeat.declaration))
                .is_some_and(|control| {
                    !control.children.iter().any(|child| {
                        steps.get(child).copied().unwrap_or(false)
                            && self
                                .form
                                .declarations
                                .get(child.0)
                                .is_some_and(|declaration| {
                                    repeat.body.start <= declaration.name.span.start
                                        && declaration.name.span.end <= repeat.body.end
                                })
                    })
                })
        })
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

    /// The root of every expression tree the protocol's slots hold: its
    /// capture initializers (the protocol's and each compensation template's),
    /// each template's trigger guard, retry relation and recovery predicate,
    /// its `each` guard, its `terminal when`
    /// condition, and each control and `finish` body.
    fn body_roots(&self) -> Vec<CheckedNode<'_>> {
        let obligations = &self.checked.obligations;
        let mut roots: Vec<CheckedNode<'_>> = obligations
            .captures
            .iter()
            .map(|capture| capture.initializer.root())
            .collect();
        for template in &obligations.compensations {
            roots.push(template.guard.root());
            roots.push(template.retry.relation.root());
            roots.push(template.recover.predicate.root());
        }
        if let CheckedActivation::OnEach {
            guard: Some(guard), ..
        } = &obligations.activation
        {
            roots.push(guard.root());
        }
        if let CheckedTerminalMember::When(body) = &self.checked.slots.terminal {
            roots.push(body.root());
        }
        let slots = &self.checked.slots;
        roots.extend(slots.bodies.iter().map(|body| body.body.root()));
        // The `choice` and `repeat` decision slots (FR-466 "Decision facts").
        for choice in &slots.choices {
            roots.extend(choice.visible.iter().map(CheckedBody::root));
            roots.extend(choice.cases.iter().map(|case| case.guard.root()));
        }
        for repeat in &slots.repeats {
            roots.extend(repeat.visible.iter().map(CheckedBody::root));
            roots.push(repeat.guard.root());
            if let Some(proof) = &repeat.loop_proof {
                roots.push(proof.invariant.root());
                roots.push(proof.variant.root());
            }
        }
        roots
    }

    /// The functions this protocol's bodies call and the protocols they
    /// refer to, each by declaration index, ascending and once. Each is a
    /// node the protocol's own key hashes, so it is keyed before this
    /// protocol is (FR-093 `Call` row, FR-464 `protocol_reference`).
    pub(crate) fn dependencies(&self) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
        let roots = self.body_roots();
        let mut functions = Vec::new();
        let mut protocols = Vec::new();
        for root in roots {
            functions.extend(root.callees());
            protocols.extend(root.protocol_references());
        }
        // A predicate join's function is referenced by the `parallel`'s
        // `join` (FR-466 "Control layouts").
        functions.extend(
            self.checked
                .pending_predicate_joins
                .iter()
                .map(|join| join.function),
        );
        for list in [&mut functions, &mut protocols] {
            list.sort_unstable();
            list.dedup();
        }
        let mut requirements: Vec<usize> = self
            .checked
            .requirements
            .iter()
            .map(|requirement| requirement.temporal)
            .collect();
        requirements.sort_unstable();
        requirements.dedup();
        (functions, protocols, requirements)
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

/// A protocol's channel nodes and queue nodes, each set ascending by digest,
/// and each channel's node by its name.
type ChannelNodes = (Vec<NodeKey>, Vec<NodeKey>, Vec<(String, NodeKey)>);

/// The already-keyed nodes a protocol clause includes as set members.
struct ProtocolClauseMembers<'a> {
    attempts: &'a [AttemptNodes],
    compensations: &'a [CompensationNodes],
    requirements: &'a [NodeKey],
}

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
    /// Start the protocols' keys: none of the `count` declarations has a
    /// node yet, so a reference to one that is not keyed refuses.
    pub(crate) fn begin_protocols(&mut self, count: usize) {
        self.protocol_keys = vec![ProtocolKey::Unkeyed; count];
        self.protocol_attempts = vec![Vec::new(); count];
        self.protocol_compensations = vec![Vec::new(); count];
    }

    /// Record a temporal clause key for the protocols that require it.
    pub(crate) fn record_temporal_key(&mut self, index: usize, key: NodeKey) {
        self.temporal_keys.insert(index, key);
    }

    /// Record that the `index`-th temporal clause's lowering refused.
    pub(crate) fn record_temporal_refused(&mut self, index: usize) {
        self.refused_temporals.insert(index);
    }

    /// The keys of the temporal clauses `protocol` requires, in requirement
    /// order, or `None` when one of them was refused: that refusal is
    /// already reported at the clause, so the protocol is left unkeyed and
    /// adds none of its own. A required clause that is neither keyed nor
    /// refused is bound to a key no emitted node carries and refuses
    /// `missing_declaration`/`missing-name` at the protocol that requires it
    /// (FR-383-AC-17).
    fn required_temporal_keys(
        &self,
        protocol: &ProtocolInput<'_>,
        location: &Location,
    ) -> Result<Option<Vec<NodeKey>>, CheckRefusal> {
        let requirements = protocol.checked.requirements.iter();
        resolve_temporal_requirement_keys(
            requirements.map(|requirement| {
                (
                    requirement.temporal,
                    protocol
                        .form
                        .requirements
                        .iter()
                        .find(|written| written.temporal.span == requirement.span)
                        .map_or("", |written| written.temporal.name.as_str()),
                )
            }),
            &self.temporal_keys,
            &self.refused_temporals,
            location,
        )
    }

    /// Lower and key the `index`-th protocol, recording its one
    /// `declaration` occurrence. A package that selects no protocol profile
    /// has no `protocol_profile` law to carry (ADR-011 §2.4): no node is
    /// built and nothing is refused, so the unit is as it was before
    /// protocol nodes; a reference to a protocol refuses `missing-selection`.
    /// With a profile selected, a protocol that names a construct whose node
    /// is not written yet refuses at its declaration, once for each such
    /// construct (`unsupported_construct`/`not-yet-implemented`), and so
    /// does a reference to it. `cyclic` is whether the protocol belongs to a
    /// cyclic component of function calls and protocol references
    /// ([`ProtocolConstruct::ReferenceCycle`]). Every function and protocol
    /// the protocol's bodies name must be keyed first (FR-093): the caller
    /// lowers the unit in dependency order.
    pub(crate) fn protocol_node_step(
        &mut self,
        index: usize,
        protocol: &ProtocolInput<'_>,
        cyclic: bool,
    ) -> Result<Vec<CheckRefusal>, CheckRefusal> {
        let location = protocol.location();
        let law = match self.law(LawRole::ProtocolProfile, &location) {
            Ok(law) => law,
            Err(CheckRefusal {
                cause: CheckCause::MissingSelection { .. },
                ..
            }) => {
                self.set_protocol_key(index, ProtocolKey::Unselected);
                return Ok(Vec::new());
            }
            Err(other) => return Err(other),
        };
        let mut unemitted = protocol.unemitted();
        if cyclic {
            unemitted.push(ProtocolConstruct::ReferenceCycle);
        }
        if self.fifo_key_unstable(protocol, &location)? {
            unemitted.push(ProtocolConstruct::ChannelKeyNotEqualityStable);
        }
        let owner = self.owner_model_object(protocol);
        if owner.is_none() {
            unemitted.push(ProtocolConstruct::NoOwnerModel);
        }
        if let Some(first) = unemitted.first() {
            self.set_protocol_key(index, ProtocolKey::NotEmitted(*first));
            return Ok(unemitted
                .iter()
                .map(|construct| not_emitted(*construct, &location))
                .collect());
        }
        let Some(owner) = owner else {
            return Err(fault(&location, KeyFault::UncheckedProtocolFact));
        };
        if protocol.has_attempt() {
            self.set_protocol_key(index, ProtocolKey::AfterClauses);
            return Ok(Vec::new());
        }
        let Some(requirements) = self.required_temporal_keys(protocol, &location)? else {
            self.set_protocol_key(index, ProtocolKey::Unkeyed);
            return Ok(Vec::new());
        };
        match self.protocol_node_for(
            protocol,
            &law,
            owner,
            ProtocolClauseMembers {
                attempts: &[],
                compensations: &[],
                requirements: &requirements,
            },
            &location,
        ) {
            Ok(key) => {
                self.set_protocol_key(index, ProtocolKey::Keyed(key));
                Ok(Vec::new())
            }
            // A protocol this one refers to has no node (its refusal is
            // reported at its own declaration): this one has none either,
            // and the reference is refused where it stands.
            Err(refusal) => match &refusal.cause {
                CheckCause::ProtocolReference(cause) if cause.construct().is_some() => {
                    if let Some(construct) = cause.construct() {
                        self.set_protocol_key(index, ProtocolKey::NotEmitted(construct));
                    }
                    Ok(vec![refusal])
                }
                _ => Err(refusal),
            },
        }
    }

    /// Record the `index`-th protocol's key.
    fn set_protocol_key(&mut self, index: usize, key: ProtocolKey) {
        if let Some(slot) = self.protocol_keys.get_mut(index) {
            *slot = key;
        }
    }

    /// Retain attempt and compensation nodes and write clauses that hold an
    /// `attempt` once their state clauses and operation anchors are keyed.
    /// The mixed dependency order has already keyed temporal requirements.
    pub(crate) fn protocol_nodes_with_attempts(
        &mut self,
        protocols: &[ProtocolInput<'_>],
        attempts: &[Vec<AttemptNodes>],
        compensations: &[Vec<CompensationNodes>],
    ) -> Result<(), CheckRefusal> {
        for index in 0..protocols.len() {
            self.protocol_attempts[index] = attempts.get(index).cloned().unwrap_or_default();
            self.protocol_compensations[index] =
                compensations.get(index).cloned().unwrap_or_default();
        }
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
            let compensation_nodes = compensations.get(index).map_or(&[][..], Vec::as_slice);
            let Some(requirements) = self.required_temporal_keys(protocol, &location)? else {
                self.protocol_keys[index] = ProtocolKey::Unkeyed;
                continue;
            };
            let key = self.protocol_node_for(
                protocol,
                &law,
                owner,
                ProtocolClauseMembers {
                    attempts: nodes,
                    compensations: compensation_nodes,
                    requirements: &requirements,
                },
                &location,
            )?;
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
            .then_some(context.clone())
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
            if !visited.insert(key.clone()) {
                continue;
            }
            // A nominal enum is keyed by its declaration node, which is
            // written with the declarations and not by this type: a scalar
            // `enum` the reader compares.
            let Some(node) = self.graph.nodes.get(&key) else {
                continue;
            };
            let content = &node.content;
            match (content.node_tag, content.semantic_form) {
                (
                    NodeTag::ScalarType,
                    "boolean" | "integer" | "rational" | "decimal" | "text" | "string" | "uuid"
                    | "timestamp" | "enum",
                )
                | (NodeTag::CompositeType, "reference") => {}
                (NodeTag::CompositeType, _) => {
                    if content
                        .body
                        .for_each_key(&mut |child| pending.push(child))
                        .is_err()
                    {
                        return false;
                    }
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
                    | NodeTag::Hybrid
                    | NodeTag::TimedModel,
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
        let parameter = self.parameter(&binder.name, level, semantic_type.clone(), location)?;
        Ok(Binder {
            slot,
            parameter: parameter.into(),
            semantic_type: semantic_type.into(),
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
        let key = self.insert_content_key(
            location,
            NodeContent {
                owner_model,
                node_tag: NodeTag::Protocol,
                semantic_form,
                semantic_type: Some(boolean.into()),
                declaration: Some(declaration),
                owner: None,
                body,
            },
        )?;
        self.record(key.clone(), OccurrenceRole::Declaration, location.clone());
        Ok(key)
    }

    fn protocol_node_for(
        &mut self,
        protocol: &ProtocolInput<'_>,
        law: &OperationLaw,
        owner: EffectiveId,
        members: ProtocolClauseMembers<'_>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let previous = self
            .active_protocol
            .replace(protocol.form.name.name.clone());
        let result = self.protocol_node(protocol, law, owner, members, location);
        self.active_protocol = previous;
        result
    }

    /// The nodes of one protocol whose every part is written, and its
    /// `protocol_clause` node's key.
    fn protocol_node(
        &mut self,
        protocol: &ProtocolInput<'_>,
        law: &OperationLaw,
        owner: EffectiveId,
        members: ProtocolClauseMembers<'_>,
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
        let mut readable: Vec<Binder> = [Some(over.clone()), trigger.clone()]
            .into_iter()
            .flatten()
            .collect();
        let guard = self.activation_guard(protocol, over.clone(), trigger.clone())?;
        let captures = self.capture_records(protocol, &mut readable)?;

        let roles_by_name =
            self.role_nodes(protocol, &operation("role")?, &readable, boolean.clone())?;
        let (channels, queues, named_channels) = self.channel_nodes(
            protocol,
            &ChannelOperations {
                channel: operation("channel")?,
                queue: operation("queue")?,
            },
            &roles_by_name,
            &readable,
            boolean.clone(),
        )?;
        let run = self.run_nodes(
            protocol,
            &RunInputs {
                law,
                roles: &roles_by_name,
                channels: &named_channels,
                attempts: members.attempts,
                clause_binders: &readable,
                boolean: boolean.clone(),
            },
        )?;
        let roles: Vec<NodeKey> = sorted(roles_by_name.iter().map(|(_, key)| key.clone()));
        let compensations = self.compensation_nodes(
            protocol,
            &operation("compensate")?,
            &roles_by_name,
            &run.nodes,
            &readable,
            boolean.clone(),
            members.compensations,
        )?;
        let named_roles: BTreeMap<String, NodeKey> = roles_by_name.iter().cloned().collect();
        let finish = self.finish_node(
            protocol,
            &operation("finish")?,
            &run.readable,
            boolean.clone(),
        )?;
        let terminal = self.terminal_node(
            protocol,
            &operation("terminal")?,
            &readable,
            trigger.clone(),
            boolean.clone(),
        )?;
        let compensation_keys = compensations.clone();
        let terminal_key = terminal.clone();
        let finish_node = protocol.checked.finish.as_ref().ok_or_else(unchecked)?.0;
        let mut nodes = run.nodes;
        nodes.insert(finish_node, finish.clone());
        let run = run.root;

        let activation = match &trigger {
            None => GroupTerm::new(vec![member("kind", self.text_literal("origin", location)?)]),
            Some(trigger) => {
                let mut members = vec![
                    member("kind", self.text_literal("each", location)?),
                    member(
                        "trigger",
                        LeafTerm::reference_node(trigger.parameter.clone()),
                    ),
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
        let clause = self.protocol_member(
            location,
            "protocol_clause",
            boolean.clone(),
            vec![identifier(name, location)?],
            Some(owner_model),
            BodyTerm::application(
                Operator::ProtocolControl,
                operation("protocol")?,
                boolean,
                vec![
                    MemberTerm::bound("over", LeafTerm::reference_node(over.parameter)),
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
                    MemberTerm::binding(
                        "requirements",
                        BindingValue::Group(GroupTerm::references(sorted(
                            members.requirements.iter().cloned(),
                        ))),
                    ),
                    MemberTerm::binding(
                        "compensations",
                        BindingValue::Group(GroupTerm::references(compensations)),
                    ),
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
        )?;
        self.protocol_members.insert(
            name.to_owned(),
            ProtocolMembers {
                clause: clause.clone(),
                nodes,
                channels: named_channels.into_iter().collect(),
                roles: named_roles,
                compensations: compensation_keys,
                terminal: terminal_key,
                binders: self.protocol_binder_keys.remove(name).unwrap_or_default(),
                expressions: self
                    .protocol_expression_binders
                    .remove(name)
                    .unwrap_or_default(),
            },
        );
        Ok(clause)
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
            readable.push(binder.clone());
            records.push(TupleMember::Group(GroupTerm::new(vec![
                member("binder", LeafTerm::reference_node(binder.parameter)),
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
            let context = role.context.clone().ok_or_else(unchecked)?;
            let model = LeafTerm::reference(self.object_node(context.clone(), &location)?);
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
                boolean.clone(),
                declaration,
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operation.clone(),
                    boolean.clone(),
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
        let parameter =
            self.parameter(&role.name, readable.len(), semantic_type.clone(), location)?;
        let mut members = vec![
            member(
                "population",
                self.text_literal(&replicated.population.name, location)?,
            ),
            member(
                "max",
                self.integer_literal(Integer::from(checked.max), location)?,
            ),
            member("binder", LeafTerm::reference(parameter.clone())),
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
                    parameter: parameter.into(),
                    semantic_type: semantic_type.into(),
                });
                let mut binders = Binders::new(&slot_names, &scope);
                members.push(member("until", self.expression(body.root(), &mut binders)?));
            }
        }
        Ok(BindingValue::Group(GroupTerm::new(members)))
    }

    /// The compensation template nodes of `protocol` (FR-383-AC-8 and
    /// QSpec FR-466). Every checked stage is emitted from its checked body;
    /// each introduced binder is a parameter at its stage-local level.
    #[allow(clippy::too_many_arguments)]
    fn compensation_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        roles: &[(String, NodeKey)],
        controls: &std::collections::BTreeMap<
            crate::check::protocol_clause::ProtocolNodeId,
            NodeKey,
        >,
        readable: &[Binder],
        boolean: NodeKey,
        anchors: &[CompensationNodes],
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let mut keys = Vec::with_capacity(protocol.checked.obligations.compensations.len());
        for (index, checked) in protocol
            .checked
            .obligations
            .compensations
            .iter()
            .enumerate()
        {
            let location = protocol_location(name, ProtocolSite::Compensation(&checked.name));
            let unchecked = || fault(&location, KeyFault::UncheckedProtocolFact);
            let source = protocol
                .form
                .compensations
                .get(index)
                .ok_or_else(unchecked)?;
            let anchor = anchors
                .iter()
                .find(|anchor| anchor.declaration == index)
                .ok_or_else(unchecked)?
                .anchor
                .clone();
            let effect = controls
                .get(&checked.effect)
                .cloned()
                .ok_or_else(unchecked)?;
            let role = roles
                .get(checked.role)
                .map(|(_, key)| key.clone())
                .ok_or_else(unchecked)?;

            let mut binder_levels = BTreeMap::new();
            if let Some(input) = protocol.checked.slots.binders.first() {
                binder_levels.insert(input.name.clone(), 0);
            }
            let mut next_level = 1;
            if let CheckedActivation::OnEach { binder, .. } =
                &protocol.checked.obligations.activation
            {
                binder_levels.insert(binder.clone(), next_level);
                next_level += 1;
            }
            for (capture, _) in self.protocol_capture_entries(protocol) {
                binder_levels.insert(capture, next_level);
                next_level += 1;
            }
            binder_levels.insert(checked.forward.name.clone(), next_level);
            next_level += 1;
            for (capture, _) in self.capture_entries(protocol, &checked.registration_captures)? {
                binder_levels.insert(capture, next_level);
                next_level += 1;
            }
            binder_levels.insert(checked.trigger.name.clone(), next_level);
            next_level += 1;
            for (capture, _) in self.capture_entries(protocol, &checked.activation_captures)? {
                binder_levels.insert(capture, next_level);
                next_level += 1;
            }
            binder_levels.insert(checked.retry.earlier.name.clone(), next_level);
            next_level += 1;
            binder_levels.insert(checked.retry.later.name.clone(), next_level);
            next_level += 1;
            binder_levels.insert(checked.recover.binder.name.clone(), next_level);
            let mut binder_parameters = BTreeMap::new();
            for binder in readable {
                let declared = protocol
                    .checked
                    .slots
                    .binders
                    .get(binder.slot)
                    .ok_or_else(unchecked)?;
                binder_parameters.insert(
                    declared.name.clone(),
                    (binder.parameter.clone(), binder.semantic_type.clone()),
                );
            }

            let mut args = Vec::with_capacity(13);
            args.push(MemberTerm::bound("forward", LeafTerm::reference(effect)));
            let forward = self.compensation_scope(
                &[(
                    checked.forward.name.clone(),
                    checked.forward.value_type.clone(),
                )],
                &binder_levels,
                &mut binder_parameters,
                &location,
            )?;
            args.push(MemberTerm::bound(
                "forward_binder",
                LeafTerm::reference_node(forward.1[0].parameter.clone()),
            ));
            args.push(MemberTerm::bound("role", LeafTerm::reference(role)));
            args.push(MemberTerm::bound("operation", LeafTerm::reference(anchor)));
            args.push(MemberTerm::bound(
                "clock",
                self.text_literal(&source.clock.text, &location)?,
            ));

            let mut registration = Vec::with_capacity(checked.registration_captures.len());
            for (position, &capture_index) in checked.registration_captures.iter().enumerate() {
                let capture = protocol
                    .checked
                    .obligations
                    .captures
                    .get(capture_index)
                    .ok_or_else(unchecked)?;
                let mut entries = self.compensation_entries(
                    protocol,
                    checked,
                    CaptureStage::Registration,
                    position,
                )?;
                entries.push((capture.name.clone(), capture.declared_type.clone()));
                let scope = self.compensation_scope(
                    &entries,
                    &binder_levels,
                    &mut binder_parameters,
                    &location,
                )?;
                let initializer = self.expression(
                    capture.initializer.root(),
                    &mut Binders::new(&scope.0, &scope.1),
                )?;
                registration.push(TupleMember::Group(GroupTerm::new(vec![
                    member(
                        "binder",
                        LeafTerm::reference_node(
                            scope.1.last().ok_or_else(unchecked)?.parameter.clone(),
                        ),
                    ),
                    member("initializer", initializer),
                ])));
            }
            args.push(MemberTerm::binding(
                "registration_captures",
                BindingValue::Tuple(TupleTerm::new(registration)),
            ));

            let mut trigger_entries = vec![(
                checked.trigger.name.clone(),
                checked.trigger.value_type.clone(),
            )];
            trigger_entries.extend(self.protocol_capture_entries(protocol));
            trigger_entries.extend(self.capture_entries(protocol, &checked.registration_captures)?);
            let trigger_scope = self.compensation_scope(
                &trigger_entries,
                &binder_levels,
                &mut binder_parameters,
                &location,
            )?;
            let guard = self.expression(
                checked.guard.root(),
                &mut Binders::new(&trigger_scope.0, &trigger_scope.1),
            )?;
            args.push(MemberTerm::binding(
                "trigger",
                BindingValue::Group(GroupTerm::new(vec![
                    member(
                        "binder",
                        LeafTerm::reference_node(trigger_scope.1[0].parameter.clone()),
                    ),
                    member("guard", guard),
                ])),
            ));

            let mut activation = Vec::with_capacity(checked.activation_captures.len());
            for (position, &capture_index) in checked.activation_captures.iter().enumerate() {
                let capture = protocol
                    .checked
                    .obligations
                    .captures
                    .get(capture_index)
                    .ok_or_else(unchecked)?;
                let mut entries = vec![(
                    checked.trigger.name.clone(),
                    checked.trigger.value_type.clone(),
                )];
                entries.extend(self.protocol_input_entries(protocol));
                entries.extend(self.protocol_input_entries(protocol));
                entries.extend(self.protocol_capture_entries(protocol));
                entries.extend(self.capture_entries(protocol, &checked.registration_captures)?);
                entries.extend(
                    self.capture_entries(protocol, &checked.activation_captures[..position])?,
                );
                entries.push((capture.name.clone(), capture.declared_type.clone()));
                let scope = self.compensation_scope(
                    &entries,
                    &binder_levels,
                    &mut binder_parameters,
                    &location,
                )?;
                let initializer = self.expression(
                    capture.initializer.root(),
                    &mut Binders::new(&scope.0, &scope.1),
                )?;
                activation.push(TupleMember::Group(GroupTerm::new(vec![
                    member(
                        "binder",
                        LeafTerm::reference_node(
                            scope.1.last().ok_or_else(unchecked)?.parameter.clone(),
                        ),
                    ),
                    member("initializer", initializer),
                ])));
            }
            args.push(MemberTerm::binding(
                "activation_captures",
                BindingValue::Tuple(TupleTerm::new(activation)),
            ));
            args.push(MemberTerm::binding(
                "within",
                BindingValue::Group(GroupTerm::new(vec![
                    member(
                        "lower",
                        self.integer_literal(Integer::from(checked.within.lower), &location)?,
                    ),
                    member(
                        "upper",
                        self.integer_literal(Integer::from(checked.within.upper), &location)?,
                    ),
                ])),
            ));
            let attempt_type = self.type_node(&checked.attempt_type, &location)?;
            args.push(MemberTerm::binding(
                "attempts",
                BindingValue::Group(GroupTerm::new(vec![
                    member(
                        "maximum",
                        self.integer_literal(Integer::from(checked.maximum_attempts), &location)?,
                    ),
                    member("record_type", LeafTerm::reference(attempt_type)),
                ])),
            ));

            let mut retry_entries = vec![
                (
                    checked.retry.earlier.name.clone(),
                    checked.retry.earlier.value_type.clone(),
                ),
                (
                    checked.retry.later.name.clone(),
                    checked.retry.later.value_type.clone(),
                ),
            ];
            retry_entries.extend(self.protocol_capture_entries(protocol));
            retry_entries.extend(self.capture_entries(protocol, &checked.registration_captures)?);
            retry_entries.extend(self.capture_entries(protocol, &checked.activation_captures)?);
            let retry_scope = self.compensation_scope(
                &retry_entries,
                &binder_levels,
                &mut binder_parameters,
                &location,
            )?;
            let relation = self.expression(
                checked.retry.relation.root(),
                &mut Binders::new(&retry_scope.0, &retry_scope.1),
            )?;
            args.push(MemberTerm::binding(
                "retry",
                BindingValue::Group(GroupTerm::new(vec![
                    member(
                        "earlier",
                        LeafTerm::reference_node(retry_scope.1[0].parameter.clone()),
                    ),
                    member(
                        "later",
                        LeafTerm::reference_node(retry_scope.1[1].parameter.clone()),
                    ),
                    member("relation", relation),
                ])),
            ));
            let commit = match checked.commit {
                CheckedCommit::Never => empty_group(),
                CheckedCommit::Node(node) => BindingValue::Group(GroupTerm::references([controls
                    .get(&node)
                    .cloned()
                    .ok_or_else(unchecked)?])),
            };
            args.push(MemberTerm::binding("commit", commit));
            let mut recovery_entries = vec![(
                checked.recover.binder.name.clone(),
                checked.recover.binder.value_type.clone(),
            )];
            recovery_entries.extend(self.protocol_capture_entries(protocol));
            recovery_entries
                .extend(self.capture_entries(protocol, &checked.registration_captures)?);
            recovery_entries.extend(self.capture_entries(protocol, &checked.activation_captures)?);
            let recovery_scope = self.compensation_scope(
                &recovery_entries,
                &binder_levels,
                &mut binder_parameters,
                &location,
            )?;
            let predicate = self.expression(
                checked.recover.predicate.root(),
                &mut Binders::new(&recovery_scope.0, &recovery_scope.1),
            )?;
            args.push(MemberTerm::binding(
                "recovery",
                BindingValue::Group(GroupTerm::new(vec![
                    member(
                        "binder",
                        LeafTerm::reference_node(recovery_scope.1[0].parameter.clone()),
                    ),
                    member("predicate", predicate),
                ])),
            ));
            let key = self.protocol_member(
                &location,
                "compensation",
                boolean.clone(),
                vec![
                    identifier(name, &location)?,
                    identifier("compensate", &location)?,
                    identifier(&checked.name, &location)?,
                ],
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operation.clone(),
                    boolean.clone(),
                    args,
                ),
            )?;
            keys.push(key);
        }
        Ok(sorted(keys))
    }

    fn compensation_scope(
        &mut self,
        entries: &[(String, ValueType)],
        levels: &BTreeMap<String, usize>,
        parameters: &mut BTreeMap<String, (NodeRef, NodeRef)>,
        location: &Location,
    ) -> Result<(Vec<String>, Vec<Binder>), CheckRefusal> {
        let names = entries
            .iter()
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let mut binders = Vec::with_capacity(entries.len());
        for (slot, (name, value_type)) in entries.iter().enumerate() {
            let semantic_type = self.binder_type(value_type, None, location)?;
            let level = *levels
                .get(name)
                .ok_or_else(|| fault(location, KeyFault::UncheckedProtocolFact))?;
            let parameter = if let Some((parameter, existing_type)) = parameters.get(name) {
                if *existing_type != NodeRef::from(semantic_type.clone()) {
                    return Err(fault(location, KeyFault::UncheckedProtocolFact));
                }
                parameter.clone()
            } else {
                let parameter: NodeRef = self
                    .parameter(name, level, semantic_type.clone(), location)?
                    .into();
                parameters.insert(
                    name.clone(),
                    (parameter.clone(), semantic_type.clone().into()),
                );
                parameter
            };
            binders.push(Binder {
                slot,
                parameter,
                semantic_type: semantic_type.into(),
            });
        }
        Ok((names, binders))
    }

    fn protocol_capture_entries(&self, protocol: &ProtocolInput<'_>) -> Vec<(String, ValueType)> {
        protocol
            .checked
            .obligations
            .captures
            .iter()
            .filter(|capture| capture.owner == CaptureOwner::Protocol)
            .map(|capture| (capture.name.clone(), capture.declared_type.clone()))
            .collect()
    }

    fn protocol_input_entries(&self, protocol: &ProtocolInput<'_>) -> Vec<(String, ValueType)> {
        protocol
            .checked
            .slots
            .binders
            .first()
            .map(|input| vec![(input.name.clone(), input.value_type.clone())])
            .unwrap_or_default()
    }

    fn capture_entries(
        &self,
        protocol: &ProtocolInput<'_>,
        indices: &[usize],
    ) -> Result<Vec<(String, ValueType)>, CheckRefusal> {
        indices
            .iter()
            .map(|&index| {
                let capture = protocol
                    .checked
                    .obligations
                    .captures
                    .get(index)
                    .ok_or_else(|| fault(&protocol.location(), KeyFault::UncheckedProtocolFact))?;
                Ok((capture.name.clone(), capture.declared_type.clone()))
            })
            .collect()
    }

    fn compensation_entries(
        &self,
        protocol: &ProtocolInput<'_>,
        checked: &CheckedCompensation,
        stage: CaptureStage,
        position: usize,
    ) -> Result<Vec<(String, ValueType)>, CheckRefusal> {
        let mut entries = Vec::new();
        match stage {
            CaptureStage::Registration => {
                entries.push((
                    checked.forward.name.clone(),
                    checked.forward.value_type.clone(),
                ));
                if let Some(input) = protocol.checked.slots.binders.first() {
                    entries.push((input.name.clone(), input.value_type.clone()));
                }
                if let CheckedActivation::OnEach {
                    binder,
                    record_type,
                    ..
                } = &protocol.checked.obligations.activation
                {
                    entries.push((binder.clone(), (**record_type).clone()));
                }
                entries.extend(self.protocol_capture_entries(protocol));
                entries.extend(
                    self.capture_entries(protocol, &checked.registration_captures[..position])?,
                );
            }
            CaptureStage::Activation => {
                entries.push((
                    checked.trigger.name.clone(),
                    checked.trigger.value_type.clone(),
                ));
                entries.extend(self.protocol_capture_entries(protocol));
                entries.extend(self.capture_entries(protocol, &checked.registration_captures)?);
                entries.extend(
                    self.capture_entries(protocol, &checked.activation_captures[..position])?,
                );
            }
            CaptureStage::Protocol => {}
        }
        Ok(entries)
    }

    /// The `channel` nodes of `protocol` (FR-465 "Channel") by channel
    /// name, and the one `queue` node of each
    /// (FR-465 "Queue"), ascending by digest.
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
                    .map(|(_, key)| LeafTerm::reference(key.clone()))
                    .ok_or_else(|| fault(&location, KeyFault::UncheckedProtocolFact))
            };
            let from = end(&channel.from)?;
            let to = end(&channel.to)?;
            let carries = self.type_node(&channel.carries, &location)?;
            let ordering = self.ordering(channel, carries.clone(), readable, &location)?;
            let delivery = self.text_literal(delivery_text(channel.delivery), &location)?;
            let declaration = vec![
                identifier(name, &location)?,
                identifier("channel", &location)?,
                identifier(&channel.name, &location)?,
            ];
            let key = self.protocol_member(
                &location,
                "channel",
                boolean.clone(),
                declaration,
                None,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operations.channel.clone(),
                    boolean.clone(),
                    vec![
                        MemberTerm::bound("from", from),
                        MemberTerm::bound("to", to),
                        MemberTerm::bound("carries", LeafTerm::reference(carries)),
                        MemberTerm::binding("ordering", ordering),
                        MemberTerm::bound("delivery", delivery),
                    ],
                ),
            )?;
            channels.push(key.clone());
            named.push((channel.name.clone(), key.clone()));
            queues.push(self.queue_node(operations, channel, key, boolean.clone(), &location)?);
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
                let parameter =
                    self.parameter(&binder.name, readable.len(), carries.clone(), location)?;
                let slot_names = [binder.name.clone()];
                let mut binders = Binders::new(
                    &slot_names,
                    &[Binder {
                        slot: 0,
                        parameter: parameter.clone().into(),
                        semantic_type: carries.into(),
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
        let queue = self.insert_content_key(
            location,
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Protocol,
                semantic_form: "queue",
                semantic_type: Some(boolean.clone().into()),
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
        self.record(queue.clone(), OccurrenceRole::Generated, location.clone());
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
        scope.push(binder.clone());
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
            boolean.clone(),
            declaration,
            None,
            BodyTerm::application(
                Operator::ProtocolControl,
                operation.clone(),
                boolean,
                vec![
                    MemberTerm::bound("binder", LeafTerm::reference_node(binder.parameter)),
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
                    Some(_) => readable.iter().take(1).cloned().collect(),
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
            boolean.clone(),
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
        let key = self.insert_content_key(
            node.location(),
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Protocol,
                semantic_form: "protocol_reference",
                semantic_type: Some(semantic_type.into()),
                declaration: None,
                owner: None,
                body: BodyTerm::Leaf(LeafTerm::reference(target)),
            },
        )?;
        self.record(
            key.clone(),
            OccurrenceRole::Expression,
            node.location().clone(),
        );
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
            Some(ProtocolKey::Keyed(key)) => Ok(key.clone()),
            Some(ProtocolKey::NotEmitted(construct)) => Err(not_emitted(*construct, location)),
            Some(ProtocolKey::Unkeyed) => Err(refuse(
                location,
                CheckCause::UnsupportedFeature {
                    loci: vec![location.clone()],
                },
            )),
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

fn resolve_temporal_requirement_keys<'n>(
    requirements: impl IntoIterator<Item = (usize, &'n str)>,
    temporal_keys: &BTreeMap<usize, NodeKey>,
    refused: &BTreeSet<usize>,
    location: &Location,
) -> Result<Option<Vec<NodeKey>>, CheckRefusal> {
    let mut keys = Vec::new();
    let mut any_refused = false;
    for (requirement, name) in requirements {
        match temporal_keys.get(&requirement) {
            Some(key) => keys.push(key.clone()),
            None if refused.contains(&requirement) => any_refused = true,
            None => return Err(refuse(location, CheckCause::MissingName(name.to_owned()))),
        }
    }
    Ok((!any_refused).then_some(keys))
}

impl Lowering<'_> {
    /// QSpec FR-375's class catalog of a refinement, lowered to node keys:
    /// an operation class names its owner's model node, a protocol class
    /// the node of its clause member (the clause for `activate`), and each
    /// binder type its type node. `None` when a class belongs to a protocol
    /// clause that has no node (its profile is unselected or it names a
    /// construct whose node is not written yet), so the refinement is not
    /// emitted.
    pub(crate) fn class_catalog_keys(
        &mut self,
        catalog: &RefinementClassCatalog,
        location: &Location,
    ) -> Result<Option<KeyedClassCatalog>, CheckRefusal> {
        let mut sides = [Vec::new(), Vec::new()];
        for (side, classes) in sides
            .iter_mut()
            .zip([catalog.concrete(), catalog.abstract_side()])
        {
            for class in classes {
                let Some(keyed) = self.catalog_class_keys(class, location)? else {
                    return Ok(None);
                };
                side.push(keyed);
            }
        }
        let [concrete, abstract_side] = sides;
        Ok(Some(KeyedClassCatalog {
            concrete,
            abstract_side,
        }))
    }

    /// Record `key`'s one `generated` occurrence at `location` when no
    /// declaration has located it: a catalog's class node or binder type
    /// that nothing else names.
    fn generated_at(&mut self, key: NodeKey, location: &Location) {
        if !self.occurrences.has(key.clone()) {
            self.record(key, OccurrenceRole::Generated, location.clone());
        }
    }

    fn catalog_class_keys(
        &mut self,
        class: &RefinementCatalogClass,
        location: &Location,
    ) -> Result<Option<KeyedCatalogClass>, CheckRefusal> {
        if let Some(model) = class.memory_model() {
            // A memory class keys on its typed step kind (FR-375-AC-23,
            // #1718), matched exhaustively: `flush` is a `tso` class only.
            let memory = match (class.memory_step(), model) {
                (Some(MemoryStepKind::Drain), model) => KeyedMemoryClass::Drain(model),
                (Some(MemoryStepKind::Flush), MemoryModelName::Tso) => KeyedMemoryClass::Flush,
                (Some(MemoryStepKind::Flush), MemoryModelName::Sc | MemoryModelName::Ra)
                | (None, _) => {
                    return Err(catalog_fault(
                        location,
                        "catalog-memory-class-has-its-step-kind",
                    ))
                }
            };
            return Ok(Some(KeyedCatalogClass::Memory(memory)));
        }
        let node = match class.node() {
            Some(RefinementCatalogNode::Operation(owner)) => {
                let key = self.model_node(owner, location)?;
                self.generated_at(key.clone(), location);
                key
            }
            Some(RefinementCatalogNode::ProtocolNode { protocol, node }) => {
                match self
                    .protocol_members
                    .get(protocol)
                    .and_then(|members| members.nodes.get(node))
                {
                    Some(key) => key.clone(),
                    None => return Ok(None),
                }
            }
            Some(RefinementCatalogNode::Channel { protocol, channel }) => {
                match self
                    .protocol_members
                    .get(protocol)
                    .and_then(|members| members.channels.get(channel))
                {
                    Some(key) => key.clone(),
                    None => return Ok(None),
                }
            }
            Some(RefinementCatalogNode::Role { protocol, role }) => {
                match self
                    .protocol_members
                    .get(protocol)
                    .and_then(|members| members.roles.get(role))
                {
                    Some(key) => key.clone(),
                    None => return Ok(None),
                }
            }
            Some(RefinementCatalogNode::Trigger(protocol)) => {
                match self.protocol_members.get(protocol) {
                    Some(members) => members.clause.clone(),
                    None => return Ok(None),
                }
            }
            None => return Err(catalog_fault(location, "declared-catalog-class-has-node")),
        };
        let binder_types = class
            .binder_types()
            .iter()
            .map(|value_type| {
                let key = self.type_node(value_type, location)?;
                self.generated_at(key.clone(), location);
                Ok(key)
            })
            .collect::<Result<_, CheckRefusal>>()?;
        Ok(Some(KeyedCatalogClass::Declared {
            class: class.class().to_owned(),
            node,
            binder_types,
        }))
    }
}

/// A broken invariant of the checked class catalog.
fn catalog_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
            "check.refinement-class-catalog",
            invariant,
        )),
    )
}

/// The keys of one keyed protocol clause's members that own a step class
/// (FR-432): the clause itself (`activate`), its channels and roles by name,
/// and its step nodes by declaration index: every control the `run` tree
/// writes and the `finish` node. A clause holding a control that is not
/// written yet (a predicate-joined `parallel`, a post-test `repeat`) has no
/// node, so a refinement over it stays not emitted.
#[derive(Clone, Debug)]
pub(crate) struct ProtocolMembers {
    pub(crate) clause: NodeKey,
    pub(crate) nodes: BTreeMap<ProtocolNodeId, NodeKey>,
    pub(crate) channels: BTreeMap<String, NodeKey>,
    pub(crate) roles: BTreeMap<String, NodeKey>,
    pub(crate) compensations: Vec<NodeKey>,
    pub(crate) terminal: Option<NodeKey>,
    pub(crate) binders: Vec<super::super::protocol_clause::CheckedProtocolBinderKey>,
    pub(crate) expressions: Vec<super::super::protocol_clause::CheckedProtocolExpressionBindings>,
}

/// What lowering holds for one protocol declaration.
#[derive(Clone, Debug)]
pub(super) enum ProtocolKey {
    /// The key of its `protocol_clause` node.
    Keyed(NodeKey),
    /// The package selects no protocol profile.
    Unselected,
    /// Its node is not keyed yet, or its lowering was refused and the
    /// refusal is reported.
    Unkeyed,
    /// The protocol holds an `attempt`: its node is written once the state
    /// clauses it lists are ([`Lowering::protocol_nodes_with_attempts`]).
    AfterClauses,
    /// The profile is selected and the protocol names this construct, the
    /// first of those whose node is not written yet.
    NotEmitted(ProtocolConstruct),
}

#[cfg(test)]
mod requirement_key_tests {
    use super::*;

    fn location() -> Location {
        Location::root(Origin::Expression)
    }

    /// FR-383-AC-17: a required clause no emitted node carries refuses
    /// `missing-name` naming the clause, never an internal fault.
    #[ix_trace_rs::trace("FR-383-AC-17", "TC-1063")]
    #[test]
    fn a_required_temporal_without_a_key_is_missing_name() {
        let Err(refusal) = resolve_temporal_requirement_keys(
            [(7, "Due")],
            &BTreeMap::new(),
            &BTreeSet::new(),
            &location(),
        ) else {
            panic!("an unkeyed, unrefused required clause refuses");
        };
        assert_eq!(refusal.cause, CheckCause::MissingName("Due".to_owned()));
    }

    /// A required clause whose own lowering refused leaves the protocol
    /// unkeyed without a second refusal.
    #[ix_trace_rs::trace("FR-383-AC-17", "TC-1063")]
    #[test]
    fn a_refused_required_temporal_adds_no_refusal() {
        let keys = BTreeMap::from([(1, NodeKey::from_digest([1; 32]))]);
        let refused = BTreeSet::from([7]);
        let resolved =
            resolve_temporal_requirement_keys([(1, "A"), (7, "Due")], &keys, &refused, &location());
        assert!(matches!(resolved, Ok(None)), "{resolved:?}");
        let resolved = resolve_temporal_requirement_keys([(1, "A")], &keys, &refused, &location());
        assert!(matches!(resolved, Ok(Some(found)) if found == [NodeKey::from_digest([1; 32])]));
    }
}
