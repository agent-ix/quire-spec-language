// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-311 and FR-392: S2 builds the `Probabilistic` family -- `random`,
//! `reward`, `workload` and the `probabilistic` claim -- on real complete-V1
//! source through `qsl_cst::parse` and `build_unit` (TC-890, TC-1077).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ActivationForm, BoundOp, ConfidenceKey, DeclarationForm, EventForm, ExactValueForm,
    ExactValueKind, ExpectedQuantity, ExprNode, ExpressionForm, FairnessGranularity, FairnessKind,
    HorizonForm, MeasureForm, ParsedUnit, ProbabilisticBody, ProbabilisticForm, RandomForm,
    RewardForm, SchedulerForm, TemporalNodeForm, TemporalOperator, WorkloadForm,
};
use qsl_foundation::{SourceIdentity, Span};

const EVENT_POSITION: &str = "quire.temporal.event-position.false-extension/v1";

/// The header selecting `profile`, with the model's `version` padded by
/// `padding` bytes so two profile identities of different length give the
/// header the same byte length.
fn header(profile: &str, padding: usize) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile ev = \"{profile}\";\n\
         model Service = \"example/service\" version \"1{}\" digest \
         \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n",
        "v".repeat(padding)
    )
}

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build_text(text: &str) -> ParsedUnit {
    let parsed = parse(text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"))
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    let text = format!("{}{declarations}\n", header(EVENT_POSITION, 0));
    let unit = build_text(&text);
    (text, unit)
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

fn random(form: &DeclarationForm) -> &RandomForm {
    let DeclarationForm::Random(random) = form else {
        panic!("a random declaration: {form:?}");
    };
    random
}

fn reward(form: &DeclarationForm) -> &RewardForm {
    let DeclarationForm::Reward(reward) = form else {
        panic!("a reward declaration: {form:?}");
    };
    reward
}

fn workload(form: &DeclarationForm) -> &WorkloadForm {
    let DeclarationForm::Workload(workload) = form else {
        panic!("a workload declaration: {form:?}");
    };
    workload
}

fn claim(unit: &ParsedUnit) -> &ProbabilisticForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Probabilistic(claim) = form.form() else {
        panic!("a probabilistic claim: {:?}", form.form());
    };
    claim
}

/// The kind and the two integers an exact value was written as.
fn exact(value: &ExactValueForm) -> (ExactValueKind, String, String) {
    (
        value.kind,
        value.first.to_string(),
        value.second.to_string(),
    )
}

fn natural(value: u64) -> (ExactValueKind, String, String) {
    (ExactValueKind::Natural, value.to_string(), "1".to_owned())
}

fn decimal_literal(digits: &str, scale: usize) -> (ExactValueKind, String, String) {
    (
        ExactValueKind::DecimalLiteral,
        digits.to_owned(),
        scale.to_string(),
    )
}

/// `(magnitude, unit)` of a quantity-literal expression.
fn quantity(expression: &ExpressionForm) -> (String, String) {
    let ExprNode::Quantity(quantity) = expression.expression.root_node() else {
        panic!("a quantity: {:?}", expression.expression.root_node());
    };
    (
        quantity.magnitude.first.to_string(),
        quantity.unit.name.clone(),
    )
}

/// The source text an expression was built from.
fn source_of<'t>(text: &'t str, expression: &ExpressionForm) -> &'t str {
    let span = expression.spans.at(&[]).expect("the root has a span");
    slice(text, span)
}

fn name(expression: &ExpressionForm) -> &str {
    let ExprNode::Name(name) = expression.expression.root_node() else {
        panic!("a name: {:?}", expression.expression.root_node());
    };
    name
}

const SERVICE: &str = "\
random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms: 3 };\n\
random Server::attempt(outcome) ~ { Ok: 98, Fail: 2 };\n\
reward duration on Server::attempt = d;\n\
workload Steady on Service { weight Server::request = 1; weight Server::attempt = 1; \
weight Server::reset = 1; }";

