// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1330 and QSpec TC-983 and TC-986 over hand-built declarations: an
//! `assume-guarantee` composition of `Producer` and `Consumer` is discharged
//! from the pool, the supplied constraints, the `discharges` records and the
//! fixed-point slot. No source is compiled.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};
use serde_json::{json, Value};

use super::tests::{
    constraint, declaration as guarantee_declaration, entry_of, invalid, item_json, local, node,
    node_in, node_key_json, premise, proved_entry, read, refusal, request_for as guarantee_request,
    settled, slot, weak_each, PACKAGE,
};
use super::*;
use crate::outcome::ItemLabel;
use crate::proof_result::{Certification, PremiseOutcome};
use quire_exact::NodeKey;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

const PRODUCER: &str = "producer@1";
const CONSUMER: &str = "consumer@1";
const THIRD: &str = "third@1";

const COMPONENT_PRODUCER: usize = 0;
const COMPONENT_CONSUMER: usize = 1;
const RELY: usize = 2;
const LIVENESS: usize = 3;
const FAIRNESS: usize = 4;
const ENVIRONMENT: usize = 5;

fn subject(package: &str) -> String {
    json::canonical_text(&json!({"package": package, "model": "Model"}))
}

/// A temporal guarantee item of `package`'s clause `label`.
fn guarantee_key(package: &str, label: &str, fairness: &[FairnessConstraint]) -> ItemKey {
    let mut set: Vec<FairnessKey> = fairness
        .iter()
        .map(|constraint| FairnessKey::of(constraint).expect("a fairness key"))
        .collect();
    set.sort();
    ItemKey {
        claim: OccurrenceKey::new(node_in(package, label), Origin::new(Role::new("claim"), 0)),
        kind: ClaimKind::TemporalSatisfaction,
        temporal: Some(TemporalKey {
            subject: subject(package),
            item_kind: TemporalItemKind::Formula,
            fairness: set,
            stutter_closure: true,
        }),
        correspondence: None,
        premises: Vec::new(),
        extent: ExtentClass::Unbounded,
    }
}

fn sends() -> ItemKey {
    guarantee_key(PRODUCER, "Sends", &[weak_each()])
}

fn done() -> ItemKey {
    guarantee_key(CONSUMER, "Done", &[])
}

/// A safety guarantee is not checked over the stutter closure (FR-491).
fn safe() -> ItemKey {
    let mut key = guarantee_key(PRODUCER, "Safe", &[]);
    key.temporal.as_mut().expect("temporal").stutter_closure = false;
    key
}

fn pool_slot(
    role: SlotRole,
    members: &[&str],
    subject: Option<String>,
    peer: Option<NodeKey>,
    start: u64,
) -> DerivedSlot {
    let mut derived = slot(
        role,
        SlotKey::Local(LocalKey {
            role,
            members: members.iter().map(|member| (*member).to_owned()).collect(),
        }),
        SlotBacking::Pool,
        start,
    );
    derived.premise_subject = subject;
    derived.peer_clause = peer;
    derived
}

fn composition_shell(slots: Vec<DerivedSlot>) -> PlannerDeclaration {
    PlannerDeclaration {
        role: ItemRole::Composition,
        node: node("S"),
        package: PACKAGE.to_owned(),
        rule: Some("assume-guarantee".to_owned()),
        clause: None,
        relation_sources: Vec::new(),
        relation_node: None,
        environment: vec![EnvironmentMember {
            name: "E".to_owned(),
            supplier: "kernel".to_owned(),
            supplies: EnvSupplies::Assumption(node("N")),
        }],
        slots,
        supplies: vec![weak_each()],
    }
}

