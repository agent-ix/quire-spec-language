// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-143-AC-16 (TC-400): a package selecting `quire.value.complete/v1`
//! with exactly the always-selected definitions and no trigger admits a union
//! declaration, its construction and a `case` over it, and its selection
//! equals that of the same package with the union replaced by a record.

use ix_trace_rs::trace;
use qsl_package::{emit_checked, CheckedPackage};
use qsl_semantics::check::PackageDeclarations;
use quire_semantic_value::checking::CheckingLimits;
use serde_json::Value as Json;

/// A complete-value unit holding `declarations`, checked and linked.
fn package(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}")),
    )
}

/// The lock's `definition_selections` of the emitted package.
fn definition_selections(package: &CheckedPackage) -> Json {
    let emission =
        emit_checked(package, qsl_package::EmitLimits::fixture()).expect("the package emits");
    let wire: Json = serde_json::from_slice(emission.package().bytes()).expect("the wire is JSON");
    wire["lock"]["definition_selections"].clone()
}

#[trace("QSpec-TC-400", "QSpec-FR-143-AC-16")]
#[test]
fn a_union_resolves_the_selection_a_record_resolves() {
    let with_union = package(
        "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
         function area using v(s: Shape): Integer pure {\n\
           case s { Empty: 0; Circle(r): r * r * 3; Rect(w, h): w * h; }\n\
         }\n\
         function build using v(): Shape pure { Shape::Rect(2, 3) }",
    );
    let with_record = package(
        "record Shape { width: Integer; height: Integer; }\n\
         function area using v(s: Shape): Integer pure { s.width * s.height }",
    );
    assert_eq!(
        with_union.graph().header_layers(),
        with_record.graph().header_layers()
    );
    let selections = definition_selections(&with_union);
    assert!(
        selections.as_array().is_some_and(|rows| !rows.is_empty()),
        "the lock holds the selected definitions: {selections}"
    );
    assert_eq!(selections, definition_selections(&with_record));
}
