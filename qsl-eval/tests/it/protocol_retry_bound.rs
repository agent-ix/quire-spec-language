// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-520-AC-4 / TC-1315 row 5: compensation attempt bounds at the engine.

use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use ix_trace_rs::trace;
use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{
    ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse,
};
use qsl_eval::simulation::protocol::{ProtocolSystem, ResolvedMemory, Sc, StepKind};
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

#[path = "../../../qsl-semantics/tests/support/tc1070.rs"]
#[allow(dead_code)]
mod tc1070;

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

fn fixture(bound: u64) -> Fixture {
    let mut domain = tc1070::document();
    let tally = domain["types"]
        .as_array_mut()
        .expect("domain types")
        .iter_mut()
        .find(|ty| ty["displayName"] == "Tally")
        .expect("bounded attempt tally");
    let maximum = bound.saturating_sub(1);
    let tally_maximum = bound.max(1);
    for constraint in tally["constraints"]
        .as_array_mut()
        .expect("Tally constraints")
    {
        if constraint["keyword"] == "max" {
            constraint["operands"]["value"] = tally_maximum.to_string().into();
        }
    }
    let domain = serde_json::to_vec(&domain).expect("domain fixture serializes");
    let packages = package_input([domain.as_slice()]).expect("domain package admitted");
    let mut source = tc1070::SOURCE.to_owned();
    source = source
        .replace(
            "on each (opened: M::Tick) when (opened.ready) {",
            "on origin {",
        )
        .replace(
            "not trigger.ready and target >= 0",
            "trigger.ready and target >= 0",
        )
        .replace(
            "not partialTrigger.ready and partialTarget >= 0",
            "partialTrigger.ready and partialTarget >= 0",
        )
        .replace(
            "capture activated: Boolean = trigger.ready;",
            "capture activated: Boolean = false;",
        )
        .replace(
            "capture partialActivated: Boolean = partialTrigger.ready;",
            "capture partialActivated: Boolean = false;",
        )
        .replace(
            "attempts 3 of M::Attempt",
            &format!("attempts {bound} of M::Attempt"),
        )
        .replace("earlier.tally < 3", &format!("earlier.tally < {maximum}"))
        .replace(
            "partialEarlier.tally < 3",
            &format!("partialEarlier.tally < {maximum}"),
        )
        .replace("commit Main::Committed;", "commit never;");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("qsl-1757", "unit.native"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("unit parses");
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
    .expect("domain model selected");
    let package = CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            "qsl-1757".to_owned(),
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
        "identity": {"authority": "test", "identity": format!("qsl-1757-{bound}")},
        "observation": "current",
        "model": {"identity": tc1070::model::BUSINESS},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{
            "population": tc1070::model::id("nodes"),
            "complete": true,
            "objects": [{
                "key": "node-1",
                "type": tc1070::model::id("Node"),
                "fields": {"total": {"integer": "5"}}
            }]
        }]
    }))
    .expect("snapshot serializes");
    let identity = document_reference(&bytes).expect("snapshot identity");
    let initial = DocumentRef {
        authority: "test".to_owned(),
        identity: format!("qsl-1757-{bound}"),
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
    let population = quire_checked::key::DeclarationKey {
        package: tc1070::model::BUSINESS.to_owned(),
        node: tc1070::model::id("nodes"),
    };
    let request = ModelSimulationRequest {
        package: &fixture.package,
        packages: &fixture.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: &fixture.snapshots,
        initial: vec![fixture.initial.clone()],
        universes: vec![PopulationUniverse {
            population: population.clone(),
            keys: vec!["node-1".to_owned()],
        }],
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
        .protocol("Refunds")
        .expect("checked protocol");
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: request,
            proof_bounds: Vec::new(),
        },
        protocol: "Refunds".to_owned(),
        over: ModelInstance {
            population,
            key: "node-1".to_owned(),
        },
        roles: vec![RoleBinding {
            role: "Service".to_owned(),
            model: tc1070::model::BUSINESS.to_owned(),
        }],
        memory: ResolvedMemory::resolve(protocol, &[]).expect("sc memory selected"),
    };
    ProtocolSystem::new(model, &subject, Sc).expect("protocol subject built")
}

fn maximum_cattempts(
    reachable: &qsl_eval::simulation::protocol::Reachable<Sc>,
    template: &str,
) -> usize {
    fn longest(
        reachable: &qsl_eval::simulation::protocol::Reachable<Sc>,
        index: usize,
        template: &str,
        memo: &mut [Option<usize>],
        active: &mut [bool],
    ) -> usize {
        if let Some(value) = memo[index] {
            return value;
        }
        assert!(
            !active[index],
            "the finite retry fixture has no state cycle"
        );
        active[index] = true;
        let maximum = reachable.states[index]
            .steps
            .iter()
            .map(|(transition, next)| {
                let increment = usize::from(
                    transition.kind() == StepKind::CAttempt
                        && transition.template() == Some(template),
                );
                increment + longest(reachable, *next, template, memo, active)
            })
            .max()
            .unwrap_or(0);
        active[index] = false;
        memo[index] = Some(maximum);
        maximum
    }

    let mut memo = vec![None; reachable.states.len()];
    let mut active = vec![false; reachable.states.len()];
    longest(reachable, 0, template, &mut memo, &mut active)
}

#[test]
#[trace("FR-520-AC-4", "TC-1315")]
fn compensation_attempt_bound_admits_n_and_refuses_attempt_n_plus_one() {
    for bound in [1_u64, 3, 5] {
        let fixture = fixture(bound);
        let keys = fixture
            .package
            .graph()
            .protocol_node_keys("Refunds")
            .expect("protocol node keys");
        let read = qsl_package::read_evaluation_package(
            &fixture.package,
            qsl_semantics::library::LibraryName::new("qsl-1757").expect("package name"),
            &fixture.packages,
            qsl_package::V2ReadLimits::default(),
        )
        .expect("strict reader admits the protocol");
        for (index, name) in [(0, "Refund"), (1, "Partial")] {
            let compensation_key = keys
                .compensations
                .get(&index)
                .unwrap_or_else(|| panic!("compensation {name} has an emitted node"));
            let compensation = qsl_semantics::check::wire_id::structural_id(compensation_key);
            assert_eq!(
                qsl_package::protocol_view::attempts_maximum(read.admitted(), &compensation)
                    .expect("protocol view reads attempts maximum"),
                bound,
                "{name}: the read bound equals the authored count",
            );
        }

        let reachable = system(&fixture)
            .reachable(NonZeroUsize::new(1).expect("one worker"))
            .expect("finite compensation exploration completes");
        for name in ["Refunds::Refund", "Refunds::Partial"] {
            assert_eq!(
                maximum_cattempts(&reachable, name),
                usize::try_from(bound).expect("test bound fits usize"),
                "{name}: exactly n cattempts are admitted; n+1 has no successor",
            );
        }
    }
}
