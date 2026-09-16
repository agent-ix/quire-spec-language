// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;

const PLAN: &str = include_str!("../plan/Plan-013-complete-v1-delivery/plan.md");
const ACCEPTED_AGENT_A: &str = include_str!("fixtures/complete-v1-agent-a.txt");
const TASKS: &[(&str, &str, Option<&str>)] = &[
    (
        "Task-046",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-046-adopt-complete-v1.md"),
        None,
    ),
    (
        "Task-047",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-047-complete-source-packages.md"),
        Some("Task-046"),
    ),
    (
        "Task-048",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-048-exact-scalar-semantics.md"),
        Some("Task-047"),
    ),
    (
        "Task-049",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-049-composite-expression-semantics.md"),
        Some("Task-048"),
    ),
    (
        "Task-050",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-050-model-graph-semantics.md"),
        Some("Task-049"),
    ),
    (
        "Task-051",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-051-reference-runtime.md"),
        Some("Task-050"),
    ),
    (
        "Task-052",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-052-native-tooling.md"),
        Some("Task-051"),
    ),
    (
        "Task-053",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-053-wasm-parity.md"),
        Some("Task-052"),
    ),
    (
        "Task-054",
        include_str!("../plan/Plan-013-complete-v1-delivery/tasks/Task-054-complete-v1-qualification.md"),
        Some("Task-053"),
    ),
];

fn expected_ticket(capability: &str) -> &'static str {
    let (family, number) = capability
        .strip_prefix("V1-")
        .and_then(|value| value.split_once('-'))
        .expect("complete-V1 capability id");
    let number: u8 = number.parse().expect("numeric capability suffix");

    match (family, number) {
        ("SRC", 10 | 11 | 13 | 14) => "QSL #123",
        ("SRC", 1..=9 | 12 | 15) => "QSL #117",
        ("TYPE", 1..=9 | 30..=31) | ("EXPR", 5..=9) => "QSL #118",
        ("TYPE", 10..=20) | ("EXPR", 1..=4 | 10..=16 | 24..=26) | ("TOOL", 8) => "QSL #119",
        ("TYPE", 21..=29) | ("EXPR", 17..=22) => "QSL #120",
        ("EXPR", 23) => "QSL #123",
        ("TOOL", 3..=4) => "QSL #117",
        ("RUN", 1 | 10 | 13) => "QSL #121",
        ("TOOL", 1..=2 | 6..=7) => "QSL #122",
        ("TOOL", 9) => "WASM #6",
        _ => panic!("capability is outside Agent-A ownership: {capability}"),
    }
}

#[trace("TC-144", "FR-055-AC-1", "FR-055-AC-2", "FR-055-AC-3")]
#[test]
fn complete_v1_plan_allocates_every_agent_a_capability_once() {
    let mut capabilities = BTreeSet::new();
    let mut allocations = BTreeSet::new();
    let mut ticket_counts = BTreeMap::<&str, usize>::new();

    for line in PLAN.lines().filter(|line| line.starts_with("| V1-")) {
        let cells: Vec<_> = line
            .split('|')
            .map(str::trim)
            .filter(|cell| !cell.is_empty())
            .collect();
        assert_eq!(cells.len(), 5, "malformed capability allocation: {line}");

        let [capability, requirement, ticket, test, qualification] = cells.as_slice() else {
            unreachable!("the length assertion above fixes this shape")
        };
        assert!(
            capabilities.insert(*capability),
            "duplicate capability allocation: {capability}"
        );
        allocations.insert(cells.join("|"));
        assert_eq!(*ticket, expected_ticket(capability));
        assert!(requirement.starts_with("FR-"), "malformed owner: {line}");
        assert!(test.starts_with("TC-"), "malformed test case: {line}");
        assert_eq!(*qualification, "QSL #123");
        *ticket_counts.entry(ticket).or_default() += 1;
    }

    let accepted_allocations: BTreeSet<_> = ACCEPTED_AGENT_A
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        allocations, accepted_allocations,
        "Plan-013 must remain the exact accepted QSpec Agent-A projection"
    );
    assert_eq!(capabilities.len(), 83);
    assert_eq!(ticket_counts.get("QSL #117"), Some(&13));
    assert_eq!(ticket_counts.get("QSL #118"), Some(&16));
    assert_eq!(ticket_counts.get("QSL #119"), Some(&26));
    assert_eq!(ticket_counts.get("QSL #120"), Some(&15));
    assert_eq!(ticket_counts.get("QSL #121"), Some(&3));
    assert_eq!(ticket_counts.get("QSL #122"), Some(&4));
    assert_eq!(ticket_counts.get("QSL #123"), Some(&5));
    assert_eq!(ticket_counts.get("WASM #6"), Some(&1));
}

#[trace("TC-144", "FR-055-AC-4", "FR-055-AC-5")]
#[test]
fn complete_v1_plan_preserves_the_serial_campaign_and_delivery_constraints() {
    assert!(PLAN.contains("7f9315774833c20fcfbcf425ffeb8e2619d1b84c"));
    assert!(PLAN.contains("18 complete, 36 partial and 29 missing"));
    assert!(PLAN.contains("16 complete, 38 partial and 29 missing"));
    assert!(PLAN.contains("Rust only"));
    assert!(PLAN.contains("target-codex-backends"));
    assert!(PLAN.contains("resources/native-v1"));
    assert!(PLAN.contains("No hosted CI"));

    for (id, task, predecessor) in TASKS {
        assert!(
            task.contains(&format!("id: {id}")),
            "wrong task identity: {id}"
        );
        assert!(task.contains("type: Task"), "untyped task: {id}");
        assert!(task.contains("type: references"), "unowned task: {id}");
        assert!(task.contains("type: verifies"), "unverified task: {id}");

        let dependencies: Vec<_> = task
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .filter_map(|pair| {
                let target = pair[0]
                    .trim()
                    .strip_prefix("- target: ix://agent-ix/quire-spec-language/")?;
                (pair[1].trim() == "type: depends_on" && target.starts_with("Task-"))
                    .then_some(target)
            })
            .collect();
        let expected: Vec<_> = predecessor.iter().copied().collect();
        assert_eq!(
            dependencies, expected,
            "{id} must have exactly its permitted serial predecessor"
        );
    }
}
