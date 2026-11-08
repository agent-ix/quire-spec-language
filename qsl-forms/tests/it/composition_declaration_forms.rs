// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-491 "Declaration form", QSL FR-530: S2 builds the composition
//! declaration -- `compose S rule R { component ...; members }` -- run on real
//! complete-V1 source through `qsl_cst::parse` and `build_unit` (QSpec
//! TC-985, the parse half). Names and spans stay unchecked.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, CompositionDeclarationForm, CompositionMemberForm, CompositionRuleForm,
    DeclarationForm, ParsedUnit,
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

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

fn sole_composition(unit: &ParsedUnit) -> &CompositionDeclarationForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Composition(declaration) = form.form() else {
        panic!("a composition declaration: {:?}", form.form());
    };
    declaration
}

/// A declaration builds its name, rule, components and each member in source
/// order, every name with a span that slices to its text.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-1")]
#[test]
fn a_declaration_builds_its_members() {
    let declaration = "compose Sys rule interface {
        component Producer from P using M;
        component Consumer from Q using M;
        guarantee Producer :: Sends;
        rely Consumer::Done on Producer::Counted;
        depends Consumer::Done on Producer::Sends;
        supplies fair weak each M::Producer::send;
        environment Net by \"network\" assumes Reliable;
    }";
    let (text, unit) = build(declaration);
    let form = sole_composition(&unit);
    assert_eq!(form.name.name, "Sys");
    assert_eq!(form.rule, CompositionRuleForm::Interface);
    assert_eq!(slice(&text, form.rule_span), "interface");
    assert_eq!(slice(&text, form.span), declaration);
    let [producer, consumer] = &form.components[..] else {
        panic!("two components");
    };
    assert_eq!(
        (&producer.name.name, &producer.import.name, &producer.model.name),
        (&"Producer".to_owned(), &"P".to_owned(), &"M".to_owned())
    );
    assert_eq!(consumer.name.name, "Consumer");
    assert_eq!(
        slice(&text, consumer.span),
        "component Consumer from Q using M;"
    );
    let [CompositionMemberForm::Guarantee(guarantee), CompositionMemberForm::Rely(rely), CompositionMemberForm::Depends(depends), CompositionMemberForm::Supplies(supplies), CompositionMemberForm::Environment(environment)] =
        &form.members[..]
    else {
        panic!("the five member kinds in order: {:?}", form.members);
    };
    // Trivia between the tokens of `C :: G` does not move the name spans.
    assert_eq!(slice(&text, guarantee.guarantee.component.span), "Producer");
    assert_eq!(slice(&text, guarantee.guarantee.clause.span), "Sends");
    assert_eq!(rely.guarantee.component.name, "Consumer");
    assert_eq!(rely.on.clause.name, "Counted");
    assert_eq!(depends.on.clause.name, "Sends");
    assert_eq!(
        slice(&text, supplies.span),
        "supplies fair weak each M::Producer::send;"
    );
    assert_eq!(environment.name.name, "Net");
}

/// Each of the four rule spellings, the hyphenated ones included, builds its
/// rule.
#[trace("FR-530-AC-1", "QSpec-TC-985", "QSpec-FR-491-AC-5")]
#[test]
fn every_rule_spelling_builds() {
    for (spelling, rule) in [
        ("assume-guarantee", CompositionRuleForm::AssumeGuarantee),
        ("protocol-refinement", CompositionRuleForm::ProtocolRefinement),
        ("refinement-transfer", CompositionRuleForm::RefinementTransfer),
        ("interface", CompositionRuleForm::Interface),
    ] {
        let (text, unit) = build(&format!(
            "compose Sys rule {spelling} {{ component A from P using M; component B from Q using M; }}"
        ));
        let form = sole_composition(&unit);
        assert_eq!(form.rule, rule);
        assert_eq!(slice(&text, form.rule_span), spelling);
        assert!(form.members.is_empty());
    }
}
