// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-391 (TC-1076): a state clause or a `holds` atom names a domain-package
//! population `M::P`, as the operand of `allInstances` or `lookup` or as the
//! domain of a quantifier or population form; the S3 half. The S6a
//! evaluation, the footprint and the emission refusal are in `qsl-eval`'s
//! `named_population`.

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind};
use qsl_foundation::digest::WireNodeId;
use qsl_semantics::check::{CheckedGraph, CheckedNode, NodeKind};
use qsl_semantics::family::ClaimExtent;
use qsl_semantics::model::clause_observation::Observation;
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, Identifier, IntegerInterval, ValueType,
};

#[path = "../support/named_population.rs"]
mod fixture;

use fixture::{checked, key, refused};

const ROOM: &str = "pre Room using v on M::Counter::bump { \
                    forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }";
const CAP: &str = "invariant Cap using v on M::Counter at current { \
                   forall(o in allInstances<M::Counter>(M::counters) : o.v < 4) }";
const BOUND: &str = "bound B on M::counters [3]";

/// Every node of `root`, the root first.
fn nodes(root: CheckedNode<'_>) -> Vec<CheckedNode<'_>> {
    let mut found = Vec::new();
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        pending.extend(node.kind().children().into_iter().map(|id| node.at(id)));
        found.push(node);
    }
    found
}

/// The `allInstances` node of the clause `name` whose operand is a named
/// population, and that operand.
fn named_query<'g>(graph: &'g CheckedGraph, name: &str) -> (CheckedNode<'g>, CheckedNode<'g>) {
    let clause = graph.state_clause(name).expect("the clause");
    nodes(clause.body())
        .into_iter()
        .find_map(|node| match node.kind() {
            NodeKind::AllInstances { population }
                if matches!(
                    node.at(*population).kind(),
                    NodeKind::NamedPopulation { .. }
                ) =>
            {
                Some((node, node.at(*population)))
            }
            _ => None,
        })
        .expect("an allInstances over a named population")
}

/// The `Counter` reference type: the one member type of `counters`, as the
/// clause `name`'s named operand carries it.
fn counter(graph: &CheckedGraph, name: &str) -> ValueType {
    let (_, operand) = named_query(graph, name);
    let NodeKind::NamedPopulation { members, .. } = operand.kind() else {
        unreachable!("a named population");
    };
    let [member] = members[..] else {
        panic!("one member type: {members:?}");
    };
    ValueType::Reference(member)
}

fn set_of(element: ValueType, maximum: Option<u64>) -> ValueType {
    ValueType::collection(CollectionType::new(
        CollectionKind::Set,
        element,
        maximum.map(|maximum| CardinalityBound::new(0, maximum).expect("a bound")),
    ))
}

/// Every observation the clause `name`'s reads take.
fn observations(graph: &CheckedGraph, name: &str) -> Vec<Observation> {
    graph
        .state_clause(name)
        .expect("the clause")
        .reads()
        .map(|(_, observation)| observation)
        .collect()
}

/// The unbounded domains of the claim record of the node `identity` at
/// `claim`.
fn domains(
    graph: &CheckedGraph,
    identity: quire_exact::NodeKey,
    claim: &quire_exact::Origin,
) -> Vec<(DomainKey, DomainKind)> {
    let node = WireNodeId::from_digest(*identity.as_bytes());
    let (_, record) = graph
        .requirements()
        .iter()
        .find(|(key, _)| key.node() == node && key.origin() == claim)
        .expect("the clause's claim record");
    match record.requirements().extent() {
        ClaimExtent::Unbounded(domains) => domains
            .iter()
            .map(|(key, kind)| (key.clone(), kind))
            .collect(),
        ClaimExtent::Bounded => Vec::new(),
    }
}

/// The population domains of the state clause `name`'s claim record.
fn clause_domains(graph: &CheckedGraph, name: &str) -> Vec<DomainKey> {
    let clause = graph.state_clause(name).expect("the clause");
    domains(graph, clause.identity(), clause.claim())
        .into_iter()
        .filter(|(_, kind)| *kind == DomainKind::Population)
        .map(|(key, _)| key)
        .collect()
}

