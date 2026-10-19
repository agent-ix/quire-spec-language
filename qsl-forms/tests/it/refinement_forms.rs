// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-310: S2 builds the `refinement` declaration form -- header, then
//! `population`, `object`, `step` and `assume`/`ensure` rows in source order
//! -- run on real complete-V1 source through `qsl_cst::parse` and
//! `build_unit` (TC-889). S2 resolves no name.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, AbstractSideForm, ArgumentForm, DeclarationForm, ExactNumberKind, ExprNode,
    FairnessGranularity, FairnessKind, FairnessSide, FormsCause, HistoryLiteralForm, ParsedUnit,
    RefinementForm, RefinementRowForm, StepClassKeyword, StepSourceForm, StepTargetForm,
};
use qsl_foundation::{SourceIdentity, Span};
use quire_exact::Integer;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
     profile inf = \"quire.temporal.infinite-trace/v1\";\n\
     model Spec = \"example/counter-spec\" version \"1\" digest \
     \"sha256-jcs:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
     model Impl = \"example/counter-cas\" version \"1\" digest \
     \"sha256-jcs:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\";\n";

const CAS_REFINES_COUNTER: &str =
    "refinement CasRefinesCounter using inf abstract Spec concrete Impl {\n\
     population Spec::counters from Impl::counters;\n\
     object Spec::Counter from Impl::Counter { value = self.value; }\n\
     step Impl::Counter::commitA -> Spec::Counter::inc(self);\n\
     step Impl::Counter::commitB -> Spec::Counter::inc(self);\n\
     step Impl::Counter::beginA -> stutter;\n\
     step Impl::Counter::beginB -> stutter;\n\
     step Impl::Counter::retryA -> stutter;\n\
     step Impl::Counter::retryB -> stutter;\n\
     step Impl::Counter::peek -> stutter;\n\
     assume fair weak each Impl::Counter::beginA;\n\
     assume fair weak each Impl::Counter::beginB;\n\
     assume fair weak each Impl::Counter::commitA;\n\
     assume fair weak each Impl::Counter::commitB;\n\
     assume fair weak each Impl::Counter::retryA;\n\
     assume fair weak each Impl::Counter::retryB;\n\
     ensure fair weak each Spec::Counter::inc;\n\
     }";

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

fn refinement(unit: &ParsedUnit) -> &RefinementForm {
    let [form] = unit.forms() else {
        panic!("one declaration: {:?}", unit.forms());
    };
    let DeclarationForm::Refinement(refinement) = form.form() else {
        panic!("a refinement declaration: {:?}", form.form());
    };
    refinement
}

/// The model alias of a form's abstract side.
fn abstract_model(form: &RefinementForm) -> &qsl_forms::DeclaredName {
    match &form.abstract_side {
        AbstractSideForm::Model(alias) => alias,
        AbstractSideForm::Protocol { .. } => panic!("a model side: {:?}", form.abstract_side),
    }
}

/// The qualified name of a step row's source, with the keyword it was
/// written with.
fn source_name(source: &StepSourceForm) -> (Option<StepClassKeyword>, &str) {
    match source {
        StepSourceForm::Named { kind, name } => (*kind, name.name.as_str()),
        StepSourceForm::Memory(_) => panic!("a named source: {source:?}"),
    }
}

fn slice(text: &str, span: Span) -> &str {
    &text[span.start..span.end]
}

