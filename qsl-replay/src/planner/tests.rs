// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1329 and QSpec TC-986 over hand-built declarations: admission,
//! refusals, decision order, premise results, causes and the conclusion
//! record. No source is compiled; the counter guarantee of QSpec TC-984
//! case 1 is built as plain data.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::{OccurrenceKey, RawSourceRef, SourceRegion};
use qsl_foundation::{BackendId, RequestIndex};
use quire_exact::{NodeKey, Origin, PathSegment, Role};
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
};
use serde_json::{json, Value};

use super::*;
use crate::outcome::ItemLabel;
use crate::proof_result::{
    read_plugin_inconclusive_cause, Certification, InconclusiveCause, PluginResultRefusal,
    PremiseOutcome, QslOnlyCause, TerminalRecord, TerminalValue, TransferReason,
};

pub(super) const PACKAGE: &str = "app@1";

pub(super) fn node_in(package: &str, label: &str) -> NodeKey {
    NodeKey::new(package, vec![PathSegment::Text(label.to_owned())], 0).expect("a node")
}

pub(super) fn node(label: &str) -> NodeKey {
    node_in(PACKAGE, label)
}

pub(super) fn constraint(
    kind: FairnessKind,
    granularity: FairnessGranularity,
) -> FairnessConstraint {
    FairnessConstraint::new(
        kind,
        FairnessOperation::new(crate::fixture_effective(7), "inc".to_owned()),
        granularity,
    )
}

pub(super) fn weak_each() -> FairnessConstraint {
    constraint(FairnessKind::Weak, FairnessGranularity::Each)
}

fn inc() -> OperationRef {
    OperationRef {
        context: node("Counter"),
        operation: "inc".to_owned(),
    }
}

pub(super) fn region(start: u64) -> SourceRegion {
    SourceRegion::new(
        RawSourceRef::new("fs", "counter.qsl").expect("a source"),
        start,
        start + 4,
    )
    .expect("a region")
}

fn subject_text() -> String {
    json::canonical_text(&json!({"initial_states": [], "universes": [], "proof_bounds": []}))
}

fn claim(label: &str) -> OccurrenceKey {
    OccurrenceKey::new(node(label), Origin::new(Role::new("claim"), 0))
}

fn model_claim_key(stutter: bool, fairness: &[FairnessConstraint]) -> ItemKey {
    let mut set: Vec<FairnessKey> = fairness
        .iter()
        .map(|constraint| FairnessKey::of(constraint).expect("a fairness key"))
        .collect();
    set.sort();
    ItemKey {
        claim: claim("P"),
        kind: ClaimKind::TemporalSatisfaction,
        temporal: Some(TemporalKey {
            subject: subject_text(),
            item_kind: TemporalItemKind::Formula,
            fairness: set,
            stutter_closure: stutter,
        }),
        correspondence: None,
        premises: Vec::new(),
        extent: ExtentClass::Unbounded,
    }
}

fn contract_key(role: CorrespondenceRole, operation: Option<OperationRef>) -> ItemKey {
    ItemKey {
        claim: claim("G"),
        kind: ClaimKind::OperationContract,
        temporal: None,
        correspondence: Some(Correspondence { role, operation }),
        premises: Vec::new(),
        extent: ExtentClass::Unbounded,
    }
}

pub(super) fn slot(role: SlotRole, key: SlotKey, backing: SlotBacking, start: u64) -> DerivedSlot {
    DerivedSlot {
        role,
        key,
        backing,
        constraint: None,
        operation: None,
        premise_subject: None,
        peer_clause: None,
        cycle: None,
        source: Some(region(start)),
    }
}

pub(super) fn local(role: SlotRole, member: &str) -> SlotKey {
    SlotKey::Local(LocalKey {
        role,
        members: vec![member.to_owned()],
    })
}

/// QSpec TC-984 case 1's guarantee: the counter's `Eventually2` under
/// `fair <fp>`, one `inc`, a `progress` member (or none) and one scheduling
/// environment member `E` (weak each).
fn declaration_with(with_progress: bool, fp: &FairnessConstraint) -> PlannerDeclaration {
    let operation = Some(inc());
    let progress = if with_progress {
        slot(
            SlotRole::NoDivergence,
            SlotKey::Item(contract_key(
                CorrespondenceRole::NoDivergence,
                operation.clone(),
            )),
            SlotBacking::Item,
            50,
        )
    } else {
        slot(
            SlotRole::NoDivergence,
            local(SlotRole::NoDivergence, "inc"),
            SlotBacking::Unfilled,
            50,
        )
    };
    let mut fairness = slot(
        SlotRole::FairnessCorrespondence,
        local(SlotRole::FairnessCorrespondence, "inc"),
        SlotBacking::PlannerLocal,
        60,
    );
    fairness.constraint = Some(fp.clone());
    fairness.operation = operation.clone();
    let mut frame = slot(
        SlotRole::Frame,
        SlotKey::Item(contract_key(CorrespondenceRole::Frame, operation.clone())),
        SlotBacking::Item,
        30,
    );
    frame.operation = operation.clone();
    PlannerDeclaration {
        role: ItemRole::Guarantee,
        node: node("G"),
        package: PACKAGE.to_owned(),
        rule: None,
        clause: Some(ClauseFacts {
            node: node("P"),
            form: ClauseForm::Liveness,
            fairness: vec![fp.clone()],
            fragment_operator: None,
            clause_text_span: Some(region(100)),
        }),
        relation_sources: vec!["counter::init".to_owned(), "counter::inc".to_owned()],
        relation_node: Some(node("A")),
        environment: vec![EnvironmentMember {
            name: "E".to_owned(),
            supplier: "kernel".to_owned(),
            supplies: EnvSupplies::Fairness(SchedulingPremise {
                constraint: weak_each(),
                operation: inc(),
            }),
        }],
        slots: vec![
            slot(
                SlotRole::ModelClaim,
                SlotKey::Item(model_claim_key(true, std::slice::from_ref(fp))),
                SlotBacking::Item,
                0,
            ),
            slot(
                SlotRole::Initialization,
                SlotKey::Item(contract_key(CorrespondenceRole::Initialization, None)),
                SlotBacking::Item,
                10,
            ),
            slot(
                SlotRole::TransitionSimulation,
                SlotKey::Item(contract_key(
                    CorrespondenceRole::TransitionSimulation,
                    operation,
                )),
                SlotBacking::Item,
                20,
            ),
            frame,
            slot(
                SlotRole::Environment,
                local(SlotRole::Environment, "E"),
                SlotBacking::Environment {
                    member: Some("E".to_owned()),
                },
                40,
            ),
            progress,
            fairness,
        ],
        supplies: Vec::new(),
    }
}

pub(super) fn declaration() -> PlannerDeclaration {
    declaration_with(true, &weak_each())
}

