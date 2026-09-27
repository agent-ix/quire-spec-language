// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-102 (QSL-273): the `ProtocolClause` family's S2 production —
//! `invariant`/`pre`/`post` state clauses and the `self`, `result` and
//! `reaches` expressions — run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit` (TC-456, TC-457).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, DeclarationForm, Expression, ExpressionSpans, FormsCause, FormsFailure,
    FormsLimits, ParsedUnit, SpanId, StateClauseForm, StateClauseKind,
};
use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::{SourceIdentity, Span};

/// Every unit under test starts with this header (TC-456's own text): one
/// profile and one model selection, neither of which S2 resolves.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
    model Config = \"example/config-version\" version \"1\" digest \
    \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

fn parse(declarations: &str) -> (String, qsl_cst::ParsedSource) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    (text, parsed)
}

fn admissible(declarations: &str) -> (String, qsl_cst::ParsedSource) {
    let (text, parsed) = parse(declarations);
    assert!(
        parsed.is_admissible(),
        "{declarations}: {:?}",
        parsed.diagnostics()
    );
    (text, parsed)
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let (text, parsed) = admissible(declarations);
    let unit = build_unit(&parsed, FormsLimits::default())
        .unwrap_or_else(|failure| panic!("{declarations}: {failure:?}"));
    (text, unit)
}

fn refusal(declarations: &str) -> (String, FormsCause, Option<Span>) {
    let (text, parsed) = admissible(declarations);
    match build_unit(&parsed, FormsLimits::default()) {
        Err(FormsFailure::Refused(refusal)) => (text, refusal.cause, refusal.span),
        other => panic!("{declarations}: a refusal, not {other:?}"),
    }
}

/// SR-722 FND-007 (TC-456 step 1): every node of an expression tree's own
/// spans, not just the root and one child, slices the source to text that
/// parses back as one whole expression on its own -- built the same way
/// `build`'s own caller builds a state clause body, so this needs no
/// separate parse harness and no `show()`-style rendering that would have
/// to recognize every production this fixture's body uses (`present`,
/// `deref`, `value`, `implies`, `<`, none of which `show` itself matches
/// explicitly). `spans.fits(expr)` is a build-time invariant this crate
/// already checks (`DeclarationSpans::fit`), so walking `expr.children()`
/// and `spans.child(id, index)` in lockstep never runs out of either side.
fn assert_every_span_slice_reparses(
    text: &str,
    spans: &ExpressionSpans,
    id: SpanId,
    expr: &Expression,
) {
    let span = spans
        .span(id)
        .unwrap_or_else(|| panic!("{expr:?}: every visited span node has a span"));
    let slice = &text[span.start..span.end];
    let (reparsed_text, reparsed_unit) = build(&format!(
        "invariant Z using v on Config::ConfigVersion at current {{ {slice} }}"
    ));
    let reparsed_form = state_clause(reparsed_unit.forms()[0].form());
    let reparsed_spans = &reparsed_form.spans.body;
    let root_span = reparsed_spans
        .span(reparsed_spans.root())
        .expect("the reparsed body has a root span");
    assert_eq!(
        &reparsed_text[root_span.start..root_span.end],
        slice,
        "{slice:?} does not parse back as one whole expression on its own"
    );
    for (index, child_expr) in expr.children().into_iter().enumerate() {
        let child_id = spans
            .child(id, index)
            .unwrap_or_else(|| panic!("{expr:?}: child {index} has no span (spans.fits failed)"));
        assert_every_span_slice_reparses(text, spans, child_id, child_expr);
    }
}

fn state_clause(form: &DeclarationForm) -> &StateClauseForm {
    match form {
        DeclarationForm::StateClause(state_clause) => state_clause,
        DeclarationForm::Function(_)
        | DeclarationForm::Alias(_)
        | DeclarationForm::Record(_)
        | DeclarationForm::Tuple(_)
        | DeclarationForm::Enum(_)
        | DeclarationForm::Dimension(_)
        | DeclarationForm::Unit(_) => panic!("a state clause form, not {form:?}"),
    }
}

/// A compact rendering of an expression tree, operands in order (mirrors
/// `value_forms::show`, extended with the three forms this ticket adds).
fn show(expression: &Expression) -> String {
    let children: Vec<String> = expression.children().into_iter().map(show).collect();
    let head = match expression {
        Expression::Boolean(value) => format!("{value}"),
        Expression::Integer(value) => format!("{value}"),
        Expression::Name(spelling) => spelling.clone(),
        Expression::Binary { operator, .. } => format!("{operator:?}"),
        Expression::Not(_) => "Not".into(),
        Expression::Field { field, .. } => format!("Field[{field}]"),
        Expression::Pre(_) => "Pre".into(),
        Expression::SelfRef => "SelfRef".into(),
        Expression::Result => "Result".into(),
        Expression::Reaches { edge, .. } => format!("Reaches[{edge}]"),
        other => panic!("this fixture's bodies use no other form: {other:?}"),
    };
    if children.is_empty() {
        head
    } else {
        format!("{head}({})", children.join(", "))
    }
}