/// `Consumer`'s liveness guarantee depends on `Producer`'s `Sends` under
/// `fair weak each` and relies on `Producer`'s safety guarantee `Safe`
/// (QSpec TC-983).
fn declaration() -> PlannerDeclaration {
    let mut fairness = pool_slot(SlotRole::Fairness, &["Sends", "inc"], None, None, 40);
    fairness.constraint = Some(weak_each());
    composition_shell(vec![
        slot(
            SlotRole::ComponentGuarantee,
            SlotKey::Item(sends()),
            SlotBacking::Item,
            0,
        ),
        slot(
            SlotRole::ComponentGuarantee,
            SlotKey::Item(done()),
            SlotBacking::Item,
            10,
        ),
        pool_slot(
            SlotRole::Rely,
            &["Done", "Safe"],
            Some(subject(PRODUCER)),
            Some(node_in(PRODUCER, "Safe")),
            20,
        ),
        pool_slot(
            SlotRole::LivenessDependency,
            &["Done", "Sends"],
            Some(subject(PRODUCER)),
            Some(node_in(PRODUCER, "Sends")),
            30,
        ),
        fairness,
        slot(
            SlotRole::Environment,
            local(SlotRole::Environment, "E"),
            SlotBacking::Environment {
                member: Some("E".to_owned()),
            },
            50,
        ),
    ])
}

/// A mutual liveness dependency under a lattice-declaring profile: slots 2
/// and 3 are on cycle 0, which slot 4 settles.
fn cyclic() -> PlannerDeclaration {
    let mut first = pool_slot(
        SlotRole::LivenessDependency,
        &["Done", "Sends"],
        Some(subject(PRODUCER)),
        Some(node_in(PRODUCER, "Sends")),
        20,
    );
    first.cycle = Some(0);
    let mut second = pool_slot(
        SlotRole::LivenessDependency,
        &["Sends", "Done"],
        Some(subject(CONSUMER)),
        Some(node_in(CONSUMER, "Done")),
        30,
    );
    second.cycle = Some(0);
    let premises = [sends(), done()]
        .iter()
        .map(|key| PremiseKey {
            role: SlotRole::ComponentGuarantee,
            key: json::canonical_text(&key.to_wire().expect("a key")),
        })
        .collect();
    let mut fixed_point = slot(
        SlotRole::FixedPoint,
        SlotKey::Item(ItemKey {
            claim: OccurrenceKey::new(node("S"), Origin::new(Role::new("claim"), 0)),
            kind: ClaimKind::Composition,
            temporal: None,
            correspondence: None,
            premises,
            extent: ExtentClass::Unbounded,
        }),
        SlotBacking::Item,
        40,
    );
    fixed_point.cycle = Some(0);
    composition_shell(vec![
        slot(
            SlotRole::ComponentGuarantee,
            SlotKey::Item(sends()),
            SlotBacking::Item,
            0,
        ),
        slot(
            SlotRole::ComponentGuarantee,
            SlotKey::Item(done()),
            SlotBacking::Item,
            10,
        ),
        first,
        second,
        fixed_point,
    ])
}

fn pool_entry(key: &ItemKey, index: usize) -> Value {
    json!({"request": item_json(key, index), "result": proved_entry(key, index)})
}

/// The request for `declaration`: every item slot `proved`, the environment
/// record filled and `pool` as the guarantee pool.
fn request(declaration: &PlannerDeclaration, pool: &[ItemKey]) -> Value {
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
                    wire["result"] = pool_entry(key, index);
                }
                (SlotBacking::Environment { member: Some(_) }, _) => {
                    wire["record"] = json!({
                        "name": "E",
                        "supplier": "kernel",
                        "supplies": {"assumption": node_key_json(&node("N"))},
                    });
                }
                _ => {}
            }
            wire
        })
        .collect();
    let guarantees: Vec<Value> = pool
        .iter()
        .enumerate()
        .map(|(at, key)| pool_entry(key, 100 + at))
        .collect();
    json!({
        "format": FORMAT,
        "package": PACKAGE,
        "item": {"role": "composition", "node": node_key_json(&declaration.node)},
        "slots": slots,
        "guarantees": guarantees,
    })
}