/// FR-310-AC-1 (TC-889 step 1): the compare-and-set counter's refinement
/// builds one form with its header, then its population, object, seven step,
/// `assume` and `ensure` rows in source order, each with a span that slices
/// to its text; the `ensure` row's granularity is `each` as written.
#[trace("TC-889", "FR-310-AC-1")]
#[test]
fn the_counter_refinement_builds_every_row_in_source_order() {
    let (text, unit) = build(CAS_REFINES_COUNTER);
    let form = refinement(&unit);

    assert_eq!(form.name.name, "CasRefinesCounter");
    assert_eq!(slice(&text, form.name.span), "CasRefinesCounter");
    assert_eq!(form.profile.alias, "inf");
    assert_eq!(abstract_model(form).name, "Spec");
    assert_eq!(slice(&text, abstract_model(form).span), "Spec");
    assert_eq!(form.concrete.name, "Impl");
    assert_eq!(slice(&text, form.span), CAS_REFINES_COUNTER);

    let kinds: Vec<&str> = form
        .rows
        .iter()
        .map(|row| match row {
            RefinementRowForm::Population(_) => "population",
            RefinementRowForm::Object(_) => "object",
            RefinementRowForm::Step(_) => "step",
            RefinementRowForm::History(_) => "history",
            RefinementRowForm::Fairness(_) => "fairness",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "population",
            "object",
            "step",
            "step",
            "step",
            "step",
            "step",
            "step",
            "step",
            "fairness",
            "fairness",
            "fairness",
            "fairness",
            "fairness",
            "fairness",
            "fairness",
        ]
    );

    let RefinementRowForm::Population(population) = &form.rows[0] else {
        panic!("a population row first");
    };
    assert_eq!(population.target.name, "Spec::counters");
    assert_eq!(population.source.name, "Impl::counters");
    assert_eq!(
        slice(&text, population.span),
        "population Spec::counters from Impl::counters;"
    );

    let RefinementRowForm::Object(object) = &form.rows[1] else {
        panic!("an object row second");
    };
    assert_eq!(object.target.name, "Spec::Counter");
    assert_eq!(object.source.name, "Impl::Counter");
    let [field] = &object.fields[..] else {
        panic!("one field row: {:?}", object.fields);
    };
    assert_eq!(field.field.name, "value");
    assert_eq!(slice(&text, field.span), "value = self.value;");
    assert_eq!(
        field.value.spans.at(&[]).map(|span| slice(&text, span)),
        Some("self.value")
    );

    let steps: Vec<(&str, &StepTargetForm)> = form
        .rows
        .iter()
        .filter_map(|row| match row {
            RefinementRowForm::Step(step) => Some((source_name(&step.source).1, &step.target)),
            _ => None,
        })
        .collect();
    let concrete: Vec<&str> = steps.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        concrete,
        [
            "Impl::Counter::commitA",
            "Impl::Counter::commitB",
            "Impl::Counter::beginA",
            "Impl::Counter::beginB",
            "Impl::Counter::retryA",
            "Impl::Counter::retryB",
            "Impl::Counter::peek",
        ]
    );
    let StepTargetForm::Apply {
        operation,
        arguments,
        ..
    } = steps[0].1
    else {
        panic!("commitA applies an operation: {:?}", steps[0].1);
    };
    assert_eq!(operation.name, "Spec::Counter::inc");
    let [ArgumentForm::Expression(receiver)] = &arguments[..] else {
        panic!("the receiver alone: {arguments:?}");
    };
    assert!(matches!(receiver.expression.root_node(), ExprNode::SelfRef));
    assert!(matches!(steps[2].1, StepTargetForm::Stutter { .. }));
    assert!(matches!(steps[6].1, StepTargetForm::Stutter { .. }));

    let RefinementRowForm::Fairness(assume) = &form.rows[9] else {
        panic!("an assume row");
    };
    assert_eq!(assume.side, FairnessSide::Assume);
    assert_eq!(assume.constraint.kind, Some(FairnessKind::Weak));
    assert_eq!(
        assume.constraint.granularity,
        Some(FairnessGranularity::Each)
    );
    assert_eq!(assume.constraint.operation.name, "Impl::Counter::beginA");
    let RefinementRowForm::Fairness(ensure) = &form.rows[15] else {
        panic!("an ensure row");
    };
    assert_eq!(ensure.side, FairnessSide::Ensure);
    assert_eq!(
        ensure.constraint.granularity,
        Some(FairnessGranularity::Each)
    );
    assert_eq!(
        slice(&text, ensure.constraint.span),
        "ensure fair weak each Spec::Counter::inc;"
    );
}