/// FR-311-AC-1 (TC-890 step 1): ADR-024 §7.2's `Service` builds two
/// `RandomForm`s on `Server::attempt` with their pairs in declared order, one
/// `RewardForm` and one `WorkloadForm` with three weights in declared order,
/// each with the span of its source text.
#[trace("TC-890", "FR-311-AC-1")]
#[test]
fn the_service_model_builds_its_random_reward_and_workload_forms() {
    let (text, unit) = build(SERVICE);
    let [delay, outcome, duration, steady] = unit.forms() else {
        panic!("four declarations: {:?}", unit.forms());
    };

    let delay_form = random(delay.form());
    assert_eq!(delay_form.operation.name, "Server::attempt");
    assert_eq!(delay_form.parameter.name, "d");
    assert_eq!(slice(&text, delay_form.parameter.span), "d");
    let pairs: Vec<_> = delay_form
        .support
        .iter()
        .map(|pair| (quantity(&pair.value), exact(&pair.weight)))
        .collect();
    assert_eq!(
        pairs,
        [
            (("1".into(), "ms".into()), natural(90)),
            (("3".into(), "ms".into()), natural(7)),
            (("12".into(), "ms".into()), natural(3)),
        ]
    );
    assert_eq!(
        slice(&text, delay_form.span),
        "random Server::attempt(d) ~ { 1 ms: 90, 3 ms: 7, 12 ms: 3 };"
    );
    assert_eq!(slice(&text, delay_form.support[1].span), "3 ms: 7");

    let outcome_form = random(outcome.form());
    assert_eq!(outcome_form.parameter.name, "outcome");
    let pairs: Vec<_> = outcome_form
        .support
        .iter()
        .map(|pair| (name(&pair.value).to_owned(), exact(&pair.weight)))
        .collect();
    assert_eq!(
        pairs,
        [
            ("Ok".to_owned(), natural(98)),
            ("Fail".to_owned(), natural(2))
        ]
    );

    let reward_form = reward(duration.form());
    assert_eq!(reward_form.name.name, "duration");
    assert_eq!(reward_form.operation.name, "Server::attempt");
    assert_eq!(name(&reward_form.expression), "d");
    assert_eq!(
        slice(&text, reward_form.span),
        "reward duration on Server::attempt = d;"
    );

    let workload_form = workload(steady.form());
    assert_eq!(workload_form.name.name, "Steady");
    assert_eq!(workload_form.model.name, "Service");
    let weights: Vec<_> = workload_form
        .weights
        .iter()
        .map(|weight| (weight.operation.name.as_str(), exact(&weight.weight)))
        .collect();
    assert_eq!(
        weights,
        [
            ("Server::request", natural(1)),
            ("Server::attempt", natural(1)),
            ("Server::reset", natural(1)),
        ]
    );
    assert_eq!(
        slice(&text, workload_form.weights[2].span),
        "weight Server::reset = 1;"
    );
    assert!(slice(&text, workload_form.span).starts_with("workload Steady on Service {"));
}

/// FR-311-AC-2 (TC-890 step 2): a unit whose declaration begins `random`,
/// `reward` or `workload` builds exactly the form of that word and no other.
#[trace("TC-890", "FR-311-AC-2")]
#[test]
fn each_leading_word_builds_only_its_own_form() {
    let (_, unit) = build("random Server::attempt(d) ~ { 1 ms: 1 };");
    assert!(matches!(unit.forms(), [form] if matches!(form.form(), DeclarationForm::Random(_))));
    let (_, unit) = build("reward cost on Server::request = 2;");
    assert!(matches!(unit.forms(), [form] if matches!(form.form(), DeclarationForm::Reward(_))));
    let (_, unit) = build("workload Steady on Service { weight Server::request = 1; }");
    assert!(matches!(unit.forms(), [form] if matches!(form.form(), DeclarationForm::Workload(_))));
}

/// TC-890 step 3, the S2 half of FR-311-AC-3: a zero weight and a workload
/// that omit nothing S2 checks both build. That S3 refuses them is asserted in
/// `qsl-semantics`, for the zero weight only, so this test tags no criterion.
#[trace("TC-890")]
#[test]
fn a_zero_weight_and_an_incomplete_workload_build_at_s2() {
    let (_, unit) = build("random Server::attempt(d) ~ { 1 ms: 0 };");
    let form = random(unit.forms()[0].form());
    assert_eq!(exact(&form.support[0].weight), natural(0));

    let (_, unit) = build(
        "workload Steady on Service { weight Server::request = 1; weight Server::attempt = 1; }",
    );
    assert_eq!(workload(unit.forms()[0].form()).weights.len(), 2);
}