fn positive_pool() -> Vec<ItemKey> {
    vec![sends(), safe()]
}

fn entry_for(result: &PlannerResult, slot: usize) -> &DischargeEntry {
    result
        .discharge
        .as_ref()
        .expect("a discharge record")
        .entries
        .iter()
        .find(|entry| entry.slot == slot)
        .expect("the slot is discharged")
}

fn discharged(result: &PlannerResult, slot: usize) -> bool {
    result
        .discharge
        .as_ref()
        .is_some_and(|record| record.entries.iter().any(|entry| entry.slot == slot))
}

/// An unrelated supported guarantee of the same package settles unchanged.
fn unrelated_unchanged() {
    let declaration = guarantee_declaration();
    let result = settled(&declaration, &guarantee_request(&declaration));
    assert_eq!(result.value, PlannerValue::Proved);
}

// ---- FR-530-AC-3; QSpec TC-983 case 1; FR-489-AC-3 -------------------------

#[trace("TC-1330", "FR-530-AC-3")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-3")]
#[test]
fn a_producer_consumer_composition_with_every_premise_discharged_settles_proved() {
    unrelated_unchanged();
    let declaration = declaration();
    let result = settled(&declaration, &request(&declaration, &positive_pool()));
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.value.basis().as_str(), "decisive-witness");
    assert_eq!(result.certification, Some(Certification::Uncertified));

    let rely = entry_for(&result, RELY);
    assert!(matches!(rely.source, DischargeSource::Pool(1)));
    assert_eq!(rely.relation, Relation::Identity);
    assert_eq!(rely.subject, Some(subject(PRODUCER)));
    let liveness = entry_for(&result, LIVENESS);
    assert!(matches!(liveness.source, DischargeSource::Pool(0)));
    assert_eq!(liveness.certification, Some(Certification::Uncertified));
    assert_eq!(liveness.extent, Some(ExtentClass::Unbounded));
    let fairness = entry_for(&result, FAIRNESS);
    assert!(matches!(fairness.source, DischargeSource::Supplied(_)));
    assert_eq!(fairness.relation, Relation::FairnessImplication);
    let environment = entry_for(&result, ENVIRONMENT);
    assert_eq!(environment.relation, Relation::Assumption);
    assert_eq!(
        entry_for(&result, COMPONENT_PRODUCER).relation,
        Relation::Identity
    );
    assert_eq!(
        entry_for(&result, COMPONENT_CONSUMER).relation,
        Relation::Identity
    );

    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["value"], "proved");
    assert_eq!(wire["basis"], "decisive-witness");
    let record = wire["discharge"]["discharges"].as_array().expect("entries");
    assert_eq!(record.len(), 6);
    assert_eq!(record[LIVENESS]["by"], json!({"guarantee": 0}));
    assert_eq!(record[LIVENESS]["relation"], "identity");
    assert_eq!(record[FAIRNESS]["relation"], "fairness-implication");
    assert_eq!(record[ENVIRONMENT]["relation"], "assumption");
}

// ---- FR-530-AC-2; QSpec TC-983 case 3; FR-489-AC-2 -------------------------

