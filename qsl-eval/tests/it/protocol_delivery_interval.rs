// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-509-AC-5/AC-9/AC-10 (TC-1304): protocol exploration of interval sends.

use ix_trace_rs::trace;
use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{
    explore_protocol, ProtocolSystem, ResolvedMemory, Sc, SendOutcome, StepKind,
};
use qsl_eval::simulation::{Expansion, Limits, Outcome, SubjectSystem, TransitionSystem};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::observation::{
    document_reference, DocumentProvision, DocumentRef, ObservationLimits,
};
use quire_exact::ScalarLimits;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;
use std::collections::BTreeMap;

const DOMAIN: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");
const TYPE: &str = "ix://acme/orders/Gadget";
const POPULATION: &str = "ix://acme/orders/gadgets";
const METER: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

struct Fixture {
    package: CheckedPackage,
    packages: qsl_semantics::model::intake::PackageInput,
    snapshots: DocumentProvision,
    initial: DocumentRef,
    max_candidates: u64,
}

fn fixture(delivery: &str, capacity: u64, overflow: &str, run: &str) -> Fixture {
    let mut domain: serde_json::Value =
        serde_json::from_slice(DOMAIN).expect("domain fixture JSON");
    domain["constructs"].as_array_mut().unwrap().push(json!({
        "kind": {"module": "acme/orders", "name": "population"},
        "moduleVersion": "1.0.0",
        "construct": {"identity": "none", "shape": "record", "members": {},
            "meaning": "quire.meaning.model.population/v1"}
    }));
    domain["populations"] = json!([{
        "identity": POPULATION,
        "displayName": "gadgets",
        "kind": {"module": "acme/orders", "name": "population"},
        "members": [TYPE],
        "extent": "closed",
        "origin": {"generated": {"generatorIdentity": POPULATION,
            "generatorVersion": "1.0.0", "inputIdentities": [POPULATION]}}
    }]);
    let domain = serde_json::to_vec(&domain).expect("domain fixture serializes");
    let packages = package_input([domain.as_slice()]).expect("domain package admitted");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model M = \"acme/orders\";\n\
         protocol P using p over (input: M::Gadget) on origin {{\n\
           role Seller on M::Gadget;\n\
           role Buyer on M::Gadget;\n\
           channel Pay from Seller to Buyer carries Boolean ordering fifo by (m: Boolean) {{ m }} \
             delivery {delivery} capacity {capacity} overflow {overflow};\n\
           run {run}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("protocol-delivery", "unit.native"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("unit parsed");
    assert!(
        parsed.is_admissible(),
        "{:?}\n{source}",
        parsed.diagnostics()
    );
    let unit = qsl_forms::build_unit(&parsed).expect("unit forms built");
    let models = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model selected");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "protocol delivery fixture".to_owned(),
            unit,
            models,
            Vec::new(),
        )
        .expect("unit assembled")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("protocol checks: {refusals:?}")),
    );

    let bytes = serde_json::to_vec(&json!({
        "format": "quire.state.snapshot/v1",
        "identity": {"authority": "test", "identity": "protocol-delivery-s0"},
        "observation": "current",
        "model": {"identity": "acme/orders"},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": POPULATION,
            "complete": true,
            "objects": [{"key": "g1", "type": TYPE, "fields": {"code": {"integer": "1"}}}]
        }]
    }))
    .expect("snapshot serializes");
    let identity = document_reference(&bytes).expect("snapshot identifies itself");
    let initial = DocumentRef {
        authority: "test".to_owned(),
        identity: "protocol-delivery-s0".to_owned(),
    };
    let snapshots = BTreeMap::from([(identity, bytes)]);
    Fixture {
        package,
        packages,
        snapshots,
        initial,
        max_candidates: 1_000_000,
    }
}

