// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 9 (FR-093-AC-11): FR-093's text-leaf
//! walk over declared records, unions and recursion groups.

use super::*;

fn text(max: u64, profile: TextProfile) -> ValueType {
    ValueType::Text(TextType::new(0, max, profile).unwrap())
}

fn handle(label: &str) -> NodeKey {
    fixture_key(label)
}

fn record(name: &str, fields: Vec<FieldDeclaration>) -> CompositeDeclaration {
    CompositeDeclaration::new(handle(name), name, CompositeShape::Record(fields))
}

fn required(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Required)
}

fn optional(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Optional)
}

/// `record Node { label: Text[0, 8; binary-utf8]; next: Node?; }`.
fn node_record() -> CompositeDeclaration {
    record(
        "Node",
        vec![
            required("label", text(8, TextProfile::BinaryUtf8)),
            optional("next", ValueType::Composite(handle("Node"))),
        ],
    )
}

fn named(name: &str) -> TypeForm {
    TypeForm::name(name, SPAN)
}

/// `function <name>(<left>: T, <right>: T): Boolean { <left> = <right> }`.
fn equality(name: &str, (left, right): (&str, &str), operand: TypeForm) -> FunctionDeclaration {
    function(
        name,
        &[(left, operand.clone()), (right, operand)],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr(left), name_expr(right)),
    )
}

fn record_a() -> CompositeDeclaration {
    record(
        "A",
        vec![
            required("name", text(8, TextProfile::BinaryUtf8)),
            optional("b", ValueType::Composite(handle("B"))),
        ],
    )
}

fn record_b() -> CompositeDeclaration {
    record(
        "B",
        vec![
            required("tag", text(4, TextProfile::Nfc)),
            optional("a", ValueType::Composite(handle("A"))),
        ],
    )
}

/// `eq`, `has`, `eqa` and `eqo`: equality over a recursive record, a
/// sequence membership test, equality over a mutually recursive record, and
/// equality over `Option<Node>`.
fn recursive_functions() -> Vec<FunctionDeclaration> {
    let nodes = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![named("Node")])
        .with_bounds(vec!["0".into(), "3".into()]);
    let option_node =
        TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![named("Node")]);
    vec![
        equality("eq", ("a", "b"), named("Node")),
        function(
            "has",
            &[("s", nodes), ("b", named("Node"))],
            boolean(),
            None,
            Expression::contains(name_expr("s"), name_expr("b")),
        ),
        equality("eqa", ("x", "y"), named("A")),
        equality("eqo", ("a", "b"), option_node),
    ]
}

/// A package declaring `records` and holding `functions`.
fn package(
    records: Vec<CompositeDeclaration>,
    functions: Vec<FunctionDeclaration>,
) -> PackageDeclarations {
    PackageDeclarations {
        types: TypeEnvironment::new(records, []).expect("FR-143 admits the records"),
        functions,
        ..PackageDeclarations::new(fixture_source(), "fixture", qsl_foundation::IdentityLimits::default())
    }
}

/// The leaf paths of the one structural equality of `records`' `R`-typed
/// `eq`, each joined by `/`, with its mode or `recursion`.
fn equality_leaves(records: Vec<CompositeDeclaration>, compared: &str) -> Vec<String> {
    let checked = package(records, vec![equality("eq", ("a", "b"), named(compared))])
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("eq over {compared} checks: {refusals:?}"));
    let node = application(checked.semantic_graph(), "quire.op.structural.eq");
    node["body"]["operation"]["leaves"]
        .as_array()
        .expect("leaves")
        .iter()
        .map(|leaf| {
            let path: Vec<&str> = leaf["path"]
                .as_array()
                .expect("a path")
                .iter()
                .map(|segment| segment.as_str().expect("a segment"))
                .collect();
            let mode = leaf["mode"]["value"].as_str().unwrap_or("-");
            format!("{} ({mode})", path.join("/"))
        })
        .collect()
}

