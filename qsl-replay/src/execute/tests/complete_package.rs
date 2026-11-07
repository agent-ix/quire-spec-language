// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete replay inputs are retained and natively admitted before evaluation.

use super::*;
use quire_contract_model::{CheckedPackageDispatchResult, CheckedPackageReadLimits};

fn emitted() -> qsl_package::Emission {
    let compiled = spine(&proved(), &BTreeMap::new());
    qsl_package::emit_checked(&compiled.package, qsl_package::EmitLimits::fixture())
        .expect("genuine checked source emits")
}

#[test]
#[trace("TC-1325", "FR-350-AC-16")]
fn complete_package_round_trips_and_native_admission_keeps_graph_and_lock() {
    let emission = emitted();
    let wire = small(7);
    assert_eq!(wire.package.bytes(), emission.package().bytes());
    let request = ReplayRequest::decode(wire.clone(), ReplayLimits::default()).unwrap();
    assert_eq!(request.to_wire().package.bytes(), wire.package.bytes());
    let admitted = request
        .package()
        .admit(emission.evidence(), CheckedPackageReadLimits::bounded())
        .unwrap();
    assert_eq!(admitted.name(), "fixture");
    assert!(
        !admitted.graph().nodes.is_empty(),
        "the whole native graph survives admission"
    );
    assert_eq!(admitted.lock().sources[0].authority.as_ref(), AUTHORITY);
    assert_eq!(admitted.lock().sources[0].identity.as_ref(), IDENTITY);
}

