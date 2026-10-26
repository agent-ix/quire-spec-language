// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1102: the checked-package V2 writer takes IR's source-correspondence
//! argument. A package with no refinement step row writes no
//! `refinement_step_rows` member, which is what the reader admits as absent;
//! an empty array would be a different, refused shape.

use super::admission_corpus::config_version_unit;
use super::*;

#[test]
fn a_package_without_a_refinement_writes_no_step_row_member() {
    let document = config_version_unit::DOMAIN_PACKAGE.as_bytes();
    let packages = qsl_semantics::model::intake::package_input([document]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one supplied document");
    };
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model Config = \"example/config-version\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         function noop using v(): Boolean pure {{ true }}\n",
        qsl_semantics::model::key::hex(digest)
    );
    let graph = assemble_with_models(text.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    assert!(
        wire.get("refinement_step_rows").is_none(),
        "an absent member, not an empty array: {:?}",
        wire.get("refinement_step_rows")
    );
}
