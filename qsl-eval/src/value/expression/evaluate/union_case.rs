// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-322 and FR-323 at the S6a evaluator: union construction, `case` and
//! collections of unions.
//!
//! No checked package can hold a union until the FR-319 lowering exists, so
//! each body here is a hand-built [`CheckedBody`] run on a [`Machine`] over a
//! scope declaring the sealed fixture unions
//! (`qsl_semantics::check::union_fixture`). Every assertion reads a typed
//! outcome, charge or fault, never message text.

use super::*;
use ix_trace_rs::trace;
use qsl_semantics::check::union_fixture as fixture;
use qsl_semantics::check::{
    scope_with, CaseArm, CheckedBody, PackageDeclarations, SCALAR_LIMITS_UNLIMITED,
};
use quire_exact::{CollectionType, ScalarLimits};
use quire_semantic_value::checking::CheckingLimits;

struct World {
    scope: Scope,
    graph: CheckedGraph,
}

fn world() -> World {
    let graph = PackageDeclarations::new(
        qsl_semantics::check::fixture_source(),
        qsl_foundation::IdentityLimits::default(),
    )
    .check(CheckingLimits::default())
    .expect("the empty fixture package checks");
    World {
        scope: scope_with(fixture::types(), Vec::new()),
        graph,
    }
}

fn int(number: i64) -> Value {
    Value::Integer(Integer::from(number))
}

/// The nodes of a body being built, operands before the nodes naming them
/// and the root last.
#[derive(Default)]
struct Nodes(Vec<(NodeKind, ValueType)>);

impl Nodes {
    fn push(&mut self, kind: NodeKind, value_type: ValueType) -> NodeId {
        self.0.push((kind, value_type));
        CheckedBody::id_at(self.0.len() - 1)
    }

    fn literal(&mut self, value: Value, value_type: ValueType) -> NodeId {
        self.push(NodeKind::Literal(CheckedLiteral(value)), value_type)
    }

    fn int(&mut self, number: i64) -> NodeId {
        self.literal(int(number), ValueType::Integer)
    }

    fn boolean(&mut self, value: bool) -> NodeId {
        self.literal(Value::Boolean(value), ValueType::Boolean)
    }

    fn local(&mut self, slot: Slot, value_type: ValueType) -> NodeId {
        self.push(NodeKind::Local(slot), value_type)
    }

    fn integer_local(&mut self, slot: Slot) -> NodeId {
        self.local(slot, ValueType::Integer)
    }

    fn arithmetic(&mut self, operator: Arithmetic, left: NodeId, right: NodeId) -> NodeId {
        self.push(
            NodeKind::Arithmetic(operator, left, right),
            ValueType::Integer,
        )
    }

    /// `union::member(arguments)`, `member` the position in declared order.
    fn construct(&mut self, union: &str, member: usize, arguments: Vec<NodeId>) -> NodeId {
        self.push(
            NodeKind::UnionValue {
                declaration: fixture::handle(union),
                member,
                arguments,
            },
            fixture::union_type(union),
        )
    }

    /// `Shape::Empty`, `Shape::Circle(r)` or `Shape::Rect(w, h)`.
    fn shape(&mut self, member: &str, payload: &[i64]) -> NodeId {
        let arguments = payload.iter().map(|number| self.int(*number)).collect();
        let position = ["Empty", "Circle", "Rect"]
            .iter()
            .position(|name| *name == member)
            .expect("a Shape member");
        self.construct("Shape", position, arguments)
    }

    fn case(
        &mut self,
        union: &str,
        scrutinee: NodeId,
        arms: Vec<CaseArm>,
        result: ValueType,
    ) -> NodeId {
        self.push(
            NodeKind::Case {
                scrutinee,
                declaration: fixture::handle(union),
                arms,
            },
            result,
        )
    }

    fn body(self) -> CheckedBody {
        CheckedBody::from_nodes(self.0)
    }
}

fn arm(member: usize, binders: Vec<Slot>, body: NodeId) -> CaseArm {
    CaseArm {
        member,
        binders,
        body,
    }
}

