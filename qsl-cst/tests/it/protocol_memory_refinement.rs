// SPDX-License-Identifier: AGPL-3.0-or-later
//! The grammar forms of the protocol, memory and refinement-row slice: a
//! protocol clause's `terminal` and `scheduling` members and `on each`
//! activation (FR-211, FR-212), a `parallel`'s `memory` clause (FR-219), an
//! attempt's `ordering` and the `fence` control (FR-220), and a refinement
//! declaration's `step` rows (FR-136, FR-214). Each test reads the CST nodes
//! the parser produced; the checks those forms feed belong to later slices.
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
            identity: "test:protocol-memory-refinement".into(),
            revision_namespace: "test".into(),
            revision: "1".into(),
        },
        "protocol.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("S1 reads the unit")
}

fn parse_admitted(declarations: &str) -> (String, ParsedSource) {
    let text = source(declarations);
    let parsed = parse(&text);
    assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
    (text, parsed)
}

fn texts<'t>(text: &'t str, parsed: &ParsedSource, production: Production) -> Vec<&'t str> {
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

/// A protocol around `run`, with `members` between its roles and `run`.
fn protocol(activation: &str, members: &str, run: &str) -> String {
    format!(
        "protocol Flow using Complete over (input: M::Input) {activation} {{\n\
         role R on M::Actor;\n\
         {members}\n\
         run {run}\n\
         finish End as (outcome: Boolean) {{ true }};\n\
         }}"
    )
}

/// The first token a refused declaration fails at.
fn refused_at(declaration: &str) -> String {
    let text = source(declaration);
    let parsed = parse(&text);
    assert!(!parsed.is_admissible(), "{declaration}");
    let diagnostic = &parsed.diagnostics()[0];
    assert_eq!(
        diagnostic.code,
        CompleteCode::InvalidSyntax,
        "{declaration}"
    );
    let Span { start, end } = diagnostic.byte_span().expect("located");
    text[start..end].to_owned()
}

/// FR-211-AC-1, FR-211-AC-4, FR-212-AC-2: a protocol clause holds one
/// `terminal` and one `scheduling adversarial` member after its relationships
/// and channels, and starts on `on each`.
#[trace("FR-211-AC-1", "FR-211-AC-4", "FR-212-AC-2")]
#[test]
fn a_protocol_parses_terminal_scheduling_and_on_each_activation() {
    let members = "terminal any;\nscheduling adversarial;";
    let run = "sequence Main { }";
    let (text, parsed) = parse_admitted(&protocol(
        "on each (trigger: M::Trigger) when (true)",
        members,
        run,
    ));
    assert_eq!(
        texts(&text, &parsed, Production::Activation),
        ["on each (trigger: M::Trigger) when (true)"]
    );
    assert_eq!(
        texts(&text, &parsed, Production::ProtocolTerminal),
        ["terminal any;"]
    );
    assert_eq!(
        texts(&text, &parsed, Production::ProtocolScheduling),
        ["scheduling adversarial;"]
    );

    let (text, parsed) = parse_admitted(&protocol("on origin", "terminal when { true };", run));
    assert_eq!(
        texts(&text, &parsed, Production::ProtocolTerminal),
        ["terminal when { true };"]
    );
    assert!(texts(&text, &parsed, Production::ProtocolScheduling).is_empty());
}

/// FR-211-AC-1, FR-212-AC-2: a `terminal` member without `when`/`any`, and a
/// `scheduling` member that names no `adversarial`, fail at the token the
/// grammar refuses.
#[trace("FR-211-AC-1", "FR-212-AC-2")]
#[test]
fn a_malformed_terminal_or_scheduling_member_is_refused_at_its_token() {
    let run = "sequence Main { }";
    assert_eq!(refused_at(&protocol("on origin", "terminal;", run)), ";");
    assert_eq!(
        refused_at(&protocol("on origin", "scheduling fair;", run)),
        "fair"
    );
}

/// FR-219-AC-1: a `parallel` takes an optional `memory sc | tso | ra` clause
/// between its name and its branches; a `parallel` without one still parses.
#[trace("FR-219-AC-1")]
#[test]
fn a_parallel_parses_its_memory_clause() {
    let branches = "{ branch a sequence A { } branch b sequence B { } } join all [a, b];";
    let run = format!(
        "sequence Main {{\n\
         parallel Weak memory tso {branches}\n\
         parallel Acquire memory ra {branches}\n\
         parallel Strong memory sc {branches}\n\
         parallel Plain {branches}\n\
         }}"
    );
    let (text, parsed) = parse_admitted(&protocol("on origin", "", &run));
    assert_eq!(
        texts(&text, &parsed, Production::MemoryClause),
        ["memory tso", "memory ra", "memory sc"]
    );
    assert_eq!(texts(&text, &parsed, Production::Parallel).len(), 4);
}

/// FR-219-AC-1: a memory model other than `sc`, `tso` and `ra` is refused at
/// the model word.
#[trace("FR-219-AC-1")]
#[test]
fn an_unknown_memory_model_is_refused_at_the_model_word() {
    let run = "parallel P memory pso { branch a sequence A { } branch b sequence B { } } join all [a, b];";
    assert_eq!(refused_at(&protocol("on origin", "", run)), "pso");
}