const MODEL_CLAIM: usize = 0;
const INITIALIZATION: usize = 1;
const TRANSITION: usize = 2;
const FRAME: usize = 3;
const ENVIRONMENT: usize = 4;
const NO_DIVERGENCE: usize = 5;
const FAIRNESS: usize = 6;

pub(super) fn item_json(key: &ItemKey, index: usize) -> Value {
    let mut item = key.to_wire().expect("an item");
    item["request_index"] = json!(index);
    // Members that are not slot-key members: the reader skips them.
    item["candidates"] = json!({"type": "set", "backends": ["verus"]});
    item["options"] = json!({});
    item
}

pub(super) fn proved_entry(key: &ItemKey, index: usize) -> Value {
    json!({
        "request_index": index,
        "backend": "verus",
        "kind": key.kind.as_str(),
        "result": "proved",
        "basis": {"type": "inductive", "depth": 1},
        "certification": "certified",
    })
}

pub(super) fn entry_of(key: &ItemKey, index: usize, extra: Value) -> Value {
    let mut entry = json!({
        "request_index": index,
        "backend": "verus",
        "kind": key.kind.as_str(),
    });
    for (name, value) in extra.as_object().expect("members").clone() {
        entry[name] = value;
    }
    entry
}

fn record_json() -> Value {
    json!({
        "name": "E",
        "supplier": "kernel",
        "supplies": {"fairness": {
            "kind": "weak",
            "granularity": "each",
            "context": node_key_json(&node("Counter")),
            "operation": "inc",
        }},
    })
}

pub(super) fn node_key_json(node: &NodeKey) -> Value {
    qsl_foundation::node_wire::node_key_to_json(node).expect("a node")
}

/// The request for `declaration` with every item slot `proved`.
pub(super) fn request_for(declaration: &PlannerDeclaration) -> Value {
    let slots: Vec<Value> = declaration
        .slots
        .iter()
        .enumerate()
        .map(|(index, derived)| {
            let mut wire = json!({
                "role": derived.role.as_str(),
                "identity": derived.key.to_wire().expect("an identity"),
            });
            match (&derived.backing, &derived.key) {
                (SlotBacking::Item, SlotKey::Item(key)) => {
                    wire["result"] = json!({
                        "request": item_json(key, index),
                        "result": proved_entry(key, index),
                    });
                }
                (SlotBacking::Environment { member: Some(_) }, _) => {
                    wire["record"] = record_json();
                }
                _ => {}
            }
            wire
        })
        .collect();
    json!({
        "format": FORMAT,
        "package": PACKAGE,
        "item": {"role": "guarantee", "node": node_key_json(&declaration.node)},
        "slots": slots,
    })
}

pub(super) fn read(
    declaration: &PlannerDeclaration,
    request: &Value,
) -> Result<SettlementRequest, RequestRefusal> {
    read_declared(
        declaration,
        &serde_json::to_vec(request).expect("bytes"),
        &ReadLimits::default(),
    )
}

pub(super) fn settled(declaration: &PlannerDeclaration, request: &Value) -> PlannerResult {
    let read = read(declaration, request).expect("the request is admitted");
    settle_declared(declaration, &read, Budget::default()).expect("the item settles")
}

pub(super) fn refusal(
    declaration: &PlannerDeclaration,
    request: &Value,
) -> (&'static str, &'static str, String) {
    let refusal = read(declaration, request).expect_err("the request is refused");
    (refusal.code(), refusal.cause(), refusal.pointer())
}

pub(super) fn invalid(pointer: &str) -> (&'static str, &'static str, String) {
    ("invalid_runtime_input", "invalid-value", pointer.to_owned())
}

fn set_entry(request: &mut Value, slot: usize, entry: Value) {
    request["slots"][slot]["result"]["result"] = entry;
}

fn key_of(declaration: &PlannerDeclaration, slot: usize) -> ItemKey {
    match &declaration.slots[slot].key {
        SlotKey::Item(key) => key.clone(),
        SlotKey::Local(_) => panic!("slot {slot} is not item-backed"),
    }
}

pub(super) fn premise(result: &PlannerResult) -> Option<(u64, PremiseOutcome)> {
    match &result.cause {
        Some(PlannerCause::Inconclusive(InconclusiveCause::PremiseUnproved {
            premise,
            result,
        })) => Some((*premise, *result)),
        _ => None,
    }
}

// ---- QSpec TC-986 case 1; FR-492-AC-1; FR-529-AC-1 -----------------------

#[trace("TC-1329", "FR-529-AC-1")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-1")]
#[test]
fn a_chain_of_proved_premises_settles_proved_with_its_conclusion_record() {
    let declaration = declaration();
    let result = settled(&declaration, &request_for(&declaration));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.value.basis().as_str(), "decisive-witness");
    assert_eq!(result.cause, None);
    // Certified entries read from bytes are read as uncertified.
    assert_eq!(result.certification, Some(Certification::Uncertified));
    let outcomes: Vec<&str> = result
        .slots
        .iter()
        .map(|slot| slot.value.as_str())
        .collect();
    assert_eq!(
        outcomes,
        ["proved", "proved", "proved", "proved", "supplied", "proved", "proved"]
    );
    // The planner settles the fairness-correspondence slot itself, certified.
    assert_eq!(
        result.slots[FAIRNESS].certification,
        Some(Certification::Certified)
    );

    let wire = result.to_wire().expect("a wire form");
    assert_eq!(wire["format"], "quire.planner-result/v1");
    assert_eq!(wire["item"]["role"], "guarantee");
    assert_eq!(wire["value"], "proved");
    assert_eq!(wire["basis"], "decisive-witness");
    assert_eq!(wire["certification"], "uncertified");
    assert_eq!(wire["slots"].as_array().expect("slots").len(), 7);
    assert_eq!(wire["slots"][MODEL_CLAIM]["extent"], "unbounded");
    // Every slot outcome is bound to the declaration member that produced it.
    assert_eq!(wire["slots"][FRAME]["source"]["start"], 30);
    assert_eq!(wire["slots"][FRAME]["source"]["identity"], "counter.qsl");

    let conclusion = &wire["conclusion"];
    assert_eq!(
        conclusion["implementation_sources"],
        json!(["counter::init", "counter::inc"])
    );
    assert_eq!(conclusion["relation"]["package"], PACKAGE);
    assert_eq!(conclusion["clause"]["text_span"]["start"], 100);
    assert_eq!(
        conclusion["clause"]["fairness"]
            .as_array()
            .expect("F_P")
            .len(),
        1
    );
    assert_eq!(conclusion["slots"].as_array().expect("slots").len(), 7);
    assert_eq!(conclusion["assumptions"][0]["name"], "E");
    assert_eq!(conclusion["extent"], "unbounded");
    assert_eq!(
        conclusion["certifications"]
            .as_array()
            .expect("certifications")
            .len(),
        6
    );
}

