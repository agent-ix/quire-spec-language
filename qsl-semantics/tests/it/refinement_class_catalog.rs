// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-375 (TC-310): the class catalog of a refinement whose abstract side is
//! a model, and whose concrete model has a protocol clause: the operation
//! classes of every operation owner's effective view with FR-432's attempt
//! binder record, and the catalog lowered to the package graph's node keys.

use std::collections::BTreeSet;

use ix_trace_rs::trace;
use qsl_semantics::check::{
    CheckedGraph, CheckedRefinement, KeyedCatalogClass, RefinementCatalogClass,
    RefinementCatalogNode,
};
use quire_exact::ValueType;

use quire_semantic_value::checking::CheckingLimits;

use super::refinement_declarations::{
    assemble, check_with, counter_spec, diamond_counter, sub_counter, DIAMOND, SPEC, SUB,
};

const PROFILES: &str = "profile p = \"quire.protocol.finite-global/v1\";\n";

/// A protocol clause over `Impl::Counter` whose every part is written: a
/// static role, an `unknown`-delivery channel (`duplicate` and `lose`), an
/// empty run and `finish`.
const LOCAL: &str = "protocol Local using p over (c: Impl::Counter) on origin {\n\
     role w on Impl::Counter;\n\
     channel ch from w to w carries Boolean ordering unordered delivery unknown \
     capacity 2 overflow block;\n\
     run sequence Main { }\n\
     finish Done as (d: Boolean) { true };\n\
     }\n";

/// A refinement of the counter `Spec` by the sub-counter model, every
/// concrete step a stutter.
const SUB_ROWS: &str = "refinement R using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::peek -> stutter;\n\
     step Impl::SubCounter::bump -> stutter;\n\
     }\n";

/// A refinement of the counter `Spec` by the diamond model.
const DIAMOND_ROWS: &str = "refinement R using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Child { value = self.value; }\n\
     step Impl::Base1::op -> stutter;\n\
     step Impl::Base2::op -> stutter;\n\
     }\n";

fn checked(concrete: (&str, &[u8]), body: &str) -> CheckedGraph {
    let spec = counter_spec();
    check_with(
        PROFILES,
        &[("Spec", SPEC, &spec), ("Impl", concrete.0, concrete.1)],
        body,
    )
    .unwrap_or_else(|refusals| panic!("{body}: {refusals:?}"))
}

fn the_refinement(graph: &CheckedGraph) -> &CheckedRefinement {
    let [refinement] = graph.refinements() else {
        panic!("one refinement: {:?}", graph.refinements());
    };
    refinement
}

fn classes(side: &[RefinementCatalogClass]) -> Vec<&str> {
    side.iter().map(RefinementCatalogClass::class).collect()
}

fn class<'c>(side: &'c [RefinementCatalogClass], name: &str) -> &'c RefinementCatalogClass {
    side.iter()
        .find(|class| class.class() == name)
        .unwrap_or_else(|| panic!("the class {name} in {:?}", classes(side)))
}

/// An inherited operation is a class of the subtype too, keyed by the
/// subtype and taking it as its receiver; each operation's binder record
/// is (receiver, parameters…, result). The model abstract side lists the
/// abstract model's operation classes (FR-375 "or the abstract model's
/// operation classes otherwise").
#[trace("FR-375-AC-23", "TC-310", "FR-432-AC-14")]
#[test]
fn a_model_abstract_catalog_lists_effective_operations_with_attempt_records() {
    let sub = sub_counter();
    let graph = checked((SUB, &sub), &format!("{LOCAL}{SUB_ROWS}"));
    let catalog = the_refinement(&graph)
        .class_catalog()
        .expect("the concrete protocol contributes a catalog");

    assert_eq!(
        classes(catalog.concrete()),
        [
            "Counter::peek",
            "Local::Done",
            "SubCounter::bump",
            "SubCounter::peek",
            "duplicate Local::ch",
            "lose Local::ch",
        ]
    );
    let peek = class(catalog.concrete(), "Counter::peek");
    let inherited = class(catalog.concrete(), "SubCounter::peek");
    let bump = class(catalog.concrete(), "SubCounter::bump");
    // The inherited class is the subtype's: its node and its receiver.
    assert_eq!(inherited.node(), bump.node());
    assert_ne!(inherited.node(), peek.node());
    assert!(matches!(
        inherited.node(),
        Some(RefinementCatalogNode::Operation(owner)) if owner.node.ends_with("/SubCounter")
    ));
    assert_eq!(inherited.binder_types(), bump.binder_types());
    assert_ne!(inherited.binder_types(), peek.binder_types());
    assert!(matches!(
        inherited.binder_types(),
        [ValueType::Reference(_)]
    ));

    assert_eq!(
        classes(catalog.abstract_side()),
        ["Counter::inc", "Counter::quad"]
    );
    let inc = class(catalog.abstract_side(), "Counter::inc");
    let quad = class(catalog.abstract_side(), "Counter::quad");
    let [receiver] = inc.binder_types() else {
        panic!("inc's record is its receiver: {:?}", inc.binder_types());
    };
    assert!(matches!(receiver, ValueType::Reference(_)));
    let [quad_receiver, x, result] = quad.binder_types() else {
        panic!(
            "quad's record is (receiver, x, result): {:?}",
            quad.binder_types()
        );
    };
    assert_eq!(quad_receiver, receiver);
    assert!(!matches!(x, ValueType::Reference(_)));
    assert!(!matches!(result, ValueType::Reference(_)));
}

