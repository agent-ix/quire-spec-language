// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 steps 8 and 9 (FR-093-AC-10, FR-093-AC-11): FR-093's text-leaf
//! walk, checked against its Recursive text-leaf vectors, read from this
//! repository's own FR-093 text at compile time.

use super::*;
use crate::value::definition::DefinitionRevision;

const FR_093: &str = include_str!(
    "../../../../../spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md"
);

/// FR-093's Recursive text-leaf vectors by name: `(key, preimage)`.
fn fr093_vectors() -> BTreeMap<String, (String, String)> {
    let vectors = spec_vectors(FR_093);
    assert_eq!(vectors.len(), 21, "FR-093 publishes 21 golden vectors");
    vectors
}

/// Assert `graph` holds FR-093 vector `name`, bytes and key.
fn assert_fr093(graph: &SemanticGraph, name: &str) {
    let (key, preimage) = &fr093_vectors()[name];
    let node = node_by_key(graph, key);
    assert_eq!(
        std::str::from_utf8(node.preimage()).expect("UTF-8"),
        preimage,
        "{name}: preimage bytes"
    );
}

/// QSpec's text definition that the Recursive text-leaf vectors name.
fn vector_text_definition() -> DefinitionReference {
    DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: "quire.value.text.unicode-17.0.0/v1".to_owned(),
        revision: DefinitionRevision {
            namespace: "quire-draft".to_owned(),
            value: "1-draft.1".to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "cd4a985a0d7d2f2b3d3625caee3787832c00c5244e805fb49e1c2c7075b9de5e".to_owned(),
    }
}

fn text(max: u64, profile: TextProfile) -> ValueType {
    ValueType::Text(TextType::new(0, max, profile).unwrap())
}

fn handle(label: &str) -> NodeKey {
    let mut digest = [0_u8; 32];
    for (slot, byte) in digest.iter_mut().zip(label.bytes()) {
        *slot = byte;
    }
    NodeKey::from_digest(digest)
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

/// `record Node { label: Text[0, 8; binary-utf8]; next?: Node; }`.
fn node_record() -> CompositeDeclaration {
    record(
        "Node",
        vec![
            required("label", text(8, TextProfile::BinaryUtf8)),
            optional("next", ValueType::Composite(handle("Node"))),
        ],
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

/// `eq`, `has`, `eqa` and `eqo` of FR-093's Recursive text-leaf vectors.
fn vector_functions() -> Vec<FunctionDeclaration> {
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
            Expression::Contains {
                collection: Box::new(name_expr("s")),
                item: Box::new(name_expr("b")),
            },
        ),
        equality("eqa", ("x", "y"), named("A")),
        equality("eqo", ("a", "b"), option_node),
    ]
}

/// A package declaring `records` and holding `functions`, under `lock`.
fn package(
    records: Vec<CompositeDeclaration>,
    functions: Vec<FunctionDeclaration>,
    lock: LockEvidence,
) -> PackageDeclarations {
    PackageDeclarations {
        types: TypeEnvironment::new(records, []).expect("FR-143 admits the records"),
        functions,
        lock_evidence: lock,
        ..PackageDeclarations::new(fixture_owner())
    }
}

fn vector_lock() -> LockEvidence {
    LockEvidence::default().with_text_profile(vector_text_definition())
}

/// TC-415 step 8 (FR-093-AC-10): `eq`, `has`, `eqa` and `eqo` check, and
/// their nodes key to the Recursive text-leaf vectors, with `A` declared
/// before `B` and after it.
#[trace("FR-093-AC-10", "TC-415")]
#[test]
fn recursive_text_leaf_vectors_check_and_key() {
    let all = [
        "T13", "T14", "G16", "G17", "G18", "G19", "G20", "G21", "S4", "S5", "P10", "P11", "P12",
        "P13", "P14", "P15", "P16", "E14", "E15", "E16", "E17",
    ];
    let mut keys = Vec::new();
    for records in [
        vec![node_record(), record_a(), record_b()],
        vec![node_record(), record_b(), record_a()],
    ] {
        let checked = package(records, vector_functions(), vector_lock())
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the vector functions check: {refusals:?}"));
        let graph = checked.semantic_graph();
        for vector in all {
            assert_fr093(graph, vector);
        }
        keys.push(graph.nodes().map(SemanticNode::key).collect::<Vec<_>>());
    }
    assert_eq!(keys[0], keys[1], "declaring B before A gives the same keys");
    // Every vector FR-093 publishes is asserted above.
    assert_eq!(fr093_vectors().len(), all.len());
}

/// The leaf paths of the one structural equality of `records`' `R`-typed
/// `eq`, each joined by `/`, with its mode or `recursion`.
fn equality_leaves(records: Vec<CompositeDeclaration>, compared: &str) -> Vec<String> {
    let checked = package(
        records,
        vec![equality("eq", ("a", "b"), named(compared))],
        vector_lock(),
    )
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
        Expression::Contains {
            collection: Box::new(name_expr("s")),
            item: Box::new(name_expr("b")),
        },
    );
    let checked = package(vec![list()], vec![has], vector_lock())
        .check(CheckingLimits::default())
        .expect("contains over List checks");
    let contains = application(checked.semantic_graph(), "quire.op.collection.contains");
    assert_eq!(contains["body"]["operation"]["leaves"], json!([]));
}

