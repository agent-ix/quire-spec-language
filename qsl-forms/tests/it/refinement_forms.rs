// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-310: S2 builds the `refinement` declaration form -- header, then
//! `population`, `object`, `step` and `assume`/`ensure` rows in source order
//! -- run on real complete-V1 source through `qsl_cst::parse` and
//! `build_unit` (TC-889). S2 resolves no name.

use ix_trace_rs::trace;
use qsl_forms::{
    build_unit, ArgumentForm, DeclarationForm, ExprNode, FairnessGranularity, FairnessKind,
    FairnessSide, FormsCause, ParsedUnit, RefinementForm, RefinementRowForm, StepTargetForm,
};
use qsl_foundation::{SourceIdentity, Span};

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
     step Impl::Counter::peek -> any;\n\
     assume fair weak each Impl::Counter::beginA;\n\
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
    assert_eq!(form.abstract_model.name, "Spec");
    assert_eq!(slice(&text, form.abstract_model.span), "Spec");
    assert_eq!(form.concrete_model.name, "Impl");
    assert_eq!(slice(&text, form.span), CAS_REFINES_COUNTER);

    let kinds: Vec<&str> = form
        .rows
        .iter()
        .map(|row| match row {
            RefinementRowForm::Population(_) => "population",
            RefinementRowForm::Object(_) => "object",
            RefinementRowForm::Step(_) => "step",
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
            RefinementRowForm::Step(step) => Some((step.concrete.name.as_str(), &step.target)),
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
    assert!(matches!(steps[6].1, StepTargetForm::Any { .. }));

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
    let RefinementRowForm::Fairness(ensure) = &form.rows[10] else {
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

/// An open argument builds as `Open` beside the receiver expression and an
/// expression argument; an unwritten kind and granularity stay unwritten.
#[trace("TC-889")]
#[test]
fn an_open_argument_builds_open_and_unwritten_fairness_stays_unwritten() {
    let (text, unit) = build(
        "refinement RingIsQueue using inf abstract Spec concrete Impl {\n\
         step Impl::Slot::put -> Spec::Queue::enq(self.ring, _, self.index = 0);\n\
         ensure fair Spec::Queue::deq;\n\
         assume fair strong whole Impl::Slot::put;\n\
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

    let RefinementRowForm::Fairness(unmarked) = &form.rows[1] else {
        panic!("a fairness row");
    };
    assert_eq!(unmarked.side, FairnessSide::Ensure);
    assert_eq!(unmarked.constraint.kind, None);
    assert_eq!(unmarked.constraint.granularity, None);
    let RefinementRowForm::Fairness(strong) = &form.rows[2] else {
        panic!("a fairness row");
    };
    assert_eq!(strong.constraint.kind, Some(FairnessKind::Strong));
    assert_eq!(
        strong.constraint.granularity,
        Some(FairnessGranularity::Whole)
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
    assert_eq!(refinement(&unit).abstract_model.name, "Nope");

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
