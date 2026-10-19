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
        "unique path InOneWay using v over (j: M::Job) from (not j.a and not j.b) to (j.done)",
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

/// TC-590, the form part of FR-165 (instances and node identity are deferred): `possible`, `unique` and `path` stay usable as names.
#[trace("TC-590")]
#[test]
fn the_claim_words_stay_usable_as_names() {
    let (_, claim) = build("possible possible using v over (path: M::Job) { path.unique }");
    assert_eq!(claim.name.name, "possible");
    assert_eq!(claim.over.as_ref().expect("over written").name.name, "path");
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
    let (_, unique) = build("unique path One using v from (true) to (false)");
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