#[trace("TC-1329", "FR-529-AC-8")]
#[test]
fn settling_one_item_leaves_every_other_item_unchanged() {
    let first = declaration();
    let second = declaration_with(false, &weak_each());
    let before = settled(&first, &request_for(&first))
        .to_wire()
        .expect("wire");
    let sibling = settled(&second, &request_for(&second));
    assert_eq!(sibling.value, PlannerValue::Inconclusive);
    let after = settled(&first, &request_for(&first))
        .to_wire()
        .expect("wire");
    assert_eq!(before, after);
}

// ---- case 2; FR-492-AC-2; FR-529-AC-6 ------------------------------------

#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-2")]
#[test]
fn a_dropped_swapped_relabelled_slot_or_another_package_is_invalid_value() {
    let declaration = declaration();

    let mut dropped = request_for(&declaration);
    dropped["slots"]
        .as_array_mut()
        .expect("slots")
        .remove(FRAME);
    assert_eq!(refusal(&declaration, &dropped), invalid("/slots/3"));

    let mut swapped = request_for(&declaration);
    swapped["slots"].as_array_mut().expect("slots").swap(1, 2);
    assert_eq!(refusal(&declaration, &swapped), invalid("/slots/1"));

    let mut relabelled = request_for(&declaration);
    relabelled["slots"][FRAME]["role"] = json!("transition-simulation");
    assert_eq!(refusal(&declaration, &relabelled), invalid("/slots/3"));

    let mut forged = request_for(&declaration);
    forged["slots"][INITIALIZATION]["identity"]["claim"]["node"]["package"] = json!("other@2");
    assert_eq!(refusal(&declaration, &forged), invalid("/slots/1"));

    let mut elsewhere = request_for(&declaration);
    elsewhere["package"] = json!("other@2");
    assert_eq!(refusal(&declaration, &elsewhere), invalid("/package"));

    let mut extra = request_for(&declaration);
    let duplicate = extra["slots"][NO_DIVERGENCE].clone();
    extra["slots"]
        .as_array_mut()
        .expect("slots")
        .push(duplicate);
    assert_eq!(refusal(&declaration, &extra), invalid("/slots/7"));

    // A short list names the first missing slot, not the whole list.
    let mut short = request_for(&declaration);
    short["slots"].as_array_mut().expect("slots").pop();
    assert_eq!(refusal(&declaration, &short), invalid("/slots/6"));
}

// ---- case 3; FR-492-AC-3 ---------------------------------------------------

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-3")]
#[trace("TC-1329", "FR-529-AC-6")]
#[test]
fn a_boolean_success_flag_is_wrong_value_kind() {
    let declaration = declaration();
    let wrong = |pointer: &str| {
        (
            "invalid_runtime_input",
            "wrong-value-kind",
            pointer.to_owned(),
        )
    };
    let mut slots = request_for(&declaration);
    slots["slots"] = json!(true);
    assert_eq!(refusal(&declaration, &slots), wrong("/slots"));

    let mut slot = request_for(&declaration);
    slot["slots"][3] = json!(true);
    assert_eq!(refusal(&declaration, &slot), wrong("/slots/3"));

    let mut result = request_for(&declaration);
    result["slots"][TRANSITION]["result"] = json!(true);
    assert_eq!(refusal(&declaration, &result), wrong("/slots/2/result"));

    let mut record = request_for(&declaration);
    record["slots"][ENVIRONMENT]["record"] = json!(true);
    assert_eq!(refusal(&declaration, &record), wrong("/slots/4/record"));
}

// ---- case 4; FR-492-AC-4; FR-529-AC-6, FR-529-AC-7 ------------------------

#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-4")]
#[test]
fn a_filled_slot_without_its_result_or_record_is_invalid_value() {
    let declaration = declaration();
    let mut no_result = request_for(&declaration);
    no_result["slots"][INITIALIZATION]
        .as_object_mut()
        .expect("slot")
        .remove("result");
    assert_eq!(refusal(&declaration, &no_result), invalid("/slots/1"));

    let mut no_record = request_for(&declaration);
    no_record["slots"][ENVIRONMENT]
        .as_object_mut()
        .expect("slot")
        .remove("record");
    assert_eq!(refusal(&declaration, &no_record), invalid("/slots/4"));

    // An unfilled slot carries neither.
    let unfilled = declaration_with(false, &weak_each());
    let mut forged = request_for(&unfilled);
    forged["slots"][NO_DIVERGENCE]["record"] = record_json();
    assert_eq!(refusal(&unfilled, &forged), invalid("/slots/5"));
}

#[trace("TC-1329", "FR-529-AC-2")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-4")]
#[test]
fn a_liveness_chain_without_progress_settles_premise_unproved_naming_the_absent_slot() {
    let declaration = declaration_with(false, &weak_each());
    let result = settled(&declaration, &request_for(&declaration));
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((NO_DIVERGENCE as u64, PremiseOutcome::Absent))
    );
    assert_eq!(result.slots[NO_DIVERGENCE].value, SlotValue::Absent);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["cause"]["type"], "premise-unproved");
    assert_eq!(wire["cause"]["premise"], 5);
    assert_eq!(wire["cause"]["result"], "absent");
    assert_eq!(wire["basis"], "unsettled");
}

#[trace("TC-1329", "FR-529-AC-7")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-3")]
#[test]
fn a_bare_label_or_a_reference_is_not_a_premise_result() {
    let declaration = declaration();
    let mut label = request_for(&declaration);
    label["slots"][INITIALIZATION]["result"] = json!("proved");
    assert_eq!(
        refusal(&declaration, &label),
        (
            "invalid_runtime_input",
            "wrong-value-kind",
            "/slots/1/result".to_owned()
        )
    );
    let mut reference = request_for(&declaration);
    reference["slots"][INITIALIZATION]["result"] = json!({"result": 3});
    assert_eq!(
        refusal(&declaration, &reference),
        (
            "invalid_runtime_input",
            "missing-member",
            "/slots/1/result/request".to_owned()
        )
    );
    let mut named = request_for(&declaration);
    named["slots"][INITIALIZATION]["result"] = json!({"request": {}, "result": {}});
    let (code, _, pointer) = refusal(&declaration, &named);
    assert_eq!(code, "invalid_runtime_input");
    assert!(pointer.starts_with("/slots/1/result/request"));
}

// ---- case 5; FR-492-AC-5; FR-529-AC-12 -------------------------------------

