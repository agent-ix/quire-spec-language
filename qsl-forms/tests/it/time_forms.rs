// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-393 (TC-1078): S1 parses and S2 builds the `time-decl`,
//! `time-invariant`, `urgent-decl`, `Clock` type, `quantity-literal` and
//! `timed-interval` forms, run on real complete-V1 source through
//! `qsl_cst::parse` and `build_unit`.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, BuiltinType, DeclarationForm, ExactValueKind, FormsCause, IntervalUpper,
    OperatorInterval, ParsedUnit, QuantityForm, TemporalNodeForm, TemporalOperatorForm,
    TimeInvariantForm, TimeSourceForm, TimeSourceSyntax, TypeFormHead, UrgentForm, UrgentTarget,
};
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::Integer;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile t = \"quire.temporal.infinite-trace/v1\";\n\
     model Config = \"example/config-version\" version \"1\" digest \
     \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// The declarations of FR-393-AC-1.
const RPC: &str = "time Rpc dense unit ms;\n\
     time invariant inflight when { self.inflight } { self.x <= 3 ms }\n\
     time invariant waiting when { self.phase = Waiting } { self.x <= 4 ms }\n\
     record Call { x: Clock; }\n\
     urgent hurry Call::timeout;\n\
     urgent stalled when { self.phase = Waiting };\n\
     time Rpc tick Call::send period rational(1,2) ms unit ms;";

fn parse(text: &str) -> qsl_cst::ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn build_with(header: &str, declarations: &str) -> (String, ParsedUnit) {
    let text = format!("{header}{declarations}\n");
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).unwrap_or_else(|failure| panic!("{text}: {failure:?}"));
    (text, unit)
}

