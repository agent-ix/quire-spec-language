// SPDX-License-Identifier: AGPL-3.0-or-later
//! Differential check of both S1 parsers against a recorded baseline.
//!
//! A seeded generator builds 30,000 shallow units: 10,000 complete-V1,
//! 10,000 historical native and 10,000 composed native, about a third of
//! them mutated into syntax errors. Each outcome is rendered canonically:
//! for the complete-V1 parser every token, every CST node with its
//! production, span and children, recoveries, diagnostics and selections;
//! for the native parser the whole syntax tree or the refusal. Renderings
//! are hashed in buckets of 100 inputs and compared with
//! `tests/fixtures/parser-differential/baseline.txt`.
//!
//! The baseline was recorded by running this file against the parsers
//! before the explicit-stack rewrite (recursive complete-V1 interpreter and
//! recursive-descent native parser). So a green run means the rewrite
//! produced identical trees, spans, recoveries and diagnostics for every
//! generated input. Resource refusals are out of scope: no generated input
//! comes near a ceiling. The default run checks the first 1,000 inputs of
//! each family; `make test-differential` checks all 30,000. To re-record
//! after an intended change, run that target with
//! `QSL_DIFFERENTIAL_RECORD=1` and review the diff.
use std::fmt::Write as _;

use ix_trace_rs::trace;
use qsl_cst::{CstElement, Limits as CompleteLimits};
use qsl_foundation::{ByteDigest, SourceIdentity};
use quire_spec_language::{parse_native, Limits};

/// The full run: 10,000 inputs per family, 300 buckets.
const INPUTS_PER_FAMILY: usize = 10_000;
/// The always-on run: the first 1,000 inputs of each family, 30 buckets.
const QUICK_INPUTS_PER_FAMILY: usize = 1_000;
const BUCKET: usize = 100;
const BASELINE: &str = include_str!("../fixtures/parser-differential/baseline.txt");

/// xorshift64*: a fixed, dependency-free generator so the inputs are the
/// same on every run and every machine.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % u64::try_from(bound).unwrap()).unwrap()
    }
    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[self.below(items.len())]
    }
}

/// The generated value-expression dialect.
#[derive(Clone, Copy, PartialEq)]
enum Dialect {
    Complete,
    Historical,
    Composed,
}

fn atom(rng: &mut Rng, dialect: Dialect) -> String {
    let common = ["x", "y", "true", "false", "0", "1", "42", "\"s\""];
    match dialect {
        Dialect::Complete => rng
            .pick(&[
                "x",
                "y",
                "true",
                "false",
                "0",
                "7",
                "\"t\"",
                "M::a",
                "M::E::v",
                "none",
                "rational(1, 2)",
                "decimal(-1, 2)",
                "float32(bits: 0x3f800000)",
            ])
            .into(),
        Dialect::Historical => rng
            .pick(&[
                "x", "y", "true", "false", "0", "1", "\"s\"", "self", "result", "M::E::V",
            ])
            .into(),
        Dialect::Composed => {
            if rng.chance(20) {
                rng.pick(&["rational(1, 2)", "rational(-3, 4)", "view.paid"])
                    .into()
            } else {
                rng.pick(&common).into()
            }
        }
    }
}

fn expression(rng: &mut Rng, dialect: Dialect, depth: usize) -> String {
    if depth == 0 || rng.chance(25) {
        return atom(rng, dialect);
    }
    let sub = |rng: &mut Rng| expression(rng, dialect, depth - 1);
    let binary = [
        "+", "-", "*", "and", "or", "implies", "=", "!=", "<", "<=", ">", ">=",
    ];
    match rng.below(12) {
        0 => format!("let v = {} in {}", sub(rng), sub(rng)),
        1 => format!("if {} then {} else {}", sub(rng), sub(rng), sub(rng)),
        2 => format!("not {}", sub(rng)),
        3 => format!("- {}", sub(rng)),
        4 => format!("({})", sub(rng)),
        5 => format!("{}.field", sub(rng)),
        6 => match dialect {
            Dialect::Complete => rng
                .pick(&[
                    "size(x)",
                    "contains(x, y)",
                    "f(x, y)",
                    "M::T(x)",
                    "set[x, y]",
                    "sequence[]",
                    "map(v in x: v)",
                    "count<M::T>(v in x: v)",
                    "x[0]",
                    "present(x)",
                    "M::R { a: x, b: y }",
                    "forall(v in x: v)",
                ])
                .into(),
            Dialect::Historical => format!(
                "{}({})",
                rng.pick(&["present", "value", "deref", "size", "pre"]),
                sub(rng)
            ),
            Dialect::Composed => format!("f({}, {})", sub(rng), sub(rng)),
        },
        7 => match dialect {
            Dialect::Complete => format!("{}[{}]", sub(rng), sub(rng)),
            Dialect::Historical => format!(
                "{}(v in {}: {})",
                rng.pick(&["forall", "exists"]),
                sub(rng),
                sub(rng)
            ),
            Dialect::Composed => format!("size<M::T>({})", sub(rng)),
        },
        8 => match dialect {
            Dialect::Historical => format!("reaches({}, {}, next)", sub(rng), sub(rng)),
            Dialect::Composed => format!("contains({}, {})", sub(rng), sub(rng)),
            Dialect::Complete => format!("if {} then {} else {}", sub(rng), sub(rng), sub(rng)),
        },
        9 => match dialect {
            Dialect::Composed => format!(
                "{}(v in {}: {})",
                rng.pick(&["filter", "map", "count<M::T>", "sum<M::T>"]),
                sub(rng),
                sub(rng)
            ),
            Dialect::Complete => format!(
                "{} {} {}",
                sub(rng),
                rng.pick(&["div", "rem", "mod", "/"]),
                sub(rng)
            ),
            Dialect::Historical => {
                format!("{} {} {}", sub(rng), rng.pick(&["div", "rem"]), sub(rng))
            }
        },
        10 if dialect == Dialect::Composed => {
            format!("{} {} {}", sub(rng), rng.pick(&["/", "mod"]), sub(rng))
        }
        _ => format!("{} {} {}", sub(rng), rng.pick(&binary), sub(rng)),
    }
}

