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
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::value::member::Member;

/// The `Origin::TypeDeclaration` name a protocol's declaration is located
/// under. Spaced, so it names no declared type.
pub(crate) fn declaration_origin(protocol: &str) -> String {
    format!("protocol {protocol}")
}

/// One checked protocol declaration, as lowering reads it.
pub(crate) struct ProtocolInput {
    /// The declared name.
    pub(crate) name: String,
    /// The `run` tree's root control keyword, the `profile_operator`.
    pub(crate) operator: &'static str,
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
    /// §2.4): no node is built, and a reference to a protocol refuses
    /// `missing-selection`.
    pub(crate) fn protocol_nodes(
        &mut self,
        protocols: &[ProtocolInput],
    ) -> Result<(), CheckRefusal> {
        let mut keys = Vec::with_capacity(protocols.len());
        for protocol in protocols {
            let location = protocol.location();
            let key = match self.law(LawRole::ProtocolProfile, &location) {
                Ok(law) => Some(self.protocol_node(protocol, law, &location)?),
                Err(CheckRefusal {
                    cause: CheckCause::MissingSelection { .. },
                    ..
                }) => None,
                Err(other) => return Err(other),
            };
            keys.push(key);
        }
        self.protocol_keys = keys;
        Ok(())
    }

    fn protocol_node(
        &mut self,
        protocol: &ProtocolInput,
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
                operator: identifier(protocol.operator)?,
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
    /// the package selected no protocol profile for it to carry.
    fn protocol_key(&self, protocol: usize, location: &Location) -> Result<NodeKey, CheckRefusal> {
        self.protocol_keys
            .get(protocol)
            .copied()
            .flatten()
            .ok_or_else(|| {
                refuse(
                    location,
                    CheckCause::MissingSelection {
                        role: LawRole::ProtocolProfile,
                    },
                )
            })
    }
}