/// `area`'s arms: `Empty` is 0, `Circle(r)` is `3 * (r * r)`, `Rect(w, h)`
/// is `w * h`. The binders take slots 0, then 1 and 2.
fn area_arms(nodes: &mut Nodes) -> Vec<CaseArm> {
    let empty = nodes.int(0);
    let three = nodes.int(3);
    let (r, r_again) = (nodes.integer_local(0), nodes.integer_local(0));
    let square = nodes.arithmetic(Arithmetic::Multiply, r, r_again);
    let circle = nodes.arithmetic(Arithmetic::Multiply, three, square);
    let (width, height) = (nodes.integer_local(1), nodes.integer_local(2));
    let rect = nodes.arithmetic(Arithmetic::Multiply, width, height);
    vec![
        arm(0, vec![], empty),
        arm(1, vec![0], circle),
        arm(2, vec![1, 2], rect),
    ]
}

/// `area(<shape>)`: the case over the scrutinee `shape` builds.
fn area(shape: impl FnOnce(&mut Nodes) -> NodeId) -> CheckedBody {
    let mut nodes = Nodes::default();
    let scrutinee = shape(&mut nodes);
    let arms = area_arms(&mut nodes);
    nodes.case("Shape", scrutinee, arms, ValueType::Integer);
    nodes.body()
}

fn run(
    world: &World,
    body: &CheckedBody,
    slots: usize,
    limits: ScalarLimits,
) -> (Result<Evaluation, InternalFault>, Meter) {
    let objects = ObjectEnvironment::default();
    let mut meter = Meter::new(limits);
    let result = Machine::new(
        &world.scope,
        &world.graph,
        &objects,
        &mut meter,
        world.graph.dispatch_tables(),
    )
    .run(body.root(), slots, Vec::new());
    (result, meter)
}

fn unlimited(world: &World, body: &CheckedBody, slots: usize) -> (Value, Meter) {
    let (result, meter) = run(world, body, slots, SCALAR_LIMITS_UNLIMITED);
    (completed(result), meter)
}

fn completed(result: Result<Evaluation, InternalFault>) -> Value {
    match result.expect("S6a evaluates").outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("completed, not {other:?}"),
    }
}

fn outcome(result: Result<Evaluation, InternalFault>) -> Outcome<Value> {
    match result.expect("S6a evaluates").outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

fn integer(value: &Value) -> Integer {
    let Value::Integer(number) = value else {
        panic!("an integer, not {value:?}");
    };
    number.clone()
}

/// A union value's active member identifier and integer payload.
fn describe(value: &Value) -> (String, Vec<Integer>) {
    let Value::Union(union) = value else {
        panic!("a union value, not {value:?}");
    };
    (
        union.member().identifier().as_str().to_owned(),
        union.payload().iter().map(integer).collect(),
    )
}

/// Every admitted charge, in order, with the value occurrences it requested.
fn charges(meter: &Meter) -> Vec<(ChargePoint, Option<u64>)> {
    meter
        .admitted_charges()
        .iter()
        .map(|charge| (charge.point, charge.value_occurrences))
        .collect()
}

fn work(meter: &Meter) -> u64 {
    meter.consumed(LimitKind::WorkUnits)
}

fn with_work(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        work_units,
        ..SCALAR_LIMITS_UNLIMITED
    }
}

/// FR-322-AC-1: `area` takes the arm of the active member, binding its
/// payload positionally.
#[trace("FR-322-AC-1", "TC-831")]
#[test]
fn case_selects_the_active_members_arm_and_binds_its_payload() {
    let world = world();
    let cases: [(&str, &[i64], i64); 3] = [
        ("Empty", &[], 0),
        ("Circle", &[3], 27),
        ("Rect", &[2, 3], 6),
    ];
    for (member, payload, expected) in cases {
        let body = area(|nodes| nodes.shape(member, payload));
        let (value, _) = unlimited(&world, &body, 3);
        assert_eq!(integer(&value), Integer::from(expected), "{member}");
    }
}

/// FR-322-AC-1: a `case` in an arm body evaluates its own inner arm.
#[trace("FR-322-AC-1", "TC-831")]
#[test]
fn a_case_in_an_arm_body_evaluates_its_inner_arm() {
    let world = world();
    let mut nodes = Nodes::default();
    let outer = nodes.shape("Circle", &[3]);
    let empty = nodes.int(0);
    // Circle(r): case Other::Only(r + 1) { Only(x): x * r }.
    let (r, one) = (nodes.integer_local(0), nodes.int(1));
    let argument = nodes.arithmetic(Arithmetic::Add, r, one);
    let inner_scrutinee = nodes.construct("Other", 0, vec![argument]);
    let (x, r_again) = (nodes.integer_local(1), nodes.integer_local(0));
    let product = nodes.arithmetic(Arithmetic::Multiply, x, r_again);
    let inner = nodes.case(
        "Other",
        inner_scrutinee,
        vec![arm(0, vec![1], product)],
        ValueType::Integer,
    );
    let rect = nodes.int(0);
    nodes.case(
        "Shape",
        outer,
        vec![
            arm(0, vec![], empty),
            arm(1, vec![0], inner),
            arm(2, vec![2, 3], rect),
        ],
        ValueType::Integer,
    );
    let (value, _) = unlimited(&world, &nodes.body(), 4);
    assert_eq!(integer(&value), Integer::from(12_i64));
}

