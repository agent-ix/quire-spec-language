// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-103: a domain-package operation's clause text is parsed by the same
//! production as a unit `pre`/`post` clause body.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, parse_clause_text, ClauseTextRefusal, DeclarationForm, Expression};
use qsl_foundation::SourceIdentity;

fn unit_clause_body(body: &str) -> Expression {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\npre Own using v on M::T::op {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let (_, forms) = build_unit(&parsed)
        .expect("S2 builds the unit")
        .into_parts();
    match forms.into_iter().next().map(|form| form.into_form()) {
        Some(DeclarationForm::StateClause(clause)) => clause.body,
        other => panic!("one state clause, got {other:?}"),
    }
}

/// The text parses to the expression a unit clause of that body has.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn clause_text_parses_as_a_unit_clause_body() {
    for text in ["not self.placed", "self.placed and not pre(self.placed)"] {
        let parsed = parse_clause_text(text, qsl_cst::Limits::default()).expect("parses");
        assert_eq!(parsed, unit_clause_body(text), "{text}");
    }
}

/// A text that does not parse refuses at S1 or S2, and a text that closes the
/// wrapping clause and declares another is not one expression.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn clause_text_that_is_not_one_expression_refuses() {
    assert!(matches!(
        parse_clause_text("self.placed and", qsl_cst::Limits::default()),
        Err(ClauseTextRefusal::Source(_) | ClauseTextRefusal::Forms(_))
    ));
    assert!(matches!(
        parse_clause_text(
            "true } pre Other using p on M::T::op { false",
            qsl_cst::Limits::default()
        ),
        Err(ClauseTextRefusal::NotOneExpression)
    ));
}