#[trace("TC-1330", "FR-530-AC-2")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-2")]
#[test]
fn a_supplied_guarantee_that_does_not_imply_the_premise_leaves_it_undischarged() {
    let declaration = declaration();
    let strong_each = constraint(FairnessKind::Strong, FairnessGranularity::Each);
    let mismatches: [(&str, ItemKey); 8] = [
        // The dependency's guarantee over `Consumer` instead of `Producer`.
        (
            "another subject",
            ItemKey {
                temporal: Some(TemporalKey {
                    subject: subject(CONSUMER),
                    ..sends().temporal.expect("temporal")
                }),
                ..sends()
            },
        ),
        (
            "a weaker clause",
            guarantee_key(PRODUCER, "WeakSends", &[weak_each()]),
        ),
        (
            "a safety guarantee",
            guarantee_key(PRODUCER, "SafeOnly", &[]),
        ),
        (
            "another kind",
            ItemKey {
                kind: ClaimKind::Refinement,
                temporal: None,
                ..sends()
            },
        ),
        (
            "a same-named guarantee of another package",
            guarantee_key(THIRD, "Sends", &[weak_each()]),
        ),
        (
            "another item kind",
            ItemKey {
                temporal: Some(TemporalKey {
                    item_kind: TemporalItemKind::DeadlockFreedom,
                    stutter_closure: false,
                    ..sends().temporal.expect("temporal")
                }),
                ..sends()
            },
        ),
        (
            "no stutter closure",
            ItemKey {
                temporal: Some(TemporalKey {
                    stutter_closure: false,
                    ..sends().temporal.expect("temporal")
                }),
                ..sends()
            },
        ),
        // Fairness no supplied constraint implies.
        (
            "an unimplied fairness set",
            guarantee_key(PRODUCER, "Sends", &[strong_each]),
        ),
    ];
    for (name, wrong) in mismatches {
        let result = settled(&declaration, &request(&declaration, &[wrong, safe()]));
        assert_eq!(result.value, PlannerValue::Inconclusive, "{name}");
        assert_eq!(
            premise(&result),
            Some((LIVENESS as u64, PremiseOutcome::Absent)),
            "{name}"
        );
        assert!(!discharged(&result, LIVENESS), "{name}");
        // The other premises are discharged as before.
        assert!(discharged(&result, RELY), "{name}");
        assert!(discharged(&result, FAIRNESS), "{name}");
    }
    // A pool that holds the right guarantee later still discharges it.
    let result = settled(
        &declaration,
        &request(
            &declaration,
            &[guarantee_key(PRODUCER, "WeakSends", &[]), sends(), safe()],
        ),
    );
    assert_eq!(result.value, PlannerValue::Proved);
    assert!(matches!(
        entry_for(&result, LIVENESS).source,
        DischargeSource::Pool(1)
    ));
}

// ---- FR-530-AC-5; QSpec TC-983 case 4; FR-489-AC-5 -------------------------