fn build(declarations: &str) -> (String, ParsedUnit) {
    build_with(HEADER, declarations)
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// An exact value as written: its spelling kind and its two integers.
fn exact(kind: ExactValueKind, first: i64, second: i64) -> (ExactValueKind, Integer, Integer) {
    (kind, Integer::from(first), Integer::from(second))
}

/// A `uint` magnitude `value`.
fn natural(value: i64) -> (ExactValueKind, Integer, Integer) {
    exact(ExactValueKind::Natural, value, 1)
}

/// The magnitude as written and the unit a quantity holds.
fn quantity(form: &QuantityForm) -> ((ExactValueKind, Integer, Integer), &str) {
    let magnitude = &form.magnitude;
    (
        (
            magnitude.kind,
            magnitude.first.clone(),
            magnitude.second.clone(),
        ),
        form.unit.name.as_str(),
    )
}

fn time_source(form: &DeclarationForm) -> &TimeSourceForm {
    let DeclarationForm::TimeSource(time) = form else {
        panic!("a time declaration: {form:?}");
    };
    time
}

fn time_invariant(form: &DeclarationForm) -> &TimeInvariantForm {
    let DeclarationForm::TimeInvariant(invariant) = form else {
        panic!("a time invariant: {form:?}");
    };
    invariant
}

fn urgent(form: &DeclarationForm) -> &UrgentForm {
    let DeclarationForm::Urgent(urgent) = form else {
        panic!("an urgent declaration: {form:?}");
    };
    urgent
}

/// The root operator of the one temporal clause `formula` builds.
fn clause_operator(formula: &str) -> TemporalOperatorForm {
    let (_, unit) = build(&format!(
        "temporal C using t over (k: Config::ConfigVersion) clock \"c\" on origin {{ {formula} }}"
    ));
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Temporal(clause) = form.form() else {
        panic!("a temporal clause: {:?}", form.form());
    };
    match clause.formula.node(clause.formula.root()) {
        Some(TemporalNodeForm::Operator(operator)) => operator.clone(),
        other => panic!("an operator at the root: {other:?}"),
    }
}

/// The span of the first occurrence of `needle` at or after `after`.
fn span_of(text: &str, after: &str, needle: &str) -> Span {
    let from = text.find(after).expect("the anchor is in the source");
    let start = from
        + text[from..]
            .find(needle)
            .expect("the needle follows the anchor");
    Span {
        start,
        end: start + needle.len(),
    }
}

/// FR-393-AC-1 (TC-1078 step 1): the `Rpc` declarations build one dense
/// `TimeSourceForm`, two time invariants in declared order with their `when`
/// expressions and one non-strict bound each, the `Clock` field type, both
/// urgency targets and the tick source's exact period, each with its
/// whole-form span.
#[trace("TC-1078", "FR-393-AC-1")]
#[test]
fn the_rpc_declarations_build_their_time_forms_in_order_with_spans() {
    let (text, unit) = build(RPC);
    let forms: Vec<&DeclarationForm> = unit.forms().iter().map(|form| form.form()).collect();
    let [dense, inflight, waiting, record, hurry, stalled, tick] = forms[..] else {
        panic!("seven declarations: {forms:?}");
    };

    let dense = time_source(dense);
    assert_eq!(dense.model.name, "Rpc");
    assert_eq!(dense.source, TimeSourceSyntax::Dense);
    assert_eq!(dense.unit.name, "ms");
    assert_eq!(slice(&text, dense.span), "time Rpc dense unit ms;");

    for (form, name, guard, bound) in [
        (inflight, "inflight", "self.inflight", 3),
        (waiting, "waiting", "self.phase = Waiting", 4),
    ] {
        let invariant = time_invariant(form);
        assert_eq!(invariant.name.name, name);
        let guard_span = invariant
            .guard
            .spans
            .span(invariant.guard.spans.root())
            .expect("the guard has a span");
        assert_eq!(slice(&text, guard_span), guard);
        let [clock_bound] = &invariant.bounds[..] else {
            panic!("one clock bound: {:?}", invariant.bounds);
        };
        assert!(!clock_bound.strict);
        assert_eq!(quantity(&clock_bound.bound), (natural(bound), "ms"));
        assert_eq!(
            slice(&text, clock_bound.span),
            format!("self.x <= {bound} ms")
        );
        assert!(slice(&text, invariant.span).starts_with(&format!("time invariant {name} when")));
        assert!(slice(&text, invariant.span).ends_with(&format!("{{ self.x <= {bound} ms }}")));
    }

    let DeclarationForm::Record(record) = record else {
        panic!("a record: {record:?}");
    };
    let clock = &record.fields[0].type_form;
    assert_eq!(clock.head, TypeFormHead::Builtin(BuiltinType::Clock));
    assert_eq!(slice(&text, clock.span), "Clock");

    let hurry = urgent(hurry);
    assert_eq!(hurry.name.name, "hurry");
    let UrgentTarget::Operation(operation) = &hurry.target else {
        panic!("an operation target: {:?}", hurry.target);
    };
    assert_eq!(operation.name, "Call::timeout");
    assert_eq!(slice(&text, hurry.span), "urgent hurry Call::timeout;");

    let stalled = urgent(stalled);
    let UrgentTarget::When(condition) = &stalled.target else {
        panic!("a when target: {:?}", stalled.target);
    };
    assert_eq!(
        slice(
            &text,
            condition
                .spans
                .span(condition.spans.root())
                .expect("a span")
        ),
        "self.phase = Waiting"
    );
    assert_eq!(
        slice(&text, stalled.span),
        "urgent stalled when { self.phase = Waiting };"
    );

    let tick = time_source(tick);
    let TimeSourceSyntax::Tick { operation, period } = &tick.source else {
        panic!("a tick source: {:?}", tick.source);
    };
    assert_eq!(operation.name, "Call::send");
    assert_eq!(
        quantity(period),
        (exact(ExactValueKind::Rational, 1, 2), "ms")
    );
    assert_eq!(slice(&text, period.span), "rational(1,2) ms");
    assert_eq!(
        slice(&text, tick.span),
        "time Rpc tick Call::send period rational(1,2) ms unit ms;"
    );
}

/// FR-393-AC-2 (TC-1078 step 2): `time invariant`, `time` and `urgent` each
/// build the one form of their own production, and the contextual words
/// stay identifiers elsewhere: a field named `time`, a parameter named
/// `period` and a field read `self.time` build with those names.
#[trace("TC-1078", "FR-393-AC-2")]
#[test]
fn each_time_head_dispatches_to_its_one_production_and_the_words_stay_identifiers() {
    for (declaration, kind) in [
        (
            "time invariant i when { true } { self.x < 1 ms }",
            "time invariant",
        ),
        ("time M dense unit ms;", "time"),
        ("urgent u when { true };", "urgent"),
    ] {
        let (_, unit) = build(declaration);
        let [form] = unit.forms() else {
            panic!("{declaration}: one declaration: {:?}", unit.forms());
        };
        let built = match form.form() {
            DeclarationForm::TimeInvariant(_) => "time invariant",
            DeclarationForm::TimeSource(_) => "time",
            DeclarationForm::Urgent(_) => "urgent",
            other => panic!("{declaration}: a time form, not {other:?}"),
        };
        assert_eq!(built, kind, "{declaration}");
    }

    let (_, unit) = build(
        "record Sample { time: Integer; urgent: Integer; }\n\
         function f using t (period: Integer, dense: Integer): Integer pure { period + dense }\n\
         invariant I using t on Config::ConfigVersion at current { self.time = 0 }",
    );
    let forms: Vec<&DeclarationForm> = unit.forms().iter().map(|form| form.form()).collect();
    let [DeclarationForm::Record(record), DeclarationForm::Function(function), DeclarationForm::StateClause(_)] =
        forms[..]
    else {
        panic!("a record, a function and an invariant: {forms:?}");
    };
    let fields: Vec<&str> = record
        .fields
        .iter()
        .map(|field| field.name.as_str())
        .collect();
    assert_eq!(fields, ["time", "urgent"]);
    let parameters: Vec<&str> = function
        .parameters
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(parameters, ["period", "dense"]);
}

/// FR-393-AC-3 (TC-1078 step 3): a `[` or `(` followed by a quantity opens a
/// timed interval with each end's openness and exact magnitudes; a `(`
/// followed by a formula is the operand; `[0,3]` is FR-325's step interval.
#[trace("TC-1078", "FR-393-AC-3")]
#[test]
fn timed_step_and_absent_intervals_build_as_written() {
    for (formula, closed, lower, upper) in [
        ("eventually[0 ms, 3 ms) holds(k.done)", (true, false), 0, 3),
        ("once(1 ms, 2 ms] holds(k.done)", (false, true), 1, 2),
    ] {
        let Some(OperatorInterval::Timed(timed)) = clause_operator(formula).interval else {
            panic!("{formula}: a timed interval");
        };
        assert_eq!(
            (timed.lower_closed, timed.upper_closed),
            closed,
            "{formula}"
        );
        assert_eq!(quantity(&timed.lower), (natural(lower), "ms"), "{formula}");
        assert_eq!(quantity(&timed.upper), (natural(upper), "ms"), "{formula}");
    }
    assert_eq!(clause_operator("always (holds(k.done))").interval, None);
    let Some(OperatorInterval::Steps(steps)) =
        clause_operator("eventually[0,3] holds(k.done)").interval
    else {
        panic!("a step interval");
    };
    assert_eq!((steps.lower, steps.upper), (0, IntervalUpper::Finite(3)));
}

/// FR-393 S1/S2 half of AC-4 (TC-1078 step 4): `0.003` is held as written,
/// digits `3` over three fraction digits (the exact `3/1000`, which S3
/// reduces); a punctual-open and an inverted interval and a second `time`
/// declaration build at S2; `[0 ms, x]` fails at S1 at `x`. The S3 refusals
/// AC-4 also names (FR-234-AC-2, FR-230-AC-2) are not checked yet, so this
/// test carries no AC-4 tag.
#[trace("TC-1078")]
#[test]
fn exact_magnitudes_build_and_meaning_checks_wait_for_s3() {
    let Some(OperatorInterval::Timed(timed)) =
        clause_operator("eventually[0 s, 0.003 s] holds(k.done)").interval
    else {
        panic!("a timed interval");
    };
    assert_eq!(
        quantity(&timed.upper),
        (exact(ExactValueKind::DecimalLiteral, 3, 3), "s")
    );
    for formula in [
        "eventually(2 ms, 2 ms] holds(k.done)",
        "eventually[3 ms, 2 ms] holds(k.done)",
    ] {
        assert!(
            matches!(
                clause_operator(formula).interval,
                Some(OperatorInterval::Timed(_))
            ),
            "{formula}"
        );
    }
    let (_, unit) = build("time Rpc dense unit ms;\ntime Rpc dense unit s;");
    assert_eq!(unit.forms().len(), 2);

    let (text, parsed) = refused("eventually[0 ms, x] holds(k.done)");
    assert_eq!(
        diagnostic_span(&parsed),
        span_of(&text, "[0 ms, x]", "x"),
        "{:?}",
        parsed.diagnostics()
    );
}

fn clause(formula: &str) -> String {
    format!(
        "{HEADER}temporal C using t over (k: Config::ConfigVersion) clock \"c\" on origin {{ {formula} }}\n"
    )
}

/// The source and parse of a clause S1 must refuse, after checking S2
/// builds no form from it.
fn refused(formula: &str) -> (String, qsl_cst::ParsedSource) {
    let text = clause(formula);
    let parsed = parse(&text);
    assert!(!parsed.is_admissible(), "{formula} must fail at S1");
    let refusal = build_unit(&parsed).expect_err("S2 builds no form");
    assert!(
        matches!(
            refusal.cause,
            FormsCause::RecoveringCst | FormsCause::DiagnosedSource(_)
        ),
        "{formula}: {refusal:?}"
    );
    (text, parsed)
}

fn diagnostic_span(parsed: &qsl_cst::ParsedSource) -> Span {
    parsed.diagnostics()[0]
        .byte_span()
        .expect("a located diagnostic")
}

/// FR-393-AC-5 (TC-1078 step 5): a bracketed interval with a step bound at
/// one end and a quantity at the other, or a timed interval with `*`, fails
/// at S1 at the upper bound's span and builds no form.
#[trace("TC-1078", "FR-393-AC-5")]
#[test]
fn mixed_interval_bounds_fail_at_s1_at_the_upper_bound() {
    for (formula, interval, upper) in [
        ("eventually[0, 3 ms] holds(k.done)", "[0, 3 ms]", "3 ms"),
        ("eventually[0 ms, 3] holds(k.done)", "[0 ms, 3]", "3"),
        ("eventually[0 ms, *] holds(k.done)", "[0 ms, *]", "*"),
    ] {
        let (text, parsed) = refused(formula);
        assert_eq!(
            diagnostic_span(&parsed),
            span_of(&text, interval, upper),
            "{formula}: {:?}",
            parsed.diagnostics()
        );
    }
}

/// FR-393-AC-6 (TC-1078 step 6): the `Rpc` unit builds equal forms with no
/// facet selected and with the timed facet selected (the headers padded to
/// one length, so every span agrees), and S2 reports no facet diagnostic.
#[trace("TC-1078", "FR-393-AC-6")]
#[test]
fn the_rpc_unit_builds_equal_forms_under_every_facet_selection() {
    let plain = "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"quire.value.complete/v1\";\n";
    let timed = "language \"ix:native\" edition \"1-draft\";\n\
         profile t = \"quire.temporal.timed/v1\";\n";
    assert_eq!(plain.len(), timed.len());
    let (_, without) = build_with(plain, RPC);
    let (_, with) = build_with(timed, RPC);
    assert_eq!(without.forms().len(), with.forms().len());
    for (left, right) in without.forms().iter().zip(with.forms()) {
        assert_eq!(left.span(), right.span());
        match (left.form(), right.form()) {
            (DeclarationForm::TimeSource(left), DeclarationForm::TimeSource(right)) => {
                assert_eq!(left, right);
            }
            (DeclarationForm::TimeInvariant(left), DeclarationForm::TimeInvariant(right)) => {
                assert_eq!(left, right);
            }
            (DeclarationForm::Urgent(left), DeclarationForm::Urgent(right)) => {
                assert_eq!(left, right);
            }
            (DeclarationForm::Record(left), DeclarationForm::Record(right)) => {
                assert_eq!(left.fields[0].type_form, right.fields[0].type_form);
            }
            (left, right) => panic!("different forms: {left:?} and {right:?}"),
        }
    }
}

/// FR-393-AC-7 (TC-1078 step 7): an unqualified `Clock` is the built-in
/// type, `M::Clock` is a named type, and a unit that declares a type named
/// `Clock` fails at S1 at the declared name.
#[trace("TC-1078", "FR-393-AC-7")]
#[test]
fn clock_is_built_in_only_unqualified_and_cannot_be_declared() {
    let (_, unit) = build("record R { x: Clock; y: Config::Clock; }");
    let [form] = unit.forms() else {
        panic!("one declaration");
    };
    let DeclarationForm::Record(record) = form.form() else {
        panic!("a record");
    };
    assert_eq!(
        record.fields[0].type_form.head,
        TypeFormHead::Builtin(BuiltinType::Clock)
    );
    assert_eq!(
        record.fields[1].type_form.head,
        TypeFormHead::Name("Config::Clock".into())
    );

    for declaration in [
        "record Clock { a: Integer; }",
        "enum Clock { A }",
        "type Clock = Integer;",
        "tuple Clock(Integer);",
        "union Clock { A }",
    ] {
        let text = format!("{HEADER}{declaration}\n");
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{declaration}");
        assert_eq!(
            diagnostic_span(&parsed),
            span_of(&text, declaration, "Clock"),
            "{declaration}: {:?}",
            parsed.diagnostics()
        );
        assert!(build_unit(&parsed).is_err(), "{declaration}");
    }
}