fn temporal_formula(rng: &mut Rng, dialect: Dialect, depth: usize) -> String {
    if depth == 0 || rng.chance(25) {
        return if rng.chance(50) {
            rng.pick(&["true", "false"]).into()
        } else {
            format!("holds({})", expression(rng, dialect, 2))
        };
    }
    let sub = |rng: &mut Rng| temporal_formula(rng, dialect, depth - 1);
    match rng.below(6) {
        0 => format!("not {}", sub(rng)),
        1 => format!(
            "{} [0,{}] {}",
            rng.pick(&["always", "eventually", "once", "historically"]),
            rng.below(9),
            sub(rng)
        ),
        2 => format!("({})", sub(rng)),
        3 => format!(
            "{} {} [1,2] {}",
            sub(rng),
            rng.pick(&["until", "release", "since", "triggered"]),
            sub(rng)
        ),
        _ => format!(
            "{} {} {}",
            sub(rng),
            rng.pick(&["implies", "and", "or"]),
            sub(rng)
        ),
    }
}

fn control(rng: &mut Rng, depth: usize) -> String {
    let check = "check C using S { true };".to_string();
    if depth == 0 || rng.chance(30) {
        return check;
    }
    match rng.below(5) {
        0 => format!(
            "sequence Q {{ {} {} }}",
            control(rng, depth - 1),
            control(rng, depth - 1)
        ),
        1 => format!(
            "repeat L by Service visible ({}) max 3 while {{ {} }} {} exhausted {}",
            expression(rng, Dialect::Composed, 1),
            expression(rng, Dialect::Composed, 1),
            control(rng, depth - 1),
            control(rng, depth - 1)
        ),
        2 => format!(
            "choice K by Service visible () {{ case A when {{ true }} {} case B when {{ false }} {} }}",
            control(rng, depth - 1),
            control(rng, depth - 1)
        ),
        3 => format!(
            "parallel P {{ branch A {} branch B {} }} join all [A, B];",
            control(rng, depth - 1),
            control(rng, depth - 1)
        ),
        _ => format!(
            "await W after Q using T clock \"c\" within [0,3] match event E by Service as (e: M::E) {{ true }}; then {} timeout {}",
            control(rng, depth - 1),
            control(rng, depth - 1)
        ),
    }
}

const COMPLETE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\nmodel M = \"acme/model\" version \"1\" digest \"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\";\n";
const HISTORICAL_HEADER: &str = "language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\n";
const COMPOSED_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile S = \"quire.state.graph/v1\" version \"test:state\" digest \"unresolved-state\";\nprofile T = \"quire.temporal.timestamped-event.finite-window/v1\" version \"test:temporal\" digest \"unresolved-temporal\";\nprofile P = \"quire.protocol.finite-global/v1\" version \"test:protocol\" digest \"unresolved-protocol\";\nmodel M = \"test:orders-and-refunds\" version \"test:model\" digest \"unresolved-model\";\n";

