// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043-AC-15, AC-16: duplicate-preserving valuation evidence bound to the
//! checked declaration occurrence, position, anchor, capture instance,
//! revision and operand, over the recipe `Due` clause of the handoff writer.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_package::checked_views::{temporal_subject, ClauseOccurrence, TemporalSubjectView};
use quire_contract_model::{CheckedNodeId, CheckedOccurrenceRole};
use quire_spec_language::temporal::{
    ClockBinding, Closure, Completeness, Execution, Position, Refusal, Trace, ValuationCause,
    ValuationEvidence, ValuationInstance, ValuationRow, EVENT_POSITION,
};

use super::checked_views::recipe_package;

const ANCHOR: &str = "anchor:one";
const CAPTURES: &str = "captures:one";
const REVISION: &str = "revision:7";

fn trace_of(positions: usize) -> Trace {
    Trace {
        clock: ClockBinding {
            name: "orders".into(),
            profile_identity: EVENT_POSITION.into(),
            parameters: BTreeMap::new(),
        },
        positions: (0..positions)
            .map(|at| Position {
                coordinate: i64::try_from(at).expect("a small index"),
                order: None,
                valuations: BTreeMap::new(),
                instance_valuations: BTreeMap::new(),
            })
            .collect(),
        anchor: ANCHOR.into(),
        triggers: Vec::new(),
        trigger_evidence: quire_spec_language::temporal::Evidence::Admitted,
        trigger_scope: Closure::Closed,
        decision_scope: Closure::Closed,
        surrounding_execution: Closure::Closed,
        execution: Execution::Completed,
        completeness: Completeness::Complete,
        authoritative_origin: true,
        watermark: 1,
        evicted: Vec::new(),
    }
}

const INSTANCE: ValuationInstance<'static> = ValuationInstance {
    anchor: ANCHOR,
    capture_instance: CAPTURES,
};

fn row(
    subject: &ClauseOccurrence,
    position: usize,
    operand: &CheckedNodeId,
    value: bool,
) -> ValuationRow {
    ValuationRow {
        subject: subject.clone(),
        position,
        anchor: ANCHOR.into(),
        capture_instance: CAPTURES.into(),
        revision: REVISION.into(),
        operand: operand.clone(),
        value,
    }
}

/// Run `check` over the recipe package's first clause view, its one
/// occurrence, its first `holds` operand and a node that is no operand.
fn with_clause(
    check: impl FnOnce(&TemporalSubjectView<'_>, &ClauseOccurrence, &CheckedNodeId, &CheckedNodeId),
) {
    let (package, sidecar) = recipe_package("event_position");
    let first = &sidecar["clauses"][0]["node"];
    let id: CheckedNodeId = serde_json::from_value(first.clone()).expect("a node id");
    let view = temporal_subject(&package, &id).expect("the clause reads");
    let occurrence = view.occurrences()[0].clone();
    let operand = view.predicate_leaves()[0].clone();
    assert_ne!(*view.over(), operand);
    check(&view, &occurrence, &operand, view.over());
}

/// FR-043-AC-15: three rows for one operand at one position with identical
/// anchor, capture instance and revision stay three rows in admission order,
/// two positions stay two rows, and the retained multiset equals the input.
#[trace("FR-043-AC-15")]
#[test]
fn duplicate_rows_are_retained_in_admission_order() {
    with_clause(|view, occurrence, operand, _| {
        let input = vec![
            row(occurrence, 0, operand, true),
            row(occurrence, 0, operand, true),
            row(occurrence, 1, operand, false),
            row(occurrence, 0, operand, true),
        ];
        let evidence =
            ValuationEvidence::admit(view, &trace_of(2), INSTANCE, REVISION, input.clone())
                .expect("every row is bound to values the evaluation holds");
        assert_eq!(evidence.rows(), input.as_slice());
        let at_zero = evidence
            .rows()
            .iter()
            .filter(|kept| kept.position == 0)
            .count();
        let at_one = evidence
            .rows()
            .iter()
            .filter(|kept| kept.position == 1)
            .count();
        assert_eq!((at_zero, at_one), (3, 1));
        assert_eq!(evidence.value_of(occurrence, 0, operand), Ok(true));
        assert_eq!(evidence.value_of(occurrence, 1, operand), Ok(false));
    });
}

/// The spine-model unit with two structurally equal temporal clauses, which
/// share one checked node and hold two occurrences of it.
fn twin_package() -> quire_contract_model::CheckedPackageV2 {
    let clause = |name: &str| {
        format!(
            "temporal {name} using t over (g: M::Gadget) clock \"steps\" on origin \
             {{ eventually[3,*] holds(g.code = 1) }}\n"
        )
    };
    let unit = crate::support::spine_model::unit().replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\nprofile t = \"quire.temporal.infinite-trace/v1\";\n",
    );
    let source = format!("{unit}{}{}", clause("Alpha"), clause("Beta"));
    let document = crate::support::spine_model::DOCUMENT;
    let bytes = crate::support::front_end::emitted(
        qsl_foundation::SourceIdentity::new("local", "source:twins"),
        "twins.native",
        source.as_bytes(),
        &qsl_semantics::model::intake::package_input([document]).unwrap(),
        &qsl_replay::spine::DependencyInput::default(),
    )
    .expect("the twin clauses emit")
    .package()
    .bytes()
    .to_vec();
    super::emit_checked_v2::read_back(&bytes, &[document])
}