/// The sum `sum<Small>(x in q: x)` over `q` holding `2, 2`, `Small` being
/// `Integer[0, 3]`: a running total of 4 leaves the domain.
fn small_sum(nodes: &mut Nodes) -> NodeId {
    let elements = vec![nodes.int(2), nodes.int(2)];
    let sequence = CollectionType::new(CollectionKind::Sequence, ValueType::Integer, None);
    let collection = nodes.push(
        NodeKind::Collection {
            collection_type: sequence.clone(),
            elements,
        },
        ValueType::collection(sequence),
    );
    let element = nodes.integer_local(3);
    let small = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(3_i64)).expect("a range"),
    );
    nodes.push(
        NodeKind::Query {
            visit: Visit::Sum,
            slot: 3,
            source: collection,
            body: element,
        },
        small,
    )
}

/// FR-322-AC-2: a scrutinee that is undefined is the case's outcome, selecting
/// no arm, and a construction's first stopped argument is its outcome with no
/// later argument evaluated and no value built.
#[trace("FR-322-AC-2", "TC-831")]
#[test]
fn a_stopped_scrutinee_or_argument_is_the_outcome_with_nothing_after_it() {
    let world = world();
    let alone = {
        let mut nodes = Nodes::default();
        small_sum(&mut nodes);
        nodes.body()
    };
    let (stopped, alone_meter) = run(&world, &alone, 4, SCALAR_LIMITS_UNLIMITED);
    assert!(matches!(
        outcome(stopped),
        Outcome::Undefined(Undefined::SumOutOfDomain)
    ));

    // case Shape::Circle(sum<Small>(q)) { ... }
    let case = area(|nodes| {
        let sum = small_sum(nodes);
        nodes.construct("Shape", 1, vec![sum])
    });
    let (result, case_meter) = run(&world, &case, 4, SCALAR_LIMITS_UNLIMITED);
    assert!(matches!(
        outcome(result),
        Outcome::Undefined(Undefined::SumOutOfDomain)
    ));
    // No arm was selected and no construction charged: exactly the scrutinee
    // argument's own charges.
    assert_eq!(charges(&case_meter), charges(&alone_meter));

    // Shape::Rect(sum<Small>(q), f()): `f` names no function of the graph, so
    // evaluating it would be a fault, not an outcome.
    let construction = {
        let mut nodes = Nodes::default();
        let sum = small_sum(&mut nodes);
        let call = nodes.push(
            NodeKind::Call {
                function: 99,
                arguments: vec![],
            },
            ValueType::Integer,
        );
        nodes.construct("Shape", 2, vec![sum, call]);
        nodes.body()
    };
    let (result, meter) = run(&world, &construction, 4, SCALAR_LIMITS_UNLIMITED);
    assert!(matches!(
        outcome(result),
        Outcome::Undefined(Undefined::SumOutOfDomain)
    ));
    assert_eq!(charges(&meter), charges(&alone_meter));
}

