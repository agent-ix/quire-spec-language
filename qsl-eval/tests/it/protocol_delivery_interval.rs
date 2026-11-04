// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-509-AC-5/AC-9/AC-10 (TC-1304): protocol exploration of interval sends.

use ix_trace_rs::trace;
use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::explore::TransitionSystem;
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc, SendOutcome, StepKind};
use qsl_foundation::selection::DefinitionRef;
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
        meter: ScalarLimits::default(),
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
    expansion: &qsl_eval::simulation::expansion::Expansion<
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
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; }",
    );
    let exact_system = system(&exact);
    let [initial] = exact_system.initial().as_slice() else {
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

    let optional = fixture(
        "[0,1]",
        2,
        "block",
        "sequence Main { send Sent via Pay as (sent: Boolean) { true }; \
         receive Got via Pay of Main::Sent as (got: Boolean) { true }; }",
    );
    let optional_system = system(&optional);
    let [initial] = optional_system.initial().as_slice() else {
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
}
