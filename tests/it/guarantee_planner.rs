// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1329 (FR-529-AC-1 to AC-13) and QSpec TC-984 case 1, TC-986 over a
//! real native package: QSpec TC-984 case 1's unit (the counter model, the
//! liveness clause `Eventually2`, the relation `A`, `guarantee G`) is
//! compiled from source through S1 to S4. Each item-backed slot holds the
//! FR-331 request item `qsl-route` writes for the slot, and a result entry
//! for it. A sibling clause (`guarantee H` over the safety clause) is
//! settled in every run and never changes.

use ix_trace_rs::trace;
use qsl_foundation::BackendId;
use qsl_replay::planner::{
    read_premise_item, PlannerCause, PlannerDeclaration, PlannerResult, PlannerValue, SlotKey,
    SlotRole, SlotValue,
};
use qsl_replay::{Certification, InconclusiveCause, ItemLabel, PremiseOutcome};
use qsl_route::declaration::DeclaredItem;
use qsl_route::request::{items_from_requirements, RequestWriter};
use qsl_route::{
    descriptor_from_manifest, Candidate, CandidateOutcome, FairnessKindsMember, FairnessLabel,
    ManifestMembers, ProviderOrigin, Registry,
};
use qsl_semantics::check::{CheckedImplementationGuarantee, SlotBacking as SemBacking};
use serde_json::{json, Value};

use crate::planner_chain::{compile_with, Chain, PACKAGE};

#[path = "../../qsl-semantics/tests/support/guarantee.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the semantics tests load the same fixture"
)]
mod fixture;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.model.complete/v1\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Spec = \"test/orders\";\n";

const LIVENESS: &str =
    "temporal Eventually2 using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { fair weak each Spec::Counter::inc; always eventually holds(c.value = 2) }\n";

const STRONG: &str = "temporal Strong using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { fair strong each Spec::Counter::inc; always eventually holds(c.value = 2) }\n";

const NESTED: &str = "temporal Nested using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { fair weak each Spec::Counter::inc; \
     always (holds(c.value >= 0) implies eventually always[0,1] holds(c.value = 2)) }\n";

const SAFETY: &str = "temporal NonNegative using t over (c: Spec::Counter) clock \"model-steps\" \
     on origin { always holds(c.value >= 0) }\n";

const RELATION: &str = "abstraction A using v {\n\
     type Spec::Counter to \"counter::Counter\" { value to \"value\"; };\n\
     population Spec::counters to \"state::counters\";\n\
     operation Spec::Counter::inc to \"counter::Counter::inc_cas\" receiver \"self\" ();\n\
     }\n";

const RELATION_WITHOUT_INC: &str = "abstraction A using v {\n\
     type Spec::Counter to \"counter::Counter\" { value to \"value\"; };\n\
     population Spec::counters to \"state::counters\";\n\
     }\n";

/// TC-984 case 1's declaration.
const CASE_ONE: &str = "guarantee G of Eventually2 using A { \
     initial to \"counter::Counter::new\"; \
     progress Spec::Counter::inc decreases \"counter::retries_left\"; \
     environment Sched by \"scheduler\" fair weak each Spec::Counter::inc; }\n";

/// The sibling guarantee over the safety clause.
const SIBLING: &str = "guarantee H of NonNegative using A { \
     initial to \"counter::Counter::new\"; \
     environment Sched2 by \"scheduler\" fair weak each Spec::Counter::inc; }\n";

const NO_PROGRESS: &str = "guarantee G of Eventually2 using A { \
     initial to \"counter::Counter::new\"; \
     environment Sched by \"scheduler\" fair weak each Spec::Counter::inc; }\n";

const MODEL_CLAIM: usize = 0;
const INITIALIZATION: usize = 1;
const TRANSITION: usize = 2;
const FRAME: usize = 3;
const NO_DIVERGENCE: usize = 5;
const FAIRNESS: usize = 6;

fn unit(clauses: &str, relation: &str, declarations: &str) -> String {
    format!("{HEADER}{clauses}{relation}{declarations}")
}

