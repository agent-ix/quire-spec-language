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
//! (`value`/`parameter`), the `run` tree's `sequence` controls (declared at
//! `P`, `run` and the control names from the root down, FR-466) and the
//! `finish` obligation (declared at `P`, `finish` and its name, FR-465).
//!
//! What this writes is what QSL can prove IR admits. A protocol that names a
//! construct whose node is not written yet (a role, a channel, a control
//! other than `sequence`, a capture, a `terminal` member, a requirement, a
//! compensation) is refused with that construct named
//! ([`ProtocolConstruct`]) rather than written without it: its package would
//! lack part of the graph its `package_id` stands for. A reference binds the
//! clause node's key and is itself a `{term: "reference", target}` leaf keyed
//! by `quire.structural-node/v1`.

use std::collections::BTreeMap;

use qsl_forms::{ActivationForm, ProtocolDeclarationForm, ProtocolNodeKind};
use quire_exact::{Identifier, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{fault, refuse, Binder, Binders, Lowering, NodeContent};
use crate::check::family::OccurrenceRole;
use crate::check::ir::{CheckedNode, NodeKind};
use crate::check::node_key::{
    Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, MemberTerm,
    NodeTag, Operation, OperationLaw, Operator, TupleTerm,
};
use crate::check::protocol_clause::{CheckedProtocol, ProtocolNodeId};
use crate::check::protocol_obligation::CheckedActivation;
use crate::check::protocol_slots::{CheckedScheduling, CheckedSlotBinder, CheckedTerminalMember};
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
    /// The role of this name: its `declaration` occurrence.
    Role(&'a str),
    /// The control at this path from the `run` root, the root's name first:
    /// its `declaration` occurrence.
    Control(&'a [String]),
}

/// The `Origin::TypeDeclaration` name of `site` of protocol `protocol`.
/// Spaced, so it names no declared type.
pub(crate) fn protocol_site(protocol: &str, site: ProtocolSite<'_>) -> String {
    match site {
        ProtocolSite::Declaration => declaration_origin(protocol),
        ProtocolSite::Finish(name) => format!("protocol {protocol} finish {name}"),
        ProtocolSite::Role(role) => format!("protocol {protocol} role {role}"),
        ProtocolSite::FinishBody(name) => format!("protocol {protocol} finish {name} body"),
        ProtocolSite::Control(path) => format!("protocol {protocol} run {}", path.join(" ")),
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
        if form.roles.iter().any(|role| role.replicated.is_some()) {
            unemitted.push(ProtocolConstruct::ReplicatedRoles);
        }
        if self
            .checked
            .roles
            .iter()
            .any(|role| !role.replicated && role.context.is_none())
        {
            unemitted.push(ProtocolConstruct::WholeModelRole);
        }
        if !form.channels.is_empty() {
            unemitted.push(ProtocolConstruct::Channels);
        }
        if !form.relationships.is_empty() {
            unemitted.push(ProtocolConstruct::Relationships);
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
            let held = matches!(
                control.kind,
                ProtocolNodeKind::Sequence | ProtocolNodeKind::Branch | ProtocolNodeKind::Case
            );
            if !held && !kinds.contains(&control.kind) {
                kinds.push(control.kind);
                unemitted.push(ProtocolConstruct::Control(control.kind.label()));
            }
        }
        if !form.captures.is_empty() {
            unemitted.push(ProtocolConstruct::Captures);
        }
        if matches!(
            self.checked.obligations.activation,
            CheckedActivation::OnEach { guard: Some(_), .. }
        ) {
            unemitted.push(ProtocolConstruct::ActivationGuard);
        }
        if !matches!(slots.terminal, CheckedTerminalMember::None) {
            unemitted.push(ProtocolConstruct::Terminal);
        }
        if self
            .finish_body()
            .is_some_and(|body| reads_declaration(body.root()))
        {
            unemitted.push(ProtocolConstruct::FinishReadsDeclaration);
        }
        unemitted
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
            let unemitted = protocol.unemitted();
            if let Some(first) = unemitted.first() {
                refusals.extend(
                    unemitted
                        .iter()
                        .map(|construct| not_emitted(*construct, &location)),
                );
                keys.push(ProtocolKey::NotEmitted(*first));
                continue;
            }
            let key = self.protocol_node(protocol, &law, &location)?;
            keys.push(ProtocolKey::Keyed(key));
        }
        self.protocol_keys = keys;
        Ok(refusals)
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
        body: BodyTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        let key = self.insert_node(
            location,
            NodeContent {
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
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let slots = &protocol.checked.slots;
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
        let readable: Vec<Binder> = [Some(over), trigger].into_iter().flatten().collect();

        let roles = self.role_nodes(protocol, &operation("role")?, boolean)?;
        let finish = self.finish_node(protocol, &operation("finish")?, &readable, boolean)?;
        let run = self.run_nodes(protocol, &operation("sequence")?, boolean)?;

        let activation = match &trigger {
            None => GroupTerm::new(vec![member("kind", self.text_literal("origin", location)?)]),
            Some(trigger) => GroupTerm::new(vec![
                member("kind", self.text_literal("each", location)?),
                member("trigger", LeafTerm::reference(trigger.parameter)),
            ]),
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
            BodyTerm::application(
                Operator::ProtocolControl,
                operation("protocol")?,
                boolean,
                vec![
                    MemberTerm::bound("over", LeafTerm::reference(over.parameter)),
                    MemberTerm::binding("activation", BindingValue::Group(activation)),
                    MemberTerm::binding(
                        "captures",
                        BindingValue::Tuple(TupleTerm::new(Vec::new())),
                    ),
                    MemberTerm::binding("roles", BindingValue::Group(GroupTerm::references(roles))),
                    MemberTerm::binding("relationships", empty_group()),
                    MemberTerm::binding("channels", empty_group()),
                    MemberTerm::binding("queues", empty_group()),
                    MemberTerm::binding("requirements", empty_group()),
                    MemberTerm::binding("compensations", empty_group()),
                    MemberTerm::binding("terminal", empty_group()),
                    MemberTerm::binding("scheduling", scheduling),
                    MemberTerm::bound("run", LeafTerm::reference(run)),
                    MemberTerm::bound("finish", LeafTerm::reference(finish)),
                ],
            ),
        )
    }

    /// The static `role` nodes of `protocol` (FR-465 "Role"), ascending by
    /// node digest (FR-464 "Sets"). A role on a model type refers to that
    /// type's model node. Its `replication` Group is empty. A role on a whole
    /// model is refused before this ([`ProtocolConstruct::WholeModelRole`]).
    fn role_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        boolean: NodeKey,
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let mut keys = Vec::with_capacity(protocol.checked.roles.len());
        for role in &protocol.checked.roles {
            let location = protocol_location(name, ProtocolSite::Role(&role.name));
            let context = role
                .context
                .ok_or_else(|| fault(&location, KeyFault::UncheckedProtocolFact))?;
            let model = LeafTerm::reference(self.object_node(context, &location)?);
            let declaration = vec![
                identifier(name, &location)?,
                identifier("role", &location)?,
                identifier(&role.name, &location)?,
            ];
            keys.push(self.protocol_member(
                &location,
                "role",
                boolean,
                declaration,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operation.clone(),
                    boolean,
                    vec![
                        MemberTerm::bound("model", model),
                        MemberTerm::binding("replication", empty_group()),
                    ],
                ),
            )?);
        }
        keys.sort();
        Ok(keys)
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

    /// The `sequence` control nodes of `protocol`'s `run` tree (FR-466
    /// "sequence"), each declared at `P`, `run` and the names from the root
    /// down, and the root's key. A control is keyed after its children, so
    /// the tree, which is in tree order, is read backwards.
    fn run_nodes(
        &mut self,
        protocol: &ProtocolInput<'_>,
        operation: &Operation,
        boolean: NodeKey,
    ) -> Result<NodeKey, CheckRefusal> {
        let name = protocol.form.name.name.as_str();
        let tree = &protocol.checked.controls;
        let mut keys: BTreeMap<ProtocolNodeId, NodeKey> = BTreeMap::new();
        for control in tree.controls.iter().rev() {
            let location = protocol_location(name, ProtocolSite::Control(&control.path));
            let unchecked = || fault(&location, KeyFault::UncheckedProtocolFact);
            if control.kind != ProtocolNodeKind::Sequence {
                return Err(unchecked());
            }
            let children = control
                .children
                .iter()
                .map(|child| keys.get(child).copied().ok_or_else(unchecked))
                .collect::<Result<Vec<_>, _>>()?;
            let mut declaration = vec![identifier(name, &location)?, identifier("run", &location)?];
            for segment in &control.path {
                declaration.push(identifier(segment, &location)?);
            }
            let key = self.protocol_member(
                &location,
                "control",
                boolean,
                declaration,
                BodyTerm::application(
                    Operator::ProtocolControl,
                    operation.clone(),
                    boolean,
                    vec![MemberTerm::binding(
                        "children",
                        BindingValue::Group(GroupTerm::references(children)),
                    )],
                ),
            )?;
            keys.insert(control.node, key);
        }
        tree.root()
            .and_then(|root| keys.get(&root.node))
            .copied()
            .ok_or_else(|| fault(&protocol.location(), KeyFault::UncheckedProtocolFact))
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
    /// The profile is selected and the protocol names this construct, the
    /// first of those whose node is not written yet.
    NotEmitted(ProtocolConstruct),
}