/// TC-415 step 9 (FR-093-AC-11): an optional field's leaf passes through
/// `inner`, as its `Option` twin's does; a recursion reaching a text type
/// ends in `recursion:d`, `d` counting the segments where the walk entered
/// the composite; one that reaches none adds no leaf.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn leaf_paths_follow_fr093_text_leaves() {
    let t64 = || text(64, TextProfile::Nfc);
    assert_eq!(
        equality_leaves(vec![record("R", vec![optional("t", t64())])], "R"),
        ["field:t/inner (nfc)"]
    );
    assert_eq!(
        equality_leaves(
            vec![record("S", vec![required("t", ValueType::option(t64()))])],
            "S"
        ),
        ["field:t/inner (nfc)"]
    );
    assert_eq!(
        equality_leaves(vec![record("W", vec![required("t", t64())])], "W"),
        ["field:t (nfc)"]
    );
    let tree2 = record(
        "Tree2",
        vec![
            required("label", text(8, TextProfile::BinaryUtf8)),
            required(
                "kids",
                sequence(ValueType::Composite(handle("Tree2")), Some((0, 3))),
            ),
        ],
    );
    assert_eq!(
        equality_leaves(vec![tree2], "Tree2"),
        [
            "field:label (binary-utf8)",
            "field:kids/inner/recursion:0 (-)"
        ]
    );
    let two = record(
        "Two",
        vec![
            required("x", ValueType::Composite(handle("Node"))),
            required("y", ValueType::Composite(handle("Node"))),
        ],
    );
    assert_eq!(
        equality_leaves(vec![node_record(), two], "Two"),
        [
            "field:x/field:label (binary-utf8)",
            "field:x/field:next/inner/recursion:1 (-)",
            "field:y/field:label (binary-utf8)",
            "field:y/field:next/inner/recursion:1 (-)",
        ]
    );
    // FR-092's `List`, which reaches no text type, carries no leaves, in
    // equality and in `contains`.
    let list = || {
        record(
            "List",
            vec![optional("next", ValueType::Composite(handle("List")))],
        )
    };
    assert!(equality_leaves(vec![list()], "List").is_empty());
    let lists = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![named("List")])
        .with_bounds(vec!["0".into(), "3".into()]);
    let has = function(
        "has",
        &[("s", lists), ("b", named("List"))],
        boolean(),
        None,
        Expression::contains(name_expr("s"), name_expr("b")),
    );
    let checked = package(vec![list()], vec![has])
        .check(CheckingLimits::default())
        .expect("contains over List checks");
    let contains = application(checked.semantic_graph(), "quire.op.collection.contains");
    assert_eq!(contains["body"]["operation"]["leaves"], json!([]));
}

/// A union `name` declaring `members`, each with its payload types.
fn union_declaration(name: &str, members: Vec<(&str, Vec<ValueType>)>) -> CompositeDeclaration {
    CompositeDeclaration::new(
        handle(name),
        name,
        CompositeShape::Union(
            members
                .into_iter()
                .map(|(member, payload)| {
                    quire_semantic_value::declaration::UnionMemberDeclaration::resolved(
                        quire_exact::Identifier::new(member).expect("a member identifier"),
                        payload,
                    )
                })
                .collect(),
        ),
    )
}

/// QSpec FR-440's "Structural leaves": every declared member is an anchor,
/// its text leaves at `member:<Name>` then `position:<i>`, `i` the index in
/// the member's payload; a cycle through a union is a recursion leaf when a
/// text type is reachable and adds nothing when none is.
#[trace("FR-093-AC-11", "TC-415", "QSpec-FR-440-AC-8")]
#[test]
fn union_leaf_paths_follow_member_then_position() {
    let t8 = || text(8, TextProfile::Nfc);
    let label = union_declaration(
        "Label",
        vec![
            ("Named", vec![t8()]),
            ("Tagged", vec![ValueType::Integer, t8()]),
            ("Empty", vec![]),
        ],
    );
    assert_eq!(
        equality_leaves(vec![label], "Label"),
        [
            "member:Named/position:0 (nfc)",
            "member:Tagged/position:1 (nfc)"
        ]
    );
    let chain = union_declaration(
        "Chain",
        vec![
            (
                "Link",
                vec![
                    t8(),
                    ValueType::option(ValueType::Composite(handle("Chain"))),
                ],
            ),
            ("End", vec![]),
        ],
    );
    assert_eq!(
        equality_leaves(vec![chain], "Chain"),
        [
            "member:Link/position:0 (nfc)",
            "member:Link/position:1/inner/recursion:0 (-)"
        ]
    );
    let list = union_declaration(
        "List",
        vec![
            (
                "Cons",
                vec![
                    ValueType::Integer,
                    ValueType::option(ValueType::Composite(handle("List"))),
                ],
            ),
            ("Nil", vec![]),
        ],
    );
    assert!(equality_leaves(vec![list], "List").is_empty());
}

