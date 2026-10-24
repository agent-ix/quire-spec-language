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

/// Verifies the parse only and claims no criterion.
/// A protocol clause holds one
/// `terminal` and one `scheduling adversarial` member after its relationships
/// and channels, and starts on `on each`.
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

/// Verifies the parse only and claims no criterion.
/// A `terminal` member without `when`/`any`, and a
/// `scheduling` member that names no `adversarial`, fail at the token the
/// grammar refuses.
#[test]
fn a_malformed_terminal_or_scheduling_member_is_refused_at_its_token() {
    let run = "sequence Main { }";
    assert_eq!(refused_at(&protocol("on origin", "terminal;", run)), ";");
    assert_eq!(
        refused_at(&protocol("on origin", "scheduling fair;", run)),
        "fair"
    );
}

/// Verifies the parse only and claims no criterion.
/// A `parallel` takes an optional `memory sc | tso | ra` clause
/// between its name and its branches; a `parallel` without one still parses.
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

/// Verifies the parse only and claims no criterion.
/// A memory model other than `sc`, `tso` and `ra` is refused at
/// the model word.
#[test]
fn an_unknown_memory_model_is_refused_at_the_model_word() {
    let run = "parallel P memory pso { branch a sequence A { } branch b sequence B { } } join all [a, b];";
    assert_eq!(refused_at(&protocol("on origin", "", run)), "pso");
}

/// Verifies the parse only and claims no criterion.
/// An attempt takes `ordering` with any of the five memory
/// orders, between its operation and `contracts`; one without it still parses.
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

/// Verifies the parse only and claims no criterion.
/// A `fence` control takes `ordering` with `acquire`, `release`,
/// `acq_rel` or `seq_cst`, and is one more control of a sequence.
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

/// A `fence` with `ordering relaxed` is refused at S1, at `relaxed`: QSpec's
/// `fence-order` excludes it (STD-167).
#[trace("TC-665", "FR-220-AC-6")]
#[test]
fn a_relaxed_fence_is_a_parse_refusal_at_the_relaxed_word() {
    let run = "sequence Main { fence F ordering relaxed; }";
    assert_eq!(refused_at(&protocol("on origin", "", run)), "relaxed");
}

/// Verifies the parse only and claims no criterion.
/// A `fence` with no `ordering` is refused at the token that
/// should have started it.
#[test]
fn a_fence_without_an_ordering_is_refused() {
    let run = "sequence Main { fence F; }";
    assert_eq!(refused_at(&protocol("on origin", "", run)), ";");
}

/// Verifies the parse only and claims no criterion.
/// A refinement declaration holds explicit `step`
/// rows whose left side is a concrete operation or a protocol step class
/// (`fork`, `join`, `cend`, `duplicate`, `lose`, `spawn`, `retire`,
/// `activate`, or a node, template, channel or role by name, or `memory`) and
/// whose right side is an abstract application, `stutter`, `any` or `node`.
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
    // Which alternative each row took: the source and target text are the
    // row's two sides exactly, a step-kind word or `memory` sits outside the
    // qualified name, and only an application target holds an operation name.
    let sides: Vec<(&str, &str)> = rows
        .iter()
        .map(|row| {
            let (source, target) = row
                .strip_prefix("step ")
                .and_then(|row| row.strip_suffix(';'))
                .and_then(|row| row.split_once(" -> "))
                .expect("a step row");
            (source, target)
        })
        .collect();
    let sources: Vec<&str> = sides.iter().map(|side| side.0).collect();
    let targets: Vec<&str> = sides.iter().map(|side| side.1).collect();
    assert_eq!(texts(&text, &parsed, Production::StepSource), sources);
    assert_eq!(texts(&text, &parsed, Production::StepTarget), targets);
    assert_eq!(
        texts(&text, &parsed, Production::OperationName),
        ["Spec::Counter::inc", "Spec::Counter::inc"]
    );
    let kinds = [
        "memory ",
        "fork ",
        "join ",
        "cend ",
        "duplicate ",
        "lose ",
        "spawn ",
        "retire ",
        "activate ",
    ];
    let mut names = Vec::new();
    for (source, target) in &sides {
        if source.starts_with("memory ") {
            continue;
        }
        let name = kinds
            .iter()
            .find_map(|kind| source.strip_prefix(kind))
            .unwrap_or(source);
        names.push(name);
        if let Some(node) = target.strip_prefix("node ") {
            names.push(node.split('(').next().expect("a node name"));
        }
    }
    assert_eq!(texts(&text, &parsed, Production::QualifiedName), names);
    assert_eq!(
        texts(&text, &parsed, Production::StepArgument),
        ["1", "_", "_", "1"]
    );
}

