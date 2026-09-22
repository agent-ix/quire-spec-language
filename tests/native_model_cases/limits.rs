// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-045: exact small budgets and reachable hard boundaries, without fanout.

use super::*;
use ix_trace_rs::trace;

fn admit(input: Parts, limits: ModelLimits) -> Result<NativeModel, Box<NativeModelError>> {
    NativeModel::new(input.source, input.environment, input.roles, limits)
}

struct Dimension {
    name: &'static str,
    required: usize,
    set: fn(&mut ModelLimits, usize),
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_every_model_dimension_has_an_inclusive_small_boundary() {
    let bytes = parts().model().artifact_bytes().len();
    // Independent fixture accounting: 7 scalars + object + operation; 9 scalar
    // sites + 1 frame field; 2 types + 3 values + 11 fields + 16 type nodes.
    let exact = ModelLimits {
        roles: 9,
        entries: 10,
        nodes: 32,
        depth: 2,
        artifact_bytes: bytes,
    };
    assert_eq!(admit(parts(), exact).unwrap().artifact_bytes().len(), bytes);
    let dimensions = [
        Dimension {
            name: "roles",
            required: 9,
            set: |l, n| l.roles = n,
        },
        Dimension {
            name: "entries",
            required: 10,
            set: |l, n| l.entries = n,
        },
        Dimension {
            name: "nodes",
            required: 32,
            set: |l, n| l.nodes = n,
        },
        Dimension {
            name: "depth",
            required: 2,
            set: |l, n| l.depth = n,
        },
        Dimension {
            name: "artifact",
            required: bytes,
            set: |l, n| l.artifact_bytes = n,
        },
    ];
    for Dimension {
        name,
        required,
        set,
    } in dimensions
    {
        for maximum in [0, required - 1, required, usize::MAX] {
            let mut limits = exact;
            set(&mut limits, maximum);
            let result = admit(parts(), limits);
            if maximum < required {
                let error = result.unwrap_err();
                assert_eq!(error.code, Code::ResourceExhausted, "{name}/{maximum}");
                assert!(error.is_incomplete());
            } else {
                assert_eq!(result.unwrap().artifact_bytes().len(), bytes);
            }
        }
    }
    // Ten scalar sites, two parameters, two changed fields, two created types
    // and two deleted types. Omitting any entry class breaks the 17 refusal.
    for maximum in [17, 18] {
        let result = admit(
            extra_inventory(),
            ModelLimits {
                entries: maximum,
                ..ModelLimits::default()
            },
        );
        if maximum == 18 {
            assert_eq!(result.unwrap().roles().operations.len(), 2);
        } else {
            assert_eq!(result.unwrap_err().code, Code::ResourceExhausted);
        }
    }
}

fn nested_boolean(levels: usize) -> ir::ValueType {
    (1..levels).fold(ir::ValueType::Boolean, |ty, _| ir::ValueType::option(ty))
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_formal_depth_cannot_be_elevated_past_64() {
    for levels in [64, 65] {
        let mut input = parts();
        add_value(
            &mut input,
            "deep",
            ir::ValueDeclarationKind::State,
            nested_boolean(levels),
        );
        let result = admit(
            input,
            ModelLimits {
                depth: usize::MAX,
                ..ModelLimits::default()
            },
        );
        if levels == 64 {
            assert_eq!(result.unwrap().environment().values().len(), 4);
        } else {
            assert_eq!(result.unwrap_err().code, Code::ResourceExhausted);
        }
    }
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_formal_node_ceiling_is_reachable_within_artifact_limit() {
    for tail_levels in [22, 23] {
        let mut input = parts();
        let mut values = input.environment.values().to_vec();
        let source = values[0].source().clone();
        // 32 baseline nodes + 153 * (1 declaration + 64 type levels)
        // + (1 declaration + 22 type levels) = exactly 10,000 nodes.
        for index in 0..153 {
            values.push(ir::ValueDeclaration::new(
                symbol(&format!("v{index}")),
                ir::ValueDeclarationKind::State,
                nested_boolean(64),
                source.clone(),
            ));
        }
        values.push(ir::ValueDeclaration::new(
            symbol("tail"),
            ir::ValueDeclarationKind::State,
            nested_boolean(tail_levels),
            source,
        ));
        input.environment = ir::DeclarationEnvironment::new(
            input.environment.owner().clone(),
            input.environment.types().to_vec(),
            values,
            Vec::new(),
        )
        .unwrap();
        let result = admit(
            input,
            ModelLimits {
                nodes: usize::MAX,
                ..ModelLimits::default()
            },
        );
        if tail_levels == 22 {
            let model = result.unwrap();
            assert_eq!(model.environment().values().len(), 157);
            assert!(model.artifact_bytes().len() < 1_048_576);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted);
            assert_eq!(error.message, "model nodes limit exceeded");
        }
    }
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_entry_ceiling_counts_frames_across_operations() {
    for last_frame in [91, 92] {
        let mut input = parts();
        replace_record(&mut input, "Node", |r| {
            let mut fields = r.fields().to_vec();
            fields.extend((0..100).map(|i| {
                ir::RecordFieldDeclaration::new(
                    symbol(&format!("f{i}")),
                    ir::ValueType::Boolean,
                    r.source().clone(),
                )
            }));
            ir::RecordDeclaration::new(r.name().clone(), r.source().clone(), fields).unwrap()
        });
        let template = input.roles.operations[0].clone();
        input.roles.operations = (0..100)
            .map(|index| {
                let mut op = template.clone();
                op.name = symbol(&format!("op{index}"));
                let count = if index == 99 { last_frame } else { 100 };
                op.frame.fields = (0..count)
                    .map(|i| (symbol("Node"), symbol(&format!("f{i}"))))
                    .collect();
                op
            })
            .collect();
        // Nine scalar sites + 99*100 + 91 frame fields = 10,000 entries.
        let result = admit(
            input,
            ModelLimits {
                entries: usize::MAX,
                ..ModelLimits::default()
            },
        );
        if last_frame == 91 {
            assert_eq!(result.unwrap().roles().operations.len(), 100);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted);
            assert_eq!(error.message, "model entries limit exceeded");
        }
    }
}

#[test]
#[trace("TC-045", "FR-015-AC-6")]
fn tc_045_role_hard_ceiling_precedes_the_coupled_artifact_ceiling() {
    for count in [10_000, 10_001] {
        let mut input = parts();
        let template = input.roles.operations[0].clone();
        input.roles.operations = (0..count - 8)
            .map(|i| {
                let mut operation = template.clone();
                operation.name = symbol(&format!("op{i}"));
                operation.frame.fields.clear();
                operation
            })
            .collect();
        let error = admit(
            input,
            ModelLimits {
                roles: usize::MAX,
                ..ModelLimits::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
        if count == 10_001 {
            assert_eq!(error.message, "model roles limit exceeded");
        } else {
            // 10,000 full source-bearing roles cannot fit this artifact ceiling.
            // This is a coupled-limit refusal, not a fabricated valid exact case;
            // the small 9-role test supplies the inclusive success control.
            assert!(error.message.contains("artifact"), "{error:?}");
        }
    }
}
