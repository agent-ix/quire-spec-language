// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-411-AC-1 (QSpec FR-476): the I2 read-back of the model `terminal`
//! declarations of an admitted `quire.checked-package/v2`.
//!
//! IR's reader has already refused every FR-476 defect, so this reads the
//! admitted `temporal`/`terminal` nodes alone: the model each names, its mode
//! and the nodes of its `self` parameter and condition. A model with no node
//! reads as [`ReadTerminalDeclaration::None`].

use std::collections::BTreeSet;

use qsl_semantics::check::wire_id::WireGraph;
use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm,
    LiteralKind, TemporalForm,
};
use quire_exact::NodeKey;
use quire_semantic_value::catalog::InternalFault;

/// A model's intended terminal states as the package carries them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadTerminalDeclaration {
    /// The model has no `terminal` node: it intends no terminal state.
    None,
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the condition holds at it.
    When {
        /// The `self` parameter node, present when the condition reads `self`.
        parameter: Option<NodeKey>,
        /// The Boolean `expression` node of the condition.
        condition: NodeKey,
    },
}

/// One `terminal` node read back.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadTerminal {
    /// The `temporal`/`terminal` node.
    pub node: NodeKey,
    /// The `model` node the declaration is on.
    pub model: NodeKey,
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

fn key(
    graph: &WireGraph,
    id: &quire_contract_model::CheckedNodeId,
) -> Result<NodeKey, InternalFault> {
    graph.key(id).map_err(|_| SHAPE)
}

fn reference(term: &CheckedTerm) -> Option<&quire_contract_model::CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
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

fn read_node(
    graph: &WireGraph,
    node: &CheckedSemanticNodeV2,
) -> Result<ReadTerminal, InternalFault> {
    let key = |id: &quire_contract_model::CheckedNodeId| key(graph, id);
    let [model, mode, parameters, condition] = bindings(node).ok_or(SHAPE)?;
    let model = key(reference(model).ok_or(SHAPE)?)?;
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
                .map(|member| reference(member).ok_or(SHAPE).and_then(&key))
                .transpose()?,
            condition: key(reference(condition).ok_or(SHAPE)?)?,
        },
        _ => return Err(SHAPE),
    };
    Ok(ReadTerminal {
        node: key(&node.node_id)?,
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
    read_nodes(&package.graph().nodes)
}

/// Every `terminal` node among `nodes`, a whole graph, in order.
pub(crate) fn read_nodes(
    nodes: &[CheckedSemanticNodeV2],
) -> Result<Vec<ReadTerminal>, InternalFault> {
    let graph = WireGraph::new(nodes.iter().map(|node| &node.node_id)).map_err(|_| SHAPE)?;
    nodes
        .iter()
        .filter(|node| is_terminal(node))
        .map(|node| read_node(&graph, node))
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
    model: &NodeKey,
) -> Result<ReadTerminalDeclaration, InternalFault> {
    Ok(read_terminals(package)?
        .into_iter()
        .find(|terminal| &terminal.model == model)
        .map_or(ReadTerminalDeclaration::None, |terminal| {
            terminal.declaration
        }))
}

/// One `terminal` declaration of a checked package as the package's emitted
/// node gives it: the mode comes from the node, and a `when` declaration's
/// checked predicate is the one the node's occurrence names.
#[derive(Clone, Copy, Debug)]
pub struct PackageTerminal<'p> {
    /// The model alias the declaration is on.
    pub model: &'p str,
    /// The declared name.
    pub name: &'p str,
    /// The declaration.
    pub declaration: PackageTerminalDeclaration<'p>,
}

/// A [`PackageTerminal`]'s declaration.
#[derive(Clone, Copy, Debug)]
pub enum PackageTerminalDeclaration<'p> {
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the predicate holds at it.
    When(&'p qsl_semantics::check::CheckedTerminalPredicate),
}

/// Why a package's terminal declarations could not be resolved through its
/// terminal nodes.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TerminalResolveRefusal {
    /// A checked declaration has no node of its occurrence, or the node's
    /// mode is not the declaration's, or a terminal node has no checked
    /// declaration. `model` is the model alias, or the node for a node no
    /// declaration claims.
    #[error("the terminal nodes disagree with the checked declaration of {model}")]
    Disagrees {
        /// The model alias, or the node's identity.
        model: String,
    },
}

/// The `terminal` declarations of `package`, resolved through the
/// `temporal`/`terminal` nodes of its checked graph (FR-411, QSpec FR-476:
/// the node is the only carrier of the declaration). A declaration is
/// present exactly when the graph holds its node, in model-alias order, and
/// its mode is the node's. A `when` declaration's predicate is the checked
/// body the node's `claim` occurrence names, because a checked body cannot
/// be rebuilt from the wire.
///
/// # Errors
///
/// [`TerminalResolveRefusal`] when the nodes and the checked declarations do
/// not match.
pub fn resolve_terminals(
    package: &crate::CheckedPackage,
) -> Result<Vec<PackageTerminal<'_>>, TerminalResolveRefusal> {
    use qsl_semantics::check::{TerminalDeclaration, TerminalMode};

    let nodes = package.graph().terminal_nodes();
    let mut resolved = Vec::new();
    let mut claimed = BTreeSet::new();
    for checked in package.graph().terminals() {
        let found = checked
            .occurrence()
            .and_then(|occurrence| nodes.iter().find(|node| &node.node == occurrence.node()));
        let declaration = match (found, checked.declaration()) {
            (None, TerminalDeclaration::None) => continue,
            (Some(node), TerminalDeclaration::Any) if node.mode == TerminalMode::Any => {
                PackageTerminalDeclaration::Any
            }
            (Some(node), TerminalDeclaration::When(predicate))
                if node.mode == TerminalMode::When =>
            {
                PackageTerminalDeclaration::When(predicate)
            }
            _ => return Err(disagrees(checked.model())),
        };
        if let Some(node) = found {
            claimed.insert(node.node.clone());
        }
        resolved.push(PackageTerminal {
            model: checked.model(),
            name: checked.name().unwrap_or("terminal"),
            declaration,
        });
    }
    if let Some(orphan) = nodes.iter().find(|node| !claimed.contains(&node.node)) {
        return Err(disagrees(&orphan.node.to_string()));
    }
    Ok(resolved)
}

fn disagrees(model: &str) -> TerminalResolveRefusal {
    TerminalResolveRefusal::Disagrees {
        model: model.to_owned(),
    }
}
