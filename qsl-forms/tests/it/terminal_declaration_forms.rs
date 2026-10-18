// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-382: S2 builds the `terminal` declaration form of FR-124 -- `terminal
//! Name using p on Model when { P };` and `terminal Name using p on Model
//! any;` -- run on real complete-V1 source through `qsl_cst::parse` and
//! `build_unit` (TC-1059).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, DeclarationForm, ExprNode, FormsCause, ParsedUnit, TerminalDeclarationForm,
    TerminalDeclarationKind,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\" version \"1\" digest \
     \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

fn terminal(form: &DeclarationForm) -> &TerminalDeclarationForm {
    let DeclarationForm::Terminal(declaration) = form else {
        panic!("a terminal declaration: {form:?}");
    };
    declaration
}

fn sole_terminal(unit: &ParsedUnit) -> &TerminalDeclarationForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    terminal(form.form())
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// FR-382-AC-1 (TC-1059 step 1): `terminal Done using t on Config when { not
/// false };` builds the written name, `using` alias, model alias and
/// predicate, each with a span that slices to its text.
#[trace("TC-1059", "FR-382-AC-1")]
#[test]
fn terminal_when_builds_its_name_aliases_and_predicate() {
    let declaration = "terminal Done using t on Config when { not false };";
    let (text, unit) = build(declaration);
    let form = sole_terminal(&unit);

    assert_eq!(form.name.name, "Done");
    assert_eq!(slice(&text, form.name.span), "Done");
    assert_eq!(form.profile.alias, "t");
    assert_eq!(slice(&text, form.profile.span), "t");
    assert_eq!(form.model.name, "Config");
    assert_eq!(slice(&text, form.model.span), "Config");
    assert_eq!(slice(&text, form.span), declaration);

    let TerminalDeclarationKind::When(predicate) = &form.kind else {
        panic!("a `when` declaration: {:?}", form.kind);
    };
    let ExprNode::Not(operand) = predicate.expression.root_node() else {
        panic!("`not false`: {:?}", predicate.expression.root_node());
    };
    assert!(matches!(
        predicate.expression.at(*operand).node(),
        ExprNode::Boolean(false)
    ));
    assert_eq!(
        predicate.spans.at(&[]).map(|span| slice(&text, span)),
        Some("not false")
    );
}

/// FR-382-AC-2 (TC-1059 step 2): `terminal Halt using t on Config any;`
/// builds the `Any` kind, with the span of the whole declaration.
#[trace("TC-1059", "FR-382-AC-2")]
#[test]
fn terminal_any_builds_the_any_kind() {
    let declaration = "terminal Halt using t on Config any;";
    let (text, unit) = build(declaration);
    let form = sole_terminal(&unit);

    assert_eq!(form.name.name, "Halt");
    assert_eq!(form.model.name, "Config");
    assert_eq!(form.kind, TerminalDeclarationKind::Any);
    assert_eq!(slice(&text, form.span), declaration);
}

/// FR-382-AC-3 (TC-1059 step 3): two terminal declarations over one model
/// alias, with a `temporal` clause between them, build three forms in source
/// order; S2 merges and refuses nothing.
#[trace("TC-1059", "FR-382-AC-3")]
#[test]
fn repeated_terminal_declarations_build_in_source_order_beside_other_forms() {
    let (_, unit) = build(
        "terminal First using t on Config any;\n\
         temporal Watch using t over (p: Config::ConfigVersion) clock \"steps\" on origin \
         { holds(true) }\n\
         terminal Second using t on Config when { true };",
    );
    let kinds: Vec<&str> = unit
        .forms()
        .iter()
        .map(|form| match form.form() {
            DeclarationForm::Terminal(_) => "terminal",
            DeclarationForm::Temporal(_) => "temporal",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["terminal", "temporal", "terminal"]);

    let first = terminal(unit.forms()[0].form());
    let second = terminal(unit.forms()[2].form());
    assert_eq!(first.name.name, "First");
    assert_eq!(first.kind, TerminalDeclarationKind::Any);
    assert_eq!(second.name.name, "Second");
    assert!(matches!(second.kind, TerminalDeclarationKind::When(_)));
    assert_eq!(first.model.name, second.model.name);
}

/// FR-382-AC-4 (TC-1059 step 4): a `terminal` declaration whose CST holds a
/// recovery node -- no closing `;`, or a `when` with no block -- refuses
/// `FormsCause::RecoveringCst`, with no partial form.
#[trace("TC-1059", "FR-382-AC-4")]
#[test]
fn a_recovering_terminal_declaration_builds_no_form() {
    for declaration in [
        "terminal Done using t on Config when { true }",
        "terminal Done using t on Config any",
        "terminal Done using t on Config when true;",
    ] {
        let text = format!("{HEADER}{declaration}\n");
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{declaration}");
        assert_eq!(
            build_unit(&parsed).unwrap_err().cause,
            FormsCause::RecoveringCst,
            "{declaration}"
        );
    }
}

/// FR-382-AC-5 (TC-1059 step 5): building the same unit twice gives equal
/// terminal declaration forms.
#[trace("TC-1059", "FR-382-AC-5")]
#[test]
fn building_a_terminal_declaration_twice_gives_equal_forms() {
    for declaration in [
        "terminal Done using t on Config when { not false };",
        "terminal Halt using t on Config any;",
    ] {
        let (_, first) = build(declaration);
        let (_, second) = build(declaration);
        assert_eq!(
            sole_terminal(&first),
            sole_terminal(&second),
            "{declaration}"
        );
    }
}