#[trace("TC-1329", "FR-529-AC-12")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-5", "QSpec-TC-982")]
#[test]
fn a_result_for_another_slot_key_is_invalid_value_at_the_result() {
    let declaration = declaration();
    let at = "/slots/2/result";

    // The frame item's result in the transition-simulation slot.
    let mut frame_in_transition = request_for(&declaration);
    frame_in_transition["slots"][TRANSITION]["result"] =
        frame_in_transition["slots"][FRAME]["result"].clone();
    assert_eq!(refusal(&declaration, &frame_in_transition), invalid(at));

    // A result compiled against a rebound relation.
    let mut rebound = request_for(&declaration);
    let key = key_of(&declaration, TRANSITION);
    let mut other = key.clone();
    other.claim = OccurrenceKey::new(node_in("app@2", "G"), Origin::new(Role::new("claim"), 0));
    rebound["slots"][TRANSITION]["result"] = json!({
        "request": item_json(&other, TRANSITION),
        "result": proved_entry(&other, TRANSITION),
    });
    assert_eq!(refusal(&declaration, &rebound), invalid(at));

    // A model claim checked without its stutter closure, then under another
    // fairness set.
    let claim = key_of(&declaration, MODEL_CLAIM);
    for changed in [
        model_claim_key(false, &[weak_each()]),
        model_claim_key(
            true,
            &[constraint(FairnessKind::Strong, FairnessGranularity::Each)],
        ),
    ] {
        assert_ne!(changed, claim);
        let mut request = request_for(&declaration);
        request["slots"][MODEL_CLAIM]["result"] = json!({
            "request": item_json(&changed, MODEL_CLAIM),
            "result": proved_entry(&changed, MODEL_CLAIM),
        });
        assert_eq!(refusal(&declaration, &request), invalid("/slots/0/result"));
    }
}

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-5")]
#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn a_result_entry_the_reader_refuses_keeps_its_refusal_at_the_slot() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    request["slots"][TRANSITION]["result"]["result"]
        .as_object_mut()
        .expect("entry")
        .remove("certification");
    let refusal = read(&declaration, &request).expect_err("refused");
    assert_eq!(refusal.code(), "invalid_result");
    assert_eq!(refusal.cause(), "missing-member");
    assert_eq!(refusal.pointer(), "/slots/2/result/result/certification");
}

// ---- case 6; FR-492-AC-6; FR-529-AC-4 --------------------------------------

fn frame_entry(declaration: &PlannerDeclaration, extra: Value) -> Value {
    entry_of(&key_of(declaration, FRAME), FRAME, extra)
}

#[trace("TC-1329", "FR-529-AC-4")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-6")]
#[test]
fn an_unsupported_incomplete_or_inconclusive_slot_decides_the_item() {
    let declaration = declaration();

    let mut unsupported = request_for(&declaration);
    set_entry(
        &mut unsupported,
        FRAME,
        frame_entry(
            &declaration,
            json!({"result": "unsupported", "diagnostic": {
                "code": "unsupported_projection",
                "cause": "tool-unavailable",
                "detail": {"kind": "operation-contract", "backend": "verus"},
            }}),
        ),
    );
    let result = settled(&declaration, &unsupported);
    assert_eq!(result.value, PlannerValue::Unsupported);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "unsupported");
    assert_eq!(wire["basis"], "unavailable");
    // The unsupported slot names the capability through its diagnostic.
    assert_eq!(wire["diagnostic"]["detail"]["backend"], "verus");
    assert_eq!(wire["slots"][FRAME]["value"], "unsupported");

    let mut incomplete = request_for(&declaration);
    set_entry(
        &mut incomplete,
        FRAME,
        frame_entry(
            &declaration,
            json!({"result": "incomplete", "cause": {
                "type": "limit-reached",
                "limit": "max_cases",
                "value": 10,
                "setting": "contract_check.max_cases",
            }}),
        ),
    );
    let result = settled(&declaration, &incomplete);
    assert_eq!(result.value, PlannerValue::Incomplete);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["cause"]["type"], "limit-reached");
    assert_eq!(wire["cause"]["limit"], "max_cases");
    assert_eq!(wire["cause"]["value"], 10);

    let mut inconclusive = request_for(&declaration);
    set_entry(
        &mut inconclusive,
        FRAME,
        frame_entry(
            &declaration,
            json!({"result": "inconclusive", "cause": {"type": "undecided-successor"}}),
        ),
    );
    let result = settled(&declaration, &inconclusive);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((FRAME as u64, PremiseOutcome::Label(ItemLabel::Inconclusive)))
    );

    // With two non-proved slots the earlier slot decides.
    let mut both = inconclusive.clone();
    set_entry(
        &mut both,
        INITIALIZATION,
        entry_of(
            &key_of(&declaration, INITIALIZATION),
            INITIALIZATION,
            json!({"result": "inconclusive", "cause": {"type": "no-initial-state"}}),
        ),
    );
    assert_eq!(
        premise(&settled(&declaration, &both)),
        Some((
            INITIALIZATION as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );
}

// ---- FR-529-AC-2: a refuted step -------------------------------------------

#[trace("TC-1329", "FR-529-AC-2")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-2")]
#[test]
fn a_refuted_transition_simulation_leaves_the_item_inconclusive_never_refuted() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    set_entry(
        &mut request,
        TRANSITION,
        entry_of(
            &key_of(&declaration, TRANSITION),
            TRANSITION,
            json!({"result": "refuted", "counterexample": 0}),
        ),
    );
    request["slots"][TRANSITION]["result"]["counterexamples"] =
        json!({"request_index": TRANSITION, "backend": "verus", "transcript": 0});
    let result = settled(&declaration, &request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((TRANSITION as u64, PremiseOutcome::Label(ItemLabel::Refuted)))
    );
    assert_eq!(result.to_wire().expect("wire")["value"], "inconclusive");

    // A refuted entry without its counterexamples entry is not a premise
    // result.
    request["slots"][TRANSITION]["result"]
        .as_object_mut()
        .expect("result")
        .remove("counterexamples");
    assert_eq!(
        refusal(&declaration, &request),
        invalid("/slots/2/result/counterexamples")
    );
}

// ---- case 7; FR-492-AC-7; FR-529-AC-8 --------------------------------------

#[trace("TC-1329", "FR-529-AC-8")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-7")]
#[test]
fn a_tested_premise_never_promotes_the_item() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    set_entry(
        &mut request,
        MODEL_CLAIM,
        entry_of(
            &key_of(&declaration, MODEL_CLAIM),
            MODEL_CLAIM,
            json!({"result": "tested"}),
        ),
    );
    let result = settled(&declaration, &request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((MODEL_CLAIM as u64, PremiseOutcome::Label(ItemLabel::Tested)))
    );
    assert_eq!(result.to_wire().expect("wire")["cause"]["result"], "tested");
}