/// Part of FR-310-AC-2 (TC-889 step 2; tagged TC-only while the `seq` and
/// `only` forms and `Protocol(Spec, Twice)` are unbuilt): an open argument builds as `Open` beside the
/// receiver expression and an expression argument.
#[trace("TC-889")]
#[test]
fn an_open_argument_builds_open() {
    let (text, unit) = build(
        "refinement RingIsQueue using inf abstract Spec concrete Impl {\n\
         step Impl::Slot::put -> Spec::Queue::enq(self.ring, _, self.index = 0);\n\
         }",
    );
    let form = refinement(&unit);
    let RefinementRowForm::Step(step) = &form.rows[0] else {
        panic!("a step row first");
    };
    let StepTargetForm::Apply { arguments, .. } = &step.target else {
        panic!("an application: {:?}", step.target);
    };
    let [ArgumentForm::Expression(receiver), ArgumentForm::Open { span }, ArgumentForm::Expression(third)] =
        &arguments[..]
    else {
        panic!("receiver, open, expression: {arguments:?}");
    };
    assert_eq!(
        receiver.spans.at(&[]).map(|span| slice(&text, span)),
        Some("self.ring")
    );
    assert_eq!(slice(&text, *span), "_");
    assert_eq!(
        third.spans.at(&[]).map(|span| slice(&text, span)),
        Some("self.index = 0")
    );
}

/// Part of FR-310-AC-2 (TC-889 step 2; tagged TC-only while the `seq` and
/// `only` forms and `Protocol(Spec, Twice)` are unbuilt): a step source builds as `Named` with the
/// keyword exactly as written -- `fork` for a protocol step row, none for a
/// bare name whichever class it later resolves to -- or as `Memory`; a
/// protocol step row's `node` target builds as `Node` with its binder
/// expressions. The abstract protocol side (`abstract Spec::Twice`) has no
/// QSpec production yet (Linear STD-182), so its form is not built from
/// source here.
#[trace("TC-889")]
#[test]
fn step_sources_and_node_targets_build_as_written() {
    let (_, unit) = build(
        "refinement CasTwice using inf abstract Spec concrete Impl {\n\
         step fork Impl::P::par -> node Spec::Twice::a1;\n\
         step Impl::P::a::S -> stutter;\n\
         step memory flush -> stutter;\n\
         step Impl::Counter::commitA -> node Spec::Twice::a1(self, _);\n\
         step activate Impl::P::t -> any;\n\
         }",
    );
    let rows = &refinement(&unit).rows;
    let step = |index: usize| match &rows[index] {
        RefinementRowForm::Step(step) => step,
        other => panic!("a step row: {other:?}"),
    };
    assert_eq!(
        source_name(&step(0).source),
        (Some(StepClassKeyword::Fork), "Impl::P::par")
    );
    let StepTargetForm::Node {
        node, arguments, ..
    } = &step(0).target
    else {
        panic!("a node target: {:?}", step(0).target);
    };
    assert_eq!(node.name, "Spec::Twice::a1");
    assert!(arguments.is_empty());
    assert_eq!(source_name(&step(1).source), (None, "Impl::P::a::S"));
    let StepSourceForm::Memory(kind) = &step(2).source else {
        panic!("a memory source: {:?}", step(2).source);
    };
    assert_eq!(kind.name, "flush");
    let StepTargetForm::Node { arguments, .. } = &step(3).target else {
        panic!("a node target: {:?}", step(3).target);
    };
    assert!(matches!(
        &arguments[..],
        [ArgumentForm::Expression(_), ArgumentForm::Open { .. }]
    ));
    assert_eq!(
        source_name(&step(4).source),
        (Some(StepClassKeyword::Activate), "Impl::P::t")
    );
}

