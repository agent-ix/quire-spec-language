// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411-AC-1 (QSpec FR-476): the I2 read-back of the model `terminal`
//! declarations of an admitted `quire.checked-package/v2`.
//!
//! IR's reader has already refused every FR-476 defect, so this reads the
//! admitted `temporal`/`terminal` nodes alone: the model each names, its mode
//! and the nodes of its `self` parameter and condition. A model with no node
//! reads as [`ReadTerminalDeclaration::None`].

use qsl_foundation::digest::WireNodeId;
use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm,
    LiteralKind, TemporalForm,
};
use quire_semantic_value::catalog::InternalFault;

/// A model's intended terminal states as the package carries them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadTerminalDeclaration {
    /// The model has no `terminal` node: it intends no terminal state.
    None,
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the condition holds at it.
    When {
        /// The `self` parameter node, present when the condition reads `self`.
        parameter: Option<WireNodeId>,
        /// The Boolean `expression` node of the condition.
        condition: WireNodeId,
    },
}

/// One `terminal` node read back.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReadTerminal {
    /// The `temporal`/`terminal` node.
    pub node: WireNodeId,
    /// The `model` node the declaration is on.
    pub model: WireNodeId,
    /// The declaration: never [`ReadTerminalDeclaration::None`].
    pub declaration: ReadTerminalDeclaration,
}

/// A broken invariant between IR's admission and this read.
const SHAPE: InternalFault = InternalFault::new("I2", "admitted-terminal-shape");

fn is_terminal(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Temporal(TemporalForm::Terminal))
}

fn wire_id(digest: &str) -> Result<WireNodeId, InternalFault> {
    WireNodeId::from_hex(digest).ok_or(SHAPE)
}

fn reference(term: &CheckedTerm) -> Option<&str> {
    match term {
        CheckedTerm::Reference { target } => Some(&target.digest),
        _ => None,
    }
}

/// The four bindings of a terminal body, in FR-476's order.
fn bindings(node: &CheckedSemanticNodeV2) -> Option<[&CheckedTerm; 4]> {
    let CheckedTerm::Aggregate { members } = &node.body else {
        return None;
    };
    let mut values = members.iter().map(|member| match member {
        CheckedTerm::Binding { value, .. } => Some(&**value),
        _ => None,
    });
    let found = [
        values.next()??,
        values.next()??,
        values.next()??,
        values.next()??,
    ];
    values.next().is_none().then_some(found)
}

fn read_node(node: &CheckedSemanticNodeV2) -> Result<ReadTerminal, InternalFault> {
    let [model, mode, parameters, condition] = bindings(node).ok_or(SHAPE)?;
    let model = wire_id(reference(model).ok_or(SHAPE)?)?;
    let CheckedTerm::Literal {
        value_kind: LiteralKind::Text,
        value: mode,
        ..
    } = mode
    else {
        return Err(SHAPE);
    };
    let CheckedTerm::Aggregate { members } = parameters else {
        return Err(SHAPE);
    };
    let declaration = match mode.as_str() {
        Some("any") => ReadTerminalDeclaration::Any,
        Some("when") => ReadTerminalDeclaration::When {
            parameter: members
                .first()
                .map(|member| reference(member).ok_or(SHAPE).and_then(wire_id))
                .transpose()?,
            condition: wire_id(reference(condition).ok_or(SHAPE)?)?,
        },
        _ => return Err(SHAPE),
    };
    Ok(ReadTerminal {
        node: wire_id(&node.node_id.digest)?,
        model,
        declaration,
    })
}

/// Every `terminal` node of `package`, in graph order.
///
/// # Errors
///
/// [`InternalFault`] when an admitted node has a shape IR's reader admits and
/// this read does not expect.
pub fn read_terminals(package: &CheckedPackageV2) -> Result<Vec<ReadTerminal>, InternalFault> {
    package
        .graph()
        .nodes
        .iter()
        .filter(|node| is_terminal(node))
        .map(read_node)
        .collect()
}

/// The intended terminal states of the model node `model`:
/// [`ReadTerminalDeclaration::None`] when the package holds no `terminal`
/// node on it.
///
/// # Errors
///
/// As [`read_terminals`].
pub fn read_terminal(
    package: &CheckedPackageV2,
    model: WireNodeId,
) -> Result<ReadTerminalDeclaration, InternalFault> {
    Ok(read_terminals(package)?
        .into_iter()
        .find(|terminal| terminal.model == model)
        .map_or(ReadTerminalDeclaration::None, |terminal| {
            terminal.declaration
        }))
}