fn system<'a>(fixture: &'a Fixture) -> ProtocolSystem<'a, Sc> {
    let universe = PopulationUniverse {
        population: quire_checked::key::DeclarationKey {
            package: "acme/orders".to_owned(),
            node: POPULATION.to_owned(),
        },
        keys: vec!["g1".to_owned()],
    };
    let request = ModelSimulationRequest {
        package: &fixture.package,
        packages: &fixture.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &fixture.snapshots,
        initial: vec![fixture.initial.clone()],
        universes: vec![universe],
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: fixture.max_candidates,
        },
        meter: METER,
    };
    let model = ModelSystem::new(request.clone()).expect("model subject built");
    let protocol = fixture
        .package
        .graph()
        .protocol("P")
        .expect("protocol checked");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "P".to_owned(),
        over: ModelInstance {
            population: quire_checked::key::DeclarationKey {
                package: "acme/orders".to_owned(),
                node: POPULATION.to_owned(),
            },
            key: "g1".to_owned(),
        },
        roles: vec![
            RoleBinding {
                role: "Seller".to_owned(),
                model: "acme/orders".to_owned(),
            },
            RoleBinding {
                role: "Buyer".to_owned(),
                model: "acme/orders".to_owned(),
            },
        ],
        memory: ResolvedMemory::resolve(protocol, &[]).expect("default sc memory selected"),
    };
    ProtocolSystem::new(model, &subject, Sc).expect("protocol subject built")
}

fn send_transitions(
    expansion: &Expansion<
        qsl_eval::simulation::protocol::ProtocolTransition,
        qsl_eval::simulation::protocol::ProtocolState<Sc>,
        qsl_eval::simulation::model::ModelFinding,
    >,
) -> Vec<&qsl_eval::simulation::protocol::ProtocolTransition> {
    expansion
        .successors
        .iter()
        .map(|(transition, _)| transition)
        .filter(|transition| transition.kind() == StepKind::Send)
        .collect()
}

fn pending_payload(key: &serde_json::Value) -> bool {
    key["control"]["entries"]
        .as_array()
        .expect("control entries are a list")
        .iter()
        .find(|entry| entry["key"]["type"] == "send-pending")
        .and_then(|entry| entry["value"]["payload"]["value"].as_bool())
        .unwrap_or_else(|| panic!("pending Boolean payload in {key}"))
}

fn send_record_value(record: &qsl_eval::simulation::protocol::SendRecord) -> serde_json::Value {
    let bytes = quire_canonical::to_vec(record, quire_canonical::Limits::new(1 << 20))
        .expect("send record encodes");
    serde_json::from_slice(&bytes).expect("send record is JSON")
}

