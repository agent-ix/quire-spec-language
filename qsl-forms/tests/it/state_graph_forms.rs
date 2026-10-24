// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-165: S1 and S2 read the three state-graph claim forms of ADR-022 SG-1
//! to SG-3 on real complete-V1 source through `qsl_cst::parse` and
//! `build_unit` (TC-590 step 1 and 2's syntax half).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, DeclarationForm, ExpressionForm, FairnessKind, StateGraphClaimForm,
    StateGraphClaimKind, StateGraphPredicateForm, TemporalOperator,
};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile v = \"quire.value.complete/v1\";\n";

fn build(declaration: &str) -> (String, StateGraphClaimForm) {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::StateGraphClaim(claim) = form.form() else {
        panic!("a state-graph claim: {:?}", form.form());
    };
    (text, (**claim).clone())
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// The one operand of a predicate that has no temporal operator.
fn plain(predicate: &StateGraphPredicateForm) -> &ExpressionForm {
    assert!(
        predicate.temporal_operators.is_empty(),
        "no temporal operator: {predicate:?}"
    );
    let [operand] = predicate.operands.as_slice() else {
        panic!("one operand: {predicate:?}");
    };
    operand
}

/// The source text of an operand's root.
fn operand_text<'t>(text: &'t str, operand: &ExpressionForm) -> &'t str {
    let span = operand
        .spans
        .span(operand.spans.root())
        .expect("the root has a span");
    slice(text, span)
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `possible` builds its name, `using` alias, `over` parameter
/// and one predicate, each spanned over its source text.
#[trace("TC-590")]
#[test]
fn possible_builds_its_name_alias_over_parameter_and_predicate() {
    let (text, claim) = build(
        "possible ReachesTwo using v over (c: Config::ConfigVersion) { c.versionNumber = 2 }",
    );
    assert_eq!(slice(&text, claim.name.span), "ReachesTwo");
    assert_eq!(claim.profile.alias, "v");
    assert_eq!(slice(&text, claim.profile.span), "v");
    assert_eq!(claim.over.as_ref().expect("over written").name.name, "c");
    assert!(claim.fairness.is_empty());
    let StateGraphClaimKind::Possible { target } = &claim.kind else {
        panic!("possible: {:?}", claim.kind);
    };
    assert_eq!(operand_text(&text, plain(target)), "c.versionNumber = 2");
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `always possible` without `from` builds no `from`; with
/// `from (Q)` it builds `Q` beside the target.
#[trace("TC-590")]
#[test]
fn always_possible_builds_its_optional_from() {
    let (_, bare) =
        build("always possible CanStillWin using v over (x: G::Game) { x.phase = Won }");
    let StateGraphClaimKind::AlwaysPossible { from, target } = &bare.kind else {
        panic!("always possible: {:?}", bare.kind);
    };
    assert!(from.is_none());
    plain(target);

    let (text, guarded) = build(
        "always possible CanStillWin using v over (x: G::Game) from (x.phase != Lost) \
         { x.phase = Won }",
    );
    let StateGraphClaimKind::AlwaysPossible { from, target } = &guarded.kind else {
        panic!("always possible: {:?}", guarded.kind);
    };
    let from = from.as_ref().expect("from written");
    assert_eq!(operand_text(&text, plain(from)), "x.phase != Lost");
    assert_eq!(operand_text(&text, plain(target)), "x.phase = Won");
}

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `unique path` builds `from` and `to`.
#[trace("TC-590")]
#[test]
fn unique_path_builds_from_and_to() {
    let (text, claim) = build(
        "unique path InOneWay using v over (j: M::Job) from (not j.a and not j.b) to (j.done);",
    );
    let StateGraphClaimKind::UniquePath { from, to } = &claim.kind else {
        panic!("unique path: {:?}", claim.kind);
    };
    assert_eq!(operand_text(&text, plain(from)), "not j.a and not j.b");
    assert_eq!(operand_text(&text, plain(to)), "j.done");
}

/// FR-165-AC-2: a temporal operator written in a predicate and a fairness
/// constraint written in the braces are kept at their spans for S3 to
/// refuse.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn temporal_operators_and_fairness_are_kept_at_their_spans() {
    let (text, claim) = build(
        "possible P using v over (c: Config::ConfigVersion) { eventually c.versionNumber = 2 }",
    );
    let StateGraphClaimKind::Possible { target } = &claim.kind else {
        panic!("possible: {:?}", claim.kind);
    };
    let [operator] = target.temporal_operators.as_slice() else {
        panic!("one operator: {target:?}");
    };
    assert_eq!(operator.operator, TemporalOperator::Eventually);
    assert_eq!(slice(&text, operator.span), "eventually");
    assert_eq!(target.operands.len(), 1);

    let (text, claim) = build(
        "always possible P using v over (x: G::Game) { fair weak attemptUpdate; x.phase = Won }",
    );
    let [constraint] = claim.fairness.as_slice() else {
        panic!("one constraint: {:?}", claim.fairness);
    };
    assert_eq!(constraint.kind, Some(FairnessKind::Weak));
    assert_eq!(slice(&text, constraint.span), "fair weak attemptUpdate;");
}