/// Verifies the parse only and claims no criterion.
/// A step row without its `->`, without a target, or whose
/// target is neither application, `stutter`, `any` nor `node` is refused at
/// the offending token.
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

fn refinement(rows: &str) -> String {
    format!("refinement R using Complete abstract Spec concrete Impl {{\n{rows}\n}}")
}

/// Verifies the parse only and claims no criterion.
/// A refinement declaration holds `population`, `object` (with field rows),
/// `history` (with updates) and `assume`/`ensure` fairness rows beside its
/// `step` rows, in any order, and each row is one node of its own production.
#[test]
fn a_refinement_parses_population_object_history_and_fairness_rows() {
    let population = "population Spec::Counters from Impl::Counters;";
    let object = "object Spec::Counter from Impl::Counter {\n\
        value = self.cells;\n\
        owner = self.owner;\n\
        }";
    let empty_object = "object Spec::Empty from Impl::Empty { }";
    let step = "step Impl::Counter::peek -> stutter;";
    let history = "history Impl::Counter.prev : Option<Int[0, 4]> = none {\n\
        on Impl::Counter::write : pre(self.value);\n\
        on Impl::Counter::reset : none;\n\
        }";
    let bare_history = "history Impl::Counter.seen : Boolean = false { }";
    let fairness = [
        "assume fair weak Impl::Counter::write;",
        "assume fair weak each Impl::Counter::write;",
        "ensure fair weak whole Spec::Counter::inc;",
        "assume fair strong Impl::Counter::write;",
        "ensure fair strong each Spec::Counter::inc;",
    ];
    let rows = [
        population,
        object,
        empty_object,
        step,
        history,
        bare_history,
        fairness[0],
        fairness[1],
        fairness[2],
        fairness[3],
        fairness[4],
    ];
    let declaration = refinement(&rows.join("\n"));
    let (text, parsed) = parse_admitted(&declaration);
    assert_eq!(
        texts(&text, &parsed, Production::Refinement),
        [declaration.as_str()]
    );
    assert_eq!(
        texts(&text, &parsed, Production::PopulationRow),
        [population]
    );
    assert_eq!(
        texts(&text, &parsed, Production::ObjectRow),
        [object, empty_object]
    );
    assert_eq!(
        texts(&text, &parsed, Production::FieldRow),
        ["value = self.cells;", "owner = self.owner;"]
    );
    assert_eq!(texts(&text, &parsed, Production::StepRow), [step]);
    assert_eq!(
        texts(&text, &parsed, Production::HistoryRow),
        [history, bare_history]
    );
    assert_eq!(
        texts(&text, &parsed, Production::HistoryUpdate),
        [
            "on Impl::Counter::write : pre(self.value);",
            "on Impl::Counter::reset : none;"
        ]
    );
    assert_eq!(texts(&text, &parsed, Production::FairnessRow), fairness);
}

/// Verifies the parse only and claims no criterion.
/// A history row's initial value is a closed list of constant spellings:
/// a negative or non-negative integer, `true`, `false`, `none`, a string, an
/// exact number or a full-path enum member, each read as one `HistoryLiteral`.
#[test]
fn a_history_row_parses_each_initial_literal_spelling() {
    let literals = [
        "0",
        "-3",
        "true",
        "false",
        "none",
        "\"idle\"",
        "rational(1, 2)",
        "decimal(15, 1)",
        "Impl::Mode::idle",
    ];
    let rows: Vec<String> = literals
        .iter()
        .enumerate()
        .map(|(index, literal)| format!("history Impl::T.h{index} : Int[0, 4] = {literal} {{ }}"))
        .collect();
    let (text, parsed) = parse_admitted(&refinement(&rows.join("\n")));
    assert_eq!(texts(&text, &parsed, Production::HistoryLiteral), literals);
}