/// FR-509-AC-5/AC-9 (TC-1304): interval counts are choices, overflow outcomes
/// are recorded entry by entry, and a zero count settles without queueing.
#[trace("FR-509-AC-5", "FR-509-AC-9", "TC-1304")]
#[test]
fn interval_send_records_count_entries_and_zero_delivery() {
    let exact = fixture(
        "[2,2]",
        2,
        "block",
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; \
         receive ReadA via Pay of Main::Sent as (readA: Boolean) { true }; \
         receive ReadB via Pay of Main::Sent as (readB: Boolean) { true }; \
         receive ReadC via Pay of Main::Sent as (readC: Boolean) { true }; }",
    );
    let exact_system = system(&exact);
    let initial_states = exact_system.initial();
    let [initial] = initial_states.as_slice() else {
        panic!("one initial state");
    };
    let expansion = exact_system.successors(initial).expect("send expands");
    let sends = send_transitions(&expansion);
    assert_eq!(sends.len(), 2, "one step for each Boolean payload");
    for send in sends {
        let record = send.send_record().expect("send has its record");
        assert_eq!(record.count(), Some(2));
        assert_eq!(record.entries(), Some(&[SendOutcome::Accepted; 2][..]));
        assert_eq!(record.pending(), Some(0));
        let encoded = send_record_value(record);
        assert!(encoded.get("outcome").is_none());
        assert!(encoded.get("displaced").is_none());
        assert_eq!(encoded["entries"].as_array().unwrap().len(), 2);
        for entry in encoded["entries"].as_array().unwrap() {
            assert_eq!(entry["outcome"], "accepted");
            assert_eq!(entry["displaced"], serde_json::Value::Null);
        }
    }
    let sent_state = expansion
        .successors
        .iter()
        .find(|(transition, _)| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(2))
        })
        .expect("one d=2 successor")
        .1
        .clone();
    let received_once = exact_system
        .successors(&sent_state)
        .expect("first receive expands");
    let [(_, after_first)] = received_once
        .successors
        .iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Receive)
        .collect::<Vec<_>>()[..]
    else {
        panic!("one distinct first receive");
    };
    let received_twice = exact_system
        .successors(after_first)
        .expect("second receive expands");
    assert!(received_twice
        .successors
        .iter()
        .any(|(transition, _)| transition.kind() == StepKind::Receive));
    let [(_, after_second)] = received_twice
        .successors
        .iter()
        .filter(|(transition, _)| transition.kind() == StepKind::Receive)
        .collect::<Vec<_>>()[..]
    else {
        panic!("one distinct second receive");
    };
    let received_third = exact_system
        .successors(after_second)
        .expect("third receive waits on empty queue");
    assert!(!received_third
        .successors
        .iter()
        .any(|(transition, _)| transition.kind() == StepKind::Receive));

    let rejected = fixture(
        "[2,2]",
        1,
        "reject",
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; }",
    );
    let rejected_system = system(&rejected);
    let rejected_initial = rejected_system.initial();
    let rejected_expansion = rejected_system
        .successors(&rejected_initial[0])
        .expect("overflow send expands");
    for send in send_transitions(&rejected_expansion) {
        assert_eq!(
            send.send_record().unwrap().entries(),
            Some(&[SendOutcome::Accepted, SendOutcome::Rejected][..])
        );
        assert_eq!(send.send_record().unwrap().pending(), Some(0));
    }

    let optional = fixture(
        "[0,1]",
        2,
        "block",
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; \
         receive Got via Pay of Main::Sent as (got: Boolean) { true }; }",
    );
    let optional_system = system(&optional);
    let initial_states = optional_system.initial();
    let [initial] = initial_states.as_slice() else {
        panic!("one initial state");
    };
    let sends = optional_system
        .successors(initial)
        .expect("optional send expands")
        .successors;
    let zero = sends
        .iter()
        .find(|(transition, _)| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(0))
        })
        .expect("zero delivery is enabled");
    assert_eq!(zero.0.send_record().unwrap().entries(), Some(&[][..]));
    assert_eq!(zero.0.send_record().unwrap().pending(), Some(0));
    let after_zero = optional_system
        .successors(&zero.1)
        .expect("zero-delivery successor expands");
    assert!(!after_zero
        .successors
        .iter()
        .any(|(transition, _)| transition.kind() == StepKind::Receive));

    let full = fixture(
        "[0,3]",
        1,
        "block",
        "sequence Main { send First via Pay as (readA: Boolean) { readA }; \
         send Second via Pay as (readB: Boolean) { readB }; }",
    );
    let full_system = system(&full);
    let full_initial = full_system.initial();
    let first_expansion = full_system
        .successors(&full_initial[0])
        .expect("first send expands");
    let after_first = first_expansion
        .successors
        .iter()
        .find(|(transition, _)| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(1))
        })
        .expect("first send can enqueue one entry")
        .1
        .clone();
    let second_expansion = full_system
        .successors(&after_first)
        .expect("full queue send expands");
    let second_counts = send_transitions(&second_expansion)
        .iter()
        .map(|transition| transition.send_record().unwrap().count())
        .collect::<Vec<_>>();
    assert!(
        second_counts.iter().all(|count| *count == Some(0)),
        "{second_counts:?}"
    );

    let pending = fixture(
        "[1,3]",
        2,
        "block",
        "parallel par { \
           branch l sequence Sender { \
             send Sent via Pay as (sent: Boolean) { sent }; \
             check Valid using v { sent }; \
           } \
           branch r receive Got via Pay of l::Sender::Sent as (got: Boolean) { true }; \
         } join all [l, r];",
    );
    let pending_system = system(&pending);
    let pending_initial = pending_system.initial();
    let fork_expansion = pending_system
        .successors(&pending_initial[0])
        .expect("parallel forks");
    let after_fork = fork_expansion
        .successors
        .iter()
        .find(|(transition, _)| transition.kind() == StepKind::Fork)
        .expect("parallel fork step")
        .1
        .clone();
    let send_expansion = pending_system
        .successors(&after_fork)
        .expect("parallel branches expand");
    let mut checked_payloads = Vec::new();
    for (send, partial_state) in &send_expansion.successors {
        let Some(record) = send.send_record() else {
            continue;
        };
        if record.count() != Some(3) || record.pending() != Some(1) {
            continue;
        }
        assert_eq!(record.entries(), Some(&[SendOutcome::Accepted; 2][..]));
        let (partial_key, _) = pending_system
            .state_key(partial_state)
            .expect("partial key");
        let partial_key: serde_json::Value = serde_json::from_slice(&partial_key).unwrap();
        let payload = pending_payload(&partial_key);
        let receive_expansion = pending_system
            .successors(partial_state)
            .expect("receiver frees one queue slot");
        let after_receive = receive_expansion
            .successors
            .iter()
            .find(|(transition, _)| transition.kind() == StepKind::Receive)
            .expect("receiver consumes one entry")
            .1
            .clone();
        let continuation = pending_system
            .successors(&after_receive)
            .expect("blocked send resumes");
        let (_, continued_state) = continuation
            .successors
            .iter()
            .find(|(transition, _)| {
                transition.kind() == StepKind::Send
                    && transition.send_record().is_some_and(|record| {
                        record.count() == Some(1)
                            && record.entries() == Some(&[SendOutcome::Accepted][..])
                            && record.pending() == Some(0)
                    })
            })
            .expect("continuation accepts one entry and settles");
        let rest_nodes = pending_system
            .threads(continued_state)
            .expect("sender thread view")
            .into_iter()
            .filter_map(|thread| thread.rest_point.node)
            .collect::<Vec<_>>();
        let remains_at_check = rest_nodes.iter().any(|node| node.ends_with("/Valid"));
        assert_eq!(
            remains_at_check, !payload,
            "the send binder must preserve payload {payload}: {rest_nodes:?}"
        );
        checked_payloads.push(payload);
    }
    assert!(!checked_payloads.is_empty());
}