fn checked(unit: &str) -> qsl_package::CheckedPackage {
    compile_with(unit, &[fixture::document(&["inc"])])
        .unwrap_or_else(|failure| panic!("{unit}: the unit compiles: {failure:?}"))
}

fn guarantee<'p>(
    package: &'p qsl_package::CheckedPackage,
    name: &str,
) -> &'p CheckedImplementationGuarantee {
    package
        .graph()
        .implementation_guarantees()
        .iter()
        .find(|guarantee| guarantee.name() == name)
        .unwrap_or_else(|| panic!("a guarantee {name}"))
}

/// The chain of guarantee `name` in the package compiled from `source`.
fn chain(source: &str, name: &str) -> Chain {
    let package = checked(source);
    let node = guarantee(&package, name).node().clone();
    Chain::of(package, &node).expect("the declaration derives")
}

/// The environment record of the listed `Sched` member (weak each `inc`).
fn record(chain: &Chain) -> impl Fn(&str) -> Value {
    let context = chain
        .declaration
        .environment()
        .iter()
        .find_map(|member| match &member.supplies {
            qsl_replay::planner::EnvSupplies::Fairness(premise) => {
                Some(premise.operation.context.clone())
            }
            qsl_replay::planner::EnvSupplies::Assumption(_) => None,
        })
        .expect("a scheduling member");
    let context = qsl_foundation::node_wire::node_key_to_json(&context).expect("a node");
    let members: Vec<(String, String)> = chain
        .declaration
        .environment()
        .iter()
        .map(|member| (member.name.clone(), member.supplier.clone()))
        .collect();
    move |name: &str| {
        let supplier = members
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, supplier)| supplier.clone())
            .expect("a listed member");
        json!({
            "name": name,
            "supplier": supplier,
            "supplies": {"fairness": {
                "kind": "weak",
                "granularity": "each",
                "context": context,
                "operation": "inc",
            }},
        })
    }
}

fn positive(chain: &Chain) -> Value {
    chain.request(&record(chain))
}

fn premise(result: &PlannerResult) -> Option<(u64, PremiseOutcome)> {
    match &result.cause {
        Some(PlannerCause::Inconclusive(InconclusiveCause::PremiseUnproved {
            premise,
            result,
        })) => Some((*premise, *result)),
        _ => None,
    }
}

fn invalid(pointer: &str) -> (&'static str, &'static str, String) {
    ("invalid_runtime_input", "invalid-value", pointer.to_owned())
}

fn set_entry(request: &mut Value, slot: usize, entry: Value) {
    request["slots"][slot]["result"]["result"] = entry;
}

fn text<'u>(unit: &'u str, wire: &Value) -> &'u str {
    let start = usize::try_from(wire["start"].as_u64().expect("start")).expect("offset");
    let end = usize::try_from(wire["end"].as_u64().expect("end")).expect("offset");
    &unit[start..end]
}

/// The sibling guarantee settles `proved` in every run; its result never
/// moves.
fn sibling_settles_unchanged(source: &str) {
    let sibling = chain(source, "H");
    let result = sibling.settled(&positive(&sibling));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.cause, None);
}

// ---- FR-529-AC-1: the positive chain -----------------------------------------