fn claim_source(scheduler: &str, confidence: &str, body: &str) -> String {
    format!("probabilistic C using ev on Service {scheduler} {confidence} {{ {body} }}")
}

fn confidence_of(
    form: &ProbabilisticForm,
) -> Vec<(ConfidenceKey, (ExactValueKind, String, String))> {
    form.confidence
        .iter()
        .map(|item| (item.key, exact(&item.value)))
        .collect()
}

/// The scheduler `Steady`, activation `on origin` and confidence items
/// `alpha`, `beta`, `indifference` (in that order) a claim was written with.
fn assert_steady_origin(text: &str, form: &ProbabilisticForm, iota: (&str, usize)) {
    let SchedulerForm::Workload(workload) = &form.scheduler else {
        panic!("a workload scheduler: {:?}", form.scheduler);
    };
    assert_eq!(workload.name, "Steady");
    assert_eq!(slice(text, workload.span), "Steady");
    assert_eq!(form.activation, ActivationForm::Origin);
    assert_eq!(
        confidence_of(form),
        [
            (ConfidenceKey::Alpha, decimal_literal("1", 2)),
            (ConfidenceKey::Beta, decimal_literal("1", 2)),
            (ConfidenceKey::Indifference, decimal_literal(iota.0, iota.1)),
        ]
    );
}

