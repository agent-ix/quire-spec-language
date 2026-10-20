// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185: a package whose random parameters, rewards or workloads check is
//! refused at emission, since FR-322's v2 vocabulary has no node for them,
//! rather than written without them.

use super::admission_corpus::config_version_unit;
use super::*;

/// The ConfigVersion unit with a workload and a reward on `attemptUpdate`.
fn probabilistic_package() -> CheckedPackage {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = format!(
        "{}workload Steady on Config {{ weight ConfigVersion::attemptUpdate = 1; }}\n\
         reward cost on ConfigVersion::attemptUpdate = 1;\n",
        config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest))
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert!(!graph.probabilistic_model().is_empty());
    CheckedPackage::link(graph)
}

/// FR-185 "Package emission": the checked workload and reward are recorded
/// on the graph, and the emitter refuses
/// `unsupported_construct`/`not-yet-implemented`, naming the first of them in
/// source order (the workload `Steady`), its feature and the unit's profile.
#[trace("TC-620")]
#[test]
fn a_checked_probabilistic_model_refuses_emission() {
    let refusal = emit_checked(&probabilistic_package()).expect_err("emission refuses");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "Steady".to_owned(),
            feature: qsl_semantics::check::ProbabilisticFeature::Workload,
            profile: Some("quire.model.complete/v1"),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

/// FR-186 "Outputs": S3 now checks a `probabilistic` claim, so the emitter's
/// guard is all that keeps one out of a checked-package/v2 graph: a package
/// whose first probabilistic declaration is a claim is refused naming it and
/// the feature `probabilistic-claim`.
#[trace("TC-621")]
#[test]
fn a_checked_claim_refuses_emission() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile ev = \"quire.temporal.event-position.false-extension/v1\";\n",
    );
    let unit = format!(
        "{unit}probabilistic Stable using ev on Config over (c: Config::ConfigVersion) \
         under Steady on origin {{ probability >= 0.5 [ holds(true) ] }}\n\
         workload Steady on Config {{ weight ConfigVersion::attemptUpdate = 1; }}\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    assert_eq!(graph.probabilistic_model().claims().len(), 1);
    let refusal = emit_checked(&CheckedPackage::link(graph)).expect_err("emission refuses");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            declaration: "Stable".to_owned(),
            feature: qsl_semantics::check::ProbabilisticFeature::Claim,
            profile: Some("quire.model.complete/v1"),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}