/// QSpec TC-986 case 1, TC-984 case 1: the chain settles `proved`,
/// `decisive-witness`, and the conclusion is bound to the source spans of the
/// members that produced each slot.
#[trace("TC-1329", "FR-529-AC-1")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-1")]
#[trace("QSpec-TC-984")]
#[test]
fn the_positive_chain_settles_proved_with_its_conclusion_bound_to_the_source() {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{CASE_ONE}{SIBLING}"),
    );
    let chain = chain(&source, "G");
    assert_eq!(chain.declaration.slots().len(), 7);
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Proved, "{:?}", result.to_wire());
    assert_eq!(result.value.basis().as_str(), "decisive-witness");
    assert_eq!(result.certification, Some(Certification::Uncertified));
    let wire = result.to_wire().expect("a wire form");
    assert_eq!(wire["value"], "proved");
    assert_eq!(wire["basis"], "decisive-witness");
    // Every slot outcome carries the source of the member that produced it.
    let slots = wire["slots"].as_array().expect("slots");
    assert_eq!(slots.len(), 7);
    assert_eq!(text(&source, &slots[MODEL_CLAIM]["source"]), "Eventually2");
    assert_eq!(
        text(&source, &slots[INITIALIZATION]["source"]),
        "initial to \"counter::Counter::new\";"
    );
    assert_eq!(text(&source, &slots[FRAME]["source"]), "A");
    assert_eq!(
        text(&source, &slots[NO_DIVERGENCE]["source"]),
        "progress Spec::Counter::inc decreases \"counter::retries_left\";"
    );
    let conclusion = &wire["conclusion"];
    assert_eq!(
        text(&source, &conclusion["clause"]["text_span"]),
        "Eventually2"
    );
    let sources = conclusion["implementation_sources"]
        .as_array()
        .expect("sources");
    assert!(sources.contains(&json!("counter::Counter::new")));
    assert!(sources.contains(&json!("counter::Counter::inc_cas")));
    assert!(sources.contains(&json!("counter::retries_left")));
    assert_eq!(conclusion["relation"]["package"], PACKAGE);
    assert_eq!(conclusion["extent"], "unbounded");
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-2, AC-3: each mutation defeats only its own slot ---------------

fn liveness_chain() -> (String, Chain) {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{CASE_ONE}{SIBLING}"),
    );
    let chain = chain(&source, "G");
    (source, chain)
}

#[trace("TC-1329", "FR-529-AC-2")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-2")]
#[test]
fn a_refuted_transition_simulation_leaves_the_item_inconclusive_naming_it() {
    let (source, chain) = liveness_chain();
    let mut request = positive(&chain);
    set_entry(
        &mut request,
        TRANSITION,
        chain.entry(
            TRANSITION,
            json!({"result": "refuted", "counterexample": 0}),
        ),
    );
    request["slots"][TRANSITION]["result"]["counterexamples"] = json!({
        "request_index": chain.item(TRANSITION).item.index().get(),
        "backend": "verus",
        "transcript": 0,
    });
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_ne!(result.to_wire().expect("wire")["value"], "refuted");
    assert_eq!(
        premise(&result).map(|(slot, _)| slot),
        Some(TRANSITION as u64)
    );
    sibling_settles_unchanged(&source);
}

#[trace("TC-1329", "FR-529-AC-2")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-3")]
#[test]
fn a_liveness_declaration_without_progress_names_the_first_no_divergence_slot() {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{NO_PROGRESS}{SIBLING}"),
    );
    let chain = chain(&source, "G");
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((NO_DIVERGENCE as u64, PremiseOutcome::Absent))
    );
    assert_eq!(
        chain.declaration.slots()[NO_DIVERGENCE].role,
        SlotRole::NoDivergence
    );
    sibling_settles_unchanged(&source);
}

#[trace("TC-1329", "FR-529-AC-3")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-4")]
#[test]
fn a_not_proved_no_divergence_result_names_the_no_divergence_slot() {
    let (source, chain) = liveness_chain();
    let mut request = positive(&chain);
    set_entry(
        &mut request,
        NO_DIVERGENCE,
        chain.entry(
            NO_DIVERGENCE,
            json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
        ),
    );
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result).map(|(slot, _)| slot),
        Some(NO_DIVERGENCE as u64)
    );
    sibling_settles_unchanged(&source);
}

