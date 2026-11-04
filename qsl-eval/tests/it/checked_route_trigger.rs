// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-149 (FR-053-AC-7 to AC-9): the checked-package route keeps the
//! observation owner's opaque trigger identity as the one identity of a
//! temporal instance. Each row runs `evaluate_temporal_evidence` over fixture
//! **R** (`Refund` = `eventually[0,3] holds(settled)`, `on each`, capture
//! `amount`) and compares identity bytes only; no test parses an identity.
//!
//! The route's carrier is `TriggerDelivery` / `SemanticTriggerIdentity`
//! (FR-524, FR-526). The identity has no `version` or `encoding` member to
//! carry and no absent form inside a delivery, so those row-5 cases are
//! asserted at the only places they can occur: the byte constructor, the
//! `trigger` selection of the call, and an identity that merely begins with a
//! version-like prefix, which stays a distinct opaque byte string.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::temporal::evidence::{
    evaluate_temporal_evidence, ActivationState, EmptyIdentity, EvidenceRequest, EvidenceResult,
    InstanceState, Receipt, SemanticTriggerIdentity, TriggerDelivery, Truth,
};
use qsl_eval::temporal::Valuation;
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallFailure};
use quire_semantic_value::catalog::CatalogCode;

use crate::temporal_activation::{at_t0, call, delivery, env, node, request, unlimited, Env, EACH};

const REFUND: &str = "capture amount: Int[0, 9] = q.level; eventually[0, 3] holds(settled)";
const IDA: &[u8] = b"trg-A";
const IDB: &[u8] = b"trg-B";
const IDC: &[u8] = b"trg-C";

fn ident(bytes: &[u8]) -> SemanticTriggerIdentity {
    SemanticTriggerIdentity::new(bytes).expect("a non-empty identity")
}

fn refund() -> (Env, u64) {
    let env = env(EACH, REFUND);
    let settled = env.atom("settled");
    (env, settled)
}

fn arrival(
    identity: &[u8],
    receipt: &str,
    payload: &str,
    amount: &str,
) -> qsl_eval::temporal::evidence::TriggerDelivery {
    let mut delivered = delivery("x", receipt, payload, &[amount]);
    delivered.identity = ident(identity);
    delivered
}

fn assess(env: &Env, request: &EvidenceRequest, bound: &[u64]) -> EvidenceResult {
    let subject = env.subject(bound);
    call(env, &subject, request).expect("a result")
}

/// Asserts a typed admission refusal as TC-149 states it: catalog
/// `invalid_runtime_input`/`invalid-value`, the `cause`, the offending member,
/// the named fields, and an empty charge log (nothing was charged).
pub(crate) fn refused(
    env: &Env,
    asked: &EvidenceRequest,
    cause: &str,
    member: &str,
    fields: &[(&str, String)],
) {
    let mut meter = unlimited();
    let result = evaluate_temporal_evidence(&env.subject(&[]), asked, &mut meter);
    let Err(CallFailure::Admission(failure)) = result else {
        panic!(
            "an admission refusal `{cause}`, not {:?}",
            result.map(|r| r.activation)
        );
    };
    let CallAdmissionFailure::Refused(record) = *failure else {
        panic!("a refused record for `{cause}`");
    };
    assert_eq!(
        record.record.catalog,
        CatalogCode::new("invalid_runtime_input", "invalid-value")
    );
    assert_eq!(
        record.record.fields.get("cause").map(String::as_str),
        Some(cause)
    );
    assert_eq!(
        record.locus,
        CallAdmissionLocus::Member {
            name: member.to_owned(),
            index: None
        }
    );
    for (name, value) in fields {
        assert_eq!(
            record.record.fields.get(name),
            Some(value),
            "field {name} of `{cause}`"
        );
    }
    assert!(meter.admitted_charges().is_empty(), "`{cause}` charged");
}

/// The text of an identity in a refusal's field.
pub(crate) fn named(bytes: &[u8]) -> String {
    format!("{bytes:?}")
}