/// FR-220-AC-1: an attempt takes `ordering` with any of the five memory
/// orders, between its operation and `contracts`; one without it still parses.
#[trace("FR-220-AC-1")]
#[test]
fn an_attempt_parses_each_memory_order() {
    let attempt = |name: &str, ordering: &str| {
        format!(
            "attempt {name} by R on M::Actor::act {ordering} contracts [] as (b{name}: M::Attempt) {{ true }};"
        )
    };
    let orders = ["relaxed", "acquire", "release", "acq_rel", "seq_cst"];
    let mut body = String::new();
    for (index, order) in orders.iter().enumerate() {
        body.push_str(&attempt(&format!("A{index}"), &format!("ordering {order}")));
        body.push('\n');
    }
    body.push_str(&attempt("Bare", ""));
    let run = format!("sequence Main {{\n{body}\n}}");
    let (text, parsed) = parse_admitted(&protocol("on origin", "", &run));
    assert_eq!(texts(&text, &parsed, Production::MemoryOrder), orders);
    assert_eq!(texts(&text, &parsed, Production::EventNode).len(), 6);
}

/// FR-220-AC-2: a `fence` control takes `ordering` with `acquire`, `release`,
/// `acq_rel` or `seq_cst`, and is one more control of a sequence.
#[trace("FR-220-AC-2")]
#[test]
fn a_fence_parses_each_fence_order() {
    let run = "sequence Main {\n\
         fence F1 ordering acquire;\n\
         fence F2 ordering release;\n\
         fence F3 ordering acq_rel;\n\
         fence F4 ordering seq_cst;\n\
         }";
    let (text, parsed) = parse_admitted(&protocol("on origin", "", run));
    assert_eq!(
        texts(&text, &parsed, Production::FenceOrder),
        ["acquire", "release", "acq_rel", "seq_cst"]
    );
    assert_eq!(
        texts(&text, &parsed, Production::Fence),
        [
            "fence F1 ordering acquire;",
            "fence F2 ordering release;",
            "fence F3 ordering acq_rel;",
            "fence F4 ordering seq_cst;",
        ]
    );
}

/// FR-220-AC-2: a `fence` with no `ordering` is refused at the token that
/// should have started it.
#[trace("FR-220-AC-2")]
#[test]
fn a_fence_without_an_ordering_is_refused() {
    let run = "sequence Main { fence F; }";
    assert_eq!(refused_at(&protocol("on origin", "", run)), ";");
}

/// FR-136-AC-1, FR-214-AC-1: a refinement declaration holds explicit `step`
/// rows whose left side is a concrete operation or a protocol step class
/// (`fork`, `join`, `cend`, `duplicate`, `lose`, `spawn`, `retire`,
/// `activate`, or a node, template, channel or role by name, or `memory`) and
/// whose right side is an abstract application, `stutter`, `any` or `node`.
#[trace("FR-136-AC-1", "FR-214-AC-1", "FR-214-AC-4")]
#[test]
fn a_refinement_parses_its_step_rows() {
    let rows = [
        "step Impl::Counter::commitA -> Spec::Counter::inc(self, 1);",
        "step Impl::Counter::commitB -> Spec::Counter::inc(self, _);",
        "step Impl::Counter::peek -> stutter;",
        "step Impl::Counter::spin -> any;",
        "step fork Fill::Both -> stutter;",
        "step join Fill::Both -> stutter;",
        "step Fill::Done -> stutter;",
        "step cend Fill::Refund -> stutter;",
        "step duplicate Fill::ch -> any;",
        "step lose Fill::ch -> any;",
        "step spawn Fill::Workers -> stutter;",
        "step retire Fill::Workers -> stutter;",
        "step activate Fill::Trigger -> stutter;",
        "step memory flush -> stutter;",
        "step Fill::Main::A -> node Spec::Main::Step(_, 1);",
        "step Fill::Main::B -> node Spec::Main::Step;",
    ];
    let declaration = format!(
        "refinement Simulates using Complete abstract Spec concrete Impl {{\n{}\n}}",
        rows.join("\n")
    );
    let (text, parsed) = parse_admitted(&declaration);
    assert_eq!(
        texts(&text, &parsed, Production::Refinement),
        [declaration.as_str()]
    );
    assert_eq!(texts(&text, &parsed, Production::StepRow), rows);
    assert_eq!(
        texts(&text, &parsed, Production::StepSource).len(),
        rows.len()
    );
    assert_eq!(
        texts(&text, &parsed, Production::StepTarget).len(),
        rows.len()
    );
    assert_eq!(
        texts(&text, &parsed, Production::StepArgument),
        ["1", "_", "_", "1"]
    );
}

/// FR-136-AC-1: a step row without its `->`, without a target, or whose
/// target is neither application, `stutter`, `any` nor `node` is refused at
/// the offending token.
#[trace("FR-136-AC-1")]
#[test]
fn a_malformed_step_row_is_refused_at_its_token() {
    for (row, offending) in [
        ("step Impl::Counter::peek stutter;", "stutter"),
        ("step Impl::Counter::peek -> ;", ";"),
        ("step Impl::Counter::peek -> Spec::Counter::inc;", ";"),
    ] {
        let declaration =
            format!("refinement R using Complete abstract Spec concrete Impl {{\n{row}\n}}");
        assert_eq!(refused_at(&declaration), offending, "{row}");
    }
}
