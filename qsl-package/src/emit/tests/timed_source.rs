// SPDX-License-Identifier: AGPL-3.0-or-later
//! Interim fail-closed guard: until QSL-1032 maps checked time sources into
//! `timed_model` nodes on the plain emit path, that path refuses a timed
//! package instead of omitting its time source. This asserts the guard, not
//! TC-685 step 26 (which requires the emission QSL-1032 adds).

use super::admission_corpus::config_version_unit;
use super::*;

#[test]
fn a_checked_time_source_cannot_emit_as_an_untimed_package() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let unit = config_version_unit::unit_text(&qsl_semantics::model::key::hex(digest)).replace(
        "profile v = \"quire.model.complete/v1\";\n",
        "profile v = \"quire.model.complete/v1\";\n\
         profile t = \"quire.temporal.complete/v1\";\n",
    );
    let unit = format!(
        "{unit}dimension Time;\nunit ms : Time = rational(1, 1);\ntime Config dense unit ms;\n"
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the time source checks");
    assert_eq!(graph.time_sources().len(), 1);
    let refusal = emit_checked(&CheckedPackage::link(graph))
        .expect_err("v2 emission must not omit the checked time source");
    assert_eq!(
        refusal,
        EmitRefusal::TimedModelNotEmitted {
            model: "Config".to_owned(),
        }
    );
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}
