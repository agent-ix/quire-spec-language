// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-490 "Declaration form", QSL FR-529: S2 builds the guarantee
//! declaration -- `guarantee G of P using A { initial to "f"; members }` --
//! run on real complete-V1 source through `qsl_cst::parse` and `build_unit`
//! (QSpec TC-984, the parse half). Names, spans and Rust spellings stay
//! unchecked.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, DeclarationForm, FairnessGranularity, FairnessKind, GuaranteeDeclarationForm,
    GuaranteeMeasureForm, GuaranteeMemberForm, GuaranteeSuppliesForm, ParsedUnit,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"quire.value.complete/v1\";\n";

fn build(declarations: &str) -> (String, ParsedUnit) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

fn sole_guarantee(unit: &ParsedUnit) -> &GuaranteeDeclarationForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Guarantee(declaration) = form.form() else {
        panic!("a guarantee declaration: {:?}", form.form());
    };
    declaration
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// A declaration builds its three names, `initial` member and each member in
/// source order, every name and Rust spelling with a span that slices to its
/// text; an unwritten fairness kind and granularity stay `None`.
#[trace("FR-529-AC-5", "QSpec-TC-984", "QSpec-FR-490-AC-1")]
#[test]
fn a_declaration_builds_its_members() {
    let declaration = r#"guarantee G of Eventually2 using A {
        initial to "counter::Counter::new";
        progress Spec::Counter::inc decreases "counter::retries_left";
        progress Spec::Counter::dec bound 7;
        environment Sched by "scheduler" fair weak each Spec::Counter::inc;
        environment Plain by "plain" fair Spec::Counter::dec;
        environment Net by "network" assumes Reliable;
    }"#;
    let (text, unit) = build(declaration);
    let form = sole_guarantee(&unit);
    assert_eq!(slice(&text, form.name.span), "G");
    assert_eq!(slice(&text, form.clause.span), "Eventually2");
    assert_eq!(slice(&text, form.relation.span), "A");
    assert_eq!(form.initial.function.text, "counter::Counter::new");
    assert_eq!(
        slice(&text, form.initial.function.span),
        "\"counter::Counter::new\""
    );
    let [GuaranteeMemberForm::Progress(first), GuaranteeMemberForm::Progress(second), GuaranteeMemberForm::Environment(sched), GuaranteeMemberForm::Environment(plain), GuaranteeMemberForm::Environment(net)] =
        &form.members[..]
    else {
        panic!(
            "two progress and three environment members: {:?}",
            form.members
        );
    };
    assert_eq!(first.operation.name, "Spec::Counter::inc");
    assert!(
        matches!(&first.measure, GuaranteeMeasureForm::Decreases(path)
        if path.text == "counter::retries_left")
    );
    assert_eq!(second.operation.name, "Spec::Counter::dec");
    assert!(matches!(&second.measure, GuaranteeMeasureForm::Bound { text, .. } if text == "7"));
    assert_eq!(sched.name.name, "Sched");
    assert_eq!(sched.supplier.text, "scheduler");
    let GuaranteeSuppliesForm::Fairness(fair) = &sched.supplies else {
        panic!("a scheduling premise");
    };
    assert_eq!(fair.kind, Some(FairnessKind::Weak));
    assert_eq!(fair.granularity, Some(FairnessGranularity::Each));
    assert_eq!(fair.operation.name, "Spec::Counter::inc");
    let GuaranteeSuppliesForm::Fairness(unmarked) = &plain.supplies else {
        panic!("a scheduling premise");
    };
    assert_eq!((unmarked.kind, unmarked.granularity), (None, None));
    let GuaranteeSuppliesForm::Assumes(predicate) = &net.supplies else {
        panic!("an assumption premise");
    };
    assert_eq!(predicate.name, "Reliable");
    assert_eq!(slice(&text, form.span), declaration);
    assert_eq!(
        slice(&text, first.span).trim_end_matches(';'),
        "progress Spec::Counter::inc decreases \"counter::retries_left\""
    );
}
