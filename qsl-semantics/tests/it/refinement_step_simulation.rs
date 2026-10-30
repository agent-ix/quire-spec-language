// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-146 (TC-551): the per-step simulation records S3 writes beside a
//! refinement's record, over ADR-020 §8's `CasRefinesCounter`: which records
//! it writes, their hypotheses, their row order, their occurrence keys and
//! their requirement records' extents.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBoundKind};
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::Code;
use qsl_semantics::check::{
    Capability, CheckedGraph, CheckedRefinement, StepClass, StepSimulationEntry,
};
use qsl_semantics::family::ClaimExtent;
use quire_exact::Identifier;
use quire_semantic_value::step_simulation::{StepObligation, StepRowRef, StepSimulationRecord};
use serde_json::Value;

use super::refinement_declarations::{
    cas_refines_counter, check_with, checked_counter, counter_cas_with, counter_spec,
    counter_spec_with, mutated, node, object_type, only_refinement, operation, population, Refused,
    CAS, INTEGER, SPEC,
};

/// The refinement's V2 node, which keys its records.
fn refinement_node(refinement: &CheckedRefinement) -> quire_exact::NodeKey {
    refinement
        .requirement_occurrence()
        .expect("the refinement record is written")
        .node()
        .clone()
}

fn entries(graph: &CheckedGraph) -> Vec<StepSimulationEntry> {
    only_refinement(graph).step_simulations().to_vec()
}

fn records(graph: &CheckedGraph) -> Vec<StepSimulationRecord> {
    entries(graph)
        .iter()
        .map(|entry| entry.record().clone())
        .collect()
}

/// `CasRefinesCounter` over a concrete package whose `peek` is `peek` and
/// which adds `types` and `populations`, with `extra` after the refinement.
fn checked_variant(
    peek: Value,
    types: Vec<Value>,
    populations: Vec<Value>,
    extra: &str,
) -> CheckedGraph {
    let (spec, cas) = (
        counter_spec(),
        counter_cas_with(peek, &[], types, populations),
    );
    let body = format!("{}\n{extra}", cas_refines_counter());
    check_with("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &body).unwrap_or_else(
        |refusals| {
            panic!(
                "{body}: {:?}",
                refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
            )
        },
    )
}

fn plain_peek() -> Value {
    operation(CAS, "Counter", "peek", &[], None, &[])
}

fn identifier(text: &str) -> Identifier {
    Identifier::new(text).expect("an identifier")
}

/// The population domain `model::population` keys.
fn population_key(graph: &CheckedGraph, model: &str, population: &str) -> DomainKey {
    graph
        .population_domain(&identifier(model), &identifier(population))
        .expect("keyed")
        .expect("the population resolves")
}

/// The unbounded domains of each record's requirement record, by the
/// record's obligation.
fn domains(graph: &CheckedGraph) -> Vec<(StepObligation, Vec<(DomainKey, DomainKind)>)> {
    entries(graph)
        .iter()
        .map(|entry| {
            let record = graph
                .requirements()
                .get(entry.occurrence())
                .expect("each record is a requirement record");
            let found = match record.requirements().extent() {
                ClaimExtent::Unbounded(domains) => domains
                    .iter()
                    .map(|(key, kind)| (key.clone(), kind))
                    .collect(),
                ClaimExtent::Bounded => Vec::new(),
            };
            (entry.record().obligation, found)
        })
        .collect()
}

/// The `StepRowRef` of each source row, by its concrete operation's name.
fn row_refs_by_operation(graph: &CheckedGraph) -> BTreeMap<String, StepRowRef> {
    let refinement = only_refinement(graph);
    refinement
        .steps()
        .iter()
        .enumerate()
        .map(|(source, row)| {
            let StepClass::Operation(operation) = &row.class else {
                panic!("an operation row: {:?}", row.class);
            };
            let reference = refinement
                .step_row_ref(source)
                .expect("the refinement emits");
            (operation.node.clone(), reference)
        })
        .collect()
}