// ---- case 8; FR-492-AC-8 ---------------------------------------------------

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-8")]
#[trace("TC-1329", "FR-529-AC-12")]
#[test]
fn trust_is_the_weakest_deciding_premise_and_certified_is_never_read_from_bytes() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    let key = key_of(&declaration, INITIALIZATION);
    set_entry(
        &mut request,
        INITIALIZATION,
        json!({
            "request_index": INITIALIZATION,
            "backend": "acme",
            "kind": key.kind.as_str(),
            "result": "proved",
            "basis": {"type": "exhaustive"},
            "certification": "trusted",
        }),
    );
    let limits = ReadLimits::default();
    let admitted = read_declared(
        &declaration,
        &serde_json::to_vec(&request).expect("bytes"),
        &limits,
    )
    .expect("admitted");
    let trusted = settle_declared(&declaration, &admitted, Budget::default()).expect("settles");
    assert_eq!(trusted.value, PlannerValue::Proved);
    assert_eq!(trusted.certification, Some(Certification::Trusted));

    // Every entry certified: read from bytes, each is uncertified, so the
    // item is uncertified and never certified.
    let all_certified = settled(&declaration, &request_for(&declaration));
    assert_eq!(
        all_certified.certification,
        Some(Certification::Uncertified)
    );
    assert!(all_certified
        .slots
        .iter()
        .filter(|slot| slot.value == SlotValue::Label(ItemLabel::Proved) && slot.slot != FAIRNESS)
        .all(|slot| slot.certification == Some(Certification::Uncertified)));

    // Only a record built in process keeps `certified`.
    let mut in_process = read(&declaration, &request_for(&declaration)).expect("admitted");
    for (index, derived) in declaration.slots.iter().enumerate() {
        if let SlotKey::Item(key) = &derived.key {
            if derived.backing == SlotBacking::Item {
                let item = PremiseItem {
                    key: key.clone(),
                    request_index: index as u64,
                    bounds: Vec::new(),
                };
                let record =
                    TerminalRecord::proved(RequestIndex::new(index), BackendId::new("core"), 1);
                in_process.slots[index].fill =
                    SlotFill::Result(Box::new(PremiseResult::from_record(item, record)));
            }
        }
    }
    let certified = settle_declared(&declaration, &in_process, Budget::default()).expect("settles");
    assert_eq!(certified.certification, Some(Certification::Certified));
}

// ---- case 9; FR-492-AC-9 ---------------------------------------------------

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-9")]
#[trace("TC-1329", "FR-529-AC-4")]
#[test]
fn a_work_budget_below_the_slot_count_settles_incomplete_naming_max_work() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    request["limits"] = json!({"max_work": 2});
    let result = settled(&declaration, &request);
    assert_eq!(result.value, PlannerValue::Incomplete);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "incomplete");
    assert_eq!(wire["basis"], "unavailable");
    assert_eq!(
        wire["cause"],
        json!({
            "type": "limit-reached",
            "limit": "max_work",
            "value": 2,
            "setting": "planner.max_work",
        })
    );
    // A budget equal to the slot count completes.
    request["limits"] = json!({"max_work": 7});
    assert_eq!(settled(&declaration, &request).value, PlannerValue::Proved);
    // The other stops stay distinct: unsupported and an exhausted premise
    // bound are the slot's own causes, not `max_work`.
    assert_ne!(
        settled(&declaration, &request).value,
        PlannerValue::Incomplete
    );
}

// ---- case 10; FR-492-AC-10; FR-529-AC-10 -----------------------------------

#[trace("TC-1329", "FR-529-AC-10")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-10")]
#[test]
fn planner_only_causes_are_refused_in_every_provider_entry_and_written_by_the_planner() {
    for (cause, only) in [
        (
            json!({"type": "premise-unproved", "premise": 3, "result": "absent"}),
            QslOnlyCause::PremiseUnproved,
        ),
        (
            json!({"type": "transfer-not-preserving", "reason": "fragment", "operator": "previous"}),
            QslOnlyCause::TransferNotPreserving,
        ),
    ] {
        assert_eq!(
            read_plugin_inconclusive_cause(&cause),
            Err(PluginResultRefusal::QslOnlyCause(only))
        );
        let entry = json!({
            "request_index": 0,
            "backend": "verus",
            "kind": "operation-contract",
            "result": "inconclusive",
            "cause": cause,
        });
        for origin in [EntryOrigin::FirstParty, EntryOrigin::Plugin] {
            let refusal = read_result_entry(&entry, 4, origin).expect_err("refused");
            assert_eq!(refusal.item_index, 4);
            assert_eq!(refusal.pointer, "/cause/type");
            assert_eq!(refusal.cause, EntryCause::UnknownCause);
            assert_eq!(refusal.cause.code(), "invalid_result");
        }
    }

    let unproved = PlannerCause::Inconclusive(InconclusiveCause::PremiseUnproved {
        premise: 3,
        result: PremiseOutcome::Assumption,
    });
    assert_eq!(
        unproved.to_wire().expect("wire"),
        (
            "cause",
            json!({"type": "premise-unproved", "premise": 3, "result": "assumption"})
        )
    );
    let reasons = [
        (
            TransferReason::Fragment {
                operator: "previous".to_owned(),
            },
            json!({"type": "transfer-not-preserving", "reason": "fragment", "operator": "previous"}),
        ),
        (
            TransferReason::StutterClosure,
            json!({"type": "transfer-not-preserving", "reason": "stutter-closure"}),
        ),
    ];
    for (reason, expected) in reasons {
        let cause = PlannerCause::Inconclusive(InconclusiveCause::TransferNotPreserving { reason });
        assert_eq!(cause.to_wire().expect("wire"), ("cause", expected));
    }
    let fairness = PlannerCause::Inconclusive(InconclusiveCause::TransferNotPreserving {
        reason: TransferReason::Fairness {
            constraint: weak_each(),
        },
    });
    let (_, wire) = fairness.to_wire().expect("wire");
    assert_eq!(wire["reason"], "fairness");
    assert_eq!(wire["constraint"]["kind"], "weak");
    assert_eq!(wire["constraint"]["granularity"], "each");
}

// ---- case 11; FR-492-AC-11; FR-529-AC-11 -----------------------------------

#[trace("TC-1329", "FR-529-AC-11")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-11", "QSpec-TC-982")]
#[test]
fn a_clause_outside_the_fragment_is_not_preserved_whatever_the_premises() {
    for operator in ["interval", "previous"] {
        let mut declaration = declaration();
        declaration
            .clause
            .as_mut()
            .expect("a clause")
            .fragment_operator = Some(operator.to_owned());
        let result = settled(&declaration, &request_for(&declaration));
        assert_eq!(result.value, PlannerValue::Inconclusive);
        assert_eq!(
            result.cause,
            Some(PlannerCause::Inconclusive(
                InconclusiveCause::TransferNotPreserving {
                    reason: TransferReason::Fragment {
                        operator: operator.to_owned()
                    }
                }
            ))
        );
        let wire = result.to_wire().expect("wire");
        assert_eq!(wire["cause"]["reason"], "fragment");
        assert_eq!(wire["cause"]["operator"], operator);
        assert_eq!(wire["basis"], "unsettled");
    }
}

