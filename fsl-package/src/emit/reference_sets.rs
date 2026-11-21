// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-464 reference sets are ordered by structural checked node identity.

use filament_contract_model::CheckedTerm;

use super::{encoding, EmitRefusal};

/// Order only the reference sets of the protocol layout. This runs before
/// operand compaction: the targets still hold their complete structural IDs,
/// so neither graph positions nor node-local operand slots enter comparison.
pub(super) fn order_protocol_sets(
    operator: &str,
    arguments: &mut [CheckedTerm],
) -> Result<(), EmitRefusal> {
    let names: &[&str] = match operator {
        "protocol" => &[
            "roles",
            "relationships",
            "channels",
            "queues",
            "requirements",
            "compensations",
        ],
        "choice" | "repeat" => &["visible"],
        "attempt" | "recommend" => &["contracts"],
        _ => &[],
    };
    for argument in arguments {
        let CheckedTerm::Binding { name, value, .. } = argument else {
            continue;
        };
        if !names.contains(&name.as_ref()) {
            continue;
        }
        let CheckedTerm::Aggregate { members } = value.as_mut() else {
            return Err(encoding("protocol reference set is not an aggregate"));
        };
        let mut targets = members
            .iter()
            .map(|member| match member {
                CheckedTerm::Reference { target } => Ok(target.clone()),
                _ => Err(encoding("protocol reference set holds a non-reference")),
            })
            .collect::<Result<Vec<_>, _>>()?;
        targets.sort_unstable();
        if targets.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(encoding("protocol reference set repeats a target"));
        }
        *members = targets.into_iter().map(CheckedTerm::reference).collect();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use filament_contract_model::{CheckedNodeId, CheckedNodePathSegment as Segment};
    use fsl_semantics::check::wire_id::{structural_id, WireGraph};
    use ix_trace_rs::trace;
    use serde_json::json;

    fn id(name: &str) -> CheckedNodeId {
        CheckedNodeId {
            package: "p".into(),
            path: vec![Segment::Name("value".into()), Segment::Name(name.into())],
            ordinal: 0,
        }
    }

    fn binding(name: &str, targets: &[CheckedNodeId]) -> CheckedTerm {
        CheckedTerm::Binding {
            name: name.into(),
            binder: None,
            value: Box::new(CheckedTerm::Aggregate {
                members: targets
                    .iter()
                    .cloned()
                    .map(CheckedTerm::reference)
                    .collect(),
            }),
        }
    }

    fn targets(argument: &CheckedTerm) -> Vec<CheckedNodeId> {
        let CheckedTerm::Binding { value, .. } = argument else {
            panic!("a binding");
        };
        let CheckedTerm::Aggregate { members } = value.as_ref() else {
            panic!("an aggregate");
        };
        members
            .iter()
            .map(|member| match member {
                CheckedTerm::Reference { target } => target.clone(),
                _ => panic!("a reference"),
            })
            .collect()
    }

    #[trace("FSpec-FR-464-AC-3")]
    #[test]
    fn every_protocol_reference_set_uses_structural_order() {
        let ordered = vec![id("a"), id("m"), id("z")];
        let fields = [
            ("protocol", "roles"),
            ("protocol", "relationships"),
            ("protocol", "channels"),
            ("protocol", "queues"),
            ("protocol", "requirements"),
            ("protocol", "compensations"),
            ("choice", "visible"),
            ("repeat", "visible"),
            ("attempt", "contracts"),
            ("recommend", "contracts"),
        ];
        for (operator, field) in fields {
            for permutation in [[2, 1, 0], [1, 0, 2], [0, 2, 1]] {
                let input: Vec<_> = permutation.iter().map(|&at| ordered[at].clone()).collect();
                let mut arguments = vec![binding(field, &input)];
                order_protocol_sets(operator, &mut arguments).unwrap();
                assert_eq!(targets(&arguments[0]), ordered, "{operator}.{field}");
            }
        }
    }

    #[trace("FSpec-FR-464-AC-3")]
    #[test]
    fn operand_slots_and_graph_positions_do_not_order_reference_sets() {
        // Both target IDs have identical outer paths, differing only in the
        // identity their operand resolves to. Reverse both graph order and
        // slot numbering; resolve in each document before comparing.
        let mut results = Vec::new();
        for reversed in [false, true] {
            let leaves = if reversed {
                [id("z"), id("a")]
            } else {
                [id("a"), id("z")]
            };
            let slots = if reversed { [1, 0] } else { [0, 1] };
            let node = |slot| json!({"package":"p", "path":["expression", "application", "+", {"operand":slot}], "ordinal":0});
            let wire = json!({"semantic_graph": {
                "nodes":[{"node_id":leaves[0]}, {"node_id":leaves[1]},
                    {"node_id":node(slots[0])}, {"node_id":node(slots[1])}],
                "operand_rows":[[], [], [1, 0], [1, 0]]
            }});
            let graph = WireGraph::of_package_value(&wire).unwrap();
            let input: Vec<_> = [2, 3]
                .iter()
                .map(|&at| {
                    structural_id(
                        &graph
                            .key_of_value(&wire["semantic_graph"]["nodes"][at]["node_id"])
                            .unwrap(),
                    )
                })
                .collect();
            let mut arguments = vec![binding("contracts", &input)];
            order_protocol_sets("attempt", &mut arguments).unwrap();
            let output = targets(&arguments[0]);
            let nested: Vec<_> = output
                .iter()
                .map(|target| match target.path.last().unwrap() {
                    Segment::Node(operand) => operand.as_ref().clone(),
                    _ => panic!("structural operand"),
                })
                .collect();
            assert_eq!(nested, vec![id("a"), id("z")]);
            results.push(output);
        }
        assert_eq!(results[0], results[1]);
    }

    #[trace("FSpec-FR-464-AC-3")]
    #[test]
    fn sequences_and_repeated_operands_keep_their_order() {
        let input = vec![id("z"), id("a"), id("z")];
        for (operator, field) in [
            ("sequence", "children"),
            ("attempt", "arguments"),
            ("choice", "cases"),
            ("parallel", "branches"),
        ] {
            let mut arguments = vec![binding(field, &input)];
            order_protocol_sets(operator, &mut arguments).unwrap();
            assert_eq!(targets(&arguments[0]), input, "{operator}.{field}");
        }
    }

    #[trace("FSpec-FR-464-AC-3")]
    #[test]
    fn duplicate_set_targets_are_a_producer_error() {
        let mut arguments = vec![binding("contracts", &[id("a"), id("a")])];
        assert!(order_protocol_sets("attempt", &mut arguments).is_err());
    }
}