/// FR-043-AC-15: rows over two occurrences of one declaration that share a
/// checked node and formula stay distinct under their respective keys.
#[trace("FR-043-AC-15")]
#[test]
fn rows_of_two_occurrences_sharing_a_node_keep_their_own_keys() {
    let package = twin_package();
    let id = package
        .graph()
        .nodes
        .iter()
        .zip(package.node_kinds())
        .find(|(_, kind)| {
            **kind
                == quire_contract_model::CheckedNodeKind::Temporal(
                    quire_contract_model::TemporalForm::TemporalClause,
                )
        })
        .map(|(node, _)| node.node_id.clone())
        .expect("a temporal clause");
    let view = temporal_subject(&package, &id).expect("the shared clause reads");
    let [alpha, beta] = view.occurrences() else {
        panic!("two occurrences: {:?}", view.occurrences());
    };
    let operand = view.predicate_leaves()[0].clone();
    let evidence = ValuationEvidence::admit(
        &view,
        &trace_of(1),
        INSTANCE,
        REVISION,
        vec![
            row(alpha, 0, &operand, true),
            row(beta, 0, &operand, false),
            row(alpha, 0, &operand, true),
        ],
    )
    .expect("both keys are declared");
    assert_eq!(evidence.rows().len(), 3);
    assert_eq!(evidence.value_of(alpha, 0, &operand), Ok(true));
    assert_eq!(evidence.value_of(beta, 0, &operand), Ok(false));
}

/// FR-043-AC-16: each single mutation of an otherwise valid row refuses with
/// `Refusal::Valuation` naming its cause and the offending row.
#[trace("FR-043-AC-16")]
#[test]
fn each_single_mutation_of_a_valid_row_is_refused_with_its_cause() {
    with_clause(|view, occurrence, operand, not_an_operand| {
        let valid = row(occurrence, 0, operand, true);
        let mutations: [(ValuationCause, ValuationRow); 6] = [
            (
                ValuationCause::OccurrenceNotDeclared,
                ValuationRow {
                    subject: ClauseOccurrence {
                        role: CheckedOccurrenceRole::Declaration,
                        ..occurrence.clone()
                    },
                    ..valid.clone()
                },
            ),
            (
                ValuationCause::OperandNotInFormula,
                ValuationRow {
                    operand: not_an_operand.clone(),
                    ..valid.clone()
                },
            ),
            (
                ValuationCause::PositionNotAdmitted,
                ValuationRow {
                    position: 2,
                    ..valid.clone()
                },
            ),
            (
                ValuationCause::AnchorMismatch,
                ValuationRow {
                    anchor: "anchor:two".into(),
                    ..valid.clone()
                },
            ),
            (
                ValuationCause::CaptureInstanceMismatch,
                ValuationRow {
                    capture_instance: "captures:two".into(),
                    ..valid.clone()
                },
            ),
            (
                ValuationCause::RevisionMismatch,
                ValuationRow {
                    revision: "revision:6".into(),
                    ..valid.clone()
                },
            ),
        ];
        for (cause, mutated) in mutations {
            let refused = ValuationEvidence::admit(
                view,
                &trace_of(2),
                INSTANCE,
                REVISION,
                vec![valid.clone(), mutated],
            );
            assert_eq!(
                refused,
                Err(Refusal::Valuation { cause, row: 1 }),
                "{cause:?}"
            );
        }
        // The valid row alone is admitted: each refusal above is the mutation's.
        assert!(
            ValuationEvidence::admit(view, &trace_of(2), INSTANCE, REVISION, vec![valid]).is_ok()
        );
    });
}

/// FR-043-AC-16: omitting a required row is missing-valuation
/// incompleteness, never `false`, and rows that disagree establish nothing.
#[trace("FR-043-AC-16")]
#[test]
fn an_omitted_or_disagreeing_row_establishes_no_value() {
    with_clause(|view, occurrence, operand, _| {
        let evidence = ValuationEvidence::admit(
            view,
            &trace_of(2),
            INSTANCE,
            REVISION,
            vec![
                row(occurrence, 0, operand, true),
                row(occurrence, 1, operand, true),
                row(occurrence, 1, operand, false),
            ],
        )
        .expect("admitted");
        assert_eq!(evidence.value_of(occurrence, 0, operand), Ok(true));
        for position in [1, 5] {
            let missing = evidence
                .value_of(occurrence, position, operand)
                .expect_err("no single established value");
            assert_eq!(missing.position, position);
            assert_eq!(missing.operand, *operand);
        }
    });
}
