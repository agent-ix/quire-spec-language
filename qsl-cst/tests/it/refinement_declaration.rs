// SPDX-License-Identifier: AGPL-3.0-or-later
//! The shared grammar's `refinement` declaration -- header, then
//! `population`, `object`, `step` and `assume`/`ensure` rows -- parses to one
//! `RefinementDeclaration` node with a node per row, and a malformed row
//! fails at the first token the grammar does not admit (TC-889).
use ix_trace_rs::trace;
use qsl_cst::{CompleteCode, Limits, ParsedSource, Production};
use qsl_foundation::{SourceIdentity, Span};

fn source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\";\n{declarations}"
    )
}

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity {
            authority: "test".into(),
            identity: "test:refinement".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "refinement.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn slices<'t>(parsed: &ParsedSource, text: &'t str, production: Production) -> Vec<&'t str> {
    parsed
        .cst()
        .nodes()
        .iter()
        .filter(|node| node.production() == production)
        .map(|node| {
            let Span { start, end } = node.span();
            &text[start..end]
        })
        .collect()
}

/// TC-889 step 1: every row of the compare-and-set counter's refinement
/// parses to its own node, the step targets and fairness rows included.
#[trace("TC-889")]
#[test]
fn a_refinement_parses_to_one_node_per_row() {
    let text = source(concat!(
        "refinement CasRefinesCounter using Complete abstract Spec concrete Impl {\n",
        "population Spec::counters from Impl::counters;\n",
        "object Spec::Counter from Impl::Counter { value = self.value; }\n",
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);\n",
        "step Impl::Counter::beginA -> stutter;\n",
        "step Impl::Counter::peek -> any;\n",
        "assume fair weak each Impl::Counter::beginA;\n",
        "ensure fair strong Spec::Counter::inc;\n",
        "ensure fair Spec::Counter::inc;\n",
        "}",
    ));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());

    assert_eq!(
        slices(&parsed, &text, Production::PopulationRow),
        ["population Spec::counters from Impl::counters;"]
    );
    assert_eq!(
        slices(&parsed, &text, Production::ObjectRow),
        ["object Spec::Counter from Impl::Counter { value = self.value; }"]
    );
    assert_eq!(
        slices(&parsed, &text, Production::FieldRow),
        ["value = self.value;"]
    );
    assert_eq!(
        slices(&parsed, &text, Production::StepRow),
        [
            "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
            "step Impl::Counter::beginA -> stutter;",
            "step Impl::Counter::peek -> any;",
        ]
    );
    assert_eq!(
        slices(&parsed, &text, Production::StepTarget),
        ["Spec::Counter::inc(self)", "stutter", "any"]
    );
    assert_eq!(
        slices(&parsed, &text, Production::FairnessRow),
        [
            "assume fair weak each Impl::Counter::beginA;",
            "ensure fair strong Spec::Counter::inc;",
            "ensure fair Spec::Counter::inc;",
        ]
    );
}

/// TC-889 step 2: an open argument `_` is a `StepArgument` of its own, beside
/// the expression arguments, and the receiver is not one.
#[trace("TC-889")]
#[test]
fn an_open_step_argument_is_a_step_argument_node() {
    let text = source(concat!(
        "refinement RingIsQueue using Complete abstract Q concrete R {\n",
        "step R::Slot::put -> Q::Queue::enq(self.ring, _);\n",
        "step R::Slot::take -> Q::Queue::deq(self.ring);\n",
        "}",
    ));
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    assert_eq!(slices(&parsed, &text, Production::StepArgument), ["_"]);
}

/// TC-889 step 3: a row that omits a part the grammar requires fails at the
/// token that cannot continue it, reported as invalid syntax.
#[trace("TC-889")]
#[test]
fn a_malformed_refinement_row_fails_at_the_token_the_grammar_refuses() {
    for (row, offending) in [
        ("step C::T::go stutter;", "stutter"),
        ("step C::T::go -> ;", ";"),
        ("step C::T::go -> A::T::inc();", ")"),
        ("population A::p C::p;", "C"),
        ("object A::T from C::T { value self.value; }", "self"),
        ("object A::T from C::T { value = self.value }", "}"),
        ("assume weak C::T::go;", "weak"),
        ("ensure fair weak ;", ";"),
    ] {
        let text = source(&format!(
            "refinement R using Complete abstract A concrete C {{\n{row}\n}}"
        ));
        let parsed = parse(&text);
        assert!(!parsed.is_admissible(), "{row}");
        let diagnostic = &parsed.diagnostics()[0];
        assert_eq!(diagnostic.code, CompleteCode::InvalidSyntax, "{row}");
        let located = diagnostic.byte_span().expect("located");
        assert_eq!(&text[located.start..located.end], offending, "{row}");
    }
}