fn identities(result: &EvidenceResult) -> Vec<Vec<u8>> {
    result
        .instances
        .iter()
        .map(|instance| instance.identity.as_bytes().to_vec())
        .collect()
}

/// Row 1: two identities, equal receipt, payload and amount: two instances,
/// and the value one captured does not satisfy the other's proposition.
#[trace("TC-149", "FR-053-AC-7")]
#[test]
fn distinct_identities_stay_two_instances_and_a_capture_is_not_shared() {
    let (env, settled) = refund();
    let mut asked = request(&env, at_t0(&[]));
    asked.deliveries = vec![
        arrival(IDA, "r1", "same", "5"),
        arrival(IDB, "r1", "same", "5"),
    ];
    asked.trigger = Some(ident(IDA));
    assert_eq!(
        asked.deliveries[0].anchor, asked.deliveries[1].anchor,
        "the two deliveries differ in identity only"
    );
    // `settled` is instance-bound: only IDB has an entry for it.
    let only_b = BTreeMap::from([(settled, Valuation::True)]);
    for position in &mut asked.positions {
        position
            .instance_valuations
            .insert(ident(IDB), only_b.clone());
    }
    let a = assess(&env, &asked, &[settled]);
    assert_eq!(identities(&a), [IDA.to_vec(), IDB.to_vec()]);
    assert_eq!(a.truth, None, "A reads only its own entry");
    asked.trigger = Some(ident(IDB));
    let b = assess(&env, &asked, &[settled]);
    assert_eq!(b.truth, Some(Truth::True));
    assert_eq!(b.activation, ActivationState::Active);
    for instance in &b.instances {
        let amount = instance.environment.get("amount").expect("the capture");
        assert_eq!(amount.value().as_bytes(), b"5");
    }
}

/// Row 2: `IDA` delivered twice under receipts `r1` then `r2`: one instance
/// that keeps both receipts and its first captures, in one call and across
/// the caller's ledger.
#[trace("TC-149", "FR-053-AC-7")]
#[test]
fn a_repeated_delivery_keeps_one_instance_with_both_receipts() {
    let (env, _) = refund();
    let mut twice = request(&env, at_t0(&[]));
    twice.deliveries = vec![arrival(IDA, "r1", "p", "5"), arrival(IDA, "r2", "p", "5")];
    twice.trigger = Some(ident(IDA));
    let result = assess(&env, &twice, &[]);
    assert_eq!(identities(&result), [IDA.to_vec()]);
    let receipts: Vec<&[u8]> = result.instances[0]
        .receipts
        .iter()
        .map(Receipt::as_bytes)
        .collect();
    assert_eq!(receipts, [b"r1".as_slice(), b"r2".as_slice()]);
    let amount = result.instances[0].environment.get("amount").expect("kept");
    assert_eq!(amount.value().as_bytes(), b"5");

    let mut first = request(&env, at_t0(&[]));
    first.deliveries = vec![arrival(IDA, "r1", "p", "5")];
    first.trigger = Some(ident(IDA));
    let first = assess(&env, &first, &[]);
    let mut later = request(&env, at_t0(&[]));
    later.deliveries = vec![arrival(IDA, "r2", "p", "5")];
    later.trigger = Some(ident(IDA));
    later.ledger = Some(first.ledger.clone());
    let later = assess(&env, &later, &[]);
    assert_eq!(identities(&later), [IDA.to_vec()]);
    assert_eq!(later.ledger.instance_count(), 1);
    assert_eq!(later.instances[0].receipts.len(), 2);
    assert_eq!(
        later.instances[0].environment,
        first.instances[0].environment
    );
}