/// The record key of the population `name` of `M`.
fn population(graph: &CheckedGraph, name: &str) -> DomainKey {
    graph
        .population_domain(
            &Identifier::new("M").expect("an identifier"),
            &Identifier::new(name).expect("an identifier"),
        )
        .expect("keyed")
        .expect("the population resolves")
}

/// TC-1076 rows 1 and 2 (FR-391-AC-1): `Room` checks as a `Precondition`,
/// the operand typed `Population<Counter>`, the result `Set<Reference<Counter>>`
/// and every read at `pre`; `Cap` reads at `current`.
#[trace("TC-1076", "FR-391-AC-1")]
#[test]
fn a_precondition_and_an_invariant_name_a_population_as_an_operand() {
    let graph = checked(false, &format!("{ROOM}\n{CAP}")).graph;
    let room = graph.state_clause("Room").expect("Room");
    assert_eq!(room.kind(), qsl_forms::StateClauseKind::Precondition);
    let (query, operand) = named_query(&graph, "Room");
    assert_eq!(operand.value_type(), &ValueType::Population(None));
    let NodeKind::NamedPopulation { population, .. } = operand.kind() else {
        unreachable!("a named population");
    };
    assert_eq!(population, &key("counters"));
    assert_eq!(query.value_type(), &set_of(counter(&graph, "Room"), None));
    let reads = observations(&graph, "Room");
    assert!(!reads.is_empty());
    assert!(
        reads.iter().all(|read| *read == Observation::Pre),
        "{reads:?}"
    );
    let reads = observations(&graph, "Cap");
    assert!(!reads.is_empty());
    assert!(
        reads.iter().all(|read| *read == Observation::Current),
        "{reads:?}"
    );
}

/// TC-1076 row 3 (FR-391-AC-2): `M::nothing` as the operand refuses
/// `missing_declaration`/`missing-name`; an alias `M` that is also an enum
/// name refuses `ambiguous_declaration`/`ambiguous-name` at the name.
#[trace("TC-1076", "FR-391-AC-2")]
#[test]
fn an_unknown_population_or_an_alias_that_is_also_an_enum_refuses() {
    let (code, cause, at) = refused(
        false,
        "pre Room using v on M::Counter::bump { \
         forall(o in allInstances<M::Counter>(M::nothing) : o.v < 2) }",
    );
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("missing_declaration", "missing-name")
    );
    assert_eq!(at, "nothing");
    // The domain of a population form, located at its second segment too
    // (SR-12229 FND-008), in a state clause and in a function body.
    for (declaration, segment) in [
        (
            "invariant N using v on M::Counter at current { count(o in M::nothing where true) = 0 }",
            "nothing",
        ),
        (
            "function F using v(): Boolean pure { count(o in M::counters where true) = 0 }",
            "counters",
        ),
    ] {
        let (code, cause, at) = refused(false, declaration);
        assert_eq!(
            (code.as_str(), cause.as_str()),
            ("missing_declaration", "missing-name"),
            "{declaration}"
        );
        assert_eq!(at, segment, "{declaration}");
    }
    let (code, cause, at) = refused(false, &format!("enum M {{ a, b }}\n{ROOM}"));
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("ambiguous_declaration", "ambiguous-name")
    );
    assert_eq!(at, "M::counters");
}

/// TC-1076 row 4 (FR-391-AC-3): a population is no value: compared, bound
/// by `let`, measured by `size` or returned as the body it refuses
/// `ill_typed`/`operator-ineligible` at the name; `M::Counter` names a type,
/// so as a value it refuses `missing_declaration`/`missing-name`.
#[trace("TC-1076", "FR-391-AC-3")]
#[test]
fn a_population_as_a_value_refuses_as_operator_ineligible() {
    let clause = |body: &str| format!("invariant V using v on M::Counter at current {{ {body} }}");
    for body in [
        "M::counters = M::counters",
        "let q = M::counters in size(allInstances<M::Counter>(q)) = 0",
        "size(M::counters) = 0",
        "M::counters",
    ] {
        let (code, cause, at) = refused(false, &clause(body));
        assert_eq!(
            (code.as_str(), cause.as_str()),
            ("ill_typed", "operator-ineligible"),
            "{body}"
        );
        assert_eq!(at, "M::counters", "{body}");
    }
    let (code, cause, at) = refused(false, &clause("M::Counter = M::Counter"));
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("missing_declaration", "missing-name")
    );
    assert_eq!(at, "Counter");
}

