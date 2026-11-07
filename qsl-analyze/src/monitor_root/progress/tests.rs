// SPDX-License-Identifier: AGPL-3.0-or-later
//! Component comparisons over genuine checked source facts. These tests do
//! not issue an observation view or claim completed joint admission.

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{check, parse, select, DependencyInput, ParseRequest, SpineLimits};
use qsl_semantics::model::intake::package_input;
use quire_exact::Cancel;

use super::*;

fn checked_source(digital: bool) -> (ModelTimeSource, progress::ClaimOccurrence) {
    let packages = package_input([include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../examples/config-version/model.semantic-ir.json"
    ))
    .as_slice()])
    .unwrap();
    let time = if digital {
        "time M tick M::ConfigVersion::attemptUpdate period 2 ms unit ms;"
    } else {
        "time M dense unit ms;"
    };
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile t = \"quire.temporal.timed/v1\";\n\
         profile tc = \"quire.temporal.complete/v1\";\n\
         model M = \"example/config-version\";\n\
         dimension Time; unit s : Time = rational(1, 1);\n\
         unit ms : Time = rational(1, 1000) * s;\n\
         {time}\n\
         temporal C using t over (c: M::ConfigVersion) clock \"model-time\" on origin {{\n\
             always holds(c.versionNumber = 2)\n\
         }}"
    );
    let source = SourceIdentity::new("agent-ix", "progress-component");
    let cancel = Cancel::new();
    let limits = SpineLimits::default();
    let parsed = parse(
        &ParseRequest {
            source: &source,
            path: "progress.native",
            bytes: text.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .map_err(|_| "parse")
    .unwrap()
    .into_value();
    let admitted = select(&parsed, &packages, limits.model, &cancel)
        .map_err(|_| "model admission")
        .unwrap()
        .into_value();
    let unit = check(
        &parsed,
        "fixture".to_owned(),
        &admitted,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .map_err(|_| "check")
    .unwrap()
    .into_value();
    assert_eq!(unit.package().graph().time_sources().len(), 1);
    let graph = unit.package().graph();
    let clause = &graph.temporal_clauses()[0];
    (
        graph.time_sources()[0].clone(),
        progress::ClaimOccurrence::new(
            progress::NodeIdentity::from_key(
                clause.identity(),
                quire_observation::authority::Limits::owner_max(),
            )
            .unwrap(),
            clause.claim(),
        ),
    )
}

fn row(source: &ModelTimeSource, claim: progress::ClaimOccurrence) -> progress::AssociationRow {
    progress::AssociationRow {
        model: source.model.clone(),
        model_node: progress::NodeIdentity::from_key(
            source.model_node.clone(),
            quire_observation::authority::Limits::owner_max(),
        )
        .unwrap(),
        claim,
        source: match &source.source {
            TimeSourceKind::Dense => progress::TimeSource::Dense,
            TimeSourceKind::Tick { operation, period } => progress::TimeSource::Tick {
                operation: operation.clone(),
                operation_node: progress::NodeIdentity::from_key(
                    source.tick_operation_anchor.as_ref().unwrap().clone(),
                    quire_observation::authority::Limits::owner_max(),
                )
                .unwrap(),
                period_numerator: period.numerator().to_string(),
                period_denominator: period.denominator().to_string(),
            },
        },
        unit: source.unit.clone(),
        unit_node: progress::NodeIdentity::from_key(
            source.unit_node.clone(),
            quire_observation::authority::Limits::owner_max(),
        )
        .unwrap(),
        time_dimension: progress::NodeIdentity::from_key(
            source.time_dimension.clone(),
            quire_observation::authority::Limits::owner_max(),
        )
        .unwrap(),
        observation_source: "source:events".to_owned(),
        observation_clock: "clock:events".to_owned(),
        observation_revision: "1".to_owned(),
    }
}

#[trace("TC-1081", "FR-251-AC-46")]
#[test]
fn source_field_component_checks_use_original_checked_dense_and_tick_facts() {
    let selected_source = Identity::new("source:events");
    let clock = clock::Selection::new(Identity::new("clock:events"), Identity::new("1"));
    for digital in [false, true] {
        let (source, claim) = checked_source(digital);
        let original = row(&source, claim);
        assert_eq!(
            check_time_source_fields(&source, &original, &selected_source, &clock),
            Ok(())
        );
        for (candidate, expected) in [
            (
                {
                    let mut r = original.clone();
                    r.model.push_str("_foreign");
                    r
                },
                ProgressSelectionRefusalCode::Model,
            ),
            (
                {
                    let mut r = original.clone();
                    r.unit = "s".to_owned();
                    r
                },
                ProgressSelectionRefusalCode::Unit,
            ),
            (
                {
                    let mut r = original.clone();
                    r.observation_source.push_str("_foreign");
                    r
                },
                ProgressSelectionRefusalCode::Source,
            ),
            (
                {
                    let mut r = original.clone();
                    r.observation_clock.push_str("_foreign");
                    r
                },
                ProgressSelectionRefusalCode::Clock,
            ),
            (
                {
                    let mut r = original.clone();
                    r.observation_revision = "2".to_owned();
                    r
                },
                ProgressSelectionRefusalCode::Clock,
            ),
        ] {
            assert_eq!(
                check_time_source_fields(&source, &candidate, &selected_source, &clock),
                Err(expected)
            );
        }
        let mut changed_kind = original.clone();
        changed_kind.source = if digital {
            progress::TimeSource::Dense
        } else {
            {
                let (source, claim) = checked_source(true);
                row(&source, claim).source
            }
        };
        assert_eq!(
            check_time_source_fields(&source, &changed_kind, &selected_source, &clock),
            Err(ProgressSelectionRefusalCode::TimeSource)
        );
        if digital {
            for field in 0..3 {
                let mut changed = original.clone();
                if let progress::TimeSource::Tick {
                    operation,
                    period_numerator,
                    period_denominator,
                    ..
                } = &mut changed.source
                {
                    match field {
                        0 => operation.push_str("_foreign"),
                        1 => *period_numerator = "3".to_owned(),
                        _ => *period_denominator = "2".to_owned(),
                    }
                }
                assert_eq!(
                    check_time_source_fields(&source, &changed, &selected_source, &clock),
                    Err(ProgressSelectionRefusalCode::TimeSource)
                );
            }
        }
    }
}
