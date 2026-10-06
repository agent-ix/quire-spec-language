// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-413 (FR-092-AC-7), TC-726 (FR-258-AC-2): a long chain of declared
//! composites, or a deeply nested type, checks at any length its limits
//! admit, and never overflows the stack. Each check runs on a spawned
//! thread with a 512 KiB stack, in whatever profile the suite runs in; a
//! debug build once aborted at a 30-record chain.

use std::collections::BTreeSet;

use super::*;

/// The stack each check runs on.
const STACK: usize = 512 * 1024;

fn handle(label: &str) -> NodeKey {
    NodeKey::from_digest(qsl_foundation::ByteDigest::of(label.as_bytes()).as_bytes())
}

/// `C0 .. C{length-1}`, each `record Ci { next: C{i+1}?; }`, the last one's
/// field into `record Leaf { label: Text[0, 8; nfc]; }`, declared in chain
/// order.
pub(super) fn chain(length: usize) -> Vec<CompositeDeclaration> {
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
    }
}

/// `function eq(a: T, b: T): Boolean { a = b }`, each `T` built by
/// `operand`.
fn equality_over(operand: fn() -> TypeForm) -> FunctionDeclaration {
    function(
        "eq",
        &[("a", operand()), ("b", operand())],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr("a"), name_expr("b")),
    )
}

/// The chain's head record type, `C0`.
fn c0() -> TypeForm {
    TypeForm::name("C0", SPAN)
}

/// Check a package declaring `records` and holding `functions` under
/// `limits`, on a [`STACK`]-byte thread.
pub(super) fn check_on_small_stack(
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
                ..PackageDeclarations::new(
                    fixture_source(),
                    qsl_foundation::IdentityLimits::default(),
                )
            }
            .check(limits)
        })
        .expect("the check thread spawns")
        .join()
        .expect("the check thread panicked instead of returning a result")
}

/// How many `record` nodes a checked package holds.
fn record_nodes(graph: &CheckedGraph) -> usize {
    graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "record")
        .count()
}

/// The reproduction: a 30-record chain of optional fields into one
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
        vec![equality_over(c0)],
        CheckingLimits::default(),
    )
    .unwrap_or_else(|refusals| panic!("the equality checks: {refusals:?}"));
    assert_eq!(record_nodes(&compared), 31);
}

/// A 1,000-record chain checks at the default limits, alone and under an
/// equality over its head whose text leaf's path has two segments per
/// record: no limit names a depth (ADR-030 D-1, FR-258).
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_1000_record_chain_checks_on_a_small_stack() {
    for functions in [Vec::new(), vec![equality_over(c0)]] {
        let checked = check_on_small_stack(chain(1_000), functions, CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("1,000 records check: {refusals:?}"));
        assert_eq!(record_nodes(&checked), 1_001);
    }
}

/// A 63-record cycle, `C62`'s field back into `C0`, checks on a small stack
/// as one recursion group holding all 63 records and
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

/// A walk that delegates every callback to `inner` and counts the visits
/// it enters, observing the text-leaf walk from outside.
struct Counting<'c, W> {
    inner: &'c mut W,
    entered: usize,
}

impl<W: quire_walk::Walk> quire_walk::Walk for Counting<'_, W> {
    type Node = W::Node;
    type Frame = W::Frame;
    type Stop = W::Stop;

    fn enter(
        &mut self,
        node: W::Node,
        children: &mut quire_walk::Children<'_, W::Node>,
    ) -> std::ops::ControlFlow<W::Stop, W::Frame> {
        self.entered += 1;
        self.inner.enter(node, children)
    }

    fn exit(&mut self, frame: W::Frame) -> std::ops::ControlFlow<W::Stop> {
        self.inner.exit(frame)
    }
}