/// FR-310-AC-4 (TC-889 step 3): a history row's initial value builds as the
/// `HistoryLiteralForm` of its spelling, never as an expression form, and
/// its updates and declared type are kept; a fairness row `assume fair
/// strong ...` builds kind `Strong` with the granularity absent.
#[trace("TC-889", "FR-310-AC-4")]
#[test]
fn history_literals_and_a_strong_fairness_row_build_as_written() {
    let literals = [
        "-3",
        "true",
        "none",
        "\"a\"",
        "rational(1, 2)",
        "Impl::Mode::idle",
    ];
    let rows: String = literals
        .iter()
        .enumerate()
        .map(|(index, literal)| {
            format!("history Impl::Counter.h{index} : Int[0, 4] = {literal} {{ on Impl::Counter::write : pre(self.value); }}\n")
        })
        .collect();
    let (text, unit) = build(&format!(
        "refinement R using inf abstract Spec concrete Impl {{\n{rows}\
         assume fair strong Impl::Counter::retryA;\n}}"
    ));
    let form = refinement(&unit);
    let initials: Vec<&HistoryLiteralForm> = form
        .rows
        .iter()
        .filter_map(|row| match row {
            RefinementRowForm::History(history) => Some(&history.initial),
            _ => None,
        })
        .collect();
    let [integer, boolean, none, string, exact, member] = &initials[..] else {
        panic!("six history rows: {initials:?}");
    };
    assert_eq!(
        *integer,
        &HistoryLiteralForm::Integer(Integer::from(-3_i64))
    );
    assert_eq!(*boolean, &HistoryLiteralForm::Boolean(true));
    assert_eq!(*none, &HistoryLiteralForm::None);
    assert_eq!(*string, &HistoryLiteralForm::Text("a".to_owned()));
    let HistoryLiteralForm::Exact(exact) = exact else {
        panic!("an exact literal: {exact:?}");
    };
    assert_eq!(exact.kind, ExactNumberKind::Rational);
    assert_eq!(exact.first, Integer::from(1_i64));
    assert_eq!(exact.second, Integer::from(2_i64));
    assert!(
        matches!(member, HistoryLiteralForm::EnumMember(name) if name.name == "Impl::Mode::idle")
    );
    let RefinementRowForm::History(first) = &form.rows[0] else {
        panic!("a history row first");
    };
    assert_eq!(first.object.name, "Impl::Counter");
    assert_eq!(first.field.name, "h0");
    assert_eq!(slice(&text, first.field.span), "h0");
    let [update] = &first.updates[..] else {
        panic!("one update: {:?}", first.updates);
    };
    assert_eq!(update.operation.name, "Impl::Counter::write");
    let RefinementRowForm::Fairness(strong) = form.rows.last().expect("rows") else {
        panic!("a fairness row last");
    };
    assert_eq!(strong.side, FairnessSide::Assume);
    assert_eq!(strong.constraint.kind, Some(FairnessKind::Strong));
    assert_eq!(strong.constraint.granularity, None);
}

/// A fairness row builds the granularity as written (`whole` and `each`);
/// the kind is always written, because S1 refuses a kindless refinement row
/// (the weak default is a temporal clause's, QSpec FR-375).
#[trace("TC-889")]
#[test]
fn a_fairness_row_builds_its_kind_and_granularity_and_a_kindless_row_builds_no_form() {
    let (_, unit) = build(
        "refinement R using inf abstract Spec concrete Impl {\n\
         assume fair strong whole Impl::Slot::put;\n\
         ensure fair weak Spec::Queue::deq;\n\
         }",
    );
    let rows = &refinement(&unit).rows;
    let [RefinementRowForm::Fairness(strong), RefinementRowForm::Fairness(weak)] = &rows[..] else {
        panic!("two fairness rows: {rows:?}");
    };
    assert_eq!(strong.constraint.kind, Some(FairnessKind::Strong));
    assert_eq!(
        strong.constraint.granularity,
        Some(FairnessGranularity::Whole)
    );
    assert_eq!(weak.constraint.kind, Some(FairnessKind::Weak));
    assert_eq!(weak.constraint.granularity, None);

    let text = format!(
        "{HEADER}refinement R using inf abstract Spec concrete Impl {{ ensure fair Spec::Queue::deq; }}\n"
    );
    let parsed = parse(&text);
    assert!(!parsed.is_admissible());
    assert_eq!(
        build_unit(&parsed).unwrap_err().cause,
        FormsCause::RecoveringCst
    );
}