/// FR-322-AC-2: a value that reaches S6a and matches no arm is an S6a
/// internal fault, never a refusal or an outcome.
#[trace("FR-322-AC-2", "TC-831")]
#[test]
fn a_scrutinee_matching_no_arm_is_an_s6a_internal_fault() {
    let world = world();
    // An arm set missing `Rect`, over a `Rect` scrutinee.
    let mut nodes = Nodes::default();
    let scrutinee = nodes.shape("Rect", &[2, 3]);
    let (empty, circle) = (nodes.int(0), nodes.int(1));
    nodes.case(
        "Shape",
        scrutinee,
        vec![arm(0, vec![], empty), arm(1, vec![0], circle)],
        ValueType::Integer,
    );
    let (result, _) = run(&world, &nodes.body(), 3, SCALAR_LIMITS_UNLIMITED);
    let fault = result.expect_err("no arm matches");
    assert_eq!(
        (fault.stage(), fault.invariant()),
        ("S6a", "case-arm-unmatched")
    );

    // A member of another union injected as the scrutinee.
    let mut nodes = Nodes::default();
    let foreign = fixture::value("Other", "Only", vec![int(1)]);
    let scrutinee = nodes.literal(foreign, fixture::union_type("Shape"));
    let arms = area_arms(&mut nodes);
    nodes.case("Shape", scrutinee, arms, ValueType::Integer);
    let (result, _) = run(&world, &nodes.body(), 3, SCALAR_LIMITS_UNLIMITED);
    let fault = result.expect_err("the member is no member of Shape");
    assert_eq!(
        (fault.stage(), fault.invariant()),
        ("S6a", "case-scrutinee-member-unresolved")
    );
}

/// FR-322-AC-3: construction charges one `composite.result-retain` after its
/// arguments, for the value's occurrences; a `case` charges its scrutinee and
/// then its selected body and nothing for selection, binding or an unselected
/// arm; one work unit below the total stops at the body's last work point.
#[trace("FR-322-AC-3", "TC-832")]
#[test]
fn construction_and_case_charge_exactly_their_accounting_points() {
    let world = world();
    let construction = {
        let mut nodes = Nodes::default();
        nodes.shape("Rect", &[2, 3]);
        nodes.body()
    };
    let (_, construction_meter) = unlimited(&world, &construction, 0);
    assert_eq!(
        charges(&construction_meter),
        [(ChargePoint::CompositeResultRetain, Some(3))]
    );

    // The selected body `w * h` over the same two values, run alone.
    let body = {
        let mut nodes = Nodes::default();
        let (width, height) = (nodes.int(2), nodes.int(3));
        nodes.arithmetic(Arithmetic::Multiply, width, height);
        nodes.body()
    };
    let (_, body_meter) = unlimited(&world, &body, 0);
    assert!(!charges(&body_meter).is_empty());

    let case = area(|nodes| nodes.shape("Rect", &[2, 3]));
    let (value, case_meter) = unlimited(&world, &case, 3);
    assert_eq!(integer(&value), Integer::from(6_i64));
    let expected: Vec<_> = charges(&construction_meter)
        .into_iter()
        .chain(charges(&body_meter))
        .collect();
    assert_eq!(charges(&case_meter), expected);

    let total = work(&case_meter);
    assert_eq!(total, work(&construction_meter) + work(&body_meter));
    assert!(total > 0);
    let (result, below) = run(&world, &case, 3, with_work(total - 1));
    let Outcome::Incomplete(stopped) = outcome(result) else {
        panic!("one unit below the total stops the run");
    };
    assert_eq!(stopped.limit_kind, LimitKind::WorkUnits);
    assert_eq!(stopped.limit, total - 1);
    assert_eq!(
        stopped.consumed + stopped.next_charge.to_u64().expect("a small charge"),
        total
    );
    assert!(charges(&body_meter)
        .iter()
        .any(|(point, _)| *point == stopped.charge_point));
    assert_eq!(work(&below), stopped.consumed);
    let (result, _) = run(&world, &case, 3, with_work(total));
    assert_eq!(integer(&completed(result)), Integer::from(6_i64));
}

/// The recursive sum over `tree` unrolled `depth` levels:
/// `case t { Leaf(n): n; Branch(n, c): n + <sum of value(c)> }`. The deepest
/// level's `Branch` arm is never taken.
fn tree_sum(depth: usize, tree: Value) -> CheckedBody {
    let tree_type = fixture::union_type("Tree");
    let option = ValueType::option(tree_type.clone());
    let mut nodes = Nodes::default();
    let mut below: Option<NodeId> = None;
    for level in (0..depth).rev() {
        let scrutinee = if level == 0 {
            nodes.literal(tree.clone(), tree_type.clone())
        } else {
            let child = nodes.local(1, option.clone());
            nodes.push(NodeKind::Value(child), tree_type.clone())
        };
        let leaf = nodes.integer_local(0);
        let branch = match below {
            Some(below) => {
                let head = nodes.integer_local(0);
                nodes.arithmetic(Arithmetic::Add, head, below)
            }
            None => nodes.int(0),
        };
        below = Some(nodes.case(
            "Tree",
            scrutinee,
            vec![arm(0, vec![0], leaf), arm(1, vec![0, 1], branch)],
            ValueType::Integer,
        ));
    }
    nodes.body()
}

