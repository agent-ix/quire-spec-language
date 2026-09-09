// SPDX-License-Identifier: AGPL-3.0-only
//! TC-065/066: independently lowered budgets, deterministic stops and immutable retries.

use super::setup::{authored_model, recorded};
use super::*;
use quire_spec_language::runtime::{ValidationReport, ValidationUsage};
use serde_json::json;
use std::cell::Cell;

#[derive(Clone, Copy, Debug)]
enum Counter {
    Artifacts,
    Bytes,
    Objects,
    Work,
    Text,
    Diagnostics,
}

impl Counter {
    fn usage(self, usage: ValidationUsage) -> usize {
        match self {
            Self::Artifacts => usage.artifacts,
            Self::Bytes => usage.artifact_bytes,
            Self::Objects => usage.objects,
            Self::Work => usage.work,
            Self::Text => usage.text_steps,
            Self::Diagnostics => usage.diagnostics,
        }
    }

    fn limits(self, maximum: usize) -> ValidationLimits {
        let mut limits = ValidationLimits::default();
        match self {
            Self::Artifacts => limits.artifacts = maximum,
            Self::Bytes => limits.artifact_bytes = maximum,
            Self::Objects => limits.objects = maximum,
            Self::Work => limits.work = maximum,
            Self::Text => limits.text_steps = maximum,
            Self::Diagnostics => limits.diagnostics = maximum,
        }
        limits
    }
}

