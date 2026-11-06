// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-251-AC-43 (TC-1081 step 27): a plan subject's clause node and claim
//! ordinal select the clause declaration and claim occurrence the plan-root
//! package holds, after every rename the plan-root writer performs.

use quire_contract_model::{CheckedNodeId, ClockConfiguration, TemporalBinding};

use super::*;
use crate::emit::monitor_plan::carry_rows;
use crate::emit::{emit_monitor_plan_over, MonitorPlanWire, TemporalBindingRows};

/// The id the writer renames `id` to in these tests: a different package.
fn renamed(id: &CheckedNodeId) -> CheckedNodeId {
    CheckedNodeId {
        package: format!("renamed:{}", id.package).into(),
        path: id.path.clone(),
        ordinal: id.ordinal,
    }
}

const CLAUSES: &str = "temporal C using tt over (c: Config::ConfigVersion) clock \"model-time\" on origin { always holds(c.versionNumber = 2) }\n\
    temporal D using tt over (c: Config::ConfigVersion) clock \"model-time\" on origin { eventually holds(c.versionNumber = 3) }\n\
    record State { now: Integer; }\n\
    record Inputs { x: Integer; }\n\
    record Scope { y: Integer; }\n\
    record Result { lab: String; }\n\
    function results using v(): Sequence<Result> pure { sequence[Result { lab: \"a\" }] }\n\
    function e0 using v(s: State): State pure { s }\n\
    function e1 using v(s: State, i: Inputs): State pure { s }\n\
    function e2 using v(s: State, p: Scope): State pure { s }\n\
    function e3 using v(s: State, g: Integer): State pure { s }\n";

/// A unit with two temporal clauses, emitted, and the plan root written
/// over it for the clause at `selected`; each clause's `(node, claim
/// ordinal)` subject alongside.
struct Rooted {
    base: Emission,
    rooted: Emission,
    subjects: Vec<(NodeKey, u64, String)>,
}

fn rooted(selected: usize) -> Rooted {
    use super::admission_corpus::config_version_unit;

    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]).unwrap();
    let unit = super::clock_places::clocked_unit(CLAUSES).replace(
        "profile t = \"quire.temporal.complete/v1\";\n",
        "profile t = \"quire.temporal.complete/v1\";\nprofile tt = \"quire.temporal.timed/v1\";\n",
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    let package = CheckedPackage::link(graph);
    let base = emit_checked(&package).expect("the package emits");
    let mut evidence = base.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("a fresh evidence key");
    let clauses = package.graph().temporal_clauses();
    let subjects: Vec<(NodeKey, u64, String)> = clauses
        .iter()
        .map(|c| {
            (
                c.identity(),
                c.claim().ordinal(),
                c.clause().name().to_owned(),
            )
        })
        .collect();
    let key = clauses[selected].identity();
    let formula = clauses[selected].clause().formula().clone();
    let rate = quire_exact::Rational::new(1_i64.into(), 500_i64.into()).unwrap();
    let zero = quire_exact::Rational::new(0_i64.into(), 1_i64.into()).unwrap();
    let wire_plan = MonitorPlanWire {
        clause: &key,
        formula: &formula,
        counter: "counter",
        counter_width_bits: 12,
        max_reading_gap: 3600,
        max_event_rate: &rate,
        uncertainty: &zero,
        buffers: &[],
        tick_unit: "ms",
        activation_capacity: 8,
        state: "State",
        inputs: "Inputs",
        scope: "Scope",
        result: "Result",
        entries: ["e0", "e1", "e2", "e3"],
        renamed_records: &[],
        renames: &[],
    };
    let rooted = emit_monitor_plan_over(
        &base,
        qsl_semantics::library::LibraryName::new(base.package().name()).unwrap(),
        &evidence,
        &wire_plan,
        crate::V2ReadLimits::fixture(),
        crate::EmitLimits::fixture(),
    )
    .expect("the plan root is written");
    Rooted {
        base,
        rooted,
        subjects,
    }
}

/// The emission's selection of a subject: the declaration its temporal
/// binding rows return for the clause node and claim ordinal.
fn selected<'e>(
    emission: &'e Emission,
    subject: &(NodeKey, u64, String),
) -> Option<&'e CheckedNodeId> {
    emission
        .temporal_binding_rows()
        .declaration(&subject.0, subject.1)
}