/// FR-322-AC-4 (the evaluator's half): `case` nesting 10,000 levels deep over
/// a 10,000-level union value completes under default limits with no host
/// stack growth, its work is the per-level work times the levels, and
/// `work_units` one below the total stops while the total completes. The
/// source recursion through a function call waits on the FR-319 lowering.
#[trace("FR-322-AC-4", "TC-831")]
#[test]
fn case_nesting_ten_thousand_deep_completes_and_stops_at_its_exact_work() {
    const DEPTH: usize = 10_000;
    let world = world();
    let work_at = |depth: usize| {
        let (value, meter) = unlimited(&world, &tree_sum(depth, fixture::tree(depth)), 2);
        assert_eq!(integer(&value), Integer::from(depth as u64));
        work(&meter)
    };
    let per_level = work_at(3) - work_at(2);
    assert!(per_level > 0);

    let body = tree_sum(DEPTH, fixture::tree(DEPTH));
    let (value, meter) = unlimited(&world, &body, 2);
    assert_eq!(integer(&value), Integer::from(DEPTH as u64));
    let total = work(&meter);
    assert_eq!(total, work_at(2) + per_level * (DEPTH as u64 - 2));

    let (result, _) = run(&world, &body, 2, with_work(total - 1));
    let Outcome::Incomplete(stopped) = outcome(result) else {
        panic!("one unit below the total stops the run");
    };
    assert_eq!(stopped.limit_kind, LimitKind::WorkUnits);
    assert_eq!(stopped.limit, total - 1);
    let (result, _) = run(&world, &body, 2, with_work(total));
    assert_eq!(integer(&completed(result)), Integer::from(DEPTH as u64));
}

/// A `Collection` of `kind` over the `Shape` literals `members`, in order.
fn shapes(kind: CollectionKind, members: &[(&str, &[i64])]) -> CheckedBody {
    let mut nodes = Nodes::default();
    let elements = members
        .iter()
        .map(|(member, payload)| nodes.shape(member, payload))
        .collect();
    let collection_type = CollectionType::new(kind, fixture::union_type("Shape"), None);
    nodes.push(
        NodeKind::Collection {
            collection_type: collection_type.clone(),
            elements,
        },
        ValueType::collection(collection_type),
    );
    nodes.body()
}

fn elements_of(world: &World, kind: CollectionKind, members: &[(&str, &[i64])]) -> Vec<String> {
    let (value, _) = unlimited(world, &shapes(kind, members), 0);
    let Value::Collection(collection) = value else {
        panic!("a collection");
    };
    collection
        .elements()
        .iter()
        .map(|element| {
            let (member, payload) = describe(element);
            let payload: Vec<String> = payload.iter().map(ToString::to_string).collect();
            format!("{member}({})", payload.join(", "))
        })
        .collect()
}

/// FR-323-AC-1 and AC-2: sets and bags of unions order by the union key
/// (member identifier bytes, then payload positions) whatever the input
/// order, a bag keeps both equal values, an ordered set keeps first
/// occurrences in input order, and a sequence keeps every occurrence.
#[trace("FR-323-AC-1", "FR-323-AC-2", "TC-833")]
#[test]
fn collections_of_unions_order_dedupe_and_count_by_the_union_key() {
    let world = world();
    let forward: &[(&str, &[i64])] = &[
        ("Rect", &[2, 3]),
        ("Empty", &[]),
        ("Rect", &[2, 3]),
        ("Circle", &[1]),
    ];
    let reversed: Vec<(&str, &[i64])> = forward.iter().rev().copied().collect();
    for input in [forward, &reversed] {
        assert_eq!(
            elements_of(&world, CollectionKind::Set, input),
            ["Circle(1)", "Empty()", "Rect(2, 3)"]
        );
        assert_eq!(
            elements_of(&world, CollectionKind::Bag, input),
            ["Circle(1)", "Empty()", "Rect(2, 3)", "Rect(2, 3)"]
        );
    }
    assert_eq!(
        elements_of(&world, CollectionKind::OrderedSet, forward),
        ["Rect(2, 3)", "Empty()", "Circle(1)"]
    );
    assert_eq!(
        elements_of(&world, CollectionKind::OrderedSet, &reversed),
        ["Circle(1)", "Rect(2, 3)", "Empty()"]
    );
    assert_eq!(
        elements_of(&world, CollectionKind::Sequence, forward),
        ["Rect(2, 3)", "Empty()", "Rect(2, 3)", "Circle(1)"]
    );
    // One member's payload positions order left to right.
    assert_eq!(
        elements_of(
            &world,
            CollectionKind::Set,
            &[("Rect", &[2, 4]), ("Rect", &[2, 3]), ("Rect", &[1, 9])]
        ),
        ["Rect(1, 9)", "Rect(2, 3)", "Rect(2, 4)"]
    );
}

