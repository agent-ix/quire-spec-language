// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-413 (FR-092-AC-7, QSL-224): a long chain of declared composites
//! checks or refuses on the named depth limit, and never overflows the
//! stack. Each check runs on a spawned thread with a 2 MiB stack, the
//! default test-thread size, in whatever profile the suite runs in; before
//! QSL-224 a debug build aborted at a 30-record chain.

use std::collections::BTreeSet;

use super::*;
use crate::check::MAX_CHECKING_DEPTH;

/// The stack each check runs on.
const STACK: usize = 2 * 1024 * 1024;

fn handle(label: &str) -> NodeKey {
    NodeKey::from_digest(qsl_foundation::ByteDigest::of(label.as_bytes()).as_bytes())
}

/// `C0 .. C{length-1}`, each `record Ci { next?: C{i+1}; }`, the last one's
/// field into `record Leaf { label: Text[0, 8; nfc]; }`, declared in chain
/// order.
fn chain(length: usize) -> Vec<CompositeDeclaration> {
    let mut records: Vec<CompositeDeclaration> = (0..length)
        .map(|at| {
            let next = if at + 1 < length {
                format!("C{}", at + 1)
            } else {
                "Leaf".to_owned()
            };
            CompositeDeclaration::new(
                handle(&format!("C{at}")),
                format!("C{at}"),
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "next",
                    ValueType::Composite(handle(&next)),
                    Presence::Optional,
                )]),
            )
        })
        .collect();
    records.push(CompositeDeclaration::new(
        handle("Leaf"),
        "Leaf",
        CompositeShape::Record(vec![FieldDeclaration::new(
            "label",
            ValueType::Text(TextType::new(0, 8, TextProfile::Nfc).unwrap()),
            Presence::Required,
        )]),
    ));
    records
}

/// A text-profile definition for the equality's text leaf.
fn text_definition() -> DefinitionReference {
    DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: "unicode-text".to_owned(),
        revision: crate::value::definition::DefinitionRevision {
            namespace: "unicode".to_owned(),
            value: "17.0.0".to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "ab".repeat(32),
    }
}

/// `function eq(a: C0, b: C0): Boolean { a = b }`.
fn equality_over_head() -> FunctionDeclaration {
    function(
        "eq",
        &[
            ("a", TypeForm::name("C0", SPAN)),
            ("b", TypeForm::name("C0", SPAN)),
        ],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr("a"), name_expr("b")),
    )
}

/// Check a package declaring `records` and holding `functions` under
/// `limits`, on a [`STACK`]-byte thread.
fn check_on_small_stack(
    records: Vec<CompositeDeclaration>,
    functions: Vec<FunctionDeclaration>,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            PackageDeclarations {
                types: TypeEnvironment::new(records, []).expect("FR-143 admits the chain"),
                functions,
                lock_evidence: LockEvidence::default().with_text_profile(text_definition()),
                ..PackageDeclarations::new(fixture_source())
            }
            .check(limits)
        })
        .expect("the check thread spawns")
        .join()
        .expect("the check thread panicked instead of returning a result")
}

/// Every refusal is the depth limit `limit`, and there is at least one.
fn assert_depth_refusals(refusals: &[CheckRefusal], limit: u64) {
    assert!(!refusals.is_empty());
    for refusal in refusals {
        assert_eq!(
            refusal.cause,
            CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::Depth,
                limit,
                actual: u128::from(limit) + 1,
                region: None,
            })),
            "{refusal:?}"
        );
    }
}

/// How many `record` nodes a checked package holds.
fn record_nodes(graph: &CheckedGraph) -> usize {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "record")
        .count()
}

/// QSL-224's reproduction: a 30-record chain of optional fields into one
/// text field checks at the default limits, alone and under an equality
/// that walks its text leaf.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_30_record_optional_chain_into_text_checks_on_a_small_stack() {
    let alone = check_on_small_stack(chain(30), Vec::new(), CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the chain checks: {refusals:?}"));
    assert_eq!(record_nodes(&alone), 31);
    let compared = check_on_small_stack(
        chain(30),
        vec![equality_over_head()],
        CheckingLimits::default(),
    )
    .unwrap_or_else(|refusals| panic!("the equality checks: {refusals:?}"));
    assert_eq!(record_nodes(&compared), 31);
}

/// The longest chain the maximum depth admits under an equality over its
/// head, 63 records, checks: record `Ci` is reached at depth `2i`, `Leaf` at
/// 126 and its text field at 127. One more record puts the text field at
/// 129, past the limit, and refuses on it.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn the_maximum_depth_admits_a_63_record_chain_and_refuses_on_depth_past_it() {
    let limits = CheckingLimits::new(u64::MAX, MAX_CHECKING_DEPTH).expect("the maximum depth");
    let deepest = check_on_small_stack(chain(63), vec![equality_over_head()], limits)
        .unwrap_or_else(|refusals| panic!("63 records check: {refusals:?}"));
    assert_eq!(record_nodes(&deepest), 64);
    let refusals = check_on_small_stack(chain(64), vec![equality_over_head()], limits)
        .expect_err("64 records refuse at the head");
    assert_depth_refusals(&refusals, MAX_CHECKING_DEPTH);
}

/// A 1,000-record chain refuses on the named depth limit, never a stack
/// overflow: at the default limits, at the maximum depth with unlimited
/// nodes and work, and at a smaller caller-declared depth.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_1000_record_chain_refuses_on_the_depth_limit_on_a_small_stack() {
    let raised = CheckingLimits::new(u64::MAX, MAX_CHECKING_DEPTH)
        .expect("the maximum depth")
        .with_work_budget(u64::MAX);
    let narrow = CheckingLimits::new(u64::MAX, 16).expect("a smaller depth");
    for (limits, limit) in [
        (CheckingLimits::default(), MAX_CHECKING_DEPTH),
        (raised, MAX_CHECKING_DEPTH),
        (narrow, 16),
    ] {
        for functions in [Vec::new(), vec![equality_over_head()]] {
            let refusals = check_on_small_stack(chain(1_000), functions, limits)
                .expect_err("1,000 records refuse on depth");
            assert_depth_refusals(&refusals, limit);
        }
    }
}

/// A 63-record cycle, `C62`'s field back into `C0`, checks on a small stack
/// at the maximum depth as one recursion group holding all 63 records and
/// the `option` node over each.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_63_record_cycle_checks_as_one_group_on_a_small_stack() {
    let mut records = chain(63);
    records.pop();
    let last = records.pop().expect("the chain's last record");
    records.push(CompositeDeclaration::new(
        last.key(),
        "C62",
        CompositeShape::Record(vec![FieldDeclaration::new(
            "next",
            ValueType::Composite(handle("C0")),
            Presence::Optional,
        )]),
    ));
    let checked = check_on_small_stack(records, Vec::new(), CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the cycle checks: {refusals:?}"));
    let form_count = |form: &str| {
        checked
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == form)
            .count()
    };
    assert_eq!(form_count("record"), 63);
    assert_eq!(form_count("option"), 63);
    let members: Vec<&SemanticNode> = checked
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect();
    assert_eq!(members.len(), 126, "every record and option is a member");
    let groups: BTreeSet<[u8; 32]> = members
        .iter()
        .filter_map(|node| node.recursion().map(|recursion| *recursion.group()))
        .collect();
    assert_eq!(groups.len(), 1, "one recursion group");
    assert!(members.iter().all(|node| node
        .recursion()
        .is_some_and(|recursion| recursion.size() == 126)));
}