/// FR-509-AC-9 (TC-1304): interval send records retain displacement at the
/// entry that displaced an existing queue member, with no scalar summary.
#[trace("FR-509-AC-9", "TC-1304")]
#[test]
fn interval_send_record_places_displacement_on_the_attempted_entry() {
    let source = fixture(
        "[2,2]",
        1,
        "loss head",
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; }",
    );
    let system = system(&source);
    let initial = system.initial();
    let expansion = system.successors(&initial[0]).expect("loss send expands");
    let sends = send_transitions(&expansion);
    assert_eq!(sends.len(), 2, "one send for each Boolean payload");
    for send in sends {
        let record = send.send_record().expect("send has its record");
        assert_eq!(record.entries(), Some(&[SendOutcome::Accepted; 2][..]));
        let encoded = send_record_value(record);
        assert!(encoded.get("outcome").is_none());
        assert!(encoded.get("displaced").is_none());
        let entries = encoded["entries"].as_array().expect("entry records");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["outcome"], "accepted");
        assert_eq!(entries[0]["displaced"], serde_json::Value::Null);
        assert_eq!(entries[1]["outcome"], "accepted");
        assert!(entries[1]["displaced"].is_object());
        assert_eq!(entries[1]["displaced"]["type"], "queue-entry");
    }
}