/// Row 3: `IDB` replaced by `IDC`, which differs at the last byte only: a
/// third instance whose identity bytes are `IDC`'s; `IDB`'s instance and
/// captures are untouched.
#[trace("TC-149", "FR-053-AC-8")]
#[test]
fn one_changed_identity_byte_is_a_new_instance_and_leaves_the_old_one() {
    let (env, _) = refund();
    let mut seeded = request(&env, at_t0(&[]));
    seeded.deliveries = vec![arrival(IDA, "r1", "p", "5"), arrival(IDB, "r1", "p", "7")];
    seeded.trigger = Some(ident(IDB));
    let seeded = assess(&env, &seeded, &[]);
    let kept = seeded.instances[1].clone();
    let mut changed = request(&env, at_t0(&[]));
    changed.deliveries = vec![arrival(IDC, "r1", "p", "7")];
    changed.trigger = Some(ident(IDC));
    changed.ledger = Some(seeded.ledger.clone());
    let changed = assess(&env, &changed, &[]);
    assert_eq!(identities(&changed), [IDC.to_vec()]);
    assert_ne!(changed.instances[0].identity, kept.identity);
    assert_eq!(changed.ledger.instance_count(), 3);
    let retained = changed
        .ledger
        .instance(&env.obligation(), &ident(IDB))
        .expect("IDB is still retained");
    assert_eq!(retained.receipts().len(), 1);
    assert_eq!(
        retained
            .environment()
            .get("amount")
            .expect("kept")
            .value()
            .as_bytes(),
        b"7"
    );
}

/// Row 4: no listed field stands in for the identity. One field varied at a
/// time under one identity leaves one instance (or, for a payload the
/// retained delivery contradicts, the refusal naming that identity); the
/// same fields held equal under two identities leave two.
#[trace("TC-149", "FR-053-AC-7", "FR-053-AC-8")]
#[test]
fn no_other_field_substitutes_for_the_identity() {
    let (env, _) = refund();
    let one = |second: qsl_eval::temporal::evidence::TriggerDelivery| {
        let mut asked = request(&env, at_t0(&[]));
        asked.deliveries = vec![arrival(IDA, "r1", "p", "5"), second];
        asked.trigger = Some(ident(IDA));
        asked
    };
    // Receipt varied.
    let receipt = assess(&env, &one(arrival(IDA, "r9", "p", "5")), &[]);
    assert_eq!(identities(&receipt), [IDA.to_vec()]);
    // Capture varied (payload equal): the first capture is kept.
    let capture = assess(&env, &one(arrival(IDA, "r1", "p", "9")), &[]);
    assert_eq!(identities(&capture), [IDA.to_vec()]);
    assert_eq!(
        capture.instances[0]
            .environment
            .get("amount")
            .expect("kept")
            .value()
            .as_bytes(),
        b"5"
    );
    // Payload varied: the contradiction names the identity; no second
    // instance and no replaced capture.
    let payload = one(arrival(IDA, "r1", "other", "5"));
    refused(
        &env,
        &payload,
        "delivery-contradiction",
        "deliveries",
        &[("contradicted_identity", named(IDA))],
    );
    // A redelivery of a held identity only adds its receipt: it ignores its
    // anchor, its guard and its captures, and raises no fault.
    let mut other_anchor = arrival(IDA, "r2", "p", "9");
    other_anchor.anchor = "obs1".to_owned();
    if let Some(qsl_eval::temporal::evidence::CaptureFact::Value { anchor, .. }) =
        other_anchor.captures.first_mut()
    {
        *anchor = "obs1".to_owned();
    }
    let mut guard_false = arrival(IDA, "r2", "p", "9");
    guard_false.guard = Some(false);
    for (name, second) in [("anchor", other_anchor), ("guard", guard_false)] {
        let result = assess(&env, &one(second), &[]);
        assert_eq!(identities(&result), [IDA.to_vec()], "{name}");
        let instance = &result.instances[0];
        assert_eq!(instance.state, InstanceState::Active, "{name}");
        assert_eq!(instance.receipts.len(), 2, "{name}");
        let kept = instance.environment.get("amount").expect("kept");
        assert_eq!(kept.value().as_bytes(), b"5", "{name}");
        assert_eq!(kept.anchor(), "obs0", "{name}");
    }
    // Static activation coordinate varied: the selected control. The second
    // delivery runs under the first's ledger, so a ledger keyed by control
    // would hold two instances.
    let mapped = env_with_two_controls();
    let mut first = request(&mapped, at_t0(&[]));
    first.deliveries = vec![arrival(IDA, "r1", "p", "5")];
    first.trigger = Some(ident(IDA));
    first.control = Some(node("c1"));
    let first = assess(&mapped, &first, &[]);
    assert_eq!(identities(&first), [IDA.to_vec()]);
    let mut second = request(&mapped, at_t0(&[]));
    second.deliveries = vec![arrival(IDA, "r2", "p", "5")];
    second.trigger = Some(ident(IDA));
    second.control = Some(node("c2"));
    second.ledger = Some(first.ledger.clone());
    let second = assess(&mapped, &second, &[]);
    assert_eq!(identities(&second), [IDA.to_vec()]);
    assert_eq!(second.ledger.instance_count(), 1);
    assert_eq!(second.instances[0].receipts.len(), 2);
    // Every other field equal, distinct identities: two instances.
    let mut distinct = request(&env, at_t0(&[]));
    distinct.deliveries = vec![arrival(IDA, "r1", "p", "5"), arrival(IDB, "r1", "p", "5")];
    distinct.trigger = Some(ident(IDA));
    assert_eq!(
        identities(&assess(&env, &distinct, &[])),
        [IDA.to_vec(), IDB.to_vec()]
    );
    // A version-like prefix is part of the opaque bytes, never translated.
    let mut prefixed = request(&env, at_t0(&[]));
    prefixed.deliveries = vec![
        arrival(IDA, "r1", "p", "5"),
        arrival(b"v2:trg-A", "r1", "p", "5"),
    ];
    prefixed.trigger = Some(ident(IDA));
    assert_eq!(
        identities(&assess(&env, &prefixed, &[])),
        [IDA.to_vec(), b"v2:trg-A".to_vec()]
    );
}

