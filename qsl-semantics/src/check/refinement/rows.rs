// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-375: a refinement's authored rows lowered to the package graph's
//! node keys, which the V2 refinement body names.
//!
//! S4 builds these once every node the rows name is minted: each row
//! expression as a lowered term over its own `parameter` nodes (FR-341's
//! binder encoding, one node per binder at levels `0..n`), each declaration
//! as its node key. [`super::CheckedRefinement::v2_emission`] orders them
//! canonically and writes them.

use quire_exact::NodeKey;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use crate::check::node_key::LeafTerm;

/// One `population A::p from C::q` row's two population nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyedPopulationRow {
    pub(crate) abstract_population: NodeKey,
    pub(crate) concrete_population: NodeKey,
}

/// One `object A::T from C::U` row: both type nodes and each visible
/// field's name and lowered expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyedObjectRow {
    pub(crate) abstract_type: NodeKey,
    pub(crate) concrete_type: NodeKey,
    pub(crate) fields: Vec<(String, LeafTerm)>,
}

/// One `step` row: its class spelling and node, its target, and the
/// position of the source row it was written as among the refinement's
/// authored step rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyedStepRow {
    pub(crate) source_class: String,
    pub(crate) source_node: Option<NodeKey>,
    pub(crate) target: KeyedStepTarget,
    pub(crate) source: usize,
}

/// A step row's right side, keyed. An argument `None` is `_`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeyedStepTarget {
    /// An abstract operation: its `operation_anchor`, the receiver and the
    /// arguments.
    Operation {
        operation: NodeKey,
        receiver: LeafTerm,
        arguments: Vec<Option<LeafTerm>>,
    },
    /// An abstract protocol step: its alias-free class spelling, its node
    /// when the class has one, and the arguments.
    ProtocolStep {
        class: String,
        node: Option<NodeKey>,
        arguments: Vec<Option<LeafTerm>>,
    },
    /// `stutter`.
    Stutter,
    /// `any`.
    Any,
}

/// One `assume` or `ensure` fairness row: the operation's
/// `operation_anchor`, the kind and the granularity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyedFairnessRow {
    pub(crate) operation: NodeKey,
    pub(crate) kind: FairnessKind,
    pub(crate) granularity: FairnessGranularity,
}

/// Every authored row of one refinement, keyed.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct KeyedRefinementRows {
    pub(crate) populations: Vec<KeyedPopulationRow>,
    pub(crate) objects: Vec<KeyedObjectRow>,
    pub(crate) steps: Vec<KeyedStepRow>,
    pub(crate) assume: Vec<KeyedFairnessRow>,
    pub(crate) ensure: Vec<KeyedFairnessRow>,
}

impl KeyedRefinementRows {
    /// Every node of this graph the rows name, which QSpec FR-375 makes the
    /// refinement node's dependencies: each declaration key and each
    /// `reference` an expression term holds (a literal's type is not one).
    pub(crate) fn node_keys(&self) -> Vec<&NodeKey> {
        fn term<'a>(keys: &mut Vec<&'a NodeKey>, term: &'a LeafTerm) {
            if let LeafTerm::Reference { target } = term {
                if let Some(key) = target.admitted() {
                    keys.push(key);
                }
            }
        }
        let mut keys = Vec::new();
        for row in &self.populations {
            keys.extend([&row.abstract_population, &row.concrete_population]);
        }
        for row in &self.objects {
            keys.extend([&row.abstract_type, &row.concrete_type]);
            for (_, expression) in &row.fields {
                term(&mut keys, expression);
            }
        }
        for row in &self.steps {
            keys.extend(row.source_node.as_ref());
            match &row.target {
                KeyedStepTarget::Operation {
                    operation,
                    receiver,
                    arguments,
                } => {
                    keys.push(operation);
                    term(&mut keys, receiver);
                    for argument in arguments.iter().flatten() {
                        term(&mut keys, argument);
                    }
                }
                KeyedStepTarget::ProtocolStep {
                    node, arguments, ..
                } => {
                    keys.extend(node.as_ref());
                    for argument in arguments.iter().flatten() {
                        term(&mut keys, argument);
                    }
                }
                KeyedStepTarget::Stutter | KeyedStepTarget::Any => {}
            }
        }
        keys.extend(
            self.assume
                .iter()
                .chain(&self.ensure)
                .map(|row| &row.operation),
        );
        keys
    }
}