/// FR-165-AC-2: a temporal operator inside a connective, a negation or
/// parentheses is kept at its span, as one leading an operand is.
#[trace("TC-590", "FR-165-AC-2")]
#[test]
fn a_temporal_operator_inside_a_formula_is_kept_at_its_span() {
    for predicate in [
        "c.n = 1 and eventually c.n = 2",
        "(eventually c.n = 2)",
        "not eventually c.n = 2",
    ] {
        let (text, claim) = build(&format!(
            "possible P using v over (c: M::Thing) {{ {predicate} }}"
        ));
        let StateGraphClaimKind::Possible { target } = &claim.kind else {
            panic!("possible: {:?}", claim.kind);
        };
        let [operator] = target.temporal_operators.as_slice() else {
            panic!("{predicate}: one operator: {target:?}");
        };
        assert_eq!(
            operator.operator,
            TemporalOperator::Eventually,
            "{predicate}"
        );
        assert_eq!(slice(&text, operator.span), "eventually", "{predicate}");
    }
}

/// FR-165-AC-4: each of the three forms parses with `over` absent, to a claim
/// with no `over` binding.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn each_form_parses_without_over() {
    let (_, possible) = build("possible Anything using v { true }");
    let (_, always) = build("always possible Still using v from (true) { true }");
    let (_, bare_always) = build("always possible Bare using v { true }");
    let (_, unique) = build("unique path One using v from (true) to (false);");
    for claim in [&possible, &always, &bare_always, &unique] {
        assert!(claim.over.is_none(), "{:?}", claim.name);
    }
    assert!(matches!(
        possible.kind,
        StateGraphClaimKind::Possible { .. }
    ));
    assert!(matches!(
        unique.kind,
        StateGraphClaimKind::UniquePath { .. }
    ));
}

/// The operator spellings S2 kept for `declaration`, in source order.
fn operators(declaration: &str) -> Vec<(TemporalOperator, String)> {
    let (text, claim) = build(declaration);
    let predicate = match &claim.kind {
        StateGraphClaimKind::Possible { target } => target,
        StateGraphClaimKind::AlwaysPossible { target, .. } => target,
        StateGraphClaimKind::UniquePath { to, .. } => to,
    };
    predicate
        .temporal_operators
        .iter()
        .map(|operator| (operator.operator, slice(&text, operator.span).to_owned()))
        .collect()
}