/// A `TriggerDelivery` has exactly these members: a timestamp, display-text
/// or declaration-position member would break this destructuring and so the
/// build, and the same distinct-identity deliveries in either order resolve
/// the same set of instances.
#[trace("TC-149", "FR-053-AC-7", "FR-053-AC-8")]
#[test]
fn a_delivery_has_no_member_that_could_stand_for_identity_and_order_is_not_identity() {
    let TriggerDelivery {
        identity: _,
        receipt: _,
        anchor: _,
        payload: _,
        guard: _,
        captures: _,
    } = arrival(IDA, "r1", "p", "5");
    let (env, _) = refund();
    let in_order = |deliveries: Vec<TriggerDelivery>| {
        let mut asked = request(&env, at_t0(&[]));
        asked.deliveries = deliveries;
        asked.trigger = Some(ident(IDA));
        let mut seen: Vec<_> = assess(&env, &asked, &[])
            .instances
            .into_iter()
            .map(|instance| {
                (
                    instance.identity.as_bytes().to_vec(),
                    instance.receipts.len(),
                    instance.environment,
                )
            })
            .collect();
        seen.sort_by(|left, right| left.0.cmp(&right.0));
        seen
    };
    let forward = in_order(vec![
        arrival(IDA, "r1", "p", "5"),
        arrival(IDB, "r1", "p", "5"),
        arrival(IDC, "r1", "p", "5"),
    ]);
    let backward = in_order(vec![
        arrival(IDC, "r1", "p", "5"),
        arrival(IDB, "r1", "p", "5"),
        arrival(IDA, "r1", "p", "5"),
    ]);
    assert_eq!(forward.len(), 3);
    assert_eq!(forward, backward);
}

fn env_with_two_controls() -> Env {
    env(EACH, REFUND).with_row("c1").with_row("c2")
}