#[trace("TC-1330", "FR-530-AC-5")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-5")]
#[test]
fn a_cycle_is_discharged_only_by_a_proved_fixed_point_result() {
    let declaration = cyclic();
    const FIXED_POINT: usize = 4;
    // The cycle's guarantees are in the pool and are no assumption.
    let pool = [sends(), done()];
    let mut proved = request(&declaration, &pool);
    let result = settled(&declaration, &proved);
    assert_eq!(result.value, PlannerValue::Proved);
    for member in [2, 3] {
        let entry = entry_for(&result, member);
        assert_eq!(entry.relation, Relation::FixedPoint);
        assert!(matches!(
            entry.source,
            DischargeSource::FixedPoint(FIXED_POINT)
        ));
    }
    assert_eq!(
        entry_for(&result, FIXED_POINT).relation,
        Relation::FixedPoint
    );
    let wire = result.to_wire().expect("wire");
    assert_eq!(
        wire["discharge"]["discharges"][FIXED_POINT]["relation"],
        "fixed-point"
    );

    // The same pool, the fixed-point slot `inconclusive`: the item names it.
    let key = match &declaration.slots[FIXED_POINT].key {
        SlotKey::Item(key) => key.clone(),
        SlotKey::Local(_) => panic!("the fixed-point slot is item-backed"),
    };
    proved["slots"][FIXED_POINT]["result"]["result"] = entry_of(
        &key,
        FIXED_POINT,
        json!({"result": "inconclusive", "cause": {"type": "undecided-successor"}}),
    );
    let result = settled(&declaration, &proved);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((
            FIXED_POINT as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );
    assert!(!discharged(&result, 2));
    assert!(!discharged(&result, 3));
}

// ---- FR-530-AC-6; QSpec TC-983 case 5; FR-489-AC-6, 8, 9, 11 ---------------

fn limits() -> ReadLimits {
    ReadLimits::default()
}

#[trace("TC-1330", "FR-530-AC-6")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-6", "QSpec-FR-489-AC-8")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-9", "QSpec-FR-489-AC-11")]
#[test]
fn trusted_assumption_only_failed_check_and_exhausted_budget_keep_distinct_outcomes() {
    let declaration = declaration();

    // A premise discharged by a `trusted` plugin result settles `proved`,
    // `trusted`, the record citing the pool entry.
    let mut trusted = request(&declaration, &positive_pool());
    trusted["guarantees"][0]["result"] = json!({
        "request_index": 100,
        "backend": "acme",
        "kind": "temporal-satisfaction",
        "result": "proved",
        "basis": {"type": "exhaustive"},
        "certification": "trusted",
    });
    let admitted = read_declared(
        &declaration,
        &serde_json::to_vec(&trusted).expect("bytes"),
        &limits(),
    )
    .expect("admitted");
    let result = settle_declared(&declaration, &admitted, Budget::default()).expect("settles");
    assert_eq!(result.value, PlannerValue::Proved);
    assert_eq!(result.certification, Some(Certification::Trusted));
    let entry = entry_for(&result, LIVENESS);
    assert!(matches!(entry.source, DischargeSource::Pool(0)));
    assert_eq!(entry.certification, Some(Certification::Trusted));
    assert_eq!(result.to_wire().expect("wire")["certification"], "trusted");

    // A premise whose only discharger is an environment record is an
    // assumption: not `proved`, not `trusted`.
    let mut assumed = request(&declaration, &[safe()]);
    assumed["discharges"] = json!([{"slot": LIVENESS, "environment": ENVIRONMENT}]);
    let result = settled(&declaration, &assumed);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((LIVENESS as u64, PremiseOutcome::Assumption))
    );
    assert_eq!(result.slots[LIVENESS].value, SlotValue::Assumption);
    assert_eq!(result.slots[ENVIRONMENT].value, SlotValue::Supplied);
    assert_eq!(result.certification, None);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["basis"], "unsettled");
    assert_eq!(wire["cause"]["result"], "assumption");

    // An unsupported component slot decides `unsupported`.
    let mut unsupported = request(&declaration, &positive_pool());
    let key = sends();
    unsupported["slots"][COMPONENT_PRODUCER]["result"]["result"] = entry_of(
        &key,
        COMPONENT_PRODUCER,
        json!({"result": "unsupported", "diagnostic": {
            "code": "unsupported_projection",
            "cause": "tool-unavailable",
            "detail": {"kind": "temporal-satisfaction", "backend": "verus"},
        }}),
    );
    let result = settled(&declaration, &unsupported);
    assert_eq!(result.value, PlannerValue::Unsupported);
    assert_eq!(result.to_wire().expect("wire")["basis"], "unavailable");

    // A failed component check leaves the item `inconclusive`, naming the
    // component's slot.
    let mut failed = request(&declaration, &positive_pool());
    failed["slots"][COMPONENT_CONSUMER]["result"]["result"] = entry_of(
        &done(),
        COMPONENT_CONSUMER,
        json!({"result": "inconclusive", "cause": {"type": "undecided-successor"}}),
    );
    let result = settled(&declaration, &failed);
    assert_eq!(
        premise(&result),
        Some((
            COMPONENT_CONSUMER as u64,
            PremiseOutcome::Label(ItemLabel::Inconclusive)
        ))
    );

    // An exhausted budget is `incomplete` naming `max_work`; the slots read
    // so far and their discharges are kept.
    let mut exhausted = request(&declaration, &positive_pool());
    exhausted["limits"] = json!({"max_work": 3});
    let result = settled(&declaration, &exhausted);
    assert_eq!(result.value, PlannerValue::Incomplete);
    let wire = result.to_wire().expect("wire");
    assert_eq!(wire["basis"], "unavailable");
    assert_eq!(wire["cause"]["limit"], "max_work");
    assert_eq!(wire["cause"]["value"], 3);
    assert_eq!(result.slots.len(), 2);
    // The budget counts the slot reads and the pool entries compared.
    exhausted["limits"] = json!({"max_work": 6});
    assert_eq!(
        settled(&declaration, &exhausted).value,
        PlannerValue::Incomplete
    );
    exhausted["limits"] = json!({"max_work": 9});
    assert_eq!(
        settled(&declaration, &exhausted).value,
        PlannerValue::Proved
    );
}