/// FR-392-AC-1 (TC-1077 step 1): ADR-024 §7's `NoFault`, `P95`, `LongRun` and
/// `Monthly` build a `ProbabilisticForm` each, with `Workload(Steady)`, their
/// confidence items in declared order and the span of their source text. The
/// `Monthly` threshold is spelled `rational(999, 1000)`: a decimal literal is
/// an `exact-value` only, and the event's threshold is an `expr`.
#[trace("TC-1077", "FR-392-AC-1")]
#[test]
fn the_adr_024_claims_build_their_probabilistic_forms() {
    let no_fault = "probabilistic NoFault using ev on Health under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.0005 \
         { probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }";
    let (text, unit) = build(no_fault);
    let form = claim(&unit);
    assert_eq!(form.name.name, "NoFault");
    assert_eq!(form.profile.alias, "ev");
    assert_eq!(form.subject.name, "Health");
    assert_eq!(slice(&text, form.span), no_fault);
    let SchedulerForm::Workload(workload) = &form.scheduler else {
        panic!("a workload scheduler: {:?}", form.scheduler);
    };
    assert_eq!(workload.name, "Steady");
    assert_eq!(slice(&text, workload.span), "Steady");
    assert_eq!(form.activation, ActivationForm::Origin);
    assert_eq!(
        confidence_of(form),
        [
            (ConfidenceKey::Alpha, decimal_literal("1", 2)),
            (ConfidenceKey::Beta, decimal_literal("1", 2)),
            (ConfidenceKey::Indifference, decimal_literal("5", 4)),
        ]
    );
    let ProbabilisticBody::Probability {
        op,
        threshold,
        event: EventForm::Temporal(formula),
    } = &form.body
    else {
        panic!("a probability over a temporal event: {:?}", form.body);
    };
    assert_eq!(*op, BoundOp::AtLeast);
    assert_eq!(exact(threshold), decimal_literal("999", 3));
    let Some(TemporalNodeForm::Operator(always)) = formula.node(formula.root()) else {
        panic!("an operator root: {:?}", formula.node(formula.root()));
    };
    assert_eq!(always.operator, TemporalOperator::Always);
    assert_eq!(slice(&text, always.span), "always[0,1000] holds(n.healthy)");

    let p95 = "probabilistic P95 using ev on Service under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.01 { quantile 0.95 of (accumulate duration \
         from holds(s.phase = Busy) until holds(s.phase = Done) within 10) <= 5 ms }";
    let (text, unit) = build(p95);
    let form = claim(&unit);
    assert_eq!(slice(&text, form.span), p95);
    assert_eq!(form.name.name, "P95");
    assert_steady_origin(&text, form, ("1", 2));
    let ProbabilisticBody::Quantile {
        level,
        measure,
        op,
        threshold,
    } = &form.body
    else {
        panic!("a quantile: {:?}", form.body);
    };
    assert_eq!(exact(level), decimal_literal("95", 2));
    assert_eq!(*op, BoundOp::AtMost);
    assert_eq!(quantity(threshold), ("5".into(), "ms".into()));
    let MeasureForm::Accumulate {
        reward,
        window,
        span,
    } = measure
    else {
        panic!("an accumulate measure: {measure:?}");
    };
    assert_eq!(reward.name, "duration");
    assert_eq!(
        slice(&text, *span),
        "accumulate duration from holds(s.phase = Busy) until holds(s.phase = Done) within 10"
    );
    assert_eq!(source_of(&text, &window.from), "s.phase = Busy");
    assert_eq!(source_of(&text, &window.until), "s.phase = Done");
    assert!(matches!(&window.within, HorizonForm::Steps(steps) if steps.value == 10));
    assert_eq!(confidence_of(form).len(), 3);

    let long_run = "probabilistic LongRun using ev on Avail under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.0002 \
         { long_run_fraction holds(v.up) >= 0.999 }";
    let (text, unit) = build(long_run);
    let form = claim(&unit);
    assert_eq!(slice(&text, form.span), long_run);
    assert_eq!(form.name.name, "LongRun");
    assert_steady_origin(&text, form, ("2", 4));
    let ProbabilisticBody::LongRunFraction {
        predicate,
        weighting,
        op,
        threshold,
    } = &form.body
    else {
        panic!("a long-run fraction: {:?}", form.body);
    };
    assert_eq!(source_of(&text, predicate), "v.up");
    assert!(weighting.is_none());
    assert_eq!(*op, BoundOp::AtLeast);
    assert_eq!(exact(threshold), decimal_literal("999", 3));

    let monthly = "probabilistic Monthly using ev on Avail under Steady on origin \
         with alpha 0.01, beta 0.01, indifference 0.005 \
         { probability >= 0.99 [ fraction holds(v.up) over [0, 10000] >= rational(999, 1000) ] }";
    let (text, unit) = build(monthly);
    let form = claim(&unit);
    assert_eq!(slice(&text, form.span), monthly);
    assert_eq!(form.name.name, "Monthly");
    assert_steady_origin(&text, form, ("5", 3));
    let ProbabilisticBody::Probability {
        event:
            EventForm::Compare {
                measure,
                op,
                threshold,
            },
        ..
    } = &form.body
    else {
        panic!("a probability over a compared measure: {:?}", form.body);
    };
    let MeasureForm::Fraction(fraction) = measure.as_ref() else {
        panic!("a fraction measure: {measure:?}");
    };
    assert_eq!(*op, BoundOp::AtLeast);
    assert!(matches!(&fraction.horizon, HorizonForm::Steps(steps) if steps.value == 10000));
    assert!(matches!(
        threshold.expression.root_node(),
        ExprNode::Rational(..)
    ));
    assert_eq!(
        slice(&text, fraction.span),
        "fraction holds(v.up) over [0, 10000]"
    );
}

/// FR-392-AC-2 (TC-1077 step 2): a unit whose declaration begins
/// `probabilistic` builds the claim form and no other, and the profile,
/// subject and workload names stay as written, resolved to nothing. The
/// dispatch arm's one call is checked in `dispatch`'s own tests.
#[trace("TC-1077", "FR-392-AC-2")]
#[test]
fn a_probabilistic_declaration_builds_one_claim_with_its_names_unresolved() {
    let (text, unit) = build(
        "probabilistic C using nowhere on Nothing under Absent on origin \
         { probability >= 0.5 [ holds(true) ] }",
    );
    let form = claim(&unit);
    assert_eq!(form.profile.alias, "nowhere");
    assert_eq!(slice(&text, form.profile.span), "nowhere");
    assert_eq!(form.subject.name, "Nothing");
    assert_eq!(slice(&text, form.subject.span), "Nothing");
    let SchedulerForm::Workload(workload) = &form.scheduler else {
        panic!("a workload scheduler: {:?}", form.scheduler);
    };
    assert_eq!(workload.name, "Absent");
    assert_eq!(slice(&text, workload.span), "Absent");
    assert!(form.confidence.is_empty());
}