/// TC-1076 rows 5 and 6 (FR-391-AC-4): with `bound B on M::counters [3]` the
/// operand is `Population<Counter>[3]`, the result `Set<Reference<Counter>>[0,3]`
/// and `counters` a bounded domain; with `archive` a clause on `M::Counter`
/// is ambiguous at its `on` whichever population it names; a clause on
/// `M::Widget` naming `M::counters` records `widgets` and `counters`.
#[trace("TC-1076", "FR-391-AC-4")]
#[test]
fn a_bound_names_a_bounded_domain_and_a_named_population_is_a_domain() {
    let unbounded = checked(false, ROOM).graph;
    assert_eq!(
        clause_domains(&unbounded, "Room"),
        [population(&unbounded, "counters")]
    );
    let graph = checked(false, &format!("{BOUND}\n{ROOM}")).graph;
    let (query, operand) = named_query(&graph, "Room");
    assert_eq!(operand.value_type(), &ValueType::Population(Some(3)));
    assert_eq!(
        query.value_type(),
        &set_of(counter(&graph, "Room"), Some(3))
    );
    assert!(
        clause_domains(&graph, "Room").is_empty(),
        "the bounded counters is no unbounded domain"
    );
    let room = graph.state_clause("Room").expect("Room");
    let [bound] = room.population_bounds() else {
        panic!("one bound: {:?}", room.population_bounds());
    };
    assert_eq!((bound.label.as_str(), bound.maximum), ("B", 3));

    for name in ["M::counters", "M::archive"] {
        let declaration = format!(
            "pre Room using v on M::Counter::bump {{ \
             forall(o in allInstances<M::Counter>({name}) : o.v < 2) }}"
        );
        let (code, cause, at) = refused(true, &declaration);
        assert_eq!(
            (code.as_str(), cause.as_str()),
            ("ambiguous_declaration", "ambiguous-name"),
            "{name}"
        );
        assert_eq!(at, "M::Counter", "{name}");
    }

    let graph = checked(
        false,
        "pre Grow using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::counters) : true) }",
    )
    .graph;
    let mut found = clause_domains(&graph, "Grow");
    found.sort();
    let mut expected = vec![
        population(&graph, "widgets"),
        population(&graph, "counters"),
    ];
    expected.sort();
    assert_eq!(found, expected);
}

/// TC-1076 row 8, S3 half (FR-391-AC-6): with `archive`, two preconditions
/// that differ only in the population they name check to two different
/// terms, each carrying its own population's declaration key.
#[trace("TC-1076", "FR-391-AC-6")]
#[test]
fn two_populations_of_one_type_check_to_two_terms() {
    let graph = checked(
        true,
        "pre A using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::counters) : true) }\n\
         pre B using v on M::Widget::grow { \
         forall(o in allInstances<M::Counter>(M::archive) : true) }",
    )
    .graph;
    let key_of = |name: &str| {
        let (_, operand) = named_query(&graph, name);
        let NodeKind::NamedPopulation { population, .. } = operand.kind() else {
            unreachable!("a named population");
        };
        population.clone()
    };
    assert_eq!(key_of("A"), key("counters"));
    assert_eq!(key_of("B"), key("archive"));
    let identity = |name: &str| graph.state_clause(name).expect("the clause").identity();
    assert_ne!(identity("A"), identity("B"));
}

