// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 builds the timed declarations, the `Clock` type and quantity
//! literals at S2 and leaves whether a unit admits them to S3. Until
//! FR-230's check reads them, S3 refuses each `unsupported_construct`/
//! `not-yet-implemented` at its own span, so none compiles unchecked. Run
//! over TC-458's `ConfigVersion` fixture package
//! (`crate::model_operations`).

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, TimedConstruct};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::json;

use crate::model_operations::{
    admit_and_assemble_with_body, attempt_update_modifies_version_number, config_unit_with_body,
    config_version_document,
};

fn document() -> Vec<u8> {
    config_version_document(
        attempt_update_modifies_version_number(),
        Vec::new(),
        Vec::new(),
        json!([]),
    )
}

/// The byte offset of `needle` in the unit `body` is written into.
fn offset(body: &str, needle: &str) -> usize {
    let (text, _) = config_unit_with_body(&document(), body);
    text.find(needle).expect("the needle is in the unit")
}

#[trace("TC-1078")]
#[test]
fn each_timed_declaration_and_the_clock_type_refuse_at_assembly() {
    for (body, construct, at) in [
        (
            "time Config dense unit ms;\n",
            TimedConstruct::TimeSource,
            "time Config",
        ),
        (
            "time invariant i when { true } { self.x <= 3 ms }\n",
            TimedConstruct::TimeInvariant,
            "time invariant",
        ),
        (
            "urgent u when { true };\n",
            TimedConstruct::Urgent,
            "urgent u",
        ),
        ("record R { x: Clock; }\n", TimedConstruct::Clock, "Clock"),
    ] {
        let refusal = admit_and_assemble_with_body(&document(), body)
            .expect_err("a timed construct is not checked yet");
        let [error] = refusal.errors.as_slice() else {
            panic!("{body}: one error: {:?}", refusal.errors);
        };
        assert_eq!(
            error.cause,
            AssemblyCause::TimedNotChecked(construct),
            "{body}"
        );
        assert_eq!(error.cause.code(), Code::UnsupportedConstruct, "{body}");
        assert_eq!(error.cause.catalog_code().cause(), "not-yet-implemented");
        assert_eq!(error.span.start, offset(body, at), "{body}");
    }
}

#[trace("TC-1078")]
#[test]
fn a_quantity_literal_refuses_at_check() {
    let body = "function f using v(): Boolean pure { 3 ms = 3 ms }\n";
    let refusals = admit_and_assemble_with_body(&document(), body)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect_err("a quantity literal is not checked yet");
    let causes: Vec<_> = refusals
        .iter()
        .map(|refusal| (refusal.cause.code(), refusal.cause.cause()))
        .collect();
    assert_eq!(
        causes,
        [(Code::UnsupportedConstruct, Some("not-yet-implemented"))]
    );
}