#[trace("TC-1329", "FR-529-AC-3")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-5")]
#[test]
fn a_fair_strong_clause_with_a_weak_scheduling_premise_fails_the_fairness_slot() {
    let source = unit(
        &format!("{STRONG}{SAFETY}"),
        RELATION,
        &format!("{}{SIBLING}", CASE_ONE.replace("Eventually2", "Strong")),
    );
    let chain = chain(&source, "G");
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result).map(|(slot, _)| slot),
        Some(FAIRNESS as u64)
    );
    let outcome = &result.slots[FAIRNESS];
    assert_eq!(outcome.role, SlotRole::FairnessCorrespondence);
    assert!(matches!(
        &outcome.cause,
        Some(PlannerCause::Inconclusive(
            InconclusiveCause::TransferNotPreserving { .. }
        ))
    ));
    // Every other slot proved: the mutation defeats only its own slot.
    for (index, slot) in result.slots.iter().enumerate() {
        if index != FAIRNESS {
            assert!(
                matches!(slot.value, SlotValue::Supplied)
                    || slot.value == SlotValue::Label(ItemLabel::Proved),
                "slot {index}"
            );
        }
    }
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-4: an unbound element, no backend, an exhausted budget ---------

/// QSpec TC-984 case 6: the relation does not bind `inc`'s frame, so the
/// guarantee item alone is refused `missing_declaration`/`missing-name`; the
/// package's other items are the records the bound unit has.
#[trace("TC-1329", "FR-529-AC-4")]
#[trace("QSpec-TC-984", "QSpec-FR-490-AC-6")]
#[test]
fn an_unbound_frame_refuses_the_guarantee_item_alone() {
    let clauses = format!("{LIVENESS}{SAFETY}");
    let source = unit(&clauses, RELATION_WITHOUT_INC, CASE_ONE);
    let package = checked(&source);
    let refusal = guarantee(&package, "G")
        .item_refusal()
        .expect("the guarantee item is refused");
    assert_eq!(refusal.code(), qsl_foundation::Code::MissingDeclaration);
    assert_eq!(refusal.cause(), "missing-name");
    let node = guarantee(&package, "G").node().clone();
    assert_eq!(
        PlannerDeclaration::from_package(&package, &node).err(),
        Some(qsl_replay::planner::DeclarationRefusal::ItemRefused {
            code: qsl_foundation::Code::MissingDeclaration,
            cause: "missing-name",
        })
    );
    // route writes no item for the refused guarantee.
    let written = RequestWriter::new().declaration_items(package.graph());
    assert!(written.is_empty());
    // The other items of the package are the bound unit's.
    let bound = checked(&unit(&clauses, RELATION, ""));
    assert_eq!(package.graph().requirements(), bound.graph().requirements());
}

/// An empty registry: no Verus backend registers `operation-contract`.
#[trace("TC-1329", "FR-529-AC-4")]
#[test]
fn an_unregistered_backend_settles_the_item_unsupported_naming_the_capability() {
    let (source, chain) = liveness_chain();
    let registry = Registry::new();
    let frame = &chain.item(FRAME).item;
    let CandidateOutcome::Candidates(candidates) = registry.candidates(frame.kind(), None) else {
        panic!("a computed candidate set");
    };
    assert!(candidates.is_empty(), "no backend is registered");
    let mut request = positive(&chain);
    set_entry(
        &mut request,
        FRAME,
        chain.entry(
            FRAME,
            json!({"result": "unsupported", "diagnostic": {
                "code": "unsupported_projection",
                "cause": "unsupported-requested-capability",
                "detail": {"kind": frame.kind().to_wire()},
            }}),
        ),
    );
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Unsupported);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["basis"], "unavailable");
    assert_eq!(wire["diagnostic"]["detail"]["kind"], "operation-contract");
    assert_eq!(wire["slots"][FRAME]["value"], "unsupported");
    sibling_settles_unchanged(&source);
}

#[trace("TC-1329", "FR-529-AC-4")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-9")]
#[test]
fn an_exhausted_work_budget_settles_the_item_incomplete_naming_max_work() {
    let (source, chain) = liveness_chain();
    let mut request = positive(&chain);
    request["limits"] = json!({"max_work": 2});
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Incomplete);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["cause"]["type"], "limit-reached");
    assert_eq!(wire["cause"]["limit"], "max_work");
    assert_eq!(wire["cause"]["setting"], "planner.max_work");
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-6, AC-7: admission ---------------------------------------------

