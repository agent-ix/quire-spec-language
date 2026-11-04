// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-509-AC-5/AC-9/AC-10 (TC-1304): protocol exploration of interval sends.

use ix_trace_rs::trace;
use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc, SendOutcome, StepKind};
use qsl_eval::simulation::{Expansion, TransitionSystem};
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
const POPULATION: &str = "ix://acme/orders/Gadget";
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
}

fn fixture(delivery: &str, capacity: u64, overflow: &str, run: &str) -> Fixture {
    let packages = package_input([DOMAIN]).expect("domain package admitted");
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         model M = \"acme/orders\";\n\
         protocol P using p over (input: M::Gadget) on origin {{\n\
           role Seller on M::Gadget;\n\
           role Buyer on M::Gadget;\n\
           channel Pay from Seller to Buyer carries Boolean ordering unordered \
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
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
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
            "objects": [{"key": "g1", "type": POPULATION, "fields": {}}]
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
            max_candidates: 100_000,
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
         receive R1 via Pay of Main::Sent as (r1: Boolean) { true }; \
         receive R2 via Pay of Main::Sent as (r2: Boolean) { true }; \
         receive R3 via Pay of Main::Sent as (r3: Boolean) { true }; }",
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
        "sequence Main { send First via Pay as (first: Boolean) { true }; \
         send Second via Pay as (second: Boolean) { true }; }",
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
    assert!(send_transitions(&second_expansion)
        .iter()
        .all(|transition| transition.send_record().unwrap().count() == Some(0)));

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
    let partial = send_transitions(&send_expansion)
        .into_iter()
        .find(|transition| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(3) && record.pending() == Some(1))
        })
        .expect("count three appends two entries and remains pending");
    assert_eq!(
        partial.send_record().unwrap().entries(),
        Some(&[SendOutcome::Accepted; 2][..])
    );
    let partial_state = send_expansion
        .successors
        .iter()
        .find(|(transition, _)| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(3))
        })
        .expect("partial send successor")
        .1
        .clone();
    let receive_expansion = pending_system
        .successors(&partial_state)
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
    assert!(send_transitions(&continuation).iter().any(|transition| {
        transition.send_record().is_some_and(|record| {
            record.count() == Some(1)
                && record.entries() == Some(&[SendOutcome::Accepted][..])
                && record.pending() == Some(0)
        })
    }));
    let continued_state = continuation
        .successors
        .iter()
        .find(|(transition, _)| {
            transition
                .send_record()
                .is_some_and(|record| record.count() == Some(1))
        })
        .expect("continuation state")
        .1
        .clone();
    assert!(!pending_system
        .threads(&continued_state)
        .expect("sender thread view")
        .iter()
        .any(|thread| thread
            .rest_point
            .node
            .as_deref()
            .is_some_and(|node| node.ends_with("::Valid"))));
}
