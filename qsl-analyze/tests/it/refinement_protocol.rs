// SPDX-License-Identifier: AGPL-3.0-or-later
//! A refinement whose step rows select a concrete protocol (FR-136): S3
//! checks it, and the refinement engine, whose systems explore a concrete
//! model, refuses it with a typed cause and not a verdict. The engine over a
//! protocol subject (FR-142, FR-145) is not built yet.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, ModelCheckRefusal, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_replay::{
    check_simulation_certificate, ClosureRefusal, ModelCheckLimits, SafetyOutcome,
    SimulationCertificateRequest, SimulationRefusal,
};
use quire_exact::Cancel;

use crate::refinement_fixture::{cas_refines_counter, World, UNLIMITED};

/// `Run`, a protocol of one attempt of `peek` over the compare-and-set
/// counter, and the refinement of the counter by it.
fn protocol_refinement() -> String {
    "protocol Run using v over (k: Impl::Counter) on origin {\n\
     role w on Impl::Counter;\n\
     run sequence Main {\n\
     attempt P by w on Impl::Counter::peek contracts [] as (p: Boolean) { true };\n\
     }\n\
     finish Done as (d: Boolean) { true };\n\
     }\n\
     refinement CasRun using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::peek -> stutter;\n\
     step Impl::Run::Done -> stutter;\n\
     }"
    .to_owned()
}

/// The engine meets a refinement over a protocol and refuses it, naming the
/// cause, before it explores a state.
#[trace("TC-547")]
#[test]
fn the_engine_refuses_a_refinement_over_a_protocol() {
    let world = World::new(&protocol_refinement());
    assert!(world.refinement().concrete_protocol().is_some());
    let refused = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    );
    assert!(
        matches!(refused, Err(ModelCheckRefusal::ProtocolRefinement)),
        "{refused:?}"
    );
}

/// The certificate checker refuses a request over a protocol too, naming the
/// cause: the subjects it explores are concrete models.
#[trace("TC-556")]
#[test]
fn the_certificate_checker_refuses_a_refinement_over_a_protocol() {
    let model = World::new(&cas_refines_counter());
    let SafetyOutcome::Holds { certificate, .. } = check_refinement(
        RefinementCheckRequest {
            refinement: model.refinement(),
            concrete: ModelSubject {
                simulation: model.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: model.package(),
            abstract_initial: vec![model.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the model refinement is checked")
    .safety
    else {
        panic!("the model refinement holds");
    };
    let mut certificate = *certificate;
    certificate.refinement = "CasRun".to_owned();

    let world = World::new(&protocol_refinement());
    let refused = check_simulation_certificate(
        &SimulationCertificateRequest {
            refinement: world.refinement(),
            concrete: world.concrete_request(UNLIMITED),
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
        },
        &certificate,
        &ModelCheckLimits::default(),
        &Cancel::new(),
    );
    assert!(
        matches!(
            refused,
            Err(SimulationRefusal::Subject(
                ClosureRefusal::ProtocolRefinement
            ))
        ),
        "{refused:?}"
    );
}