/// SG-R09, SG-R10, SG-R14: temporal operators in a conjunction, under a
/// negation with an interval, in parentheses, as a binary operator and with
/// an open interval all parse, each kept at its operator.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn temporal_operators_at_any_operand_position_parse() {
    for (declaration, expected) in [
        (
            "possible R using v over (c: C::Cfg) { c.n = 1 and eventually c.n = 2 }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
        (
            "possible N using v over (c: C::Cfg) { not eventually[0,5] c.n = 2 }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
        (
            "possible P using v over (c: C::Cfg) { (eventually c.n = 2) }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
        (
            "possible U using v over (c: C::Cfg) { c.a until c.b }",
            vec![(TemporalOperator::Until, "until")],
        ),
        (
            "possible W using v over (c: C::Cfg) { eventually[1,*] c.done }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
        (
            "possible B using v over (c: C::Cfg) { if c.a then eventually c.b else c.c }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
        (
            "possible L using v over (c: C::Cfg) { let n = 1 in eventually c.n = n }",
            vec![(TemporalOperator::Eventually, "eventually")],
        ),
    ] {
        let expected: Vec<_> = expected
            .into_iter()
            .map(|(operator, text)| (operator, text.to_owned()))
            .collect();
        assert_eq!(operators(declaration), expected, "{declaration}");
    }
}

/// SG-R11: a temporal operator in a `from` or `to` operand parses.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn temporal_operators_in_from_and_to_operands_parse() {
    let (text, claim) =
        build("always possible Rec using v over (c: C::Cfg) from (always c.ok) { c.done }");
    let StateGraphClaimKind::AlwaysPossible {
        from: Some(from), ..
    } = &claim.kind
    else {
        panic!("always possible with from");
    };
    assert_eq!(slice(&text, from.temporal_operators[0].span), "always");
    let (text, claim) =
        build("unique path One using v over (c: C::Cfg) from (c.a) to (eventually c.done);");
    let StateGraphClaimKind::UniquePath { to, .. } = &claim.kind else {
        panic!("unique path");
    };
    assert_eq!(slice(&text, to.temporal_operators[0].span), "eventually");
}

/// SG-R16: `not c.n = 2` reads as the expression `(not c.n) = 2`, one operand
/// and no temporal operator.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn a_predicate_that_derives_as_an_expression_is_read_as_one() {
    let (_, claim) = build("possible N using v over (c: C::Cfg) { not c.n = 2 }");
    let StateGraphClaimKind::Possible { target } = &claim.kind else {
        panic!("possible");
    };
    assert!(target.temporal_operators.is_empty());
    assert_eq!(target.operands.len(), 1);
}

/// The S1 syntax error of `declaration`: the diagnostic code and the byte
/// offset, relative to the declaration's first byte.
fn error_offset(declaration: &str) -> (String, usize) {
    let text = format!("{HEADER}{declaration}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(!parsed.is_admissible(), "{declaration} must not parse");
    let first = &parsed.diagnostics()[0];
    let start = text.find(declaration).expect("the declaration");
    let at = first.byte_span().expect("located").start - start;
    (first.code.as_str().to_owned(), at)
}

/// Asserts `declaration` is an `invalid_syntax` error at the `nth` (0-based)
/// occurrence of `token` in it.
fn refused_at(declaration: &str, token: &str, nth: usize) {
    let (code, at) = error_offset(declaration);
    let expected = declaration
        .match_indices(token)
        .nth(nth)
        .map(|(index, _)| index)
        .expect("the token occurs");
    assert_eq!(code, "invalid_syntax", "{declaration}");
    assert_eq!(
        at, expected,
        "{declaration}: the error is at `{token}` #{nth}"
    );
}

const OVER: &str = "over (c: Config::ConfigVersion)";
const STEP: &str = "fair weak Config::ConfigVersion::attemptUpdate";

/// FR-165-AC-6: a temporal operator in an `if` condition, a `let` binding, a
/// call argument, a quantifier body or a comparison operand is
/// `invalid_syntax` at the operator.
#[trace("TC-590", "FR-165-AC-6")]
#[test]
fn a_temporal_operator_outside_a_graph_position_is_refused_at_the_operator() {
    for body in [
        "if eventually c.versionNumber = 1 then c.versionNumber = 2 else c.versionNumber = 3",
        "let n = eventually c.versionNumber = 1 in n = 2",
        "exists(d in c.versionNumber: eventually d = 2)",
        "f(c.versionNumber, eventually c.versionNumber = 1)",
        "c.versionNumber = eventually c.versionNumber = 2",
    ] {
        refused_at(
            &format!("possible P using v {OVER} {{ {body} }}"),
            "eventually",
            0,
        );
    }
}

/// FR-165-AC-6: a fairness constraint after the predicate of a body is
/// refused at the `;` that follows the predicate.
#[trace("TC-590", "FR-165-AC-6")]
#[test]
fn fairness_after_the_body_predicate_is_refused_at_the_semicolon() {
    refused_at(
        &format!("possible P using v {OVER} {{ c.versionNumber = 1; {STEP} }}"),
        ";",
        0,
    );
}

/// FR-165-AC-6: in a `from` operand a fairness constraint is refused at the
/// `;` after the predicate, or at `fair` before it.
#[trace("TC-590", "FR-165-AC-6")]
#[test]
fn fairness_in_a_from_operand_is_refused_at_the_stated_token() {
    refused_at(
        &format!("always possible P using v {OVER} from (c.versionNumber = 1; {STEP}) {{ true }}"),
        ";",
        0,
    );
    refused_at(
        &format!("always possible P using v {OVER} from ({STEP}; c.versionNumber = 1) {{ true }}"),
        "fair",
        0,
    );
}

/// FR-165-AC-6: a fairness constraint on a clause with no body is refused at
/// `fair`, and so is one in a `to` operand at its `;`.
#[trace("TC-590", "FR-165-AC-6")]
#[test]
fn fairness_on_unique_path_is_refused_at_fair() {
    refused_at(
        &format!(
            "unique path P using v {OVER} from (c.versionNumber = 1) \
             to (c.versionNumber = 2); {STEP}"
        ),
        "fair",
        0,
    );
    refused_at(
        &format!(
            "unique path P using v {OVER} from (c.versionNumber = 1) \
             to (c.versionNumber = 2; {STEP});"
        ),
        ";",
        0,
    );
}

/// FR-165-AC-6: a flat chain of binary temporal operators is refused at the
/// second operator; the parenthesised chain parses and keeps the first
/// operator first.
#[trace("TC-590", "FR-165-AC-6")]
#[test]
fn a_flat_binary_temporal_chain_is_refused_at_the_second_operator() {
    refused_at(
        &format!(
            "possible P using v {OVER} {{ c.versionNumber = 1 until c.versionNumber = 2 \
             until c.versionNumber = 3 }}"
        ),
        "until",
        1,
    );
    let chain = format!(
        "possible P using v {OVER} {{ (c.versionNumber = 1 until c.versionNumber = 2) \
         until c.versionNumber = 3 }}"
    );
    let (text, claim) = build(&chain);
    let StateGraphClaimKind::Possible { target } = &claim.kind else {
        panic!("possible");
    };
    let starts: Vec<usize> = target
        .temporal_operators
        .iter()
        .map(|operator| operator.span.start)
        .collect();
    let first = text.find("until").expect("inner until");
    assert_eq!(starts, [first, text.rfind("until").expect("outer until")]);
}

/// FR-165-AC-4: operators are kept in source order, a leading unary one
/// before the binary one it leads.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn temporal_operators_are_kept_in_source_order() {
    let ops = operators(&format!(
        "possible P using v {OVER} {{ eventually c.versionNumber = 1 until c.versionNumber = 2 }}"
    ));
    let kinds: Vec<_> = ops.iter().map(|(operator, _)| *operator).collect();
    assert_eq!(
        kinds,
        [TemporalOperator::Eventually, TemporalOperator::Until]
    );
}

/// FR-165-AC-4: the recognised placements parse, with an interval or not,
/// leading fairness and a `let` body.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn recognised_placements_parse() {
    for body in [
        "not eventually[0,5] c.versionNumber = 2",
        "eventually[1,*] c.versionNumber = 2",
        "let n = c.versionNumber in eventually n = 2",
        "(c.versionNumber = 1 until c.versionNumber = 2)",
    ] {
        let (_, claim) = build(&format!("possible P using v {OVER} {{ {body} }}"));
        let StateGraphClaimKind::Possible { target } = &claim.kind else {
            panic!("possible");
        };
        assert!(!target.temporal_operators.is_empty(), "{body}");
    }
    let (_, claim) = build(&format!(
        "possible P using v {OVER} {{ {STEP}; c.versionNumber = 1 }}"
    ));
    assert_eq!(claim.fairness.len(), 1);
}

/// FR-165-AC-4: `let` names are kept for S3 to resolve.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn let_bindings_are_kept() {
    let (_, claim) = build(&format!(
        "possible P using v {OVER} {{ let n = c.versionNumber in eventually n = 2 }}"
    ));
    let StateGraphClaimKind::Possible { target } = &claim.kind else {
        panic!("possible");
    };
    assert_eq!(target.scopes.len(), target.operands.len());
    let bound: Vec<Vec<&str>> = target
        .scopes
        .iter()
        .map(|scope| scope.iter().map(|(name, _)| name.as_str()).collect())
        .collect();
    // The binding value sits outside its own `let`; the body sits inside.
    assert_eq!(bound, [Vec::<&str>::new(), vec!["n"]]);
}

/// SG-R08: `possible`, `unique` and `path` are reserved words, refused where
/// an identifier stands.
#[trace("TC-590", "FR-165-AC-4")]
#[test]
fn the_claim_words_are_reserved() {
    refused_at("possible possible using v { true }", "possible", 1);
    refused_at("predicate unique using v (): Boolean { true }", "unique", 0);
}
