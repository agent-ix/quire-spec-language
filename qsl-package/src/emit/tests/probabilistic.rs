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

/// The checked workload and reward are recorded on the graph, and the
/// emitter refuses `unsupported_construct`/`not-yet-implemented`, naming how
/// many of each it cannot write.
#[ix_trace_rs::trace("TC-620")]
#[test]
fn a_checked_probabilistic_model_refuses_emission() {
    let refusal = emit_checked(&probabilistic_package()).expect_err("emission refuses");
    assert_eq!(
        refusal,
        EmitRefusal::ProbabilisticModelNotEmitted {
            random_parameters: 0,
            rewards: 1,
            workloads: 1,
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}