/// FR-146-AC-1: `CasRefinesCounter` writes eight records, one `Initial`
/// and one `Step` per row, each with no hypotheses; with `commitB -> any`
/// it writes none, and no `generated` requirement record at its node.
#[trace("TC-551", "FR-146-AC-1")]
#[test]
fn the_counter_refinement_writes_an_initial_and_a_step_record_per_row() {
    let graph = checked_counter(&cas_refines_counter());
    let node = refinement_node(only_refinement(&graph));
    let found = records(&graph);
    let mut expected = vec![StepObligation::Initial];
    expected.extend((0..7).map(|row| StepObligation::Step {
        row: StepRowRef::new(row),
    }));
    assert_eq!(
        found
            .iter()
            .map(|record| record.obligation)
            .collect::<Vec<_>>(),
        expected
    );
    for record in &found {
        assert_eq!(record.refinement, node);
        assert!(record.hypotheses.is_empty());
    }

    let any = checked_counter(&mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitB -> any;",
    ));
    let refinement = only_refinement(&any);
    assert_eq!(refinement.step_simulations(), []);
    let node = refinement_node(refinement);
    assert!(any
        .requirements()
        .keys()
        .all(|key| key.node() != node || key.origin().role().as_str() != "generated"));
}

/// FR-146-AC-2, the hidden-field case: a refinement with a hidden field
/// writes no record and still writes its refinement record. The history
/// case is in `qsl-replay`'s `refinement_history`.
#[trace("TC-551", "FR-146-AC-2")]
#[test]
fn a_hidden_field_writes_no_simulation_record() {
    let graph = checked_counter(&mutated(&cas_refines_counter(), "value = self.value;", ""));
    let refinement = only_refinement(&graph);
    assert_eq!(refinement.step_simulations(), []);
    let occurrence = refinement
        .requirement_occurrence()
        .expect("the refinement record is written");
    assert!(graph.requirements().contains_key(occurrence));
}

const CAS_INV: &str = "invariant CasInv using inf on Impl::Counter at current { \
     self.busyA or self.tmpA <= self.value }\n";

const ABS_INV: &str =
    "invariant AbsInv using inf on Spec::Counter at current { self.value >= 0 }\n";

/// FR-146-AC-3: `CasInv` on `Impl::Counter` is a hypothesis of every
/// record, and is its own `operation-contract` record.
#[trace("TC-551", "FR-146-AC-3")]
#[test]
fn a_concrete_invariant_is_a_hypothesis_of_every_record() {
    let graph = checked_counter(&format!("{}\n{CAS_INV}", cas_refines_counter()));
    let [invariant] = graph.state_clauses() else {
        panic!("one invariant clause");
    };
    let found = records(&graph);
    assert_eq!(found.len(), 8);
    for record in &found {
        assert_eq!(record.hypotheses, [invariant.identity()]);
    }
    let key = OccurrenceKey::new(invariant.identity(), invariant.claim().clone());
    let record = graph
        .requirements()
        .get(&key)
        .expect("the invariant has a requirement record");
    assert_eq!(record.requirements().kind(), Capability::OperationContract);
}