/// The type of the first `count` query of the clause `name`.
fn count_type(graph: &CheckedGraph, name: &str) -> ValueType {
    let clause = graph.state_clause(name).expect("the clause");
    nodes(clause.body())
        .into_iter()
        .find(|node| {
            matches!(
                node.kind(),
                NodeKind::Query {
                    visit: qsl_semantics::check::Visit::Count,
                    ..
                }
            )
        })
        .expect("a count")
        .value_type()
        .clone()
}

/// TC-1076 row 9, S3 half (FR-391-AC-7): a population as the domain of a
/// quantifier in a postcondition (read at `post`) and a precondition (read
/// at `pre`), and of `count` in an invariant (typed `Int[0,3]` with the
/// bound, unbounded without it); a temporal `holds` atom ranging over
/// `M::counters` checks with `counters` a domain of its record.
#[trace("TC-1076", "FR-391-AC-7")]
#[test]
fn a_population_is_the_domain_of_a_quantifier_a_count_and_a_holds_atom() {
    const P: &str = "post P using v on M::Counter::bump { forall(o in M::counters : o.v < 2) }";
    const Q: &str = "pre Q using v on M::Counter::bump { exists(o in M::counters : o.v = 2) }";
    const COUNT: &str = "invariant Cap using v on M::Counter at current { \
                         count(o in M::counters where o.v = 0) < 4 }";
    const C: &str = "temporal C using t over (w: M::Watch) clock \"model-steps\" on origin { \
                     eventually holds(forall(x in M::counters : x.v < 2)) }";
    let graph = checked(false, &format!("{BOUND}\n{P}\n{Q}\n{COUNT}")).graph;
    assert_eq!(
        graph.state_clause("P").expect("P").kind(),
        qsl_forms::StateClauseKind::Postcondition
    );
    for (name, observation) in [
        ("P", Observation::Post),
        ("Q", Observation::Pre),
        ("Cap", Observation::Current),
    ] {
        let (query, operand) = named_query(&graph, name);
        assert_eq!(
            operand.value_type(),
            &ValueType::Population(Some(3)),
            "{name}"
        );
        assert_eq!(
            query.value_type(),
            &set_of(counter(&graph, name), Some(3)),
            "{name}"
        );
        let reads = observations(&graph, name);
        assert!(
            reads.contains(&observation) && reads.iter().all(|read| *read == observation),
            "{name}: {reads:?}"
        );
    }
    assert_eq!(
        count_type(&graph, "Cap"),
        ValueType::Int(IntegerInterval::new(0_i64.into(), 3_i64.into()).expect("an interval"))
    );
    let unbounded = checked(false, COUNT).graph;
    assert_eq!(count_type(&unbounded, "Cap"), ValueType::Integer);

    let graph = checked(false, C).graph;
    let [clause] = graph.temporal_clauses() else {
        panic!("one temporal clause");
    };
    let found = domains(&graph, clause.identity(), clause.claim());
    let counters = population(&graph, "counters");
    assert!(
        found
            .iter()
            .any(|(key, kind)| *key == counters && *kind == DomainKind::Population),
        "{found:?}"
    );
}

/// TC-1076 row 10, two of FR-391-AC-8's three cases: `M::counters` in a
/// function body names no population, `missing_declaration`/`missing-name`
/// at `counters`; `seq` in an invariant refuses
/// `unsupported_construct`/`expression-form` at its span. The type-level
/// invariant case has no fixture here (a domain-package clause record), so
/// the test claims no criterion.
#[trace("TC-1076")]
#[test]
fn a_function_body_names_no_population_and_seq_stays_refused() {
    let (code, cause, at) = refused(
        false,
        "function F using v(): Boolean pure { size(M::counters) = 0 }",
    );
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("missing_declaration", "missing-name")
    );
    assert_eq!(at, "counters");
    let (code, cause, at) = refused(
        false,
        "invariant S using v on M::Counter at current { size(seq(i in 0 .. 2: i)) = 3 }",
    );
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("unsupported_construct", "expression-form")
    );
    assert_eq!(at, "seq(i in 0 .. 2: i)");
}