/// Verifies the parse only and claims no criterion.
/// A collection, record, float or other non-constant initial value is refused
/// at its first token as a syntax error.
#[test]
fn a_history_row_refuses_a_non_literal_initial_value_at_its_first_token() {
    for (initial, offending) in [
        ("[1, 2]", "["),
        ("set[1]", "["),
        ("float64(bits: 0x3FF0000000000000)", "float64"),
        ("self.x", "self"),
        ("1 + 2", "+"),
    ] {
        let declaration = refinement(&format!("history Impl::T.h : Int[0, 4] = {initial} {{ }}"));
        assert_eq!(refused_at(&declaration), offending, "{initial}");
    }
}

/// Verifies the parse only and claims no criterion.
/// A fairness row must write its kind (`weak` or `strong`; the weak default of
/// a temporal clause does not apply), a granularity only before the operation,
/// and a row that is neither `assume` nor `ensure` is refused at its word.
#[test]
fn a_malformed_fairness_row_is_refused_at_its_token() {
    for (row, offending) in [
        ("assume fair Impl::T::op;", "Impl"),
        ("ensure fair weak Spec::T::op each;", "each"),
        ("require fair weak Impl::T::op;", "require"),
    ] {
        assert_eq!(refused_at(&refinement(row)), offending, "{row}");
    }
}

/// Verifies the parse only and claims no criterion.
/// A population row needs both model names and its `from`; an object row
/// needs its braces and each field row its `=`.
#[test]
fn a_malformed_population_object_or_field_row_is_refused_at_its_token() {
    for (row, offending) in [
        ("population Spec::P;", ";"),
        ("population Spec::P Impl::P;", "Impl"),
        ("object Spec::T from Impl::T;", ";"),
        ("object Spec::T from Impl::T { value self.v; }", "self"),
        ("object Spec::T from Impl::T { value = ; }", ";"),
        ("history Impl::T.h : Boolean = false;", ";"),
        ("history Impl::T : Boolean = false { }", ":"),
    ] {
        assert_eq!(refused_at(&refinement(row)), offending, "{row}");
    }
}

/// QSpec FR-375's `abstract-subject`: `abstract A` is a model alias and
/// `abstract A::protocol` a model alias and a protocol clause name. The
/// parser tells them apart by the `::` and resolves neither.
#[test]
fn a_refinement_header_parses_a_model_or_a_protocol_abstract_subject() {
    for header in [
        "abstract Spec concrete Impl",
        "abstract Spec::Twice concrete Impl",
        // `member-name` admits reserved words, so the protocol may be one.
        "abstract Spec::concrete concrete Impl",
    ] {
        let declaration = format!("refinement R using Complete {header} {{\n}}");
        let (text, parsed) = parse_admitted(&declaration);
        assert_eq!(
            texts(&text, &parsed, Production::Refinement),
            [declaration.as_str()]
        );
    }
}

/// A third segment, a leading or trailing `::` and a protocol name that is
/// no `member-name` are refused at the offending token. The token after `::`
/// is always the protocol name, so `abstract Spec:: concrete Impl {` reads
/// `concrete` as the name and refuses at `Impl`.
#[test]
fn a_malformed_abstract_subject_is_refused_at_its_token() {
    for (header, offending) in [
        ("abstract Spec::Twice::Extra concrete Impl", "::"),
        ("abstract ::Twice concrete Impl", "::"),
        ("abstract Spec:: concrete Impl", "Impl"),
        ("abstract Spec:: {", "{"),
        ("abstract Spec::1 concrete Impl", "1"),
    ] {
        let declaration = format!("refinement R using Complete {header} {{\n}}");
        assert_eq!(refused_at(&declaration), offending, "{header}");
    }
}