/// FR-146-AC-5, model subject: the seven `Step` records name positions 0
/// to 6 once each, the rows at those positions ascend by the node-key digest
/// of their concrete operation, and writing the rows in exact reverse gives
/// each the same `StepRowRef`.
#[trace("TC-551", "FR-146-AC-5")]
#[test]
fn step_row_refs_follow_fr375_order_whatever_the_source_order() {
    let base = cas_refines_counter();
    let mut lines: Vec<&str> = base.split('\n').collect();
    let steps: Vec<usize> = (0..lines.len())
        .filter(|index| lines[*index].starts_with("step "))
        .collect();
    assert_eq!(steps.len(), 7);
    let forward: Vec<&str> = steps.iter().map(|index| lines[*index]).collect();
    for (index, line) in steps.iter().zip(forward.iter().rev()) {
        lines[*index] = line;
    }
    let reversed = lines.join("\n");
    assert_ne!(base, reversed);

    let graph = checked_counter(&base);
    let refs = row_refs_by_operation(&graph);
    assert_eq!(refs, row_refs_by_operation(&checked_counter(&reversed)));
    let mut positions: Vec<u32> = refs.values().map(|reference| reference.index()).collect();
    positions.sort_unstable();
    assert_eq!(positions, (0..7).collect::<Vec<_>>());

    let emission = only_refinement(&graph)
        .v2_emission()
        .expect("the refinement emits");
    let body = serde_json::to_value(&emission.body).expect("the body serializes");
    let digests: Vec<&str> = body["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .map(|step| step["source_node"]["digest"].as_str().expect("a node"))
        .collect();
    assert_eq!(digests.len(), 7);
    assert!(
        digests.windows(2).all(|pair| pair[0] <= pair[1]),
        "{digests:?}"
    );
}

/// FR-146-AC-6: with `AbsInv` on `Spec::Counter` beside `CasInv` on
/// `Impl::Counter` in the one package, every record lists `CasInv` and not
/// `AbsInv`, in either source order.
#[trace("TC-551", "FR-146-AC-6")]
#[test]
fn an_abstract_model_invariant_is_not_a_hypothesis_in_either_order() {
    let base = cas_refines_counter();
    let forward = checked_counter(&format!("{base}\n{CAS_INV}{ABS_INV}"));
    let backward = checked_counter(&format!("{base}\n{ABS_INV}{CAS_INV}"));
    let concrete = forward
        .state_clauses()
        .iter()
        .find(|clause| clause.name() == "CasInv")
        .expect("CasInv")
        .identity();
    for record in records(&forward) {
        assert_eq!(record.hypotheses, [concrete]);
    }
    assert_eq!(records(&forward), records(&backward));
}

/// FR-146-AC-9 and the first half of FR-146-AC-10: each record is keyed by
/// its O-07 occurrence (the refinement node, role `generated`, ordinal 0
/// for `Initial` and `i + 1` for `Step{row: i}`), and
/// `CheckedGraph::requirements()` holds an `operation-contract` record there.
#[trace("TC-551", "FR-146-AC-9")]
#[test]
fn each_record_has_its_generated_occurrence_key() {
    let graph = checked_counter(&cas_refines_counter());
    let node = refinement_node(only_refinement(&graph));
    let found = entries(&graph);
    assert_eq!(found.len(), 8);
    for (ordinal, entry) in found.iter().enumerate() {
        let origin = entry.occurrence().origin();
        assert_eq!(entry.occurrence().node(), node);
        assert_eq!(origin.role().as_str(), "generated");
        assert_eq!(origin.ordinal(), ordinal as u64);
        let expected = match ordinal {
            0 => StepObligation::Initial,
            row => StepObligation::Step {
                row: StepRowRef::new(u32::try_from(row - 1).expect("small")),
            },
        };
        assert_eq!(entry.record().obligation, expected);
        let record = graph
            .requirements()
            .get(entry.occurrence())
            .expect("a requirement record");
        assert_eq!(record.requirements().kind(), Capability::OperationContract);
    }
}

/// FR-146-AC-10: each of the eight records is `Unbounded` with one domain,
/// `Impl::counters`, of kind population and boundable by `Cardinality`;
/// `Spec::counters` adds none. Bounding `Impl::counters` makes each
/// `Bounded`, and an unread population `audits` adds no domain.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn each_record_ranges_over_the_concrete_population_only() {
    let graph = checked_counter(&cas_refines_counter());
    let counters = population_key(&graph, "Impl", "counters");
    let found = domains(&graph);
    assert_eq!(found.len(), 8);
    for (obligation, domains) in &found {
        assert_eq!(
            domains,
            &[(counters.clone(), DomainKind::Population)],
            "{obligation:?}"
        );
    }
    assert_eq!(
        DomainKind::Population.finite_kind(),
        Some(FiniteBoundKind::Cardinality)
    );
    let spec = population_key(&graph, "Spec", "counters");
    assert!(found
        .iter()
        .all(|(_, domains)| domains.iter().all(|(key, _)| *key != spec)));

    let bounded = checked_counter(&format!(
        "{}\nbound CountersBound on Impl::counters [2]\n",
        cas_refines_counter()
    ));
    let found = domains(&bounded);
    assert_eq!(found.len(), 8);
    assert!(
        found.iter().all(|(_, domains)| domains.is_empty()),
        "{found:?}"
    );

    let audit = object_type(CAS, "Audit", &[("seen", INTEGER)], Vec::new());
    let audited = checked_variant(
        plain_peek(),
        vec![audit],
        vec![population(CAS, "audits", "Audit")],
        "",
    );
    let counters = population_key(&audited, "Impl", "counters");
    for (obligation, domains) in domains(&audited) {
        assert_eq!(
            domains,
            [(counters.clone(), DomainKind::Population)],
            "{obligation:?}"
        );
    }
}

