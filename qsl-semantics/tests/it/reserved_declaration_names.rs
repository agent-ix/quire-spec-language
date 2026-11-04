// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035-AC-7 through S1, S2 and S3: a function named by a reserved
//! spelling refuses at its name before any call is resolved, and a function
//! named by a longer identifier starting with one is declared and called.

use ix_trace_rs::trace;
use qsl_cst::{CompleteCode, ParsedSource};
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::intake::SelectedModels;
use quire_semantic_value::checking::CheckingLimits;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

/// A function `name` and a function `answer` that calls it.
fn called(name: &str) -> ParsedSource {
    let text = format!(
        "{HEADER}function {name} using v(x: Int[0, 9]): Int[0, 10] pure {{ x + 1 }}\n\
         function answer using v(x: Int[0, 9]): Int[0, 10] pure {{ {name}(x) }}\n"
    );
    qsl_cst::parse(
        SourceIdentity::new("test", "reserved-declaration-names"),
        "reserved.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_function_named_by_a_reserved_spelling_refuses_at_s1() {
    for name in ["helper", "rec", "cast", "Tuple"] {
        let parsed = called(name);
        assert!(!parsed.is_admissible(), "{name}");
        let [refusal] = parsed.diagnostics() else {
            panic!("{name}: one refusal: {:?}", parsed.diagnostics());
        };
        assert_eq!(refusal.code, CompleteCode::InvalidSyntax, "{name}");
        let text = parsed.source().text();
        let start = text.find(&format!("function {name} ")).expect("the name") + "function ".len();
        assert_eq!(
            refusal.byte_span().map(|span| (span.start, span.end)),
            Some((start, start + name.len())),
            "{name}"
        );
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_function_named_by_a_near_spelling_is_declared_and_called() {
    for name in ["helperz", "recount", "caster", "TupleX"] {
        let parsed = called(name);
        assert!(parsed.is_admissible(), "{name}: {:?}", parsed.diagnostics());
        let unit = build_unit(&parsed).unwrap_or_else(|refusal| panic!("{name}: {refusal:?}"));
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            crate::TEST_PACKAGE_NAME.to_owned(),
            unit,
            SelectedModels::default(),
            Vec::new(),
        )
        .unwrap_or_else(|refusal| panic!("{name}: assembly: {refusal:?}"))
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{name}: the call checks: {refusals:?}"));
    }
}