/// FR-092 "Recursion groups" (FR-093-AC-10's P15 and P16): `Option<Node>`
/// is G17 wherever it is named, whichever function names it first, and
/// the vectors key the same under every order of the functions.
#[trace("FR-093-AC-10", "TC-415")]
#[test]
fn an_in_group_type_is_its_member_in_every_declaration_order() {
    let keys_of = |functions: Vec<FunctionDeclaration>| {
        let checked = package(
            vec![node_record(), record_a(), record_b()],
            functions,
            vector_lock(),
        )
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the functions check: {refusals:?}"));
        let graph = checked.semantic_graph();
        for vector in ["G17", "P15", "P16"] {
            assert_fr093(graph, vector);
        }
        // P15 and P16 name their type by key, so their preimage bytes pin
        // both parameters to G17; no second `Option<Node>` names G16.
        let vectors = fr093_vectors();
        let g17 = node_by_key(graph, &vectors["G17"].0).key();
        let g16 = vectors["G16"].0.as_bytes();
        let options = graph
            .nodes()
            .filter(|node| {
                node.semantic_form() == "option"
                    && (node.key() == g17 || node.preimage().windows(g16.len()).any(|w| w == g16))
            })
            .map(SemanticNode::key)
            .collect::<Vec<_>>();
        assert_eq!(options, [g17]);
        graph.nodes().map(SemanticNode::key).collect::<Vec<_>>()
    };
    let eqo = || vector_functions().remove(3);
    keys_of(vec![eqo()]);
    keys_of(vec![eqo(), vector_functions().remove(0)]);
    let forward = keys_of(vector_functions());
    let mut reversed = vector_functions();
    reversed.reverse();
    assert_eq!(keys_of(reversed), forward);
    let mut rotated = vector_functions();
    rotated.rotate_left(2);
    assert_eq!(keys_of(rotated), forward);
}

/// FR-092 G8: a `Sequence<Tree>` named after `Tree`'s group is keyed, with
/// any bound, is over the group's `sequence` member, not a second
/// `sequence` node.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn a_collection_over_a_recursive_record_reuses_the_group_base() {
    let tree = record(
        "Tree",
        vec![required(
            "kids",
            sequence(ValueType::Composite(handle("Tree")), Some((0, 3))),
        )],
    );
    let trees = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![named("Tree")])
        .with_bounds(vec!["0".into(), "5".into()]);
    let wide = function(
        "wide",
        &[("s", trees)],
        boolean(),
        None,
        Expression::Boolean(true),
    );
    let checked = package(vec![tree], vec![wide], vector_lock())
        .check(CheckingLimits::default())
        .expect("the parameter checks");
    let graph = checked.semantic_graph();
    let g8 = node_by_key(graph, &vector_key("G8")).key();
    let sequences: Vec<NodeKey> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "sequence")
        .map(SemanticNode::key)
        .collect();
    assert_eq!(sequences, [g8]);
    assert_eq!(
        parameter_named(graph, "s")
            .semantic_type()
            .and_then(|bounds| graph.node(bounds))
            .and_then(SemanticNode::semantic_type),
        Some(g8)
    );
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
    lock: LockEvidence,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    package(
        records,
        vec![equality("eq", ("a", "b"), named(compared))],
        lock,
    )
    .check(limits)
}

