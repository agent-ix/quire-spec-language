// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-483 step 5 (FR-091-AC-44): genuine complete-V1 source names a unit as
//! a type, `Quantity<m>`, and compiles through `compile_admitted`; a
//! dimension or an unknown name in that position is refused.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedRefusal, DependencyInput, ReplayLimits, SourceIdentity, StageLimits,
};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const UNITS: &str = "dimension Length;\nunit m : Length = rational(1, 1);\n";

fn admit(body: &str) -> Result<(), AdmittedRefusal> {
    compile_admitted(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        format!("{HEADER}{UNITS}{body}").as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .map(drop)
}

#[trace("FR-091-AC-44", "TC-483")]
#[test]
fn a_quantity_typed_function_compiles_from_source() {
    admit("function f using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a = b }\n")
        .expect("Quantity<m> over the declared unit m compiles");
}

#[trace("FR-091-AC-44", "TC-483")]
#[test]
fn a_dimension_or_unknown_name_is_not_a_quantity_unit() {
    for name in ["Length", "nothing"] {
        let refusal = admit(&format!(
            "function f using v(a: Quantity<{name}>): Boolean pure {{ true }}\n"
        ))
        .expect_err("only a declared unit names a quantity type");
        assert!(
            format!("{refusal:?}").contains(name),
            "{name}: the refusal names it: {refusal:?}"
        );
    }
}