fn complete_unit(rng: &mut Rng) -> String {
    let declaration = match rng.below(5) {
        0 | 1 => format!(
            "function f using Complete (x: Integer, y: Integer): Integer pure {{ {} }}",
            expression(rng, Dialect::Complete, 4)
        ),
        2 => format!(
            "temporal W using Complete over (s: Integer) clock \"t\" on origin {{ {} }}",
            temporal_formula(rng, Dialect::Complete, 4)
        ),
        3 => format!(
            "type T = {};",
            rng.pick(&[
                "Option<Integer>",
                "Sequence<Option<Text[0, 3; nfc]>>[0, 4]",
                "Reference<M::Thing>",
                "Set<Decimal[-2, 4; 18, 4; nearest-even]>[1, 2]",
                "M::Thing",
            ])
        ),
        _ => format!(
            "predicate P using Complete (x: Integer): Boolean {{ {} }}",
            expression(rng, Dialect::Complete, 4)
        ),
    };
    format!("{COMPLETE_HEADER}{declaration}\n")
}

fn historical_unit(rng: &mut Rng) -> String {
    format!(
        "{HISTORICAL_HEADER}invariant I on M::Thing at current {{ {} }}\n",
        expression(rng, Dialect::Historical, 4)
    )
}

fn composed_unit(rng: &mut Rng) -> String {
    let declaration = match rng.below(3) {
        0 => format!(
            "invariant I using S on M::OrderView at current {{ {} }}",
            expression(rng, Dialect::Composed, 4)
        ),
        1 => format!(
            "temporal W using T over (view: M::OrderView) clock \"c\" on origin {{ {} }}",
            temporal_formula(rng, Dialect::Composed, 4)
        ),
        _ => format!(
            "protocol O using P over (view: M::OrderView) on origin {{ role Service on M::OrderView; run {} finish Closed as (closed: M::OrderView) {{ true }}; }}",
            control(rng, 3)
        ),
    };
    format!("{COMPOSED_HEADER}{declaration}\n")
}

/// Mutate about a third of the units at one whitespace-separated word:
/// delete it, duplicate it, or insert a stray token after it.
fn mutate(rng: &mut Rng, text: String) -> String {
    if !rng.chance(35) {
        return text;
    }
    let words: Vec<&str> = text.split(' ').collect();
    let at = rng.below(words.len());
    let mut output = Vec::with_capacity(words.len() + 1);
    let operation = rng.below(3);
    for (index, word) in words.iter().enumerate() {
        if index == at && operation == 0 {
            continue;
        }
        output.push(*word);
        if index == at && operation == 1 {
            output.push(word);
        }
        if index == at && operation == 2 {
            output.push(rng.pick(&["(", ")", "in", "then", ",", "not", "+", "[", "}", "x"]));
        }
    }
    output.join(" ")
}

fn identity() -> SourceIdentity {
    SourceIdentity {
        identity: "test:differential".into(),
        revision: "1".into(),
    }
}

fn render_complete(text: &str) -> String {
    let mut out = String::new();
    match qsl_cst::parse(
        identity(),
        "differential.native",
        text.as_bytes(),
        CompleteLimits::default(),
    ) {
        Err(refusal) => {
            let _ = write!(
                out,
                "refused {:?} {:?} {:?} {}..{} {}",
                refusal.code,
                refusal.cause,
                refusal.phase,
                refusal.span.start.byte,
                refusal.span.end.byte,
                refusal.message
            );
        }
        Ok(parsed) => {
            let cst = parsed.cst();
            for token in cst.tokens() {
                let _ = write!(
                    out,
                    "t {:?} {:?} {}..{} {:?};",
                    token.class(),
                    token.kind(),
                    token.span().start,
                    token.span().end,
                    token.spelling()
                );
            }
            for node in cst.nodes() {
                let children: Vec<String> = node
                    .children()
                    .iter()
                    .map(|child| match child {
                        CstElement::Token(index) => format!("t{index}"),
                        CstElement::Node(index) => format!("n{index}"),
                    })
                    .collect();
                let _ = write!(
                    out,
                    "n {:?} {}..{} [{}];",
                    node.production(),
                    node.span().start,
                    node.span().end,
                    children.join(",")
                );
            }
            let _ = write!(out, "root {:?};", cst.root().span());
            for recovery in cst.recoveries() {
                let _ = write!(out, "r {recovery:?};");
            }
            for diagnostic in parsed.diagnostics() {
                let related: Vec<_> = diagnostic
                    .related
                    .iter()
                    .map(|span| (span.start.byte, span.end.byte))
                    .collect();
                let _ = write!(
                    out,
                    "d {:?} {:?} {:?} {}..{} {:?} {};",
                    diagnostic.code,
                    diagnostic.cause,
                    diagnostic.phase,
                    diagnostic.span.start.byte,
                    diagnostic.span.end.byte,
                    related,
                    diagnostic.message
                );
            }
            let _ = write!(out, "s {:?}", parsed.selections());
        }
    }
    out
}