/// Row 5: an empty identity is refused at construction; an absent identity
/// (a delivery list with no `trigger`, or a `trigger` no delivery resolves)
/// is refused without creating an instance; the identity type has no
/// `version` or `encoding` member to carry.
#[trace("TC-149", "FR-053-AC-9")]
#[test]
fn an_empty_or_absent_identity_is_refused_and_creates_no_instance() {
    assert_eq!(
        SemanticTriggerIdentity::new(Vec::<u8>::new()),
        Err(EmptyIdentity)
    );
    assert_eq!(EvidenceRequest::trigger_of(b""), Err(EmptyIdentity));
    assert_eq!(
        EvidenceRequest::trigger_of(IDA)
            .expect("non-empty")
            .as_bytes(),
        IDA
    );
    let (env, _) = refund();
    // No delivery at all: Inactive, no instance, no truth, never an aliased one.
    let none = assess(&env, &request(&env, at_t0(&[])), &[]);
    assert_eq!(none.activation, ActivationState::Inactive);
    assert!(none.instances.is_empty());
    assert_eq!(none.truth, None);
    let mut absent = request(&env, at_t0(&[]));
    absent.deliveries = vec![arrival(IDA, "r1", "p", "5")];
    refused(&env, &absent, "trigger-required", "trigger", &[]);
    absent.trigger = Some(ident(IDC));
    refused(&env, &absent, "trigger-unknown", "trigger", &[]);
    // Surplus capture facts are malformed input, not an aliased activation.
    let mut surplus = request(&env, at_t0(&[]));
    surplus.deliveries = vec![delivery("x", "r1", "p", &["5", "6"])];
    surplus.deliveries[0].identity = ident(IDA);
    surplus.trigger = Some(ident(IDA));
    refused(
        &env,
        &surplus,
        "surplus-capture",
        "captures",
        &[
            ("surplus_capture_identity", named(IDA)),
            ("surplus_capture_index", "1".to_owned()),
        ],
    );
}

/// Row 6: the refused calls of row 5 leave the caller's ledger as it was and
/// `IDA` then activates normally.
#[trace("TC-149", "FR-053-AC-9")]
#[test]
fn refusals_leave_no_state_and_the_identity_then_activates() {
    let (env, settled) = refund();
    let seed = {
        let mut seed = request(&env, at_t0(&[]));
        seed.deliveries = vec![arrival(IDB, "r1", "p", "5")];
        seed.trigger = Some(ident(IDB));
        assess(&env, &seed, &[])
    };
    let before = seed.ledger.clone();
    let mut absent = request(&env, at_t0(&[]));
    absent.deliveries = vec![arrival(IDA, "r1", "p", "5")];
    absent.ledger = Some(before.clone());
    refused(&env, &absent, "trigger-required", "trigger", &[]);
    let mut clash = request(&env, at_t0(&[]));
    clash.deliveries = vec![arrival(IDB, "r2", "different", "5")];
    clash.trigger = Some(ident(IDB));
    clash.ledger = Some(before.clone());
    refused(
        &env,
        &clash,
        "delivery-contradiction",
        "deliveries",
        &[("contradicted_identity", named(IDB))],
    );
    let mut ok = request(&env, at_t0(&[(settled, Valuation::True)]));
    ok.deliveries = vec![arrival(IDA, "r1", "p", "5")];
    ok.trigger = Some(ident(IDA));
    ok.ledger = Some(before);
    let result = assess(&env, &ok, &[]);
    assert_eq!(result.activation, ActivationState::Active);
    assert_eq!(result.truth, Some(Truth::True));
    assert_eq!(result.trigger, Some(ident(IDA)));
    // The ledger that call returns holds IDA and, unchanged by the refusals,
    // IDB with its first payload, one receipt and its first capture.
    assert_eq!(result.ledger.instance_count(), 2);
    let retained = result
        .ledger
        .instance(&env.obligation(), &ident(IDB))
        .expect("IDB is retained");
    assert_eq!(retained.payload().as_bytes(), b"p");
    assert_eq!(retained.receipts().len(), 1);
    assert_eq!(retained.receipts()[0].as_bytes(), b"r1");
    assert_eq!(
        retained
            .environment()
            .get("amount")
            .expect("kept")
            .value()
            .as_bytes(),
        b"5"
    );
}