/// A name two unrelated ancestors declare has no effective operation on
/// the subtype, so the subtype has no class of it; each ancestor keeps its
/// own.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn an_ambiguous_inherited_operation_is_no_class_of_the_subtype() {
    let diamond = diamond_counter();
    let local = LOCAL.replace("Impl::Counter", "Impl::Base1");
    let graph = checked((DIAMOND, &diamond), &format!("{local}{DIAMOND_ROWS}"));
    let catalog = the_refinement(&graph)
        .class_catalog()
        .expect("the concrete protocol contributes a catalog");
    let operations: Vec<&str> = classes(catalog.concrete())
        .into_iter()
        .filter(|class| !class.contains("Local"))
        .collect();
    assert_eq!(operations, ["Base1::op", "Base2::op"]);
}

/// The catalog lowered to node keys keeps every class in order, and every
/// class node and binder type is a node of the package graph.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn the_catalog_lowers_to_nodes_of_the_package_graph() {
    let sub = sub_counter();
    let graph = checked((SUB, &sub), &format!("{LOCAL}{SUB_ROWS}"));
    let refinement = the_refinement(&graph);
    let catalog = refinement.class_catalog().expect("a catalog");
    let keyed = refinement
        .class_catalog_keys()
        .expect("every class node is minted");
    let spelled = |side: &[KeyedCatalogClass]| -> Vec<String> {
        side.iter().map(|class| class.class().to_owned()).collect()
    };
    assert_eq!(spelled(keyed.concrete()), classes(catalog.concrete()));
    assert_eq!(
        spelled(keyed.abstract_side()),
        classes(catalog.abstract_side())
    );
    let nodes: BTreeSet<_> = graph
        .semantic_graph()
        .nodes()
        .map(|node| node.key())
        .collect();
    let used = keyed.node_keys();
    assert!(!used.is_empty());
    for key in &used {
        assert!(nodes.contains(key), "{key} is a graph node");
    }
    // The inherited class shares the subtype's node and record; the
    // ancestor's own class has another node.
    let declared = |name: &str| match keyed.concrete().iter().find(|class| class.class() == name) {
        Some(KeyedCatalogClass::Declared {
            node, binder_types, ..
        }) => (node.clone(), binder_types.clone()),
        other => panic!("{name}: {other:?}"),
    };
    assert_eq!(declared("SubCounter::peek"), declared("SubCounter::bump"));
    assert_ne!(declared("SubCounter::peek").0, declared("Counter::peek").0);
}

