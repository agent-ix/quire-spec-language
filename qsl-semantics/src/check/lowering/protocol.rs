// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092's protocol nodes: the `protocol`/`protocol_clause` node each
//! checked protocol declaration emits, and the `protocol`/`protocol_reference`
//! carrier a protocol reference expression emits (FR-308, FR-218, QSpec
//! FR-322-AC-66).
//!
//! A protocol's node is a `quire.op.protocol.control` application carrying
//! the package's selected `protocol_profile` law and a `profile_operator`
//! member naming the `run` tree's root control. It is keyed by
//! `quire.application-node/v1` with the declaration's qualified name and no
//! owner. The protocol's roles, `run` tree and `finish` have no node of their
//! own (FR-113). A reference binds the node's key and is itself a
//! `{term: "reference", target}` leaf keyed by `quire.structural-node/v1`.

use quire_exact::{Identifier, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{refuse, Lowering, NodeContent};
use crate::check::family::OccurrenceRole;
use crate::check::ir::CheckedNode;
use crate::check::node_key::{BodyTerm, LawRole, LeafTerm, NodeTag, Operation, Operator};
use crate::check::refusal::{CheckCause, CheckRefusal, ProtocolReferenceCause};
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

/// One checked protocol declaration, as lowering reads it.
pub(crate) struct ProtocolInput {
    /// The declared name.
    pub(crate) name: String,
    /// The `run` tree's root keyword.
    pub(crate) root: &'static str,
    /// The root's `profile_operator`, when it is a FR-052 control construct.
    pub(crate) operator: Option<&'static str>,
}

impl ProtocolInput {
    /// Where the declaration's `declaration` occurrence is recorded.
    fn location(&self) -> Location {
        Location::root(Origin::TypeDeclaration {
            name: declaration_origin(&self.name),
        })
    }
}

impl Lowering<'_> {
    /// Lower and key each protocol of `protocols`, in declaration order,
    /// recording its one `declaration` occurrence. A package that selects no
    /// protocol profile has no `protocol_profile` law to carry (ADR-011
    /// §2.4): no node is built and nothing is refused, so the unit is as it
    /// was before protocol nodes; a reference to a protocol refuses
    /// `missing-selection`. With a profile selected, a protocol whose root
    /// names no `profile_operator` refuses at its declaration
    /// (`unsupported_construct`/`declaration-form`), and so does a reference
    /// to it.
    pub(crate) fn protocol_nodes(
        &mut self,
        protocols: &[ProtocolInput],
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
            match protocol.operator {
                Some(operator) => {
                    let key = self.protocol_node(protocol, operator, law, &location)?;
                    keys.push(ProtocolKey::Keyed(key));
                }
                None => {
                    refusals.push(unsupported_root(protocol.root, &location));
                    keys.push(ProtocolKey::UnsupportedRoot(protocol.root));
                }
            }
        }
        self.protocol_keys = keys;
        Ok(refusals)
    }

    fn protocol_node(
        &mut self,
        protocol: &ProtocolInput,
        operator: &'static str,
        law: crate::check::node_key::OperationLaw,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let identifier = |text: &str| {
            Identifier::new(text.to_owned()).map_err(|_| {
                refuse(
                    location,
                    CheckCause::NodePreimage(
                        crate::check::node_key::NodeKeyRefusal::EmptyQualifiedName,
                    ),
                )
            })
        };
        let boolean = self.type_node(&ValueType::Boolean, location)?;
        let operation = Operation {
            laws: vec![law],
            member: Some(Member::ProfileOperator {
                operator: identifier(operator)?,
            }),
            ..Operation::plain("quire.op.protocol.control")
        };
        let key = self.insert_node(
            location,
            NodeContent {
                node_tag: NodeTag::Protocol,
                semantic_form: "protocol_clause",
                semantic_type: Some(boolean),
                declaration: Some(vec![identifier(&protocol.name)?]),
                owner: None,
                body: BodyTerm::application(
                    Operator::ProtocolControl,
                    operation,
                    boolean,
                    Vec::new(),
                ),
            },
        )?;
        self.record(key, OccurrenceRole::Declaration, location.clone());
        Ok(key)
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
    /// the package selected no protocol profile for it to carry, or that its
    /// root names no operator.
    fn protocol_key(&self, protocol: usize, location: &Location) -> Result<NodeKey, CheckRefusal> {
        match self.protocol_keys.get(protocol) {
            Some(ProtocolKey::Keyed(key)) => Ok(*key),
            Some(ProtocolKey::UnsupportedRoot(root)) => Err(unsupported_root(root, location)),
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
    /// The profile is selected and the root names no `profile_operator`.
    UnsupportedRoot(&'static str),
}

/// `unsupported_construct`/`declaration-form` at `location`: a protocol
/// whose `root` is no FR-052 control construct.
fn unsupported_root(root: &'static str, location: &Location) -> CheckRefusal {
    refuse(
        location,
        CheckCause::ProtocolReference(Box::new(ProtocolReferenceCause::UnsupportedRoot { root })),
    )
}