/// FR-146-AC-10: `peek(n: Integer)` gives `peek`'s `Step` record a second
/// unbounded domain, the `Integer` of `n`, and `peek(): Integer` the
/// result's; the other seven records keep the one population domain.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn an_unbounded_parameter_or_result_of_the_operation_is_a_domain() {
    for peek in [
        operation(CAS, "Counter", "peek", &[("n", INTEGER)], None, &[]),
        operation(CAS, "Counter", "peek", &[], Some(INTEGER), &[]),
    ] {
        let graph = checked_variant(peek, Vec::new(), Vec::new(), "");
        let counters = population_key(&graph, "Impl", "counters");
        let peek_row = row_refs_by_operation(&graph)
            .into_iter()
            .find(|(operation, _)| operation.ends_with("/peek"))
            .map(|(_, reference)| reference)
            .expect("the peek row");
        for (obligation, domains) in domains(&graph) {
            if obligation == (StepObligation::Step { row: peek_row }) {
                assert_eq!(domains.len(), 2, "{domains:?}");
                assert!(domains.contains(&(counters.clone(), DomainKind::Population)));
                assert!(domains
                    .iter()
                    .any(|(key, kind)| *kind == DomainKind::Integer
                        && matches!(key, DomainKey::Node { .. })));
            } else {
                assert_eq!(
                    domains,
                    [(counters.clone(), DomainKind::Population)],
                    "{obligation:?}"
                );
            }
        }
    }
}

/// FR-146-AC-10: a second population `spare` of `Impl::Counter` leaves the
/// receiver with no single context population: each step row is refused
/// `ambiguous_declaration`/`ambiguous-name` at its operation, naming both
/// populations in ascending `DeclarationKey` order.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn a_receiver_two_populations_cover_refuses_at_its_row() {
    let (spec, cas) = (
        counter_spec(),
        counter_cas_with(
            plain_peek(),
            &[],
            Vec::new(),
            vec![population(CAS, "spare", "Counter")],
        ),
    );
    let found: Vec<Refused> = check_with(
        "",
        &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)],
        &cas_refines_counter(),
    )
    .expect_err("the rows refuse");
    assert_eq!(
        found.len(),
        7,
        "{:?}",
        found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
    );
    for refused in &found {
        assert_eq!(
            refused.code_and_cause(),
            (Code::AmbiguousDeclaration, Some("ambiguous-name"))
        );
        assert!(refused.at.starts_with("Impl::Counter::"), "{}", refused.at);
        let message = format!("{:?}", refused.refinement());
        let counters = message.find("counters").expect("names counters");
        let spare = message.find("spare").expect("names spare");
        assert!(counters < spare, "{message}");
    }
}

/// `CasRefinesCounter` over a concrete package whose `Counter` has a `log`
/// into `Audit`s (population `audits`, a `next` edge) and a `mark` into
/// `Tag`s (population `tags`, a `next` edge), with a `Note` population
/// `notes`. `peek`'s precondition walks `audits` by `reaches` and its
/// postcondition names `notes` (FR-391); the state mapping walks `tags`.
fn walked() -> CheckedGraph {
    let linked = |name: &str| object_type(CAS, name, &[("next", &node(CAS, name))], Vec::new());
    let cas = counter_cas_with(
        plain_peek(),
        &[("log", &node(CAS, "Audit")), ("mark", &node(CAS, "Tag"))],
        vec![
            linked("Audit"),
            linked("Tag"),
            object_type(CAS, "Note", &[("seen", INTEGER)], Vec::new()),
        ],
        vec![
            population(CAS, "audits", "Audit"),
            population(CAS, "tags", "Tag"),
            population(CAS, "notes", "Note"),
        ],
    );
    let body = mutated(
        &cas_refines_counter(),
        "value = self.value;",
        "value = if reaches(self.mark, self.mark, next) then self.value else self.value;",
    );
    let clauses = "pre PeekLog using inf on Impl::Counter::peek { \
                   reaches(self.log, self.log, next) }\n\
                   post PeekNotes using inf on Impl::Counter::peek { \
                   forall(o in allInstances<Impl::Note>(Impl::notes) : o.seen >= 0) }\n";
    let body = format!("{body}\n{clauses}");
    check_with(
        "",
        &[("Spec", SPEC, &counter_spec()), ("Impl", CAS, &cas)],
        &body,
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "{body}: {:?}",
            refusals.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        )
    })
}