/// The one claim `body` builds, under `Steady` on origin with no confidence.
fn body_of(body: &str) -> (String, ProbabilisticBody) {
    let (text, unit) = build(&claim_source("under Steady on origin", "", body));
    let form = claim(&unit).clone();
    (text, form.body)
}

/// FR-392-AC-3 (TC-1077 step 3): `mean of` with its weighting, both
/// `expected` quantities, and the `steps` and `elapsed` measures with their
/// `Steps` and `Quantity` horizons each build.
#[trace("TC-1077", "FR-392-AC-3")]
#[test]
fn mean_expected_steps_and_elapsed_build_their_bodies() {
    let (text, body) = body_of(
        "mean of (fraction holds(s.phase = Up) over [0, 100] weighted by duration) >= 0.99",
    );
    let ProbabilisticBody::MeanOfFraction {
        fraction,
        op,
        threshold,
    } = body
    else {
        panic!("a mean of a fraction: {body:?}");
    };
    assert_eq!(op, BoundOp::AtLeast);
    assert_eq!(exact(&threshold), decimal_literal("99", 2));
    assert!(matches!(&fraction.horizon, HorizonForm::Steps(steps) if steps.value == 100));
    let weighting = fraction.weighting.expect("a weighting");
    assert_eq!(weighting.name.name, "duration");
    assert_eq!(slice(&text, weighting.span), "weighted by duration");

    let (text, body) = body_of("expected accumulate duration until holds(s.phase = Done) <= 5 ms");
    let ProbabilisticBody::ExpectedUntil {
        quantity: ExpectedQuantity::Accumulate(reward),
        target,
        op,
        threshold,
    } = body
    else {
        panic!("an expected accumulate: {body:?}");
    };
    assert_eq!(reward.name, "duration");
    assert_eq!(source_of(&text, &target), "s.phase = Done");
    assert_eq!(op, BoundOp::AtMost);
    assert_eq!(quantity(&threshold), ("5".into(), "ms".into()));

    let (_, body) = body_of("expected elapsed until holds(s.phase = Done) <= 5 ms");
    assert!(matches!(
        body,
        ProbabilisticBody::ExpectedUntil {
            quantity: ExpectedQuantity::Elapsed,
            ..
        }
    ));

    let (_, body) = body_of("quantile 0.5 of (steps from holds(A) until holds(B) within 10) <= 5");
    let ProbabilisticBody::Quantile {
        measure: MeasureForm::Steps { window, .. },
        threshold,
        ..
    } = body
    else {
        panic!("a quantile over steps: {body:?}");
    };
    assert!(matches!(window.within, HorizonForm::Steps(steps) if steps.value == 10));
    assert!(matches!(
        threshold.expression.root_node(),
        ExprNode::Integer(_)
    ));

    let (_, body) =
        body_of("quantile 0.5 of (elapsed from holds(A) until holds(B) within 10 ms) <= 5 ms");
    let ProbabilisticBody::Quantile {
        measure: MeasureForm::Elapsed { window, .. },
        ..
    } = body
    else {
        panic!("a quantile over elapsed: {body:?}");
    };
    let HorizonForm::Quantity(horizon) = window.within else {
        panic!("a quantity horizon: {:?}", window.within);
    };
    assert_eq!(horizon.unit.name, "ms");
    assert_eq!(horizon.magnitude.first.to_string(), "10");
}