// ---- FR-530-AC-7; QSpec TC-983 case 6; FR-489-AC-10 ------------------------

#[trace("TC-1330", "FR-530-AC-7")]
#[trace("QSpec-TC-983", "QSpec-FR-489-AC-10")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-12")]
#[test]
fn a_discharge_record_is_admitted_against_its_slot_and_a_non_proved_result_discharges_nothing() {
    let declaration = declaration();

    // Naming no pool entry.
    let mut no_entry = request(&declaration, &positive_pool());
    no_entry["discharges"] = json!([{"slot": LIVENESS, "guarantee": 9}]);
    assert_eq!(refusal(&declaration, &no_entry), invalid("/discharges/0"));

    // A pool entry over another subject.
    let mut other_subject = request(&declaration, &[done(), sends()]);
    other_subject["discharges"] = json!([{"slot": LIVENESS, "guarantee": 0}]);
    assert_eq!(
        refusal(&declaration, &other_subject),
        invalid("/discharges/0")
    );

    // A slot that is neither a `rely` nor a `liveness-dependency` slot, and
    // an `environment` entry that is no filled environment slot.
    let mut wrong_slot = request(&declaration, &positive_pool());
    wrong_slot["discharges"] = json!([{"slot": FAIRNESS, "guarantee": 0}]);
    assert_eq!(refusal(&declaration, &wrong_slot), invalid("/discharges/0"));
    let mut wrong_environment = request(&declaration, &positive_pool());
    wrong_environment["discharges"] = json!([{"slot": RELY, "environment": RELY}]);
    assert_eq!(
        refusal(&declaration, &wrong_environment),
        invalid("/discharges/0")
    );

    // A boolean in place of the records or the pool is `wrong-value-kind`.
    for member in ["discharges", "guarantees", "slots"] {
        let mut flag = request(&declaration, &positive_pool());
        flag[member] = json!(true);
        assert_eq!(
            refusal(&declaration, &flag),
            (
                "invalid_runtime_input",
                "wrong-value-kind",
                format!("/{member}")
            )
        );
    }

    // A named entry over the right subject whose result is `inconclusive`
    // discharges nothing, and a named record does not bypass the check.
    let mut inconclusive = request(&declaration, &[sends(), safe()]);
    inconclusive["guarantees"][0]["result"] = entry_of(
        &sends(),
        100,
        json!({"result": "inconclusive", "cause": {"type": "undecided-successor"}}),
    );
    inconclusive["discharges"] = json!([{"slot": LIVENESS, "guarantee": 0}]);
    let result = settled(&declaration, &inconclusive);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((LIVENESS as u64, PremiseOutcome::Absent))
    );
    assert!(!discharged(&result, LIVENESS));

    // A valid record is used after the same check as the pool scan.
    let mut named = request(&declaration, &[safe(), sends()]);
    named["discharges"] = json!([{"slot": LIVENESS, "guarantee": 1}]);
    let result = settled(&declaration, &named);
    assert_eq!(result.value, PlannerValue::Proved);
    assert!(matches!(
        entry_for(&result, LIVENESS).source,
        DischargeSource::Pool(1)
    ));
}

// ---- FR-530-AC-8; QSpec TC-986 case 2/5 ------------------------------------