/// FR-509-AC-9 (TC-1304): pending-only states preserve count and payload in
/// the sole differing state-key member, and each continuation keeps its own
/// payload binding.
#[trace("FR-509-AC-9", "TC-1304")]
#[test]
fn fifo_pending_states_keep_count_payload_and_distinct_keys() {
    let pending = fixture(
        "[2,3]",
        1,
        "block",
        "parallel par { \
           branch l sequence Sender { \
             send Sent via Pay as (sent: Boolean) { true }; \
             check Valid using v { sent }; \
           } \
           branch r receive Got via Pay of l::Sender::Sent as (got: Boolean) { true }; \
         } join all [l, r];",
    );
    let system = system(&pending);
    let initial_states = system.initial();
    let [initial] = initial_states.as_slice() else {
        panic!("one initial state");
    };
    let fork = system.successors(initial).expect("parallel forks");
    let after_fork = fork
        .successors
        .iter()
        .find(|(step, _)| step.kind() == StepKind::Fork)
        .expect("fork transition")
        .1
        .clone();
    let send_expansion = system.successors(&after_fork).expect("send expands");
    let mut pending_states = Vec::new();
    for (send, after_send) in &send_expansion.successors {
        let Some(record) = send.send_record() else {
            continue;
        };
        let Some(count @ (2 | 3)) = record.count() else {
            continue;
        };
        let receives = system.successors(after_send).expect("receive expands");
        for (receive, after_receive) in &receives.successors {
            if receive.kind() != StepKind::Receive {
                continue;
            }
            let (bytes, _) = system
                .state_key(after_receive)
                .expect("pending key encodes");
            let key: serde_json::Value =
                serde_json::from_slice(&bytes).expect("canonical JSON key");
            let entries = key["control"]["entries"]
                .as_array()
                .expect("control entries are a list");
            let pending_member = entries
                .iter()
                .find(|entry| entry["key"]["type"] == "send-pending")
                .unwrap_or_else(|| panic!("one pending send member: {key}"));
            let pending_value = &pending_member["value"];
            let remaining = pending_value["remaining"]
                .as_str()
                .and_then(|text| text.parse::<u64>().ok())
                .or_else(|| pending_value["remaining"].as_u64())
                .expect("pending count encoded");
            let payload = &pending_value["payload"];
            let value = payload["value"]
                .as_bool()
                .unwrap_or_else(|| panic!("unexpected Boolean payload encoding: {payload}"));
            let mut without_pending = key.clone();
            without_pending["control"]["entries"]
                .as_array_mut()
                .unwrap()
                .retain(|entry| entry["key"]["type"] != "send-pending");
            pending_states.push((
                count - 1,
                value,
                remaining,
                bytes,
                without_pending,
                after_receive.clone(),
            ));
        }
    }
    let expected = [(1, true), (1, false), (2, true)];
    for (count, payload) in expected {
        assert!(
            pending_states
                .iter()
                .any(|(candidate, value, remaining, ..)| {
                    *candidate == count && *value == payload && *remaining == count
                }),
            "missing pending ({count}, {payload}): {pending_states:?}"
        );
    }
    for (pending_count, payload, remaining, _, _, state) in &pending_states {
        if *pending_count != 1 || *remaining != 1 {
            continue;
        }
        let continuation = system.successors(state).expect("one pending entry resumes");
        let (_, continued_state) = continuation
            .successors
            .iter()
            .find(|(transition, _)| {
                transition.kind() == StepKind::Send
                    && transition.send_record().is_some_and(|record| {
                        record.count() == Some(1)
                            && record.entries() == Some(&[SendOutcome::Accepted][..])
                            && record.pending() == Some(0)
                    })
            })
            .expect("continuation settles its one remaining entry");
        let rests = system
            .threads(continued_state)
            .expect("sender thread view")
            .into_iter()
            .filter_map(|thread| thread.rest_point.node)
            .collect::<Vec<_>>();
        let remains_at_check = rests.iter().any(|node| node.ends_with("/Valid"));
        assert_eq!(
            remains_at_check, !*payload,
            "continuation must bind its own payload {payload}: {rests:?}"
        );
    }
    for left in 0..pending_states.len() {
        for right in left + 1..pending_states.len() {
            if pending_states[left].0 == pending_states[right].0
                && pending_states[left].1 == pending_states[right].1
            {
                continue;
            }
            assert_ne!(pending_states[left].3, pending_states[right].3);
            assert_eq!(
                pending_states[left].4, pending_states[right].4,
                "only the send-pending entry may distinguish these states"
            );
        }
    }
}