/// FR-392-AC-4 (TC-1077 step 4): `over (n: Node)` with `on each` keeps the
/// parameter and the activation, and `under every scheduler` keeps two
/// fairness items with the kind and granularity each was written with.
#[trace("TC-1077", "FR-392-AC-4")]
#[test]
fn over_each_and_fairness_items_keep_what_was_written() {
    let (text, unit) = build(
        "probabilistic C using ev on Service over (n: Service::Node) \
         under every scheduler fair weak Node::tick fair strong each Node::send \
         on each (m: Service::Node) when (m.up) \
         { probability >= 0.5 [ holds(true) ] }",
    );
    let form = claim(&unit);
    let over = form.over.as_ref().expect("an over parameter");
    assert_eq!(over.name.name, "n");
    assert_eq!(slice(&text, over.name.span), "n");
    let ActivationForm::Each { parameter, when } = &form.activation else {
        panic!("an each activation: {:?}", form.activation);
    };
    assert_eq!(parameter.name.name, "m");
    assert_eq!(
        source_of(&text, when.as_ref().expect("a when condition")),
        "m.up"
    );
    let SchedulerForm::Every { fairness } = &form.scheduler else {
        panic!("an every scheduler: {:?}", form.scheduler);
    };
    let items: Vec<_> = fairness
        .iter()
        .map(|item| {
            (
                item.kind,
                item.granularity,
                item.operation.name.as_str(),
                slice(&text, item.span),
            )
        })
        .collect();
    assert_eq!(
        items,
        [
            (
                Some(FairnessKind::Weak),
                None,
                "Node::tick",
                "fair weak Node::tick"
            ),
            (
                Some(FairnessKind::Strong),
                Some(FairnessGranularity::Each),
                "Node::send",
                "fair strong each Node::send"
            ),
        ]
    );

    let (_, unit) = build(
        "probabilistic C using ev on Service under every scheduler on origin \
         { probability >= 0.5 [ holds(true) ] }",
    );
    assert_eq!(
        claim(&unit).scheduler,
        SchedulerForm::Every {
            fairness: Vec::new()
        }
    );
}

/// FR-392-AC-5, the S2 half (TC-1077 step 5): a lone `alpha` keeps one
/// confidence item and no default; a repeated `alpha` keeps both items in
/// order; the threshold-`1` and `5 m` claims build with no S2 diagnostic. The
/// S3 refusals of these claims are asserted in `qsl-semantics`.
#[trace("TC-1077")]
#[test]
fn partial_and_repeated_confidence_and_out_of_range_thresholds_build_at_s2() {
    let source = |confidence: &str| {
        claim_source(
            "under Steady on origin",
            confidence,
            "probability >= 0.99 [ holds(true) ]",
        )
    };
    let (_, unit) = build(&source("with alpha 0.01"));
    assert_eq!(
        confidence_of(claim(&unit)),
        [(ConfidenceKey::Alpha, decimal_literal("1", 2))]
    );

    let (text, unit) = build(&source("with alpha 0.01, alpha 0.05"));
    let form = claim(&unit);
    assert_eq!(
        confidence_of(form),
        [
            (ConfidenceKey::Alpha, decimal_literal("1", 2)),
            (ConfidenceKey::Alpha, decimal_literal("5", 2)),
        ]
    );
    assert_eq!(slice(&text, form.confidence[1].key_span), "alpha");
    assert!(form.confidence[0].key_span.end < form.confidence[1].key_span.start);

    let (_, unit) = build("probabilistic C using ev on Traces under Steady on origin { probability >= 1 [ holds(true) ] }");
    let ProbabilisticBody::Probability { threshold, .. } = &claim(&unit).body else {
        panic!("a probability claim");
    };
    assert_eq!(exact(threshold), natural(1));

    let (_, body) = body_of(
        "quantile 0.95 of (accumulate duration from holds(A) until holds(B) within 10) <= 5 m",
    );
    let ProbabilisticBody::Quantile { threshold, .. } = body else {
        panic!("a quantile claim");
    };
    assert_eq!(quantity(&threshold), ("5".into(), "m".into()));
}

/// FR-392-AC-6 (TC-1077 step 6): `NoFault` in a unit that selects no complete
/// facet builds the same form, spans included, as in a unit that selects the
/// complete value and model layers, with no S2 diagnostic.
#[trace("TC-1077", "FR-392-AC-6")]
#[test]
fn the_claim_form_is_the_same_whether_or_not_the_unit_selects_the_complete_facets() {
    const CORE: &str = "quire.state.core/v1";
    const COMPLETE: &str = "quire.model.complete/v1";
    let source = |header: &str| {
        format!(
            "{header}probabilistic NoFault using ev on Health under Steady on origin \
             with alpha 0.01, beta 0.01, indifference 0.0005 \
             {{ probability >= 0.999 [ always[0,1000] holds(n.healthy) ] }}\n"
        )
    };
    let padding = COMPLETE.len() - CORE.len();
    let without = build_text(&source(&header(CORE, padding)));
    let with = build_text(&source(&header(COMPLETE, 0)));
    assert_eq!(claim(&without), claim(&with));
    assert_eq!(
        format!("{:?}", claim(&without)),
        format!("{:?}", claim(&with))
    );
}