/// FR-375-AC-27, FR-432's binder-record table: a clause that activates
/// `on each` has the class `activate <P>`, whose node is the clause and
/// whose binder record is the trigger's: one field of the trigger's type
/// when it is no record, so `[Boolean]` for a Boolean trigger and one
/// reference type for an object trigger. An `origin` clause has no such
/// class.
#[trace("FR-375-AC-27", "FR-375-AC-23", "TC-310")]
#[test]
fn an_each_clause_has_an_activate_class_with_the_trigger_record() {
    let sub = sub_counter();
    for (trigger, reference) in [("(t: Boolean)", false), ("(k: Impl::Counter)", true)] {
        let local = LOCAL.replace("on origin", &format!("on each {trigger}"));
        let graph = checked((SUB, &sub), &format!("{local}{SUB_ROWS}"));
        let refinement = the_refinement(&graph);
        let catalog = refinement.class_catalog().expect("a catalog");
        let activate = class(catalog.concrete(), "activate Local");
        assert_eq!(
            activate.node(),
            Some(&RefinementCatalogNode::Trigger("Local".to_owned()))
        );
        match activate.binder_types() {
            [ValueType::Boolean] => assert!(!reference, "{trigger}"),
            [ValueType::Reference(_)] => assert!(reference, "{trigger}"),
            other => panic!("{trigger}: the one trigger field, got {other:?}"),
        }
        // The class node is the clause's own node.
        let keyed = refinement.class_catalog_keys().expect("every node minted");
        let Some(KeyedCatalogClass::Declared {
            node, binder_types, ..
        }) = keyed
            .concrete()
            .iter()
            .find(|class| class.class() == "activate Local")
        else {
            panic!("{trigger}: a keyed activate class");
        };
        let clause = graph
            .semantic_graph()
            .nodes()
            .find(|candidate| candidate.semantic_form() == "protocol_clause")
            .expect("the clause node");
        assert_eq!(*node, clause.key(), "{trigger}");
        assert_eq!(binder_types.len(), 1, "{trigger}");
    }
    let graph = checked((SUB, &sub), &format!("{LOCAL}{SUB_ROWS}"));
    let catalog = the_refinement(&graph).class_catalog().expect("a catalog");
    assert!(!classes(catalog.concrete()).contains(&"activate Local"));
}

/// A contributing clause that is not written yet (it holds a post-test
/// `repeat`, STD-493) has no node for its classes, so it leaves the catalog
/// unlowered: the refinement has its checked catalog but no keyed one, so
/// it is not in the V2 emission slice.
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn a_clause_class_with_no_node_leaves_the_catalog_unlowered() {
    let sub = sub_counter();
    let local = LOCAL.replace(
        "run sequence Main { }",
        "run sequence Main {\n\
         attempt Try by w on Impl::Counter::peek contracts [] as (r: Boolean) { true };\n\
         repeat Again by w visible (e.ok) while { not e.ok } \
         sequence Body { event E by w as (e: Ack) { true }; }\n\
         }",
    );
    let spec = counter_spec();
    let (_, declarations) = assemble(
        PROFILES,
        &[("Spec", SPEC, &spec), ("Impl", SUB, &sub)],
        &format!("record Ack {{ ok: Boolean; }}\n{local}{SUB_ROWS}"),
    );
    let graph = declarations
        .expect("the unit assembles")
        .with_unwritten_refinements()
        .with_unwritten_protocols()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let refinement = the_refinement(&graph);
    let catalog = refinement.class_catalog().expect("a checked catalog");
    assert!(classes(catalog.concrete()).contains(&"Local::Try"));
    assert!(refinement.class_catalog_keys().is_none());
    assert!(!refinement.v2_emittable());
}

/// A contributing clause holding a `choice` and a pre-test `repeat`, both
/// written as V2 control nodes (QSL-1186b), keys its class catalog: the
/// events inside the case and the repeat body are classes with nodes
/// (SR-1737002 FND-003).
#[trace("FR-375-AC-23", "TC-310")]
#[test]
fn a_clause_holding_a_choice_and_a_repeat_keys_its_catalog() {
    let sub = sub_counter();
    let local = LOCAL.replace(
        "run sequence Main { }",
        "run sequence Main {\n\
         choice Pick by w visible () { \
         case a when { true } sequence A { event Ea by w as (ea: Boolean) { true }; } \
         case b when { false } sequence B { event Eb by w as (eb: Boolean) { true }; } }\n\
         repeat Again by w visible () max 1 while { true } \
         sequence Body { event E by w as (e: Boolean) { true }; } exhausted sequence Out { }\n\
         }",
    );
    let spec = counter_spec();
    let (_, declarations) = assemble(
        PROFILES,
        &[("Spec", SPEC, &spec), ("Impl", SUB, &sub)],
        &format!("{local}{SUB_ROWS}"),
    );
    let graph = declarations
        .expect("the unit assembles")
        .with_unwritten_refinements()
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let refinement = the_refinement(&graph);
    let catalog = refinement.class_catalog().expect("a checked catalog");
    let concrete = classes(catalog.concrete());
    for class in ["Local::Ea", "Local::Eb", "Local::E"] {
        assert!(concrete.contains(&class), "{class} in {concrete:?}");
    }
    assert!(refinement.class_catalog_keys().is_some());
}