#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-2", "QSpec-FR-492-AC-3")]
#[test]
fn a_forged_reordered_or_missing_slot_is_invalid_value_and_a_flag_is_wrong_kind() {
    let (source, chain) = liveness_chain();
    let mut dropped = positive(&chain);
    dropped["slots"]
        .as_array_mut()
        .expect("slots")
        .remove(FRAME);
    assert_eq!(chain.refusal(&dropped), invalid("/slots/3"));

    let mut swapped = positive(&chain);
    swapped["slots"].as_array_mut().expect("slots").swap(1, 2);
    assert_eq!(chain.refusal(&swapped), invalid("/slots/1"));

    let mut relabelled = positive(&chain);
    relabelled["slots"][FRAME]["role"] = json!("transition-simulation");
    assert_eq!(chain.refusal(&relabelled), invalid("/slots/3"));

    let mut forged = positive(&chain);
    forged["slots"][INITIALIZATION]["identity"]["claim"]["node"]["package"] = json!("other@2");
    assert_eq!(chain.refusal(&forged), invalid("/slots/1"));

    let mut elsewhere = positive(&chain);
    elsewhere["package"] = json!("other@2");
    assert_eq!(chain.refusal(&elsewhere), invalid("/package"));

    // A listed environment premise with no record.
    let mut no_record = positive(&chain);
    no_record["slots"][4]
        .as_object_mut()
        .expect("slot")
        .remove("record");
    assert_eq!(chain.refusal(&no_record), invalid("/slots/4"));

    let wrong = |pointer: &str| {
        (
            "invalid_runtime_input",
            "wrong-value-kind",
            pointer.to_owned(),
        )
    };
    let mut flag = positive(&chain);
    flag["slots"] = json!(true);
    assert_eq!(chain.refusal(&flag), wrong("/slots"));
    let mut result = positive(&chain);
    result["slots"][TRANSITION]["result"] = json!(true);
    assert_eq!(chain.refusal(&result), wrong("/slots/2/result"));
    sibling_settles_unchanged(&source);
}