/// FR-310-AC-3 (TC-889 step 3, the S2 half): a declaration whose abstract
/// alias names no model, and one with a duplicate `population` row, both
/// build forms with every row kept; S2 resolves no name and merges no row.
#[trace("TC-889", "FR-310-AC-3")]
#[test]
fn an_unresolvable_alias_and_a_duplicate_row_still_build_forms() {
    let (_, unit) = build(
        "refinement Nowhere using inf abstract Nope concrete Impl {\n\
         population Spec::counters from Impl::counters;\n\
         }",
    );
    assert_eq!(abstract_model(refinement(&unit)).name, "Nope");

    let (_, unit) = build(
        "refinement Twice using inf abstract Spec concrete Impl {\n\
         population Spec::counters from Impl::counters;\n\
         population Spec::counters from Impl::counters;\n\
         }",
    );
    let rows = &refinement(&unit).rows;
    let [RefinementRowForm::Population(first), RefinementRowForm::Population(second)] = &rows[..]
    else {
        panic!("two population rows: {rows:?}");
    };
    assert_ne!(first.span, second.span);
}

/// A refinement whose CST holds a recovery node -- a row with no closing `;`
/// -- refuses `FormsCause::RecoveringCst`, with no partial form.
#[trace("TC-889")]
#[test]
fn a_recovering_refinement_builds_no_form() {
    for declaration in [
        "refinement R using inf abstract Spec concrete Impl { step Impl::C::a -> stutter }",
        "refinement R using inf abstract Spec concrete Impl { population Spec::p Impl::p; }",
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

/// Building the same unit twice gives equal forms.
#[trace("TC-889")]
#[test]
fn building_a_refinement_twice_gives_equal_forms() {
    let (_, first) = build(CAS_REFINES_COUNTER);
    let (_, second) = build(CAS_REFINES_COUNTER);
    assert_eq!(refinement(&first), refinement(&second));
}

/// FR-139: the six population-valued forms build as `Population` and `Seq`
/// nodes in a field row, with the condition and body as children in source
/// order and the population as its written qualified name.
#[trace("TC-544")]
#[test]
fn population_forms_build_in_a_field_row() {
    let (_, unit) = build(
        "refinement RingIsQueue using inf abstract Spec concrete Impl {\n\
         object Spec::Queue from Impl::Ring {\n\
           a = count(s in Impl::slots where s.ring = self);\n\
           b = sum(s in Impl::slots: s.value);\n\
           c = exists(s in Impl::slots);\n\
           d = all(s in Impl::slots where s.value = 1);\n\
           e = only(s in Impl::slots where s.ring = self: s.value);\n\
           f = seq(i in 0 .. self.size - 1: i);\n\
         }\n\
         }",
    );
    let RefinementRowForm::Object(object) = &refinement(&unit).rows[0] else {
        panic!("an object row");
    };
    let shape = |index: usize| {
        let expression = &object.fields[index].value.expression;
        match expression.root_node() {
            ExprNode::Population {
                form,
                binder,
                population,
                condition,
                body,
                ..
            } => format!(
                "{form:?} {binder} {population} {} {}",
                condition.is_some(),
                body.is_some()
            ),
            ExprNode::Seq { binder, .. } => format!("Seq {binder}"),
            other => panic!("a population form: {other:?}"),
        }
    };
    assert_eq!(shape(0), "Count s Impl::slots true false");
    assert_eq!(shape(1), "Sum s Impl::slots false true");
    assert_eq!(shape(2), "Exists s Impl::slots false false");
    assert_eq!(shape(3), "All s Impl::slots true false");
    assert_eq!(shape(4), "Only s Impl::slots true true");
    assert_eq!(shape(5), "Seq i");
}