#[test]
#[trace("TC-1325", "FR-350-AC-16")]
fn native_package_fault_is_not_laundered_by_its_valid_name_and_sources() {
    let emission = emitted();
    let mut wire = small(7);
    wire.package = crate::request::mutate_raw_package(&wire.package, |package| {
        package["semantic_graph"]["nodes"][0]["node_tag"] = serde_json::json!("unknown_native_tag");
    });
    let native = match quire_contract_model::read_checked_package(
        wire.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        emission.evidence(),
    ) {
        CheckedPackageDispatchResult::Refused(refusal) => refusal,
        other => panic!("the native owner refuses this graph: {other:?}"),
    };
    let error = replay(wire, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Package(crate::ReplayPackageAdmissionRefusal::Refused { refusal, .. }) =
        &error
    else {
        panic!("the actual native cause survives replay: {error:?}");
    };
    assert_eq!(refusal.as_ref(), &native);
    let expected_pointer = format!("/package{}", native.loci[0].path.as_ref().unwrap().as_str());
    assert_eq!(error.pointer().as_deref(), Some(expected_pointer.as_str()));
    let expected_cause = serde_json::to_value(native.cause).unwrap();
    assert_eq!(error.cause(), expected_cause.as_str());
    let result = Err(error);
    let outcome = crate::OutcomeDocument::from_replay(&result);
    let output: serde_json::Value = serde_json::from_slice(&outcome.to_bytes().unwrap()).unwrap();
    assert_eq!(
        output["diagnostics"][0]["locus"]["pointer"],
        expected_pointer
    );
}

#[test]
#[trace("TC-1325", "FR-350-AC-16")]
fn original_noncanonical_package_spelling_reaches_the_native_owner_unchanged() {
    let emission = emitted();
    let mut bytes = emission.package().bytes().to_vec();
    bytes.insert(1, b' ');
    let mut wire = small(7);
    wire.package = crate::ReplayPackage::read(&bytes, ReplayLimits::default().input_bytes).unwrap();
    assert_eq!(wire.package.bytes(), bytes);
    let error = replay(wire, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Package(crate::ReplayPackageAdmissionRefusal::Refused { refusal, .. }) =
        &error
    else {
        panic!("native refusal: {error:?}")
    };
    assert_eq!(
        refusal.code,
        quire_contract_model::CheckedPackageRefusalCode::NoncanonicalWire
    );
    assert_eq!(error.pointer().as_deref(), Some("/package"));
}

#[test]
#[trace("TC-1325", "FR-350-AC-16")]
fn native_admission_uses_the_callers_finite_i2_input_ceiling() {
    let mut wire = small(7);
    wire.stage_limits.insert("i2.input_bytes".to_owned(), 0);
    let error = replay(wire, ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::Package(crate::ReplayPackageAdmissionRefusal::Incomplete(limit)) = &error
    else {
        panic!("native reader stop: {error:?}")
    };
    assert_eq!(limit.limit, 0);
    assert_eq!(error.code(), Code::StageLimitExceeded);
    assert_eq!(error.pointer().as_deref(), Some("/package"));
}

#[test]
#[trace("TC-1325", "FR-350-AC-16")]
fn admitted_offered_package_does_not_restore_the_retired_source_staleness_check() {
    let mut wire = small(7);
    let changed = proved().replace("x < 5", "x > 5");
    wire.byte_provision[0].2 = changed.into_bytes();
    let ReplayResult::Input(result) = replay(wire, ReplayLimits::default()).unwrap() else {
        panic!("an input replay")
    };
    assert_eq!(
        result.value(),
        Some(EvaluatedValue::Boolean(true)),
        "evaluation uses the genuine fresh recompile"
    );
}

#[test]
#[trace("TC-444", "FR-098-AC-20")]
#[trace("TC-1325", "FR-350-AC-16")]
fn supplied_root_internal_lists_reach_the_semantic_owner_before_native_admission() {
    let mut wire = small(7);
    wire.package = crate::request::mutate_raw_package(&wire.package, |package| {
        package["lock"]["refinement_internals"] =
            serde_json::json!([{"refinement":"NoSuchRefinement", "internal":["tau"]}]);
    });
    let request = ReplayRequest::decode(wire.clone(), ReplayLimits::default()).unwrap();
    let error = replay(wire, ReplayLimits::default()).unwrap_err();
    let expected_pointer = "/package/lock/refinement_internals/0/refinement";
    assert_eq!(error.pointer().as_deref(), Some(expected_pointer));
    let outcome = crate::OutcomeDocument::from_replay(&Err(error));
    let output: serde_json::Value = serde_json::from_slice(&outcome.to_bytes().unwrap()).unwrap();
    assert_eq!(
        output["diagnostics"][0]["locus"]["pointer"],
        expected_pointer
    );
    // Obtain another genuine compiler failure; request-location composition
    // must retain its entire native value and its relative list locus.
    let error = replay(request.to_wire(), ReplayLimits::default()).unwrap_err();
    let ReplayRefusal::RefinementInternals {
        refusal,
        protocol_results,
        ..
    } = error
    else {
        panic!("the source owner checks its supplied lists: {error:?}")
    };
    let native_before = format!("{refusal:?}");
    let code = refusal.code();
    let cause = refusal.cause().map(str::to_owned);
    let ordinary = match protocol_results {
        Some(protocol_results) => ReplayRefusal::RecompileWithProtocolResults {
            refusal,
            protocol_results,
        },
        None => ReplayRefusal::Recompile(refusal),
    };
    let located = locate_refinement_internals(&request, ordinary);
    assert_eq!(located.code(), code);
    assert_eq!(located.cause(), cause.as_deref());
    assert_eq!(located.pointer().as_deref(), Some(expected_pointer));
    let ReplayRefusal::RefinementInternals { refusal, .. } = located else {
        panic!("located native refusal");
    };
    assert_eq!(format!("{refusal:?}"), native_before);
    let CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("S3 list check: {refusal:?}")
    };
    let qsl_semantics::check::CheckCause::Refinement(refusal) = &refusals[0].cause else {
        panic!("typed refinement list cause: {refusals:?}")
    };
    let qsl_semantics::check::RefinementCause::InternalList(defect) = &refusal.cause else {
        panic!("typed internal list: {refusal:?}")
    };
    assert_eq!(defect.pointer, "/0/refinement");
}

#[test]
#[trace("TC-444", "FR-098-AC-20")]
fn malformed_raw_internal_list_refuses_at_its_full_carrier_location() {
    let mut wire = small(7);
    wire.package = crate::request::mutate_raw_package(&wire.package, |package| {
        package["lock"]["refinement_internals"] = serde_json::json!({});
    });
    let before = super::super::spine_run_count();
    let error = replay(wire, ReplayLimits::default()).unwrap_err();
    assert_eq!(super::super::spine_run_count(), before);
    assert_eq!(error.code(), Code::InvalidPackage);
    assert_eq!(error.cause(), Some("wrong-value-kind"));
    assert_eq!(
        error.pointer().as_deref(),
        Some("/package/lock/refinement_internals")
    );
    assert!(matches!(
        error,
        ReplayRefusal::Request(crate::ReplayRequestRefusal::PackageMember {
            cause: crate::request::PackageMemberCause::WrongKind,
            ..
        })
    ));
}