/// FR-093 "Text leaves" rule 4: a composite from which no text type is
/// reachable is not entered, so a cluster of twelve text-free records that
/// each hold an optional field of every other adds no leaf and no walk, and
/// the compared record's one text field is its only leaf.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn a_text_free_recursive_cluster_is_not_walked() {
    let n = 12;
    let cluster: Vec<CompositeDeclaration> = (0..n)
        .map(|at| {
            record(
                &format!("R{at}"),
                (0..n)
                    .filter(|other| *other != at)
                    .map(|other| {
                        optional(
                            &format!("r{other}"),
                            ValueType::Composite(handle(&format!("R{other}"))),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    let mut records = cluster;
    records.push(record(
        "Top",
        vec![
            required("s", text(8, TextProfile::Nfc)),
            required("r", ValueType::Composite(handle("R0"))),
        ],
    ));
    // Under a small work budget too: each composite entered charges it.
    assert_eq!(equality_leaves(records, "Top"), ["field:s (nfc)"]);
}

/// `eq` over `compared`, one of `records`.
fn eq_over(
    records: Vec<CompositeDeclaration>,
    compared: &str,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    package(records, vec![equality("eq", ("a", "b"), named(compared))]).check(limits)
}

fn eq_over_node(limits: CheckingLimits) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    eq_over(vec![node_record()], "Node", limits)
}

fn node_limit(nodes: u64) -> CheckingLimits {
    CheckingLimits::new(nodes)
}

/// The smallest node limit `checks` passes under.
fn smallest_limit(checks: impl Fn(u64) -> bool) -> u64 {
    let (mut refused, mut admitted) = (0_u64, 4096_u64);
    assert!(checks(admitted));
    while admitted - refused > 1 {
        let middle = refused + (admitted - refused) / 2;
        if checks(middle) {
            admitted = middle;
        } else {
            refused = middle;
        }
    }
    admitted
}

/// TC-415 step 9 (FR-093-AC-11): `eq` over `Node` charges its two leaves
/// to the node limit: under a limit that admits the package's nodes but not
/// also its two leaves, it refuses on the node limit and yields no node.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn a_leaf_walk_charges_the_node_limit() {
    let admitted = smallest_limit(|limit| eq_over_node(node_limit(limit)).is_ok());
    // The same `eq` over a record with no text leaf types the same nodes:
    // `Node`'s two leaves are the last two units.
    let plain = record(
        "Plain",
        vec![
            required("label", int(0, 9)),
            optional("next", ValueType::Composite(handle("Plain"))),
        ],
    );
    let typed =
        smallest_limit(|limit| eq_over(vec![plain.clone()], "Plain", node_limit(limit)).is_ok());
    assert_eq!(admitted, typed + 2);

    let exhausted = |refusals: Vec<CheckRefusal>, limit: u64| {
        assert!(
            matches!(
                refusals[0].cause,
                CheckCause::ResourceExhausted(ref exceeded)
                    if exceeded.kind == CheckingLimitKind::Nodes && exceeded.limit == limit
            ),
            "{refusals:?}"
        );
        assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
        assert_eq!(refusals[0].cause.cause(), Some("node-count-exceeded"));
    };
    for limit in [admitted - 1, typed] {
        exhausted(
            eq_over_node(node_limit(limit)).expect_err("the leaves pass the limit"),
            limit,
        );
    }
}

/// `n` records `R0` .. `R{n-1}`, each with a `label` text field and an
/// optional field of every other record: every record reaches a text type,
/// so FR-093's leaf list over `R0` holds one leaf per simple path through
/// the cluster, on the order of `(n - 1)!` leaves.
fn text_cluster(n: usize) -> Vec<CompositeDeclaration> {
    (0..n)
        .map(|at| {
            let mut fields = vec![required("label", text(8, TextProfile::Nfc))];
            fields.extend((0..n).filter(|other| *other != at).map(|other| {
                optional(
                    &format!("r{other}"),
                    ValueType::Composite(handle(&format!("R{other}"))),
                )
            }));
            record(&format!("R{at}"), fields)
        })
        .collect()
}

/// TC-423 steps 1 and 2 (NFR-011-M-1): at the default checking limits,
/// structural equality over a nine-record Text-reachable cluster refuses on
/// the node ceiling, naming the node limit kind and its default bound, and
/// yields no node; the six-record cluster's leaves fit.
#[trace("NFR-011-M-1", "TC-423")]
#[test]
fn a_text_reachable_cluster_refuses_on_the_default_node_ceiling() {
    let refusals = eq_over(text_cluster(9), "R0", CheckingLimits::default())
        .expect_err("the nine-record cluster's leaves pass the default node ceiling");
    assert_eq!(
        refusals[0].cause,
        CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            limit: 100_000,
            actual: 100_001,
            region: None,
        }))
    );
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("node-count-exceeded"));
    eq_over(text_cluster(6), "R0", CheckingLimits::default())
        .expect("the six-record cluster checks at the default limits");
}