// ---- case 12; FR-492-AC-12 -------------------------------------------------

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-12")]
#[test]
fn a_model_claim_naming_an_unsupplied_assumption_is_premise_unproved() {
    let declaration = declaration();
    let mut admitted = read(&declaration, &request_for(&declaration)).expect("admitted");
    let SlotFill::Result(claim) = &mut admitted.slots[MODEL_CLAIM].fill else {
        panic!("the model claim is filled");
    };
    claim.model_claim_assumptions = ["Fair".to_owned()].into();
    let result = settle_declared(&declaration, &admitted, Budget::default()).expect("settles");
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((
            MODEL_CLAIM as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );
    // An environment record of that name supplies it.
    let SlotFill::Record(record) = &mut admitted.slots[ENVIRONMENT].fill else {
        panic!("the environment slot is filled");
    };
    record.name = "Fair".to_owned();
    let SlotFill::Result(claim) = &mut admitted.slots[MODEL_CLAIM].fill else {
        panic!("the model claim is filled");
    };
    claim.model_claim_assumptions = ["Fair".to_owned()].into();
    let result = settle_declared(&declaration, &admitted, Budget::default()).expect("settles");
    assert_eq!(result.value, PlannerValue::Proved);
}

// ---- FR-529-AC-3: progress and fairness correspondence --------------------

#[trace("TC-1329", "FR-529-AC-3")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-4")]
#[test]
fn a_not_proved_no_divergence_result_is_premise_unproved_naming_its_slot() {
    let declaration = declaration();
    let mut request = request_for(&declaration);
    set_entry(
        &mut request,
        NO_DIVERGENCE,
        entry_of(
            &key_of(&declaration, NO_DIVERGENCE),
            NO_DIVERGENCE,
            json!({"result": "inconclusive", "cause": {"type": "induction-not-closed", "depth": 3}}),
        ),
    );
    let result = settled(&declaration, &request);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((
            NO_DIVERGENCE as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );
}

#[trace("TC-1329", "FR-529-AC-3")]
#[trace("QSpec-TC-982", "QSpec-FR-488-AC-4")]
#[test]
fn a_constraint_stronger_than_every_scheduling_premise_fails_the_fairness_slot() {
    let strong = constraint(FairnessKind::Strong, FairnessGranularity::Each);
    let declaration = declaration_with(true, &strong);
    let result = settled(&declaration, &request_for(&declaration));
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((
            FAIRNESS as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );
    assert_eq!(
        result.slots[FAIRNESS].cause,
        Some(PlannerCause::Inconclusive(
            InconclusiveCause::TransferNotPreserving {
                reason: TransferReason::Fairness { constraint: strong }
            }
        ))
    );
    let wire = result.to_wire().expect("wire");
    assert_eq!(
        wire["slots"][FAIRNESS]["cause"]["type"],
        "transfer-not-preserving"
    );
    assert_eq!(wire["slots"][FAIRNESS]["cause"]["reason"], "fairness");
    assert_eq!(wire["cause"]["premise"], 6);
}

#[trace("TC-1329", "FR-529-AC-3")]
#[test]
fn fairness_implication_follows_the_kind_and_granularity_rule() {
    use FairnessGranularity::{Each, Whole};
    use FairnessKind::{Strong, Weak};
    let supplies = |kind, granularity| {
        EnvSupplies::Fairness(SchedulingPremise {
            constraint: constraint(kind, granularity),
            operation: inc(),
        })
    };
    let cases = [
        // (supplied, needed, implied)
        ((Strong, Each), (Strong, Each), true),
        ((Strong, Each), (Weak, Each), true),
        ((Weak, Each), (Weak, Each), true),
        ((Weak, Each), (Strong, Each), false),
        ((Weak, Whole), (Strong, Whole), false),
        ((Weak, Each), (Weak, Whole), true),
        ((Strong, Each), (Strong, Whole), true),
        ((Weak, Each), (Strong, Whole), false),
        ((Weak, Whole), (Weak, Each), false),
    ];
    for ((supplied_kind, supplied_granularity), (needed_kind, needed_granularity), implied) in cases
    {
        assert_eq!(
            supplies(supplied_kind, supplied_granularity)
                .implies(&inc(), &constraint(needed_kind, needed_granularity)),
            implied,
            "{supplied_kind:?} {supplied_granularity:?} => {needed_kind:?} {needed_granularity:?}"
        );
    }
    // Another operation is not implied, and an assumption supplies no
    // scheduling.
    let elsewhere = OperationRef {
        context: node("Counter"),
        operation: "dec".to_owned(),
    };
    assert!(!supplies(Strong, Each).implies(&elsewhere, &weak_each()));
    assert!(!EnvSupplies::Assumption(node("N")).implies(&inc(), &weak_each()));
}

// ---- case 13; FR-529-AC-12 ------------------------------------------------

#[trace("TC-1329", "FR-529-AC-12")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-5")]
#[test]
fn a_fairness_slot_with_no_frame_binding_is_not_implied() {
    // Remove inc's frame binding: the scheduling premise no longer implies
    // the constraint.
    let mut declaration = declaration();
    declaration.slots[FRAME].operation = None;
    let result = settled(&declaration, &request_for(&declaration));
    assert_eq!(
        result.slots[FAIRNESS].cause,
        Some(PlannerCause::Inconclusive(
            InconclusiveCause::TransferNotPreserving {
                reason: TransferReason::Fairness {
                    constraint: weak_each()
                }
            }
        ))
    );
}

// ---- step 1 and step 2 of admission ----------------------------------------

#[trace("TC-1329", "FR-529-AC-6")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-2")]
#[test]
fn the_document_and_its_members_are_checked_first() {
    let declaration = declaration();
    let bytes = |text: &str| {
        read_declared(&declaration, text.as_bytes(), &ReadLimits::default()).expect_err("refused")
    };
    let cause = |refusal: RequestRefusal| (refusal.code(), refusal.cause(), refusal.pointer());
    assert_eq!(
        cause(bytes("{")),
        ("invalid_runtime_input", "malformed-json", String::new())
    );
    assert_eq!(
        cause(bytes("[]")),
        ("invalid_runtime_input", "malformed-json", String::new())
    );
    assert_eq!(
        cause(bytes(r#"{"format":"a","format":"b"}"#)),
        (
            "invalid_runtime_input",
            "duplicate-member",
            "/format".to_owned()
        )
    );
    assert_eq!(
        cause(bytes("{}")),
        (
            "invalid_runtime_input",
            "missing-member",
            "/format".to_owned()
        )
    );

    let mut extra = request_for(&declaration);
    extra["zeta"] = json!(1);
    assert_eq!(
        refusal(&declaration, &extra),
        (
            "invalid_runtime_input",
            "unknown-member",
            "/zeta".to_owned()
        )
    );
    // A guarantee item has no pool.
    let mut pool = request_for(&declaration);
    pool["guarantees"] = json!([]);
    assert_eq!(
        refusal(&declaration, &pool),
        (
            "invalid_runtime_input",
            "unknown-member",
            "/guarantees".to_owned()
        )
    );
    let mut format = request_for(&declaration);
    format["format"] = json!("quire.planner-settlement/v2");
    assert_eq!(refusal(&declaration, &format), invalid("/format"));
    let mut node = request_for(&declaration);
    node["item"]["node"] = node_key_json(&self::node("Other"));
    assert_eq!(refusal(&declaration, &node), invalid("/item/node"));
    let mut role = request_for(&declaration);
    role["item"]["role"] = json!("composition");
    assert_eq!(refusal(&declaration, &role), invalid("/item/role"));
    let mut limits = request_for(&declaration);
    limits["limits"] = json!({"max_work": "many"});
    assert_eq!(
        refusal(&declaration, &limits),
        (
            "invalid_runtime_input",
            "wrong-value-kind",
            "/limits/max_work".to_owned()
        )
    );

    let tight = ReadLimits { input_bytes: 8 };
    let over = read_declared(
        &declaration,
        &serde_json::to_vec(&request_for(&declaration)).expect("bytes"),
        &tight,
    )
    .expect_err("over the reader bound");
    assert_eq!(over.code(), "stage_limit_exceeded");
}

#[trace("TC-1329", "FR-529-AC-6")]
#[test]
fn an_environment_record_that_differs_from_its_member_is_invalid_value() {
    let declaration = declaration();
    for edit in [
        |record: &mut Value| record["name"] = json!("F"),
        |record: &mut Value| record["supplier"] = json!("someone"),
        |record: &mut Value| record["supplies"]["fairness"]["kind"] = json!("strong"),
        |record: &mut Value| record["supplies"]["fairness"]["operation"] = json!("dec"),
    ] {
        let mut request = request_for(&declaration);
        edit(&mut request["slots"][ENVIRONMENT]["record"]);
        assert_eq!(refusal(&declaration, &request), invalid("/slots/4/record"));
    }
}

#[trace("TC-1329", "FR-529-AC-6")]
#[test]
fn a_composition_whose_rule_is_not_assume_guarantee_is_refused_at_admission() {
    let mut declaration = declaration();
    declaration.role = ItemRole::Composition;
    declaration.rule = Some("interface".to_owned());
    let mut request = request_for(&declaration);
    request["item"]["role"] = json!("composition");
    assert_eq!(refusal(&declaration, &request), invalid("/item/role"));
}

// ---- FR-529-AC-13: premise items and result entries ----------------------

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn every_derived_item_reads_back_with_an_equal_slot_key() {
    let declaration = declaration();
    for (index, derived) in declaration.slots.iter().enumerate() {
        let SlotKey::Item(key) = &derived.key else {
            continue;
        };
        let item = read_premise_item(&item_json(key, index)).expect("the item reads");
        assert_eq!(&item.key, key, "slot {index}");
        assert_eq!(item.request_index, index as u64);
        assert_eq!(item.key.identity(), key.identity());
    }
    // A bounded extent keeps its bounds beside the key.
    let mut bounded = item_json(&key_of(&declaration, MODEL_CLAIM), 0);
    bounded["extent"] = json!({"classification": "bounded", "bounds": [{"domain": 1}]});
    let item = read_premise_item(&bounded).expect("reads");
    assert_eq!(item.key.extent, ExtentClass::Bounded);
    assert_eq!(item.bounds, vec![json!({"domain": 1})]);
}

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn a_malformed_slot_key_member_is_refused_and_other_members_are_skipped() {
    let key = key_of(&declaration(), TRANSITION);
    let mut item = item_json(&key, 2);
    item["limits"] = json!({"max_cases": 9});
    item["check"] = json!({"limits": {"max_cases": 9}});
    assert!(read_premise_item(&item).is_ok());

    let fault = |edit: fn(&mut Value)| {
        let mut item = item_json(&key, 2);
        edit(&mut item);
        let fault = read_premise_item(&item).expect_err("refused");
        (fault.cause.as_str(), fault.pointer)
    };
    assert_eq!(
        fault(|item| item["claim"]["node"] = json!("n")),
        ("wrong-value-kind", "/claim/node".to_owned())
    );
    assert_eq!(
        fault(|item| {
            item.as_object_mut().expect("item").remove("kind");
        }),
        ("missing-member", "/kind".to_owned())
    );
    assert_eq!(
        fault(|item| item["kind"] = json!("nonsense")),
        ("invalid-value", "/kind".to_owned())
    );
    assert_eq!(
        fault(|item| item["temporal"] = json!({})),
        ("invalid-value", "/temporal".to_owned())
    );
    assert_eq!(
        fault(|item| item["correspondence"]["operation"] = json!(3)),
        ("wrong-value-kind", "/correspondence/operation".to_owned())
    );
    assert_eq!(
        fault(|item| item["premises"] = json!([])),
        ("invalid-value", "/premises".to_owned())
    );
    assert_eq!(
        fault(|item| item["extent"]["classification"] = json!("both")),
        ("invalid-value", "/extent/classification".to_owned())
    );
    assert_eq!(
        fault(|item| item["request_index"] = json!(-1)),
        ("invalid-value", "/request_index".to_owned())
    );
    // A temporal member on a model claim: a bad `stutter_closure`.
    let claim = key_of(&declaration(), MODEL_CLAIM);
    let mut item = item_json(&claim, 0);
    item["temporal"]["stutter_closure"] = json!(false);
    let fault = read_premise_item(&item).expect_err("refused");
    assert_eq!(fault.pointer, "/temporal/stutter_closure");
}

fn entry_defect(entry: &Value, origin: EntryOrigin) -> (String, EntryCause) {
    let refusal = read_result_entry(entry, 0, origin).expect_err("the entry is refused");
    (refusal.pointer, refusal.cause)
}

fn base(result: &str) -> Value {
    json!({
        "request_index": 0,
        "backend": "verus",
        "kind": "operation-contract",
        "result": result,
    })
}

fn with(mut entry: Value, members: Value) -> Value {
    for (name, value) in members.as_object().expect("members").clone() {
        entry[name] = value;
    }
    entry
}

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn a_result_entry_is_refused_at_its_first_defect_in_the_reading_order() {
    let first = EntryOrigin::FirstParty;
    // 1 to 2.
    assert_eq!(
        entry_defect(&json!(1), first),
        (String::new(), EntryCause::WrongValueKind)
    );
    assert_eq!(
        entry_defect(&json!({}), first),
        ("/result".to_owned(), EntryCause::MissingMember)
    );
    assert_eq!(
        entry_defect(&json!({"result": 1}), first),
        ("/result".to_owned(), EntryCause::WrongValueKind)
    );
    assert_eq!(
        entry_defect(&json!({"result": "nope"}), first),
        ("/result".to_owned(), EntryCause::InvalidValue)
    );
    // 3.
    assert_eq!(
        entry_defect(
            &with(base("measured"), json!({"basis": {"type": "exhaustive"}})),
            first
        ),
        ("/basis".to_owned(), EntryCause::MeasuredWithTruth)
    );
    assert_eq!(
        entry_defect(&with(base("measured"), json!({"truth": true})), first),
        ("/truth".to_owned(), EntryCause::MeasuredWithTruth)
    );
    // 4 before 5.
    let mut missing = base("tested");
    missing
        .as_object_mut()
        .expect("entry")
        .remove("request_index");
    missing["zeta"] = json!(1);
    assert_eq!(
        entry_defect(&missing, first),
        ("/request_index".to_owned(), EntryCause::MissingMember)
    );
    assert_eq!(
        entry_defect(
            &with(base("proved"), json!({"basis": {"type": "exhaustive"}})),
            first
        ),
        ("/certification".to_owned(), EntryCause::MissingMember)
    );
    // 5.
    assert_eq!(
        entry_defect(
            &with(
                base("refuted"),
                json!({"counterexample": 0, "certification": 7})
            ),
            first
        ),
        ("/certification".to_owned(), EntryCause::UnknownMember)
    );
    assert_eq!(
        entry_defect(
            &with(
                base("unsupported"),
                json!({"cause": {"type": "missing-confidence"}, "diagnostic": {}})
            ),
            first
        ),
        ("/diagnostic".to_owned(), EntryCause::UnknownMember)
    );
    // 6.
    let proved = |certification: Value, basis: Value| {
        with(
            base("proved"),
            json!({"basis": basis, "certification": certification}),
        )
    };
    let exhaustive = json!({"type": "exhaustive"});
    assert_eq!(
        entry_defect(&proved(json!(7), exhaustive.clone()), first),
        ("/certification".to_owned(), EntryCause::WrongValueKind)
    );
    assert_eq!(
        entry_defect(&proved(json!("bogus"), exhaustive.clone()), first),
        ("/certification".to_owned(), EntryCause::InvalidValue)
    );
    assert_eq!(
        entry_defect(
            &proved(json!("certified"), exhaustive.clone()),
            EntryOrigin::Plugin
        ),
        (
            "/certification".to_owned(),
            EntryCause::CertificationMismatch {
                expected: Some(Certification::Trusted),
                actual: Certification::Certified
            }
        )
    );
    assert_eq!(
        entry_defect(&proved(json!("trusted"), exhaustive.clone()), first),
        (
            "/certification".to_owned(),
            EntryCause::CertificationMismatch {
                expected: None,
                actual: Certification::Trusted
            }
        )
    );
    assert_eq!(
        entry_defect(
            &proved(
                json!("certified"),
                json!({"type": "inductive-solver", "method": "inductive-enclosure"})
            ),
            first
        ),
        (
            "/certification".to_owned(),
            EntryCause::CertificationMismatch {
                expected: Some(Certification::Uncertified),
                actual: Certification::Certified
            }
        )
    );
    // 7: the basis or cause first, then the other members in name order.
    assert_eq!(
        entry_defect(&proved(json!("certified"), json!({"type": "nope"})), first),
        ("/basis/type".to_owned(), EntryCause::UnknownCause)
    );
    assert_eq!(
        entry_defect(
            &proved(
                json!("certified"),
                json!({"type": "exact-value", "value": "1"})
            ),
            first
        ),
        (
            "/basis/type".to_owned(),
            EntryCause::UnsupportedFeature {
                feature: "exact-value".to_owned()
            }
        )
    );
    assert_eq!(
        entry_defect(
            &with(
                proved(json!("certified"), exhaustive.clone()),
                json!({"certificate": {}, "kind": "bogus"})
            ),
            first
        ),
        (
            "/certificate".to_owned(),
            EntryCause::UnsupportedFeature {
                feature: "certificate".to_owned()
            }
        )
    );
    assert_eq!(
        entry_defect(
            &with(
                proved(json!("certified"), exhaustive),
                json!({"kind": "bogus"})
            ),
            first
        ),
        ("/kind".to_owned(), EntryCause::InvalidValue)
    );
    assert_eq!(
        entry_defect(
            &with(
                base("measured"),
                json!({"verdict": {"decision": "accepted"}})
            ),
            first
        ),
        (
            "/verdict".to_owned(),
            EntryCause::UnsupportedFeature {
                feature: "verdict".to_owned()
            }
        )
    );
    assert_eq!(
        entry_defect(
            &with(base("inconclusive"), json!({"cause": {"type": "made-up"}})),
            first
        ),
        ("/cause/type".to_owned(), EntryCause::UnknownCause)
    );
    assert_eq!(
        entry_defect(
            &with(
                base("inconclusive"),
                json!({"cause": {"type": "replay-parity"}})
            ),
            first
        ),
        (
            "/cause/type".to_owned(),
            EntryCause::UnsupportedFeature {
                feature: "replay-parity".to_owned()
            }
        )
    );
}

#[trace("TC-1329", "FR-529-AC-13")]
#[test]
fn every_result_value_reads_into_its_terminal_value() {
    let read = |entry: Value, origin| {
        read_result_entry(&entry, 0, origin)
            .expect("the entry reads")
            .record
            .value()
            .clone()
    };
    let first = EntryOrigin::FirstParty;
    assert!(matches!(
        read(
            with(
                base("proved"),
                json!({"basis": {"type": "bounded-proof", "checks": 4}, "certification": "certified"})
            ),
            first
        ),
        TerminalValue::Proved {
            certification: Certification::Certified,
            ..
        }
    ));
    assert!(matches!(
        read(
            with(
                base("proved"),
                json!({"basis": {"type": "exhaustive"}, "certification": "trusted"})
            ),
            EntryOrigin::Plugin
        ),
        TerminalValue::Proved {
            certification: Certification::Trusted,
            ..
        }
    ));
    assert_eq!(read(base("tested"), first), TerminalValue::Tested);
    assert_eq!(
        read(with(base("refuted"), json!({"counterexample": 0})), first),
        TerminalValue::Refuted
    );
    assert!(matches!(
        read(
            with(
                base("inconclusive"),
                json!({"cause": {"type": "kani_vacuous_proof"}})
            ),
            first
        ),
        TerminalValue::Inconclusive(InconclusiveCause::KaniVacuousProof)
    ));
    assert!(matches!(
        read(
            with(base("incomplete"), json!({"cause": {"type": "timed-out"}})),
            first
        ),
        TerminalValue::Incomplete(_)
    ));
    assert!(matches!(
        read(
            with(
                base("incomplete"),
                json!({"cause": {"type": "cancelled", "source": "deadline"}})
            ),
            first
        ),
        TerminalValue::Incomplete(_)
    ));
    assert!(matches!(
        read(
            with(
                base("declined"),
                json!({
                    "cause": {"type": "refused"},
                    "diagnostic": {"code": "kani_vacuous_proof", "cause": "x", "detail": {}},
                })
            ),
            first
        ),
        TerminalValue::Declined { .. }
    ));
    assert_eq!(
        read(
            with(
                base("failed"),
                json!({"cause": {"type": "tool-failure", "defect": "crash"}})
            ),
            first
        ),
        TerminalValue::Failed
    );
    assert!(matches!(
        read(
            with(
                base("unsupported"),
                json!({"cause": {"type": "confidence-beyond-precision"}})
            ),
            first
        ),
        TerminalValue::Unsupported(_)
    ));
}