/// `isRect(s)`: `case s { Empty: false; Circle(r): false; Rect(w, h): true }`
/// over the value in slot `subject`, whose binders take the slots after it.
fn is_rect(nodes: &mut Nodes, subject: Slot) -> NodeId {
    let scrutinee = nodes.local(subject, fixture::union_type("Shape"));
    let (empty, circle, rect) = (
        nodes.boolean(false),
        nodes.boolean(false),
        nodes.boolean(true),
    );
    nodes.case(
        "Shape",
        scrutinee,
        vec![
            arm(0, vec![], empty),
            arm(1, vec![subject + 1], circle),
            arm(2, vec![subject + 2, subject + 3], rect),
        ],
        ValueType::Boolean,
    )
}

/// The `Sequence<Shape>` `[Empty, Rect(2, 3), Rect(2, 3)]`.
fn shape_sequence(nodes: &mut Nodes) -> NodeId {
    let elements = vec![
        nodes.shape("Empty", &[]),
        nodes.shape("Rect", &[2, 3]),
        nodes.shape("Rect", &[2, 3]),
    ];
    let collection_type =
        CollectionType::new(CollectionKind::Sequence, fixture::union_type("Shape"), None);
    nodes.push(
        NodeKind::Collection {
            collection_type: collection_type.clone(),
            elements,
        },
        ValueType::collection(collection_type),
    )
}

/// FR-046-AC-9: a predicate whose body is a `case` answers per member, and
/// `filter` and `count` by it, and `contains`, read a `Sequence<Shape>` in
/// occurrence order.
#[trace("FR-046-AC-9", "TC-833")]
#[test]
fn predicates_and_queries_over_a_union_sequence_follow_occurrence_order() {
    let world = world();
    for (member, payload, expected) in [("Rect", &[2_i64, 3][..], true), ("Empty", &[][..], false)]
    {
        let mut nodes = Nodes::default();
        let shape = nodes.shape(member, payload);
        let predicate = is_rect(&mut nodes, 0);
        nodes.push(
            NodeKind::Let {
                slot: 0,
                value: shape,
                body: predicate,
            },
            ValueType::Boolean,
        );
        let (value, _) = unlimited(&world, &nodes.body(), 4);
        assert!(
            matches!(value, Value::Boolean(answer) if answer == expected),
            "{member}"
        );
    }

    let query = |visit: Visit, result: ValueType| {
        let mut nodes = Nodes::default();
        let source = shape_sequence(&mut nodes);
        let body = is_rect(&mut nodes, 0);
        nodes.push(
            NodeKind::Query {
                visit,
                slot: 0,
                source,
                body,
            },
            result,
        );
        unlimited(&world, &nodes.body(), 4).0
    };
    let sequence = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        fixture::union_type("Shape"),
        None,
    ));
    let Value::Collection(kept) = query(Visit::Filter, sequence) else {
        panic!("filter yields a collection");
    };
    let kept: Vec<_> = kept.elements().iter().map(describe).collect();
    let rect = || {
        (
            "Rect".to_owned(),
            vec![Integer::from(2_i64), Integer::from(3_i64)],
        )
    };
    assert_eq!(kept, [rect(), rect()]);
    assert_eq!(
        integer(&query(Visit::Count, ValueType::Integer)),
        Integer::from(2_i64)
    );

    for (member, payload, expected) in
        [("Rect", &[2_i64, 3][..], true), ("Circle", &[1][..], false)]
    {
        let mut nodes = Nodes::default();
        let source = shape_sequence(&mut nodes);
        let item = nodes.shape(member, payload);
        nodes.push(NodeKind::Contains(source, item), ValueType::Boolean);
        let (value, _) = unlimited(&world, &nodes.body(), 0);
        assert!(
            matches!(value, Value::Boolean(answer) if answer == expected),
            "{member}"
        );
    }
}