#[trace("TC-1330", "FR-530-AC-8")]
#[trace("QSpec-TC-986", "QSpec-FR-492-AC-2")]
#[test]
fn a_forged_reordered_or_misidentified_slot_is_refused_and_an_unrelated_item_settles() {
    let declaration = declaration();
    let good = request(&declaration, &positive_pool());

    let mut reordered = good.clone();
    reordered["slots"]
        .as_array_mut()
        .expect("slots")
        .swap(RELY, LIVENESS);
    assert_eq!(
        refusal(&declaration, &reordered),
        invalid(&format!("/slots/{RELY}"))
    );

    let mut forged = good.clone();
    forged["slots"][LIVENESS]["identity"]["members"] = json!(["Done", "Forged"]);
    assert_eq!(
        refusal(&declaration, &forged),
        invalid(&format!("/slots/{LIVENESS}"))
    );

    // A premise result for another slot's identity.
    let mut misidentified = good.clone();
    let swapped = misidentified["slots"][COMPONENT_CONSUMER]["result"].clone();
    misidentified["slots"][COMPONENT_PRODUCER]["result"] = swapped;
    assert_eq!(
        refusal(&declaration, &misidentified),
        invalid(&format!("/slots/{COMPONENT_PRODUCER}/result"))
    );

    // The same request, unforged, settles, and so does an unrelated item.
    unrelated_unchanged();
    assert_eq!(settled(&declaration, &good).value, PlannerValue::Proved);
    let other = {
        let mut declaration = declaration.clone();
        declaration.node = node("T");
        declaration
    };
    let mut other_request = request(&other, &positive_pool());
    other_request["item"]["node"] = node_key_json(&other.node);
    assert_eq!(settled(&other, &other_request).value, PlannerValue::Proved);
    assert!(read(&declaration, &other_request).is_err());
}

// ---- QSpec TC-986 case 12; FR-492-AC-12 ------------------------------------

#[trace("QSpec-TC-986", "QSpec-FR-492-AC-12")]
#[trace("TC-1330", "FR-530-AC-6")]
#[test]
fn a_rely_slot_with_only_an_environment_discharger_is_premise_unproved_assumption() {
    let declaration = declaration();
    let mut assumed = request(&declaration, &[sends()]);
    assumed["discharges"] = json!([{"slot": RELY, "environment": ENVIRONMENT}]);
    let result = settled(&declaration, &assumed);
    assert_eq!(result.value, PlannerValue::Inconclusive);
    assert_eq!(
        premise(&result),
        Some((RELY as u64, PremiseOutcome::Assumption))
    );
    let wire = result.to_wire().expect("wire");
    assert_eq!(
        wire["cause"],
        json!({"type": "premise-unproved", "premise": RELY, "result": "assumption"})
    );
    // The `environment` premise itself is supplied and listed.
    assert_eq!(wire["slots"][ENVIRONMENT]["value"], "supplied");
    assert!(!discharged(&result, RELY));
}

/// FR-489 "Implication": a guarantee of the peer clause with another item
/// kind, or checked over the stutter closure, does not discharge a safety
/// `rely` (admission refuses a `temporal` member on any other claim kind).
#[test]
fn a_rely_is_discharged_only_by_a_guarantee_of_the_premises_form() {
    let declaration = declaration();
    let temporal = safe().temporal.expect("temporal");
    let wrong_forms = [
        ItemKey {
            temporal: Some(TemporalKey {
                stutter_closure: true,
                ..temporal.clone()
            }),
            ..safe()
        },
        ItemKey {
            temporal: Some(TemporalKey {
                item_kind: TemporalItemKind::DeadlockFreedom,
                ..temporal
            }),
            ..safe()
        },
    ];
    for wrong in wrong_forms {
        let result = settled(&declaration, &request(&declaration, &[sends(), wrong]));
        assert!(!discharged(&result, RELY));
        assert!(discharged(&result, LIVENESS));
    }
}