fn eq_over_node(
    lock: LockEvidence,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    eq_over(vec![node_record()], "Node", lock, limits)
}

fn node_limit(nodes: u64) -> CheckingLimits {
    CheckingLimits::new(nodes, crate::check::MAX_CHECKING_DEPTH).expect("the depth is allowed")
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

/// TC-415 step 9 (FR-093-AC-11): `eq` over `Node` with no text-profile
/// evidence refuses `missing-selection`; under a node limit that admits the
/// package's nodes but not also its two leaves, it refuses on the node
/// limit, before `missing-selection`, and yields no node.
#[trace("FR-093-AC-11", "TC-415")]
#[test]
fn a_leaf_walk_charges_the_node_limit_before_the_law_is_read() {
    let refusals = eq_over_node(LockEvidence::default(), CheckingLimits::default())
        .expect_err("no text-profile evidence refuses");
    assert_eq!(
        refusals[0].cause,
        CheckCause::MissingSelection {
            role: LawRole::TextProfile
        }
    );
    assert_eq!(refusals[0].cause.cause(), Some("missing-selection"));

    let admitted = smallest_limit(|limit| eq_over_node(vector_lock(), node_limit(limit)).is_ok());
    // The same `eq` over a record with no text leaf types the same nodes:
    // `Node`'s two leaves are the last two units.
    let plain = record(
        "Plain",
        vec![
            required("label", int(0, 9)),
            optional("next", ValueType::Composite(handle("Plain"))),
        ],
    );
    let typed = smallest_limit(|limit| {
        eq_over(
            vec![plain.clone()],
            "Plain",
            vector_lock(),
            node_limit(limit),
        )
        .is_ok()
    });
    assert_eq!(admitted, typed + 2);

    let exhausted = |refusals: Vec<CheckRefusal>, limit: u64| {
        assert!(
            matches!(
                refusals[0].cause,
                CheckCause::ResourceExhausted {
                    kind: CheckingLimitKind::Nodes,
                    limit: named,
                    ..
                } if named == limit
            ),
            "{refusals:?}"
        );
        assert_eq!(refusals[0].cause.cause(), Some("insufficient-next-charge"));
    };
    for limit in [admitted - 1, typed] {
        exhausted(
            eq_over_node(vector_lock(), node_limit(limit)).expect_err("the leaves pass the limit"),
            limit,
        );
        // The walk refuses before the missing law is read.
        exhausted(
            eq_over_node(LockEvidence::default(), node_limit(limit))
                .expect_err("the leaves pass the limit"),
            limit,
        );
    }
    // With the limit met, the missing law refuses instead.
    let refusals = eq_over_node(LockEvidence::default(), node_limit(admitted))
        .expect_err("no text-profile evidence refuses");
    assert_eq!(refusals[0].cause.cause(), Some("missing-selection"));
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

/// TC-420 steps 1 and 2 (NFR-011-M-1): at the default checking limits,
/// structural equality over a nine-record Text-reachable cluster refuses on
/// the node ceiling, naming the node limit kind and its default bound, and
/// yields no node; the six-record cluster's leaves fit.
#[trace("NFR-011-M-1", "TC-420")]
#[test]
fn a_text_reachable_cluster_refuses_on_the_default_node_ceiling() {
    let refusals = eq_over(
        text_cluster(9),
        "R0",
        vector_lock(),
        CheckingLimits::default(),
    )
    .expect_err("the nine-record cluster's leaves pass the default node ceiling");
    assert_eq!(
        refusals[0].cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Nodes,
            limit: 100_000,
        }
    );
    assert_eq!(refusals[0].cause.cause(), Some("insufficient-next-charge"));
    eq_over(
        text_cluster(6),
        "R0",
        vector_lock(),
        CheckingLimits::default(),
    )
    .expect("the six-record cluster checks at the default limits");
}

/// TC-420 step 3 (NFR-011-M-1 to NFR-011-M-4): a checked package and a
/// checked expression each record the ceilings they were checked under --
/// the defaults when the caller sets none, and a caller's ceilings as
/// given, above or below the defaults.
#[trace("NFR-011-M-1", "NFR-011-M-2", "NFR-011-M-3", "NFR-011-M-4", "TC-420")]
#[test]
fn a_checked_result_records_its_effective_limits() {
    let defaults = CheckingLimits::default();
    assert_eq!(
        (
            defaults.nodes(),
            defaults.depth(),
            defaults.input_bytes(),
            defaults.work_budget()
        ),
        (100_000, 128, 16_777_216, 16_777_216)
    );
    let checked = eq_over_node(vector_lock(), defaults).expect("eq over Node checks");
    assert_eq!(checked.effective_limits(), defaults);

    let raised = CheckingLimits::new(u64::MAX, 64)
        .expect("64 is within the maximum depth")
        .with_input_bytes(u64::MAX)
        .with_work_budget(u64::MAX);
    let lowered = node_limit(64)
        .with_work_budget(5_000)
        .with_input_bytes(4_096);
    for limits in [raised, lowered] {
        let checked = eq_over_node(vector_lock(), limits).expect("eq over Node checks");
        assert_eq!(checked.effective_limits(), limits);
        let expression = checked
            .check_expression(
                vec![("x".to_owned(), ValueType::Boolean)],
                &name_expr("x"),
                None,
                crate::check::CheckMode::Linked,
                limits,
            )
            .expect("a Boolean name checks");
        assert_eq!(expression.effective_limits(), limits);
    }
}

/// TC-420 step 3 (NFR-011-M-3, NFR-011-M-4): `CheckingLimits::new` sets the
/// node and depth ceilings and keeps the default input-byte and work
/// ceilings.
#[trace("NFR-011-M-3", "NFR-011-M-4", "TC-420")]
#[test]
fn new_keeps_the_default_byte_and_work_ceilings() {
    let limits = CheckingLimits::new(7, 9).expect("9 is within the maximum depth");
    assert_eq!(
        (
            limits.nodes(),
            limits.depth(),
            limits.input_bytes(),
            limits.work_budget()
        ),
        (7, 9, 16_777_216, 16_777_216)
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

/// TC-420 step 5 (NFR-011-M-4): a leaf list whose count fits the node
/// ceiling but whose paths are long -- 65,536 text leaves under 256-byte
/// field names, each path 35 segments -- refuses at the default ceilings
/// on the work budget, which each leaf's materialized key bytes are
/// charged to, and yields no node. (The chain is one record long: a
/// 30-record chain overflows a debug test thread's 2 MiB stack in
/// composite lowering, before any leaf walk; `qsl-bench`'s `deep-wide`
/// probe runs the reviewer's 46-record chain in release.)
#[trace("NFR-011-M-4", "TC-420")]
#[test]
fn long_leaf_paths_refuse_on_the_default_work_budget() {
    let name = format!("n{}", "x".repeat(255));
    let refusals = eq_over(
        deep_wide(1, 17, &name),
        "C0",
        vector_lock(),
        CheckingLimits::default(),
    )
    .expect_err("the leaves' key bytes pass the default work budget");
    assert_eq!(
        refusals[0].cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::WorkBudget,
            limit: 16_777_216,
        }
    );
    // The same shape with one-letter names and a shorter tree fits.
    eq_over(
        deep_wide(4, 5, "n"),
        "C0",
        vector_lock(),
        CheckingLimits::default(),
    )
    .expect("sixteen short leaves check");
}