fn render_native(text: &str) -> String {
    match parse_native(
        identity(),
        "differential.native",
        text.as_bytes(),
        Limits::default(),
    ) {
        Ok(unit) => format!("{unit:?}"),
        Err(refusal) => format!(
            "refused {:?} {:?} {}..{} {}",
            refusal.code,
            refusal.phase,
            refusal.span.start.byte,
            refusal.span.end.byte,
            refusal.message
        ),
    }
}

/// One baseline line per bucket: `family bucket digest`.
fn bucket_digests(
    family: &str,
    seed: u64,
    generate: fn(&mut Rng) -> String,
    render: fn(&str) -> String,
    inputs: usize,
) -> Vec<String> {
    let mut rng = Rng(seed);
    let mut lines = Vec::new();
    let mut bucket = Vec::new();
    for index in 0..inputs {
        let text = generate(&mut rng);
        let text = mutate(&mut rng, text);
        bucket.extend_from_slice(render(&text).as_bytes());
        bucket.push(0);
        if (index + 1) % BUCKET == 0 {
            lines.push(format!(
                "{family} {} {}",
                index / BUCKET,
                ByteDigest::of(&bucket)
            ));
            bucket.clear();
        }
    }
    lines
}

fn all_digests(inputs: usize) -> Vec<String> {
    let mut lines = bucket_digests(
        "complete",
        0x5eed_0001,
        complete_unit,
        render_complete,
        inputs,
    );
    lines.extend(bucket_digests(
        "historical",
        0x5eed_0002,
        historical_unit,
        render_native,
        inputs,
    ));
    lines.extend(bucket_digests(
        "composed",
        0x5eed_0003,
        composed_unit,
        render_native,
        inputs,
    ));
    lines
}

/// Compare the first `inputs` of each family with the baseline's buckets
/// for them.
fn assert_matches_baseline(inputs: usize) {
    let lines = all_digests(inputs);
    let buckets = inputs / BUCKET;
    let baseline: Vec<&str> = BASELINE
        .lines()
        .filter(|line| {
            line.split(' ')
                .nth(1)
                .and_then(|bucket| bucket.parse::<usize>().ok())
                .is_some_and(|bucket| bucket < buckets)
        })
        .collect();
    assert_eq!(baseline.len(), lines.len(), "baseline bucket count");
    let differing: Vec<&str> = lines
        .iter()
        .zip(&baseline)
        .filter(|(now, then)| now.as_str() != **then)
        .map(|(now, _)| now.as_str())
        .collect();
    assert!(
        differing.is_empty(),
        "{} buckets differ from the baseline, first: {:?}",
        differing.len(),
        differing.first()
    );
}

#[trace("TC-012", "TC-222", "FR-302-AC-1", "FR-302-AC-2")]
#[test]
fn both_parsers_match_the_recorded_baseline_on_generated_units() {
    assert_matches_baseline(QUICK_INPUTS_PER_FAMILY);
}

// The whole baseline: `make test-differential`.
#[trace("TC-012", "TC-222", "FR-302-AC-1", "FR-302-AC-2")]
#[test]
#[ignore = "30,000 inputs; run by `make test-differential`"]
fn both_parsers_match_the_whole_recorded_baseline() {
    if std::env::var_os("QSL_DIFFERENTIAL_RECORD").is_some() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/parser-differential/baseline.txt"
        );
        let lines = all_digests(INPUTS_PER_FAMILY);
        std::fs::write(path, lines.join("\n") + "\n").expect("record the baseline");
        return;
    }
    assert_matches_baseline(INPUTS_PER_FAMILY);
}

// The generator must exercise both outcomes and every family.
#[trace("TC-012")]
#[test]
fn generated_units_include_admitted_and_refused_parses() {
    let mut rng = Rng(0x5eed_0001);
    let (mut admitted, mut failed) = (0, 0);
    for _ in 0..500 {
        let text = complete_unit(&mut rng);
        let text = mutate(&mut rng, text);
        match qsl_cst::parse(
            identity(),
            "d.native",
            text.as_bytes(),
            CompleteLimits::default(),
        ) {
            Ok(parsed) if parsed.is_admissible() => admitted += 1,
            _ => failed += 1,
        }
    }
    assert!(
        admitted > 100 && failed > 100,
        "{admitted} admitted, {failed} failed"
    );
    for (seed, generate) in [
        (0x5eed_0002_u64, historical_unit as fn(&mut Rng) -> String),
        (0x5eed_0003, composed_unit),
    ] {
        let mut rng = Rng(seed);
        let (mut admitted, mut failed) = (0, 0);
        for _ in 0..500 {
            let text = generate(&mut rng);
            let text = mutate(&mut rng, text);
            match parse_native(identity(), "d.native", text.as_bytes(), Limits::default()) {
                Ok(_) => admitted += 1,
                Err(_) => failed += 1,
            }
        }
        assert!(
            admitted > 100 && failed > 100,
            "{admitted} admitted, {failed} failed"
        );
    }
}