#[trace("TC-1329", "FR-529-AC-7")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-3")]
#[test]
fn a_bare_label_or_a_reference_is_not_a_premise_result() {
    let (source, chain) = liveness_chain();
    let mut label = positive(&chain);
    label["slots"][INITIALIZATION]["result"] = json!("proved");
    assert_eq!(
        chain.refusal(&label),
        (
            "invalid_runtime_input",
            "wrong-value-kind",
            "/slots/1/result".to_owned()
        )
    );
    let mut reference = positive(&chain);
    reference["slots"][INITIALIZATION]["result"] = json!({"result": 3});
    assert_eq!(
        chain.refusal(&reference),
        (
            "invalid_runtime_input",
            "missing-member",
            "/slots/1/result/request".to_owned()
        )
    );
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-8: a finite trace ----------------------------------------------

#[trace("TC-1329", "FR-529-AC-8")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-7")]
#[test]
fn a_tested_ten_thousand_step_trace_never_settles_the_item_proved() {
    let (source, chain) = liveness_chain();
    let mut request = positive(&chain);
    set_entry(
        &mut request,
        MODEL_CLAIM,
        chain.entry(MODEL_CLAIM, json!({"result": "tested"})),
    );
    let result = chain.settled(&request);
    assert_ne!(result.value, PlannerValue::Proved);
    assert_ne!(result.to_wire().expect("wire")["value"], "refuted");
    assert_eq!(
        premise(&result),
        Some((MODEL_CLAIM as u64, PremiseOutcome::Label(ItemLabel::Tested)))
    );
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-9: the relation stays inert -----------------------------------

fn registry_for(graph: &qsl_semantics::check::CheckedGraph) -> Registry {
    let mut registry = Registry::new();
    let mut kinds: Vec<&'static str> = graph
        .requirements()
        .values()
        .map(|record| record.requirements().kind().to_wire())
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    for kind in kinds {
        let descriptor = descriptor_from_manifest(
            Candidate::new(BackendId::new(format!("backend-{kind}"))),
            ProviderOrigin::Process,
            [(Some(kind), Some("unbounded"))],
            ManifestMembers {
                domains: None,
                fairness_kinds: (kind == "temporal-satisfaction")
                    .then_some(FairnessKindsMember::List(&[FairnessLabel::Text("weak")])),
                evidence_kinds: None,
                memory_models: None,
                reductions: None,
            },
        )
        .expect("a valid manifest");
        registry.register(descriptor).expect("admitted");
    }
    registry
}

#[trace("TC-1329", "FR-529-AC-9")]
#[trace("QSpec-TC-984", "QSpec-FR-490-AC-8")]
#[test]
fn a_relation_with_no_guarantee_has_the_records_and_route_results_it_has_without() {
    let clauses = format!("{LIVENESS}{SAFETY}");
    let with = checked(&unit(&clauses, RELATION, ""));
    let without = checked(&unit(&clauses, "", ""));
    assert!(with.abstraction().is_some());
    assert!(without.abstraction().is_none());
    assert_eq!(with.graph().requirements(), without.graph().requirements());
    let route_of = |package: &qsl_package::CheckedPackage| {
        let registry = registry_for(package.graph());
        items_from_requirements(package.graph().requirements(), &registry, None)
            .iter()
            .map(|item| format!("{:?}", item.candidates()))
            .collect::<Vec<_>>()
    };
    assert_eq!(route_of(&with), route_of(&without));
    // route writes no declaration item for the relation alone.
    assert!(RequestWriter::new()
        .declaration_items(with.graph())
        .is_empty());
}

// ---- FR-529-AC-10: the planner-only causes ---------------------------------

#[trace("TC-1329", "FR-529-AC-10")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-10")]
#[test]
fn a_plugin_entry_carrying_a_planner_only_cause_is_refused_unknown_cause() {
    use qsl_replay::planner::{read_result_entry, EntryCause, EntryOrigin};
    for cause in [
        json!({"type": "premise-unproved", "premise": 3, "result": "absent"}),
        json!({"type": "transfer-not-preserving", "reason": "fragment", "operator": "previous"}),
    ] {
        let entry = json!({
            "request_index": 0,
            "backend": "verus",
            "kind": "operation-contract",
            "result": "inconclusive",
            "cause": cause,
        });
        let refusal = read_result_entry(&entry, 4, EntryOrigin::Plugin).expect_err("refused");
        assert_eq!(refusal.pointer, "/cause/type");
        assert_eq!(refusal.cause, EntryCause::UnknownCause);
    }
}

// ---- FR-529-AC-11: the stuttering-invariant fragment ---------------------------

#[trace("TC-1329", "FR-529-AC-11")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-11", "QSpec-TC-982")]
#[test]
fn an_interval_operator_under_an_unbounded_one_is_not_preserved_with_every_slot_proved() {
    let source = unit(
        &format!("{NESTED}{SAFETY}"),
        RELATION,
        &format!("{}{SIBLING}", CASE_ONE.replace("Eventually2", "Nested")),
    );
    let chain = chain(&source, "G");
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Inconclusive);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["cause"]["type"], "transfer-not-preserving");
    assert_eq!(wire["cause"]["reason"], "fragment");
    assert_eq!(wire["cause"]["operator"], "always[0,1]");
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-12: the slot key binds the claim -----------------------------

#[trace("TC-1329", "FR-529-AC-12")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-5", "QSpec-TC-982")]
#[test]
fn a_result_for_another_slot_key_is_invalid_value_and_certified_reads_uncertified() {
    let (source, chain) = liveness_chain();
    // A frame result in the transition-simulation slot.
    let mut frame_in_transition = positive(&chain);
    frame_in_transition["slots"][TRANSITION]["result"] =
        frame_in_transition["slots"][FRAME]["result"].clone();
    assert_eq!(
        chain.refusal(&frame_in_transition),
        invalid("/slots/2/result")
    );
    // A model claim checked without its stutter closure.
    let mut no_stutter = positive(&chain);
    no_stutter["slots"][MODEL_CLAIM]["result"]["request"]["temporal"]
        .as_object_mut()
        .expect("temporal")
        .remove("stutter_closure");
    assert_eq!(chain.refusal(&no_stutter), invalid("/slots/0/result"));
    // A model claim under another fairness set.
    let mut other_fairness = positive(&chain);
    other_fairness["slots"][MODEL_CLAIM]["result"]["request"]["temporal"]["fairness"] = json!([]);
    assert_eq!(chain.refusal(&other_fairness), invalid("/slots/0/result"));
    // A subject of another package.
    let mut other_subject = positive(&chain);
    other_subject["slots"][MODEL_CLAIM]["result"]["request"]["temporal"]["subject"]["identity"] =
        json!("other");
    assert_eq!(chain.refusal(&other_subject), invalid("/slots/0/result"));
    // Every slot filled with `certified` entries read from bytes settles
    // `proved`, `uncertified`.
    let result = chain.settled(&positive(&chain));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.certification, Some(Certification::Uncertified));
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-13: every route-written item reads back ------------------------

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn every_derived_item_route_writes_reads_back_with_an_equal_slot_key() {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{CASE_ONE}{SIBLING}"),
    );
    let package = checked(&source);
    for name in ["G", "H"] {
        let node = guarantee(&package, name).node().clone();
        let chain = Chain::of(checked(&source), &node).expect("derives");
        let mut seen = 0;
        for (index, slot) in chain.declaration.slots().iter().enumerate() {
            let SlotKey::Item(derived) = &slot.key else {
                continue;
            };
            let item = read_premise_item(&chain.item_json(index)).expect("the item reads");
            assert_eq!(&item.key, derived, "{name} slot {index}");
            assert_eq!(
                item.request_index,
                chain.item(index).item.index().get() as u64
            );
            seen += 1;
        }
        assert!(seen >= 4, "{name} has item-backed slots");
    }
    // route wrote an item for every item-backed slot, with the FR-331
    // members the slot's role gives it.
    let written = RequestWriter::new().declaration_items(package.graph());
    let g = guarantee(&package, "G");
    let expected = g
        .slots(PACKAGE)
        .iter()
        .filter(|slot| slot.backing == SemBacking::Item)
        .count()
        + guarantee(&package, "H")
            .slots(PACKAGE)
            .iter()
            .filter(|slot| slot.backing == SemBacking::Item)
            .count();
    assert_eq!(written.len(), expected);
    let claims = written
        .iter()
        .filter(|written: &&DeclaredItem| written.item.stutter_closure())
        .count();
    assert_eq!(
        claims, 1,
        "only the liveness model claim has stutter_closure"
    );
    let contracts = written
        .iter()
        .filter(|written| written.item.correspondence().is_some())
        .count();
    assert_eq!(
        contracts,
        written.len() - 2,
        "every other item is a contract"
    );
    let indexes: Vec<usize> = written.iter().map(|w| w.item.index().get()).collect();
    assert_eq!(indexes, (0..written.len()).collect::<Vec<_>>());
}

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn a_result_entry_defect_is_refused_at_its_slot_in_the_reading_order() {
    let (source, chain) = liveness_chain();
    let mut request = positive(&chain);
    request["slots"][TRANSITION]["result"]["result"]
        .as_object_mut()
        .expect("entry")
        .remove("certification");
    let refusal = chain.read(&request).expect_err("refused");
    assert_eq!(refusal.code(), "invalid_result");
    assert_eq!(refusal.cause(), "missing-member");
    assert_eq!(refusal.pointer(), "/slots/2/result/result/certification");
    sibling_settles_unchanged(&source);
}