/// TC-423 step 3 (NFR-011-M-1 to NFR-011-M-4): a checked package and a
/// checked expression each record the ceilings they were checked under --
/// the defaults when the caller sets none, and a caller's ceilings as
/// given, above or below the defaults.
#[trace("NFR-011-M-1", "NFR-011-M-2", "NFR-011-M-3", "NFR-011-M-4", "TC-423")]
#[test]
fn a_checked_result_records_its_effective_limits() {
    let defaults = CheckingLimits::default();
    assert_eq!(
        (
            defaults.nodes(),
            defaults.input_bytes(),
            defaults.work_budget()
        ),
        (100_000, 16_777_216, 16_777_216)
    );
    let checked = eq_over_node(defaults).expect("eq over Node checks");
    assert_eq!(checked.effective_limits(), defaults);

    let raised = CheckingLimits::new(u64::MAX)
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    let lowered = node_limit(64)
        .with_work_budget(5_000)
        .with_input_bytes(4_096);
    for limits in [raised, lowered] {
        let checked = eq_over_node(limits).expect("eq over Node checks");
        assert_eq!(checked.effective_limits(), limits);
        let expression = checked
            .check_expression(
                vec![("x".to_owned(), ValueType::Boolean)],
                &name_expr("x"),
                None,
                quire_semantic_value::checking::CheckMode::Linked,
                limits,
            )
            .expect("a Boolean name checks");
        assert_eq!(expression.effective_limits(), limits);
    }
}

/// TC-423 step 3 (NFR-011-M-3, NFR-011-M-4): `CheckingLimits::new` sets the
/// node ceiling and keeps the default input-byte and work ceilings.
#[trace("NFR-011-M-3", "NFR-011-M-4", "TC-423")]
#[test]
fn new_keeps_the_default_byte_and_work_ceilings() {
    let limits = CheckingLimits::new(7);
    assert_eq!(
        (limits.nodes(), limits.input_bytes(), limits.work_budget()),
        (7, 16_777_216, 16_777_216)
    );
}

/// A `chain`-record chain of optional fields named `name` into a binary
/// tree of `levels` levels of optional fields, whose last level holds a
/// text field: `2^(levels - 1)` text leaves, each under a path of about
/// `2 * (chain + levels)` segments.
fn deep_wide(chain: usize, levels: usize, name: &str) -> Vec<CompositeDeclaration> {
    let link = |to: String| ValueType::Composite(handle(&to));
    let mut records: Vec<CompositeDeclaration> = (0..chain)
        .map(|at| {
            let next = if at + 1 < chain {
                format!("C{}", at + 1)
            } else {
                "T0".to_owned()
            };
            record(&format!("C{at}"), vec![optional(name, link(next))])
        })
        .collect();
    records.extend((0..levels).map(|level| {
        let fields = if level + 1 < levels {
            let next = format!("T{}", level + 1);
            vec![
                optional(&format!("{name}l"), link(next.clone())),
                optional(&format!("{name}r"), link(next)),
            ]
        } else {
            vec![required(name, text(8, TextProfile::Nfc))]
        };
        record(&format!("T{level}"), fields)
    }));
    records
}

