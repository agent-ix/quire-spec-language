// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-449: the activation relation projected from the in-process protocol nodes.

use ix_trace_rs::string_edge;
use qsl_semantics::check::{
    BindingValue, BodyTerm, GroupMember, LeafTerm, LiteralValue, MemberTerm, NodeTag,
};
use quire_contract_model::{CheckedProtocolActivation, CheckedProtocolActivationRow};

use crate::CheckedPackage;
use qsl_semantics::check::wire_id::structural_id;

impl CheckedPackage {
    /// Derives the local protocols' control-to-temporal activation map (FR-449).
    ///
    /// Each row names the protocol clause, its origin or exact each-trigger
    /// node, and one required temporal clause. Rows ascend by protocol clause
    /// key and then temporal clause key; empty requirements yield no rows.
    /// Only the checked protocol nodes' activation and requirements are read.
    /// No package bytes, source text, clocks or observations are consulted.
    #[string_edge]
    pub fn protocol_activation_relation(&self) -> Vec<CheckedProtocolActivationRow> {
        let mut rows = Vec::new();
        for node in self.graph().semantic_graph().nodes() {
            if node.node_tag() != NodeTag::Protocol || node.semantic_form() != "protocol_clause" {
                continue;
            }
            let BodyTerm::Application(body) = node.body() else {
                unreachable!("checked protocol clause has an application body");
            };
            let group = |name: &str| {
                body.arguments
                    .iter()
                    .find_map(|argument| match argument {
                        MemberTerm::Binding(binding) if binding.name == name => {
                            match &binding.value {
                                BindingValue::Group(group) => Some(group),
                                _ => None,
                            }
                        }
                        _ => None,
                    })
                    .expect("checked protocol activation and requirements are groups")
            };
            let activation = group("activation");
            let leaf = |name: &str| {
                activation
                    .members
                    .iter()
                    .find_map(|member| match member {
                        GroupMember::Binding(binding) if binding.name == name => {
                            Some(&binding.value)
                        }
                        _ => None,
                    })
                    .expect("checked activation carries its kind and each trigger")
            };
            let LeafTerm::Literal {
                value: LiteralValue::Text(kind),
                ..
            } = leaf("kind")
            else {
                unreachable!("checked activation kind is a text literal");
            };
            let activation = match kind.as_str() {
                "origin" => CheckedProtocolActivation::Origin,
                "each" => CheckedProtocolActivation::Each {
                    trigger: reference(leaf("trigger")),
                },
                _ => unreachable!("checked activation kind is origin or each"),
            };
            let clause = structural_id(&node.key());
            for member in &group("requirements").members {
                let GroupMember::Leaf(target) = member else {
                    unreachable!("checked requirements are reference leaves");
                };
                rows.push(CheckedProtocolActivationRow {
                    clause: clause.clone(),
                    activation: activation.clone(),
                    required_temporal: reference(target),
                });
            }
        }
        rows.sort_unstable_by(|left, right| {
            left.clause
                .cmp(&right.clause)
                .then_with(|| left.required_temporal.cmp(&right.required_temporal))
        });
        rows
    }
}

fn reference(leaf: &LeafTerm) -> quire_contract_model::CheckedNodeId {
    let LeafTerm::Reference { target } = leaf else {
        unreachable!("checked activation and requirements name local nodes");
    };
    structural_id(
        target
            .admitted()
            .expect("checked protocol references are resolved"),
    )
}