// ---- FR-529-AC-5: compile and read back ---------------------------------------

#[trace("TC-1329", "FR-529-AC-5")]
#[trace("QSpec-TC-984", "QSpec-FR-490-AC-1")]
#[test]
fn the_guarantee_node_reads_back_to_the_same_declaration_and_slots() {
    let source = unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{CASE_ONE}{SIBLING}"),
    );
    let package = checked(&source);
    let guarantee = guarantee(&package, "G");
    let node = qsl_package::write_implementation_guarantee(guarantee).expect("the node writes");
    let read = qsl_package::read_implementation_guarantee(&package, &node).expect("it reads back");
    assert_eq!(&read, guarantee);
    assert_eq!(read.slots(PACKAGE), guarantee.slots(PACKAGE));
}

// ---- finding review: identity, route wiring, wire assumptions, origin --------

/// The premise items of a package's declarations reach the finished request
/// the production writer (`ModelRequest`) finishes, and are negotiated as
/// every other item: each is in `items()` at its index and routes to the
/// candidates the registry computes for its kind.
#[trace("TC-1329", "FR-529-AC-1")]
#[test]
fn the_model_request_writes_and_negotiates_the_declaration_premise_items() {
    use qsl_route::model_request::ModelRequest;
    use qsl_route::request::DerivedLimits;
    let package = checked(&unit(
        &format!("{LIVENESS}{SAFETY}"),
        RELATION,
        &format!("{CASE_ONE}{SIBLING}"),
    ));
    let registry = registry_for(package.graph());
    let request = ModelRequest::<()>::new().derived_with_declarations(
        &registry,
        None,
        &DerivedLimits::default(),
        package.graph(),
    );
    assert!(!request.declared().is_empty());
    for declared in request.declared() {
        let at = declared.item.index();
        assert_eq!(request.items()[at.get()], declared.item);
        assert!(request.obligation(at).is_none());
        assert!(matches!(
            registry.candidates(declared.item.kind(), None),
            CandidateOutcome::Candidates(_)
        ));
    }
    // Without the graph the writer adds none.
    let plain = ModelRequest::<()>::new().derived(&registry, None, &DerivedLimits::default());
    assert!(plain.declared().is_empty());
}