/// TC-423 step 5 (NFR-011-M-4): a leaf list whose count fits the node
/// ceiling but whose paths are long -- 65,536 text leaves under 256-byte
/// field names, each path 35 segments -- refuses at the default ceilings
/// on the work budget, which each leaf's materialized key bytes are
/// charged to, and yields no node. (The chain is one record long: a
/// 30-record chain overflows a debug test thread's 2 MiB stack in
/// composite lowering, before any leaf walk; `qsl-bench`'s `deep-wide`
/// probe runs the reviewer's 46-record chain in release.)
#[trace("NFR-011-M-4", "TC-423")]
#[test]
fn long_leaf_paths_refuse_on_the_default_work_budget() {
    let name = format!("n{}", "x".repeat(255));
    let refusals = eq_over(deep_wide(1, 17, &name), "C0", CheckingLimits::default())
        .expect_err("the leaves' key bytes pass the default work budget");
    assert!(
        matches!(
            refusals[0].cause,
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.stage == CheckingStage::Typing
                    && exceeded.kind == CheckingLimitKind::WorkBudget
                    && exceeded.limit == 16_777_216
        ),
        "{:?}",
        refusals[0]
    );
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("work-budget-exceeded"));
    // The same shape with one-letter names and a shorter tree fits.
    eq_over(deep_wide(4, 5, "n"), "C0", CheckingLimits::default())
        .expect("sixteen short leaves check");
}

/// TC-415 step 8 (FR-093-AC-11): `eq`, `has`, `eqa` and `eqo` check over
/// recursive text-leaf records, and their nodes key the same with `A`
/// declared before `B` and after it.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn recursive_text_leaf_functions_check_and_key_in_either_order() {
    let mut keys = Vec::new();
    for records in [
        vec![node_record(), record_a(), record_b()],
        vec![node_record(), record_b(), record_a()],
    ] {
        let checked = package(records, recursive_functions())
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the recursive functions check: {refusals:?}"));
        keys.push(
            checked
                .semantic_graph()
                .nodes()
                .map(SemanticNode::key)
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(keys[0], keys[1], "declaring B before A gives the same keys");
}

/// FR-092 "Recursion groups" (FR-092-AC-11): `Option<Node>` is one node
/// wherever it is named, whichever function names it first, and the nodes key
/// the same under every order of the functions.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn an_in_group_type_is_its_member_in_every_declaration_order() {
    let keys_of = |functions: Vec<FunctionDeclaration>| {
        let checked = package(vec![node_record(), record_a(), record_b()], functions)
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the functions check: {refusals:?}"));
        let graph = checked.semantic_graph();
        let node_record = graph
            .nodes()
            .find(|node| {
                node.semantic_form() == "record"
                    && node.recursion().is_some()
                    && preimage(node)["declaration"]["qualified_name"] == json!(["Node"])
            })
            .expect("the graph holds record Node");
        let node_key = node_record.key().to_string();
        let options = graph
            .nodes()
            .filter(|node| {
                node.semantic_form() == "option"
                    && node.recursion().is_some()
                    && preimage(node)["body"]["members"][0]["target"]["digest"] == json!(node_key)
            })
            .count();
        assert_eq!(options, 1, "exactly one Option<Node> option node");
        graph.nodes().map(SemanticNode::key).collect::<Vec<_>>()
    };
    let eqo = || recursive_functions().remove(3);
    keys_of(vec![eqo()]);
    keys_of(vec![eqo(), recursive_functions().remove(0)]);
    let forward = keys_of(recursive_functions());
    let mut reversed = recursive_functions();
    reversed.reverse();
    assert_eq!(keys_of(reversed), forward);
    let mut rotated = recursive_functions();
    rotated.rotate_left(2);
    assert_eq!(keys_of(rotated), forward);
}