#[trace("TC-456", "FR-102-AC-1")]
#[test]
fn an_invariant_builds_one_state_clause_form_with_no_operation() {
    let (text, unit) = build(
        "invariant ParentOrder using v on Config::ConfigVersion at current \
         { present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Invariant);
    assert_eq!(form.name.name, "ParentOrder");
    assert_eq!(form.profile.alias, "v");
    assert_eq!(form.context.name, "Config::ConfigVersion");
    assert!(form.operation.is_none());

    // Every sub-expression's span slices the source to its own text.
    let spans = &form.spans.body;
    let whole_body =
        "present(self.parent) implies deref(value(self.parent)).versionNumber < self.versionNumber";
    let root = spans.span(spans.root()).expect("the root span");
    assert_eq!(&text[root.start..root.end], whole_body);
    let present_call_span = {
        let start = text
            .find("present(self.parent)")
            .expect("present() is in the unit");
        Span {
            start,
            end: start + "present(self.parent)".len(),
        }
    };
    let left = spans
        .span(spans.child(spans.root(), 0).unwrap())
        .expect("the implies' left operand span");
    assert_eq!(left, present_call_span);
    assert_eq!(&text[left.start..left.end], "present(self.parent)");

    // Every node, not just the root and its left child (SR-722 FND-007).
    assert_every_span_slice_reparses(&text, spans, spans.root(), &form.body);
}

#[trace("TC-456", "FR-102-AC-2")]
#[test]
fn pre_and_post_build_their_own_kind_with_the_operation_member() {
    let (_, unit) = build("pre P using v on Config::ConfigVersion::attemptUpdate { true }");
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Precondition);
    assert_eq!(form.context.name, "Config::ConfigVersion");
    assert_eq!(
        form.operation.as_ref().map(|o| o.name.as_str()),
        Some("attemptUpdate")
    );

    let (_, unit) = build("post P using v on Config::ConfigVersion::attemptUpdate { true }");
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(form.kind, StateClauseKind::Postcondition);
    assert_eq!(
        form.operation.as_ref().map(|o| o.name.as_str()),
        Some("attemptUpdate")
    );
}

#[trace("TC-456", "FR-102-AC-3")]
#[test]
fn self_result_and_reaches_build_and_a_qualified_edge_refuses() {
    let (_, unit) = build(
        "invariant NoCycle using v on Config::ConfigVersion at current \
         { not reaches(self, self, parent) }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(show(&form.body), "Not(Reaches[parent](SelfRef, SelfRef))");

    let (_, unit) = build(
        "post VersionUnchanged using v on Config::ConfigVersion::attemptUpdate \
         { self.versionNumber = pre(self.versionNumber) }",
    );
    let form = state_clause(unit.forms()[0].form());
    assert_eq!(
        show(&form.body),
        "Equal(Field[versionNumber](SelfRef), Pre(Field[versionNumber](SelfRef)))"
    );

    let (_, unit) = build("post R using v on Config::ConfigVersion::attemptUpdate { result }");
    let form = state_clause(unit.forms()[0].form());
    assert!(matches!(form.body, Expression::Result));

    let (text, cause, span) = refusal(
        "invariant Q using v on Config::ConfigVersion at current \
         { reaches(self, self, Config::ConfigVersion::parent) }",
    );
    assert_eq!(
        cause,
        FormsCause::UnrepresentedConstruct {
            production: qsl_cst::Production::QualifiedName
        }
    );
    let needle = "Config::ConfigVersion::parent";
    let start = text.rfind(needle).expect("the edge is in the unit");
    assert_eq!(
        span,
        Some(Span {
            start,
            end: start + needle.len()
        })
    );
}

#[trace("TC-457", "FR-102-AC-5")]
#[test]
fn a_state_clause_body_obeys_the_same_nesting_depth_limit_as_a_function_body() {
    // A bare `(e)` is a transparent pass-through (`value`'s own doc: "a
    // single-operand precedence level maps to its operand"), so it adds no
    // arena node and no depth; `not` is the nesting device `value_forms`'s
    // own FR-091 depth tests use for exactly this reason.
    let limits = FormsLimits { nesting_depth: 4 };
    let nots = |count: u64| "not ".repeat(count as usize) + "true";
    let source = |count: u64| {
        format!(
            "invariant N using v on Config::ConfigVersion at current {{ {} }}",
            nots(count)
        )
    };

    // `nesting_depth` counts the root at depth 1, so `limit - 1` `not`s
    // reach exactly the bound (the leaf `true` at depth `limit`).
    let (_, parsed) = admissible(&source(limits.nesting_depth - 1));
    build_unit(&parsed, limits).expect("exactly at the limit builds");

    let (text, parsed) = admissible(&source(limits.nesting_depth));
    match build_unit(&parsed, limits) {
        Err(FormsFailure::Limit { limit, span }) => {
            assert_eq!(limit.kind(), LimitKind::NestingDepth);
            let expected_start = text.find("true").expect("the leaf is in the unit");
            assert_eq!(span.start, expected_start);
        }
        other => panic!("a limit refusal, not {other:?}"),
    }
}