/// FR-258 behaviour 3: the text-leaf walk's heap stack grows by a constant
/// per charged composite. The walk names each visit once and enters every
/// visit it names, so its stack never holds more entries than the visits it
/// enters. Over a 1,000-record chain each record is one entered type and
/// one entered field, and each record charges one work unit on entry.
#[trace("FR-258-AC-6", "TC-726")]
#[test]
fn the_text_leaf_walk_enters_two_visits_per_charged_record() {
    const RECORDS: usize = 1_000;
    let types = TypeEnvironment::new(chain(RECORDS), []).expect("FR-143 admits the chain");
    let location = Location::root(quire_semantic_value::location::Origin::Expression);
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut reach = BTreeMap::new();
    let mut walk = LeafWalk {
        types: &types,
        location: &location,
        meter: &mut meter,
        reach: &mut reach,
        stack: Vec::new(),
        prefixes: Vec::new(),
        leaves: Vec::new(),
        open: Vec::new(),
        entered_at: BTreeMap::new(),
        budget: u64::MAX,
        limit: u64::MAX,
    };
    let root = ValueType::Composite(handle("C0"));
    let mut counting = Counting {
        inner: &mut walk,
        entered: 0,
    };
    let walked = quire_walk::walk(
        &mut counting,
        LeafVisit::Type {
            segment: None,
            value_type: &root,
        },
    );
    assert!(walked.is_continue(), "the chain walks to its text leaf");
    let entered = counting.entered;
    assert_eq!(walk.leaves.len(), 1, "one text leaf");
    // The 1,000 records and `Leaf` are each charged one work unit; the
    // leaf's key bytes are charged on top.
    let charged =
        usize::try_from(meter.consumed(quire_exact::LimitKind::WorkUnits)).expect("a work count");
    assert!(charged > RECORDS, "each record is charged");
    // Each record and `Leaf`: its type and its field. Then the text type.
    assert_eq!(entered, 2 * (RECORDS + 1) + 1);
    assert!(entered <= 2 * charged + 1);
}

/// The path of the one text leaf of `graph`'s structural equality.
fn text_leaf_path(graph: &CheckedGraph) -> Vec<Json> {
    let node = application(graph.semantic_graph(), "quire.op.structural.eq");
    let leaves = node["body"]["operation"]["leaves"]
        .as_array()
        .expect("the equality's leaves");
    assert_eq!(leaves.len(), 1, "one text leaf");
    leaves[0]["path"].as_array().expect("a leaf path").clone()
}

/// The levels each TC-726 type nests.
const LEVELS: usize = 100_000;

/// Limits raised to fit a [`LEVELS`]-deep type.
fn raised() -> CheckingLimits {
    CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX)
}

/// TC-726 (FR-258-AC-2): on a 512 KiB stack, structural equality over a
/// parameter typed with 100,000 nested `Option`s around `Text[0, 8; nfc]`
/// lowers with one text leaf whose path has an `inner` segment per level.
#[trace("FR-258-AC-2", "TC-726")]
#[test]
fn a_100000_deep_option_lowers_its_text_leaf_on_a_small_stack() {
    let options = || {
        (0..LEVELS).fold(
            TypeForm::builtin(BuiltinType::Text, SPAN).with_bounds(vec![
                "0".into(),
                "8".into(),
                "nfc".into(),
            ]),
            |inner, _| TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![inner]),
        )
    };
    let checked = check_on_small_stack(Vec::new(), vec![equality_over(options)], raised())
        .unwrap_or_else(|refusals| panic!("the nested options check: {refusals:?}"));
    let path = text_leaf_path(&checked);
    assert_eq!(path.len(), LEVELS);
    assert!(path.iter().all(|segment| segment == "inner"));
}

/// TC-726 (FR-258-AC-2): on a 512 KiB stack, structural equality over a
/// chain of 100,000 records into a `Text[0, 8; nfc]` field lowers with one
/// text leaf whose path has a `field:next` and an `inner` segment per
/// record, then `field:label`.
#[trace("FR-258-AC-2", "TC-726")]
#[test]
fn a_100000_record_chain_lowers_its_text_leaf_on_a_small_stack() {
    let checked = check_on_small_stack(chain(LEVELS), vec![equality_over(c0)], raised())
        .unwrap_or_else(|refusals| panic!("the record chain checks: {refusals:?}"));
    let path = text_leaf_path(&checked);
    assert_eq!(path.len(), 2 * LEVELS + 1);
    assert_eq!(path.last(), Some(&Json::from("field:label")));
}