/// FR-509-AC-10 (TC-1304): work exhaustion preserves the send state on the
/// frontier after exactly the d=0 and d=1 work, while unlimited expansion
/// generates every count in ascending order.
#[trace("FR-509-AC-10", "TC-1304")]
#[test]
fn interval_send_work_limit_keeps_frontier_and_unlimited_counts() {
    let run = "sequence Main { send P via Pay as (payload: Boolean) { payload }; }";
    let unlimited = fixture("[0,1000]", 2, "reject", run);
    let unlimited_system = system(&unlimited);
    let initial_states = unlimited_system.initial();
    let [initial] = initial_states.as_slice() else {
        panic!("one initial state");
    };
    let all = unlimited_system
        .successors(initial)
        .expect("unlimited send expands");
    let counts = send_transitions(&all)
        .iter()
        .map(|step| step.send_record().unwrap().count().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(counts, (0..=1000).collect::<Vec<_>>());

    let mut d0 = fixture("[0,0]", 2, "reject", run);
    d0.max_candidates = 3;
    let d0_system = system(&d0);
    let d0_initial = d0_system.initial();
    let [state] = d0_initial.as_slice() else {
        panic!("one d=0 initial state");
    };
    let d0_expansion = d0_system.successors(state).expect("W0 + 1 admits d=0");
    let d0_steps = send_transitions(&d0_expansion);
    assert_eq!(d0_steps.len(), 1);
    assert_eq!(d0_steps[0].send_record().unwrap().count(), Some(0));

    let mut d1 = fixture("[0,1]", 2, "reject", run);
    d1.max_candidates = 5;
    let d1_system = system(&d1);
    let d1_initial = d1_system.initial();
    let [state] = d1_initial.as_slice() else {
        panic!("one d=1 initial state");
    };
    let d1_expansion = d1_system
        .successors(state)
        .expect("W0 + 3 admits d=0 and d=1");
    let d1_steps = send_transitions(&d1_expansion);
    assert_eq!(
        d1_steps
            .iter()
            .map(|step| step.send_record().unwrap().count().unwrap())
            .collect::<Vec<_>>(),
        [0, 1]
    );

    let mut limited = fixture("[0,1000]", 2, "reject", run);
    limited.max_candidates = 5; // W0 = 2 Boolean values; W0 + 3 charges d=0, d=1 and its entry.
    let limited_system = system(&limited);
    let initial_states = limited_system.initial();
    let [initial] = initial_states.as_slice() else {
        panic!("one initial state");
    };
    let (send_key, _) = limited_system.state_key(initial).expect("send state key");
    let exploration = explore_protocol(&limited_system, 1_000_000, Limits::default(), || false)
        .expect("work limit is an incomplete result");
    let Outcome::Stopped {
        cause, frontier, ..
    } = &exploration.outcome
    else {
        panic!(
            "expected a stopped exploration, got {:?}",
            exploration.outcome
        );
    };
    assert_eq!(cause.code(), "resource_exhausted");
    assert_eq!(cause.cause(), "insufficient-next-charge");
    assert_eq!(frontier.len(), 1);
    assert_eq!(frontier[0].as_bytes(), send_key.as_slice());
    assert_eq!(
        exploration.outcome.category(),
        quire_semantic_value::catalog::Category::Incomplete
    );
}