/// The package identity is the checked package's: a request naming another
/// package is refused at `/package`.
#[trace("TC-1329", "FR-529-AC-6")]
#[test]
fn the_package_identity_is_the_checked_packages_not_the_callers() {
    let (_, chain) = liveness_chain();
    assert_eq!(chain.package.graph().package_name(), PACKAGE);
    let mut request = positive(&chain);
    request["package"] = json!("other");
    assert_eq!(chain.refusal(&request), invalid("/package"));
}

/// A model claim whose premise result names an assumption no environment
/// record supplies settles `premise-unproved` naming the model-claim slot,
/// from request bytes alone; any other slot's result refuses the member.
#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-12")]
#[test]
fn a_wire_model_claim_naming_an_unsupplied_assumption_is_premise_unproved() {
    let (_, chain) = liveness_chain();
    let mut request = positive(&chain);
    request["slots"][MODEL_CLAIM]["result"]["assumptions"] = json!(["Fair"]);
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result).map(|(slot, _)| slot),
        Some(MODEL_CLAIM as u64)
    );
    let mut elsewhere = positive(&chain);
    elsewhere["slots"][FRAME]["result"]["assumptions"] = json!(["Fair"]);
    assert_eq!(
        chain.refusal(&elsewhere),
        invalid("/slots/3/result/assumptions")
    );
}

/// An entry's backend name decides nothing: its own certification does. A
/// proved entry that states `uncertified` stays first-party `uncertified`
/// whatever backend it names.
#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-8")]
#[test]
fn a_backend_name_does_not_make_an_entry_trusted() {
    let (_, chain) = liveness_chain();
    let mut request = positive(&chain);
    set_entry(
        &mut request,
        FRAME,
        chain.entry(
            FRAME,
            json!({"result": "proved", "backend": "acme", "basis": {"type": "exhaustive"}, "certification": "uncertified"}),
        ),
    );
    let result = chain.settled(&request);
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.certification, Some(Certification::Uncertified));
}
