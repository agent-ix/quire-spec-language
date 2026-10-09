// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-325: the `TemporalTrace` family's S2 production -- temporal
//! operators with an optional interval -- run on real complete-V1 source
//! through `qsl_cst::parse` and `build_unit` (TC-835).

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ActivationForm, DeclarationForm, FairnessGranularity, FairnessKind, FormsCause,
    IntervalForm, IntervalUpper, ParsedUnit, TemporalClauseForm, TemporalFormulaForm,
    TemporalNodeForm, TemporalNodeId, TemporalOperator, TemporalOperatorForm,
};
use qsl_foundation::{SourceIdentity, Span};

/// The header selecting the temporal profile `profile`, with the model's
/// `version` padded so every profile identity gives the header the same
/// byte length.
fn header(profile: &str, padding: usize) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"{profile}\";\n\
         model Config = \"example/config-version\" version \"1{}\" digest \"d\";\n",
        "v".repeat(padding)
    )
}

const INFINITE: &str = "quire.temporal.infinite-trace/v1";
const EVENT_POSITION: &str = "quire.temporal.event-position.false-extension/v1";

fn clause_source(header: &str, formula: &str) -> String {
    format!(
        "{header}temporal C using t over (p: Config::ConfigVersion) clock \"steps\" on origin \
         {{ {formula} }}\n"
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

fn build(text: &str) -> ParsedUnit {
    let parsed = parse(text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"))
}

fn clause(unit: &ParsedUnit) -> &TemporalClauseForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Temporal(clause) = form.form() else {
        panic!("a temporal clause: {:?}", form.form());
    };
    clause
}

/// The unit of `formula` under the infinite-trace profile, built.
fn formula_unit(formula: &str) -> (String, ParsedUnit) {
    let text = clause_source(&header(INFINITE, 0), formula);
    let unit = build(&text);
    (text, unit)
}

fn operator(formula: &TemporalFormulaForm, id: TemporalNodeId) -> &TemporalOperatorForm {
    match formula.node(id) {
        Some(TemporalNodeForm::Operator(operator)) => operator,
        other => panic!("an operator node, not {other:?}"),
    }
}

fn sole_operand(formula: &TemporalFormulaForm, id: TemporalNodeId) -> TemporalNodeId {
    let [operand] = operator(formula, id).operands[..] else {
        panic!("one operand: {:?}", operator(formula, id));
    };
    operand
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

fn interval(
    lower: u64,
    upper: IntervalUpper,
    text: &str,
    written: &str,
) -> (u64, IntervalUpper, Span) {
    let start = text.find(written).expect("the interval is in the source");
    (
        lower,
        upper,
        Span {
            start,
            end: start + written.len(),
        },
    )
}

fn written(interval: Option<IntervalForm>) -> Option<(u64, IntervalUpper, Span)> {
    interval.map(|interval| (interval.lower, interval.upper, interval.span))
}

/// FR-325-AC-1 (TC-835 step 1): the recovery-stability clause holds `None`
/// on the outer `always` and on `eventually`, and `Some{0, Finite(10)}` on
/// the inner `always`, each operator with its own span.
#[trace("TC-835", "FR-325-AC-1")]
#[test]
fn an_unbounded_operator_over_a_bounded_one_keeps_each_operators_interval() {
    let formula = "always (holds(not s.healthy) implies eventually always[0,10] holds(s.healthy))";
    let (text, unit) = formula_unit(formula);
    let form = &clause(&unit).formula;

    let outer = operator(form, form.root());
    assert_eq!(outer.operator, TemporalOperator::Always);
    assert_eq!(outer.interval, None);
    assert_eq!(slice(&text, outer.span), formula);
    assert_eq!(slice(&text, outer.operator_span), "always");

    let implication = sole_operand(form, form.root());
    assert_eq!(
        operator(form, implication).operator,
        TemporalOperator::Implies
    );
    let [premise, conclusion] = operator(form, implication).operands[..] else {
        panic!("two operands: {:?}", operator(form, implication));
    };
    assert!(matches!(
        form.node(premise),
        Some(TemporalNodeForm::Holds { span, .. }) if slice(&text, *span) == "holds(not s.healthy)"
    ));

    let eventually = operator(form, conclusion);
    assert_eq!(eventually.operator, TemporalOperator::Eventually);
    assert_eq!(eventually.interval, None);
    assert_eq!(
        slice(&text, eventually.span),
        "eventually always[0,10] holds(s.healthy)"
    );
    assert_eq!(slice(&text, eventually.operator_span), "eventually");

    let inner = operator(form, sole_operand(form, conclusion));
    assert_eq!(inner.operator, TemporalOperator::Always);
    assert_eq!(
        written(inner.interval),
        Some(interval(0, IntervalUpper::Finite(10), &text, "[0,10]"))
    );
    assert_eq!(slice(&text, inner.span), "always[0,10] holds(s.healthy)");
    assert_eq!(slice(&text, inner.operator_span), "always");
    assert_ne!(outer.span, eventually.span);
    assert_ne!(eventually.span, inner.span);
}

/// FR-325-AC-2 (TC-835 step 2): an operator written with no interval, with
/// `[a, b]` and with `[a, *]` holds `None`, `Some{a, Finite(b)}` and
/// `Some{a, Open}`; `[5, 3]` is built as written with no diagnostic.
#[trace("TC-835", "FR-325-AC-2")]
#[test]
fn binary_and_unary_operators_build_none_closed_open_and_inverted_intervals() {
    let (_, unit) = formula_unit("holds(p) until holds(q)");
    let form = &clause(&unit).formula;
    assert_eq!(
        operator(form, form.root()).operator,
        TemporalOperator::Until
    );
    assert_eq!(operator(form, form.root()).interval, None);

    let (text_since, unit) = formula_unit("holds(p) since[2,4] holds(q)");
    let form = &clause(&unit).formula;
    let since = operator(form, form.root());
    assert_eq!(since.operator, TemporalOperator::Since);
    assert_eq!(
        written(since.interval),
        Some(interval(2, IntervalUpper::Finite(4), &text_since, "[2,4]"))
    );
    assert_eq!(since.operands.len(), 2);
    assert_eq!(slice(&text_since, since.operator_span), "since");

    let (text_open, unit) = formula_unit("eventually[3,*] holds(p)");
    let form = &clause(&unit).formula;
    assert_eq!(
        written(operator(form, form.root()).interval),
        Some(interval(3, IntervalUpper::Open, &text_open, "[3,*]"))
    );

    let (text_inverted, unit) = formula_unit("eventually[5,3] holds(p)");
    let form = &clause(&unit).formula;
    assert_eq!(
        written(operator(form, form.root()).interval),
        Some(interval(
            5,
            IntervalUpper::Finite(3),
            &text_inverted,
            "[5,3]"
        ))
    );
}

/// FR-325-AC-2 and "Nesting SHALL be free": every unary and binary
/// operator builds with each of the three interval shapes, and an operator
/// with an interval and one without nest in either operand position.
#[trace("TC-835", "FR-325-AC-2")]
#[test]
fn every_temporal_operator_takes_each_interval_shape_and_nests_freely() {
    let unary = [
        ("eventually", TemporalOperator::Eventually),
        ("always", TemporalOperator::Always),
        ("once", TemporalOperator::Once),
        ("historically", TemporalOperator::Historically),
    ];
    let binary = [
        ("until", TemporalOperator::Until),
        ("release", TemporalOperator::Release),
        ("since", TemporalOperator::Since),
        ("triggered", TemporalOperator::Triggered),
    ];
    let shapes = [
        ("", None),
        ("[1,2]", Some((1, IntervalUpper::Finite(2)))),
        ("[1,*]", Some((1, IntervalUpper::Open))),
    ];
    for (keyword, expected) in unary {
        for (spelling, shape) in shapes {
            let (_, unit) = formula_unit(&format!("{keyword}{spelling} holds(p)"));
            let form = &clause(&unit).formula;
            let built = operator(form, form.root());
            assert_eq!(built.operator, expected, "{keyword}{spelling}");
            assert_eq!(
                built.interval.map(|i| (i.lower, i.upper)),
                shape,
                "{keyword}{spelling}"
            );
        }
    }
    for (keyword, expected) in binary {
        for (spelling, shape) in shapes {
            let (_, unit) = formula_unit(&format!("holds(p) {keyword}{spelling} holds(q)"));
            let form = &clause(&unit).formula;
            let built = operator(form, form.root());
            assert_eq!(built.operator, expected, "{keyword}{spelling}");
            assert_eq!(
                built.interval.map(|i| (i.lower, i.upper)),
                shape,
                "{keyword}{spelling}"
            );
        }
    }

    let (_, unit) = formula_unit("(holds(p) until[0,2] holds(q)) release eventually[1,*] holds(r)");
    let form = &clause(&unit).formula;
    let release = operator(form, form.root());
    assert_eq!(release.operator, TemporalOperator::Release);
    assert_eq!(release.interval, None);
    let [left, right] = release.operands[..] else {
        panic!("two operands: {release:?}");
    };
    assert_eq!(operator(form, left).operator, TemporalOperator::Until);
    assert!(operator(form, left).interval.is_some());
    assert_eq!(operator(form, right).operator, TemporalOperator::Eventually);
    assert!(operator(form, right).interval.is_some());
}

/// FR-325-AC-3 (TC-835 step 3): `eventually[0,x] holds(p)` fails at S1
/// with a parse diagnostic located at `x`, a bound beyond `u64` fails at
/// that bound, and no S2 form is built for either.
#[trace("TC-835", "FR-325-AC-3")]
#[test]
fn a_bound_that_is_not_a_u64_fails_at_s1_at_the_bound() {
    let too_large = "18446744073709551616";
    for (formula, bound) in [
        ("eventually[0,x] holds(p)", "x"),
        (&format!("eventually[0,{too_large}] holds(p)"), too_large),
        (&format!("eventually[{too_large},*] holds(p)"), too_large),
    ] {
        let text = clause_source(&header(INFINITE, 0), formula);
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{formula}");
        let start = text.rfind(formula).expect("the formula is in the source")
            + formula.find(bound).expect("the bound is in the formula");
        assert_eq!(
            parsed.diagnostics()[0].byte_span(),
            Some(Span {
                start,
                end: start + bound.len()
            }),
            "{formula}"
        );
        assert!(build_unit(&parsed).is_err(), "{formula}: no form is built");
    }
    let (_, unit) = formula_unit("eventually[0,18446744073709551615] holds(p)");
    let form = &clause(&unit).formula;
    assert_eq!(
        operator(form, form.root()).interval.map(|i| i.upper),
        Some(IntervalUpper::Finite(u64::MAX))
    );
}

/// FR-325-AC-4 (TC-835 step 4): one unit's text builds byte-equal forms
/// when it selects the infinite-trace profile and when it selects the
/// event-position false-extension profile, with the clause's activation,
/// fairness constraints and bounded and unbounded intervals all in it.
#[trace("TC-835", "FR-325-AC-4")]
#[test]
fn the_clause_forms_are_the_same_under_every_profile_selection() {
    let padding = EVENT_POSITION.len() - INFINITE.len();
    let body = "fair Config::ConfigVersion::attemptUpdate; \
                fair strong each Config::ConfigVersion::reset; \
                fair weak whole Config::ConfigVersion::tick; \
                eventually[0,5] holds(p.value = 3) and always[1,*] holds(true)";
    let source = |header: &str| {
        format!(
            "{header}temporal C using t over (p: Config::ConfigVersion) clock \"steps\" \
             on each (q: Config::ConfigVersion) when (q.value = 1) {{ {body} }}\n"
        )
    };
    let infinite = build(&source(&header(INFINITE, padding)));
    let event_position = build(&source(&header(EVENT_POSITION, 0)));

    assert_eq!(clause(&infinite), clause(&event_position));
    assert_eq!(
        format!("{:?}", infinite.forms()),
        format!("{:?}", event_position.forms())
    );

    let built = clause(&infinite);
    assert_eq!(built.profile.alias, "t");
    assert_eq!(built.over.name.name, "p");
    let ActivationForm::Each { parameter, when } = &built.activation else {
        panic!("an `on each` activation: {:?}", built.activation);
    };
    assert_eq!(parameter.name.name, "q");
    assert!(when.is_some());
    let fairness: Vec<_> = built
        .fairness
        .iter()
        .map(|constraint| {
            (
                constraint.kind,
                constraint.granularity,
                constraint.operation.name.as_str(),
            )
        })
        .collect();
    assert_eq!(
        fairness,
        [
            (
                FairnessKind::Weak,
                None,
                "Config::ConfigVersion::attemptUpdate"
            ),
            (
                FairnessKind::Strong,
                Some(FairnessGranularity::Each),
                "Config::ConfigVersion::reset"
            ),
            (
                FairnessKind::Weak,
                Some(FairnessGranularity::Whole),
                "Config::ConfigVersion::tick"
            ),
        ]
    );
    let form = &built.formula;
    let conjunction = operator(form, form.root());
    assert_eq!(conjunction.operator, TemporalOperator::And);
    assert_eq!(
        conjunction
            .operands
            .iter()
            .map(|id| operator(form, *id).interval.map(|i| i.upper))
            .collect::<Vec<_>>(),
        [Some(IntervalUpper::Finite(5)), Some(IntervalUpper::Open)]
    );
}

/// FR-325 "Behavior": the TemporalTrace forms are the only S2 producer for
/// a temporal clause, and a `capture` (no form yet) refuses rather than
/// being dropped.
#[trace("TC-835", "FR-325-AC-1")]
#[test]
fn a_capture_refuses_as_an_unrepresented_construct() {
    let text = format!(
        "{}temporal C using t over (p: Config::ConfigVersion) clock \"steps\" on origin \
         {{ capture v: Boolean = true; holds(v) }}\n",
        header(INFINITE, 0)
    );
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let refusal = build_unit(&parsed).expect_err("a capture has no form");
    assert!(matches!(
        refusal.cause,
        FormsCause::UnrepresentedConstruct { .. }
    ));
}