/// FR-146 "Claim extent" (TC-551): a population a `reaches` in an
/// operation's contract clauses walks (`audits`) and one its contract
/// clauses name (`notes`, FR-391) enter that operation's `Step` record only;
/// a population a `reaches` in the state mapping walks (`tags`) enters every
/// record, the `Initial` one included, since each obligation evaluates the
/// mapping.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn contract_and_mapping_populations_enter_the_records() {
    let graph = walked();
    let key = |population: &str| population_key(&graph, "Impl", population);
    let peek_row = row_refs_by_operation(&graph)
        .into_iter()
        .find(|(operation, _)| operation.ends_with("/peek"))
        .map(|(_, reference)| reference)
        .expect("the peek row");
    let found = domains(&graph);
    assert_eq!(found.len(), 8);
    for (obligation, domains) in found {
        let keys: Vec<DomainKey> = domains.into_iter().map(|(key, _)| key).collect();
        let mut expected = vec![key("counters"), key("tags")];
        if obligation == (StepObligation::Step { row: peek_row }) {
            expected.extend([key("audits"), key("notes")]);
        }
        let (mut keys, mut expected) = (keys, expected);
        keys.sort();
        expected.sort();
        assert_eq!(keys, expected, "{obligation:?}");
    }
}

/// FR-146 "Claim extent" (TC-551): a mapped concrete type with no operation
/// row that two concrete populations cover gives the `Initial` record no
/// single population, so the refinement is refused
/// `ambiguous_declaration`/`ambiguous-name` at its name, naming both
/// populations in ascending `DeclarationKey` order.
#[trace("TC-551")]
#[test]
fn a_mapped_type_two_populations_cover_refuses_at_the_refinement() {
    let spec = counter_spec_with(
        vec![object_type(SPEC, "Box", &[("size", INTEGER)], Vec::new())],
        vec![population(SPEC, "boxes", "Box")],
    );
    let cas = counter_cas_with(
        plain_peek(),
        &[],
        vec![object_type(CAS, "Crate", &[("size", INTEGER)], Vec::new())],
        vec![
            population(CAS, "crates", "Crate"),
            population(CAS, "spares", "Crate"),
        ],
    );
    let body = mutated(
        &cas_refines_counter(),
        "object Spec::Counter from Impl::Counter { value = self.value; }",
        "object Spec::Counter from Impl::Counter { value = self.value; }\n\
         population Spec::boxes from Impl::crates;\n\
         object Spec::Box from Impl::Crate { size = self.size; }",
    );
    let found: Vec<Refused> = check_with("", &[("Spec", SPEC, &spec), ("Impl", CAS, &cas)], &body)
        .expect_err("the refinement refuses");
    let [refused] = &found[..] else {
        panic!(
            "one refusal: {:?}",
            found.iter().map(|r| &r.refusal).collect::<Vec<_>>()
        );
    };
    assert_eq!(
        refused.code_and_cause(),
        (Code::AmbiguousDeclaration, Some("ambiguous-name"))
    );
    assert_eq!(refused.at, "CasRefinesCounter");
    let message = format!("{:?}", refused.refinement());
    let crates = message.find("crates").expect("names crates");
    let spares = message.find("spares").expect("names spares");
    assert!(crates < spares, "{message}");
}

/// FR-146 "Claim extent" (TC-551): every record reads its hypotheses, so a
/// population an invariant hypothesis ranges over (`notes`, through
/// `allInstances`) enters every record, the `Initial` one included.
#[trace("TC-551", "FR-146-AC-10")]
#[test]
fn a_hypothesis_population_enters_every_record() {
    let graph = checked_variant(
        plain_peek(),
        vec![object_type(CAS, "Note", &[("seen", INTEGER)], Vec::new())],
        vec![population(CAS, "notes", "Note")],
        "invariant NotesSeen using inf on Impl::Counter at current { \
         forall(o in allInstances<Impl::Note>(Impl::notes) : o.seen >= 0) }\n",
    );
    let mut expected = vec![
        population_key(&graph, "Impl", "counters"),
        population_key(&graph, "Impl", "notes"),
    ];
    expected.sort();
    let found = domains(&graph);
    assert_eq!(found.len(), 8);
    for (obligation, domains) in found {
        let mut keys: Vec<DomainKey> = domains.into_iter().map(|(key, _)| key).collect();
        keys.sort();
        assert_eq!(keys, expected, "{obligation:?}");
    }
}