fn compare_reports(left: &ValidationReport, right: &ValidationReport) {
    assert_eq!(left.status, right.status);
    assert_eq!(left.diagnostics, right.diagnostics);
    assert_eq!(left.terminal, right.terminal);
    assert_eq!(left.usage, right.usage);
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn unselected_artifact_bytes_count_but_their_objects_are_not_inspected() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let selected_artifact = snapshot(draft(&models[0]));
    let selected = selection(&models[0], selected_artifact.reference());
    let mut unselected = draft(&models[0]);
    unselected.populations[0].objects = vec![
        ObjectEntry {
            key: "unselected invalid object".into(),
            fields: Vec::new(),
        };
        17
    ];
    let unselected = Snapshot::new(
        SourceIdentity {
            identity: "test:unselected".into(),
            revision: "1".into(),
        },
        unselected,
        ArtifactLimits::default(),
    )
    .unwrap();
    let bytes = selected_artifact.bytes().len() + unselected.bytes().len();
    let offered = RuntimeInput {
        snapshots: vec![selected_artifact, unselected],
        invocations: Vec::new(),
    };
    let context = validate(
        &checked,
        offered.clone(),
        selected.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    assert_eq!(context.usage().artifacts, 2);
    assert_eq!(context.usage().artifact_bytes, bytes);
    assert_eq!(context.usage().objects, 1);
    assert_eq!(
        context.input().snapshots[1].draft().populations[0]
            .objects
            .len(),
        17
    );
    let report = validate(
        &checked,
        offered,
        selected,
        Counter::Bytes.limits(bytes - 1),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Incomplete);
    assert!(report.diagnostics.is_empty());
    assert_eq!(
        report.terminal.as_ref().unwrap().code,
        Code::ResourceExhausted
    );
    assert_eq!(
        report.usage.objects, 0,
        "aggregate byte admission precedes population work"
    );
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn measured_required_counters_accept_exact_and_stop_one_below_or_zero() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    for count in [1, 2, 5] {
        let mut data = draft(&models[0]);
        for index in 1..count {
            let mut object = data.populations[0].objects[0].clone();
            object.key = format!("é界{index}");
            data.populations[0].objects.push(object);
        }
        let artifact = snapshot(data);
        let selected = selection(&models[0], artifact.reference());
        let offered = input(artifact);
        let baseline = validate(
            &checked,
            offered.clone(),
            selected.clone(),
            ValidationLimits::default(),
            || false,
        )
        .unwrap();
        assert_eq!(baseline.usage().objects, count);
        for counter in [
            Counter::Artifacts,
            Counter::Bytes,
            Counter::Objects,
            Counter::Work,
            Counter::Text,
        ] {
            let measured = counter.usage(baseline.usage());
            assert!(measured > 0, "fixture exercises {counter:?}");
            let exact = validate(
                &checked,
                offered.clone(),
                selected.clone(),
                counter.limits(measured),
                || false,
            )
            .unwrap();
            assert_eq!(exact.usage(), baseline.usage(), "exact {counter:?}");
            for maximum in [0, measured - 1] {
                let report = validate(
                    &checked,
                    offered.clone(),
                    selected.clone(),
                    counter.limits(maximum),
                    || false,
                )
                .unwrap_err();
                assert_eq!(
                    report.status,
                    ValidationStatus::Incomplete,
                    "{counter:?} at {maximum}"
                );
                assert_eq!(
                    report.terminal.as_ref().unwrap().code,
                    Code::ResourceExhausted
                );
                assert!(report.diagnostics.is_empty());
                assert!(counter.usage(report.usage) <= maximum);
            }
        }
        let no_details = validate(
            &checked,
            offered,
            selected,
            Counter::Diagnostics.limits(0),
            || false,
        )
        .unwrap();
        assert_eq!(no_details.usage(), baseline.usage());
    }
}

#[test]
#[trace("TC-065", "TC-064", "FR-007-AC-5", "FR-007-AC-12")]
fn detail_capacity_retains_invalid_classification_even_at_zero() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    let mut data = draft(&models[0]);
    data.arena[0] = ValueNode::Integer { value: 1001 };
    let artifact = snapshot(data);
    let selected = selection(&models[0], artifact.reference());
    let offered = input(artifact);
    let baseline = validate(
        &checked,
        offered.clone(),
        selected.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap_err();
    assert_eq!(baseline.status, ValidationStatus::Refused);
    assert!(baseline.terminal.is_none());
    let count = baseline.diagnostics.len();
    assert!(
        count >= 2,
        "shared integer has independently invalid nominal uses"
    );
    for maximum in [0, 1, count - 1, count] {
        let report = validate(
            &checked,
            offered.clone(),
            selected.clone(),
            Counter::Diagnostics.limits(maximum),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        assert_eq!(report.usage.diagnostics, maximum);
        assert_eq!(report.diagnostics.len(), maximum);
        if maximum < count {
            assert_eq!(
                report.terminal.as_ref().unwrap().code,
                Code::ResourceExhausted
            );
        } else {
            compare_reports(&report, &baseline);
        }
    }
    let mut data = draft(&models[0]);
    data.populations[0].complete = false;
    let artifact = snapshot(data);
    let selected = super::selection(&models[0], artifact.reference());
    let report = validate(
        &checked,
        input(artifact),
        selected,
        Counter::Diagnostics.limits(0),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Incomplete);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.usage.diagnostics, 0);
    assert_eq!(
        report.terminal.as_ref().unwrap().code,
        Code::ResourceExhausted
    );
}

#[test]
#[trace("TC-065", "TC-066", "FR-007-AC-12", "FR-007-AC-15")]
fn cancellation_at_each_observed_poll_is_repeatable_and_preserves_known_defects() {
    let models = [native_rule_model::parts().model()];
    for operation in [false, true] {
        let checked = checked_kind(
            &models,
            "false",
            if operation {
                ClauseKind::Postcondition
            } else {
                ClauseKind::Invariant
            },
        );
        for invalid in [false, true] {
            let mut data = draft(&models[0]);
            if invalid {
                data.arena[0] = ValueNode::Integer { value: 1001 };
            }
            let (offered, selected) = if operation {
                recorded(&models[0], data.clone(), data, |_| {})
            } else {
                let artifact = snapshot(data);
                let selected = selection(&models[0], artifact.reference());
                (input(artifact), selected)
            };
            let observed = Cell::new(0);
            let baseline = validate(
                &checked,
                offered.clone(),
                selected.clone(),
                ValidationLimits::default(),
                || {
                    observed.set(observed.get() + 1);
                    false
                },
            );
            assert_eq!(baseline.is_err(), invalid);
            let mut retained_invalid = false;
            for stop in 1..=observed.get() {
                let run = || {
                    let polls = Cell::new(0);
                    let report = validate(
                        &checked,
                        offered.clone(),
                        selected.clone(),
                        ValidationLimits::default(),
                        || {
                            polls.set(polls.get() + 1);
                            polls.get() == stop
                        },
                    )
                    .unwrap_err();
                    assert_eq!(polls.get(), stop, "no callback after a true poll");
                    assert_eq!(report.terminal.as_ref().unwrap().code, Code::Cancelled);
                    report
                };
                let report = run();
                compare_reports(&report, &run());
                if stop == 1 {
                    assert_eq!(report.usage, ValidationUsage::default());
                    assert_eq!(report.status, ValidationStatus::Incomplete);
                }
                if !report.diagnostics.is_empty() {
                    assert!(invalid);
                    assert_eq!(report.status, ValidationStatus::Refused);
                    retained_invalid = true;
                }
            }
            assert_eq!(retained_invalid, invalid);
        }
    }
}

#[test]
#[trace("TC-066", "TC-065", "FR-007-AC-15")]
fn fresh_requests_preserve_success_after_refusal_exhaustion_cancellation_and_panic() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let artifact = snapshot(draft(&models[0]));
    let bytes = artifact.bytes().to_vec();
    let source = checked.linked().unit().source().text().to_owned();
    let model_bytes = models[0].artifact_bytes().to_vec();
    let selected = selection(&models[0], artifact.reference());
    let offered = input(artifact);
    let baseline = validate(
        &checked,
        offered.clone(),
        selected.clone(),
        ValidationLimits::default(),
        || false,
    )
    .unwrap();
    let run = |limits, cancelled| {
        validate(&checked, offered.clone(), selected.clone(), limits, || {
            cancelled
        })
    };
    assert_eq!(
        run(Counter::Work.limits(baseline.usage().work - 1), false)
            .unwrap_err()
            .status,
        ValidationStatus::Incomplete
    );
    assert_eq!(
        run(ValidationLimits::default(), true)
            .unwrap_err()
            .terminal
            .as_ref()
            .unwrap()
            .code,
        Code::Cancelled
    );
    let mut foreign = selected.clone();
    foreign.clause = ir::ClauseId::new("foreign").unwrap();
    assert_eq!(
        validate(
            &checked,
            offered.clone(),
            foreign,
            ValidationLimits::default(),
            || false
        )
        .unwrap_err()
        .status,
        ValidationStatus::Refused
    );
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut polls = 0;
        validate(
            &checked,
            offered.clone(),
            selected.clone(),
            ValidationLimits::default(),
            || {
                polls += 1;
                assert!(polls != 12, "deliberate caller poll panic");
                false
            },
        )
    }));
    assert!(
        panicked.is_err(),
        "the caller panic must unwind, not return a runtime report"
    );
    let repeated = run(ValidationLimits::default(), false).unwrap();
    assert_eq!(repeated.usage(), baseline.usage());
    for context in [&baseline, &repeated] {
        assert!(std::ptr::eq(context.checked(), &checked));
        assert_eq!(context.input().snapshots[0].bytes(), bytes);
        assert_eq!(context.selection(), &selected);
    }
    assert_eq!(models[0].artifact_bytes(), model_bytes);
    assert_eq!(checked.linked().unit().source().text(), source);
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn elevated_options_still_enforce_inventory_object_and_detail_hard_ceilings() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "true");
    let artifact = snapshot(draft(&models[0]));
    let selected = selection(&models[0], artifact.reference());
    let count = ValidationLimits::default().artifacts + 1;
    let offered = RuntimeInput {
        snapshots: vec![artifact; count],
        invocations: Vec::new(),
    };
    let report = validate(
        &checked,
        offered,
        selected,
        Counter::Artifacts.limits(usize::MAX),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Incomplete);
    assert_eq!(report.usage, ValidationUsage::default());
    assert_eq!(
        report.terminal.as_ref().unwrap().message,
        "validation inventory count limit"
    );

    let mut data = draft(&models[0]);
    data.populations[0].objects = vec![
        ObjectEntry {
            key: String::new(),
            fields: Vec::new()
        };
        10_001
    ];
    let artifact = snapshot(data);
    let selected = selection(&models[0], artifact.reference());
    let report = validate(
        &checked,
        input(artifact),
        selected,
        Counter::Objects.limits(usize::MAX),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Incomplete);
    assert_eq!(report.usage.objects, 0);
    assert_eq!(
        report.terminal.as_ref().unwrap().message,
        "validation selected object limit"
    );

    let mut data = draft(&models[0]);
    data.populations[0].objects = vec![
        ObjectEntry {
            key: "duplicate".into(),
            fields: Vec::new()
        };
        258
    ];
    let artifact = snapshot(data);
    let selected = selection(&models[0], artifact.reference());
    let report = validate(
        &checked,
        input(artifact),
        selected,
        Counter::Diagnostics.limits(usize::MAX),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Refused);
    assert_eq!(report.usage.diagnostics, 256);
    assert_eq!(report.diagnostics.len(), 256);
    assert_eq!(
        report.terminal.as_ref().unwrap().message,
        "validation detail diagnostic limit"
    );
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn shared_values_cannot_raise_hard_work_or_unicode_limits() {
    for text in [false, true] {
        let mut ty = json!({"kind": "scalar", "name": if text { "ObjectId" } else { "Version" }});
        let levels = if text { 1 } else { 20 };
        let width = if text { 40_000 } else { 2 };
        for _ in 0..levels {
            ty = json!({"kind": "sequence", "maximum": width, "value": ty});
        }
        let models = [authored_model(|data| {
            data["records"][0]["fields"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name": "shared", "type": ty}));
        })];
        let checked = checked(&models, "false");
        let mut data = draft(&models[0]);
        let mut root = ValueId::new(u32::try_from(data.arena.len()).unwrap());
        data.arena.push(if text {
            ValueNode::Text {
                value: "界".repeat(256),
            }
        } else {
            ValueNode::Integer { value: 1 }
        });
        for _ in 0..levels {
            let next = ValueId::new(u32::try_from(data.arena.len()).unwrap());
            data.arena.push(ValueNode::Sequence {
                values: vec![root; width],
            });
            root = next;
        }
        data.populations[0].objects[0].fields.push(FieldBinding {
            name: symbol("shared"),
            value: root,
        });
        let artifact = snapshot(data);
        let selected = selection(&models[0], artifact.reference());
        let counter = if text { Counter::Text } else { Counter::Work };
        let report = validate(
            &checked,
            input(artifact),
            selected,
            counter.limits(usize::MAX),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Incomplete);
        assert!(report.diagnostics.is_empty());
        let terminal = report.terminal.as_ref().unwrap();
        assert_eq!(terminal.code, Code::ResourceExhausted);
        let hard = if text { 8_388_608 } else { 1_000_000 };
        assert_eq!(counter.usage(report.usage), hard);
        assert_eq!(
            terminal.message,
            if text {
                "validation Unicode inspection limit"
            } else {
                "validation work limit"
            }
        );
    }
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn exact_aggregate_hard_bytes_succeed_and_next_byte_stops_before_indexing() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    let artifact = snapshot(draft(&models[0]));
    let selected = selection(&models[0], artifact.reference());
    let mut offered = input(artifact);
    let unselected = |index, padding| {
        let mut data = draft(&models[0]);
        data.arena.push(ValueNode::Text {
            value: "x".repeat(padding),
        });
        Snapshot::new(
            SourceIdentity {
                identity: format!("test:unselected-{index}"),
                revision: "1".into(),
            },
            data,
            ArtifactLimits::default(),
        )
        .unwrap()
    };
    for index in 0..10 {
        offered.snapshots.push(unselected(index, 800_000));
    }
    let used: usize = offered
        .snapshots
        .iter()
        .map(|snapshot| snapshot.bytes().len())
        .sum();
    let padding = 8_388_608 - used - unselected(10, 0).bytes().len();
    offered.snapshots.push(unselected(10, padding));
    let exact = validate(
        &checked,
        offered.clone(),
        selected.clone(),
        Counter::Bytes.limits(usize::MAX),
        || false,
    )
    .unwrap();
    assert_eq!(exact.usage().artifact_bytes, 8_388_608);
    assert_eq!(exact.usage().objects, 1);
    *offered.snapshots.last_mut().unwrap() = unselected(10, padding + 1);
    let report = validate(
        &checked,
        offered,
        selected,
        Counter::Bytes.limits(usize::MAX),
        || false,
    )
    .unwrap_err();
    assert_eq!(report.status, ValidationStatus::Incomplete);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.usage.objects, 0);
    assert_eq!(
        report.terminal.as_ref().unwrap().message,
        "validation inventory content limit"
    );
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn maximum_inventory_and_selected_object_counts_can_complete() {
    let models = [native_rule_model::parts().model()];
    let checked_current = checked(&models, "false");
    let artifact = snapshot(draft(&models[0]));
    let selected = selection(&models[0], artifact.reference());
    let mut offered = input(artifact);
    for index in 1..64 {
        offered.snapshots.push(
            Snapshot::new(
                SourceIdentity {
                    identity: format!("test:extra-{index}"),
                    revision: "1".into(),
                },
                draft(&models[0]),
                ArtifactLimits::default(),
            )
            .unwrap(),
        );
    }
    let context = validate(
        &checked_current,
        offered,
        selected,
        Counter::Artifacts.limits(usize::MAX),
        || false,
    )
    .unwrap();
    assert_eq!(context.usage().artifacts, 64);
    assert_eq!(
        context.usage().objects,
        1,
        "unselected inventories do not inflate selected objects"
    );

    let models = [authored_model(|data| {
        data["records"][0]["fields"] = json!([{"name": "flag", "type": {"kind": "boolean"}}]);
        data["operations"][0]["frame"]["fields"] = json!([]);
        data["scalars"]
            .as_array_mut()
            .unwrap()
            .retain(|scalar| scalar["name"] == "ObjectId");
    })];
    let checked = checked_kind(&models, "false", ClauseKind::Postcondition);
    let mut before = draft(&models[0]);
    before.arena = vec![ValueNode::Boolean { value: false }];
    before.populations[0].objects = (0..5_000)
        .map(|index| ObjectEntry {
            key: if index == 0 {
                "self".into()
            } else {
                format!("object-{index}")
            },
            fields: vec![field("flag", 0)],
        })
        .collect();
    let (offered, selected) = recorded(&models[0], before.clone(), before, |_| {});
    let context = validate(
        &checked,
        offered,
        selected,
        Counter::Objects.limits(usize::MAX),
        || false,
    )
    .unwrap();
    assert_eq!(context.usage().objects, 10_000);
    assert_eq!(
        context
            .object(
                ir::StateObservation::Post,
                &object(&models[0], "object-4999")
            )
            .unwrap()
            .fields,
        [field("flag", 0)]
    );
}

#[test]
#[trace("TC-065", "FR-007-AC-12")]
fn maximum_detail_count_is_complete_until_an_additional_defect_needs_storage() {
    let models = [native_rule_model::parts().model()];
    let checked = checked(&models, "false");
    for count in [256, 257] {
        let mut data = draft(&models[0]);
        data.arena.push(ValueNode::Boolean { value: false });
        data.populations[0].objects[0].fields[0].value = ValueId::new(5);
        let template = data.populations[0].objects[0].clone();
        for index in 1..count {
            data.populations[0].objects.push(ObjectEntry {
                key: format!("object-{index}"),
                fields: template.fields.clone(),
            });
        }
        let artifact = snapshot(data);
        let selected = selection(&models[0], artifact.reference());
        let report = validate(
            &checked,
            input(artifact),
            selected,
            Counter::Diagnostics.limits(usize::MAX),
            || false,
        )
        .unwrap_err();
        assert_eq!(report.status, ValidationStatus::Refused);
        assert_eq!(report.diagnostics.len(), 256);
        assert_eq!(report.usage.diagnostics, 256);
        assert!(report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code == Code::InvalidRuntimeInput));
        if count == 256 {
            assert!(
                report.terminal.is_none(),
                "exact maximum is a complete enumeration"
            );
        } else {
            assert_eq!(
                report.terminal.as_ref().unwrap().code,
                Code::ResourceExhausted
            );
        }
    }
}