/// FR-251-AC-43: over a unit with two clauses, each subject selects its own
/// clause declaration in the plan-root package written:
/// the declaration the plan root's `clause` binding names for the
/// selected clause, and the declaration of that ordinal's name row.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn each_subject_selects_its_own_clause_declaration_in_the_written_plan_root() {
    let mut chosen = Vec::new();
    for at in 0..2 {
        let fixture = rooted(at);
        assert_eq!(fixture.subjects.len(), 2, "two clauses, one claim each");
        assert_eq!(fixture.subjects[0].1, 0);
        assert_eq!(fixture.subjects[1].1, 0);
        let written = wire(&fixture.rooted);
        let plan = nodes(&written)
            .iter()
            .find(|n| n["semantic_form"] == "monitor_plan")
            .expect("the plan root");
        let clause: CheckedNodeId = serde_json::from_value(
            super::clock_places::member(&plan["body"], "clause")["target"].clone(),
        )
        .expect("a node id");
        let names = written["temporal_clause_names"]
            .as_array()
            .expect("name rows");
        for (index, subject) in fixture.subjects.iter().enumerate() {
            let declaration = selected(&fixture.rooted, subject).expect("the subject selects");
            let row = names
                .iter()
                .find(|row| {
                    row["declaration"] == json!(declaration) && row["occurrence"] == subject.1
                })
                .expect("the selected declaration has a name row at the ordinal");
            assert_eq!(
                row["name"],
                subject.2.as_str(),
                "the row names the subject's clause"
            );
            assert_eq!(index == at, clause == *declaration);
        }
        let own = selected(&fixture.rooted, &fixture.subjects[at]).unwrap();
        let other = selected(&fixture.rooted, &fixture.subjects[1 - at]).unwrap();
        assert_eq!(&clause, own);
        assert_ne!(own, other, "another clause is never selected");
        chosen.push(own.clone());
    }
    assert_ne!(chosen[0], chosen[1]);
}

/// FR-251-AC-43: a subject whose clause node or claim ordinal the emitted
/// package does not hold selects nothing, not another occurrence.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn a_subject_the_package_does_not_hold_selects_nothing() {
    let fixture = rooted(0);
    let (key, ordinal, name) = fixture.subjects[0].clone();
    assert!(selected(&fixture.rooted, &(key.clone(), ordinal + 1, name.clone())).is_none());
    let other = fixture.subjects[1].0.clone();
    assert_ne!(key, other);
    assert!(selected(&fixture.rooted, &(other, ordinal + 1, name)).is_none());
}

/// FR-251-AC-43: a clause node the emitted package does not hold selects
/// nothing at any ordinal, including an ordinal another clause holds.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn a_clause_node_the_package_does_not_hold_selects_nothing() {
    let fixture = rooted(0);
    let held = &fixture.subjects[0];
    assert!(selected(&fixture.rooted, held).is_some());
    let absent = fixture_type_key("Absent");
    for ordinal in [held.1, held.1 + 1] {
        assert!(selected(&fixture.rooted, &(absent.clone(), ordinal, held.2.clone())).is_none());
    }
}

fn row(declaration: &CheckedNodeId, occurrence: u64) -> TemporalBinding {
    TemporalBinding {
        declaration: declaration.clone(),
        occurrence,
        clock_name: "model-time".into(),
        clock: ClockConfiguration::EventPosition {
            sequence_authority: "authority".into(),
        },
    }
}

/// FR-251-AC-43: when the writer renames the id of one clause declaration,
/// that clause's selection returns the renamed id (every occurrence of it),
/// the other clause's selection is unchanged, and no selection returns an id
/// the renames removed.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn a_rekeyed_clause_declaration_is_selected_by_its_renamed_id() {
    let fixture = rooted(0);
    let (first, second) = (&fixture.subjects[0], &fixture.subjects[1]);
    let first_id = selected(&fixture.base, first).unwrap().clone();
    let second_id = selected(&fixture.base, second).unwrap().clone();
    // Two claim occurrences of the first clause, so a rename must move both.
    let mut base = fixture.base.clone();
    base.temporal_binding_rows = TemporalBindingRows {
        rows: vec![(first.0.clone(), row(&first_id, 0))],
        declarations: vec![
            (first.0.clone(), 0, first_id.clone()),
            (first.0.clone(), 1, first_id.clone()),
            (second.0.clone(), 0, second_id.clone()),
        ],
    };
    let rows = carry_rows(&base, &[(first_id.clone(), renamed(&first_id))]);
    let get = |key: &NodeKey, ordinal: u64| rows.temporal_binding_rows.declaration(key, ordinal);
    assert_eq!(get(&first.0, 0), Some(&renamed(&first_id)));
    assert_eq!(get(&first.0, 1), Some(&renamed(&first_id)));
    assert_eq!(
        get(&second.0, 0),
        Some(&second_id),
        "the other clause is unchanged"
    );
    assert_eq!(get(&second.0, 1), None);
    assert_eq!(
        rows.temporal_binding_rows
            .get(&first.0, 0)
            .unwrap()
            .declaration,
        renamed(&first_id)
    );
    for (_, _, declaration) in &rows.temporal_binding_rows.declarations {
        assert_ne!(declaration, &first_id, "the base id is gone");
    }
}

/// FR-251-AC-43: the writer's carry changes nothing when no rename names a
/// clause declaration.
#[trace("TC-1081", "FR-251-AC-43")]
#[test]
fn a_selection_no_rename_touches_is_carried_unchanged() {
    let fixture = rooted(1);
    let rows = carry_rows(&fixture.base, &[]);
    assert_eq!(
        rows.temporal_binding_rows,
        *fixture.base.temporal_binding_rows()
    );
    for subject in &fixture.subjects {
        assert_eq!(
            rows.temporal_binding_rows
                .declaration(&subject.0, subject.1),
            selected(&fixture.rooted, subject),
            "the plan root's selection equals the base package's"
        );
    }
}
