// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-725 step 1 (FR-258-AC-1) and the AC-1 half of step 2 (FR-258-AC-5): a
//! 100,000-term sum, a 100,000-long `else if` chain and 100,000 nested
//! `let`s, read from source through S1, each check, lower and emit on a
//! 512 KiB stack; every written body is in the five-stratum grammar and every
//! written node rebuilt from the wire keys to its `node_id`; and the checked
//! package's bodies and lowered graph clone, compare equal to their clones,
//! and the package formats for debug and drops on the same stack.

use super::*;
use qsl_semantics::check::CheckedBody;

/// The stack every step runs on.
const STACK: usize = 512 * 1024;

/// How many terms, branches or bindings each body has.
const DEEP: usize = 100_000;

/// The header every deep unit starts with.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

/// A unit whose `f(a, x)` has a body of [`DEEP`] terms, read from source: a
/// sum of `x`, an `else if` chain over `a`, or nested `let`s.
fn deep_source(label: &str) -> String {
    let mut source =
        format!("{HEADER}function f using v(a: Boolean, x: Integer): Integer pure {{ ");
    for level in 0..DEEP {
        match label {
            // 99,999 additions join 100,000 terms.
            "sum" if level == 0 => {}
            "sum" => source.push_str("x + "),
            "else if" => source.push_str("if a then x else "),
            _ => source.push_str(&format!("let b{level} = x in ")),
        }
    }
    source.push_str("x }\n");
    source
}

/// `source` through S1 and S2 and the assembler, with S1's size and work
/// limits raised to fit it.
fn deep_declarations(source: &str) -> PackageDeclarations {
    let bytes = source.len();
    let mut limits = qsl_cst::Limits::default();
    limits.source_bytes = limits.source_bytes.max(bytes);
    limits.tokens = limits.tokens.max(bytes);
    limits.nodes = limits.nodes.max(bytes.saturating_mul(16));
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        limits,
    )
    .expect("the deep source parses");
    assert!(parsed.is_admissible(), "the deep source is admissible");
    let forms = qsl_forms::build_unit(&parsed).expect("the deep source has forms");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        forms,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .expect("the deep source assembles")
}

/// Each deep body, read from source through S1, checks, lowers and emits
/// under S1 and S3 limits raised to fit it, on a 512 KiB stack. Every
/// written body is in the five-stratum grammar and keys to its `node_id`,
/// and the checked package clones, compares equal to its clone, formats for
/// debug and drops there.
#[trace("TC-725", "FR-258-AC-1", "FR-258-AC-5")]
#[test]
fn a_100000_deep_body_checks_lowers_and_emits_on_a_small_stack() {
    for label in ["sum", "else if", "let"] {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(move || {
                let graph = deep_declarations(&deep_source(label))
                    .check(
                        CheckingLimits::new(u64::MAX)
                            .with_input_bytes(u64::MAX)
                            .with_work_budget(u64::MAX),
                    )
                    .unwrap_or_else(|refusals| panic!("{label}: checks: {refusals:?}"));
                let package = CheckedPackage::link(graph);
                let emission = emit_checked(&package)
                    .unwrap_or_else(|refusal| panic!("{label}: emits: {refusal:?}"));
                let written = wire(&emission);
                let written = nodes(&written);
                assert!(written.len() >= DEEP, "{label}: one node per level");
                for node in written {
                    assert!(
                        qsl_semantics::check::stratum::is_stratified_body(&node["body"]),
                        "{label}: a written body is in the five-stratum grammar"
                    );
                    assert_eq!(
                        json!(rebuilt_key(node, &[])),
                        node["node_id"]["digest"],
                        "{label}: a written node keys to its node_id"
                    );
                }
                // The checked package: every checked function body (the
                // arena a deep expression nests in) and the lowered graph
                // clone, compare equal to their clones and format; the whole
                // package formats and drops.
                let graph = package.graph();
                let body: &CheckedBody = graph.function_state(0).expect("f is checked").body.body();
                assert!(body.len() >= DEEP, "{label}: one checked node per level");
                let body_clone = body.clone();
                assert_eq!(&body_clone, body, "{label}: the body equals its clone");
                let semantic = graph.semantic_graph();
                let semantic_clone = semantic.clone();
                assert!(
                    semantic
                        .nodes()
                        .map(|node| (node.key(), node.body()))
                        .eq(semantic_clone.nodes().map(|node| (node.key(), node.body()))),
                    "{label}: the lowered graph equals its clone"
                );
                assert!(!format!("{package:?}").is_empty());
                assert!(!format!("{body_clone:?}").is_empty());
                drop(semantic_clone);
                drop(body_clone);
                drop(package);
            })
            .expect("the thread spawns")
            .join()
            .unwrap_or_else(|_| panic!("{label}: completes on a 512 KiB stack"));
    }
}

/// FR-264-AC-1: the 100,000-term sum, emitted under limits raised to fit it,
/// reads back through the I2 read with `i2.*` raised to fit it and verifies at
/// its emitted `package_id`. The `V2Read` clones, compares equal to its clone,
/// formats for debug and drops, all on a 512 KiB stack.
#[trace("TC-738", "FR-264-AC-1")]
#[test]
fn a_100000_term_sum_reads_back_verifies_clones_compares_and_drops_on_a_small_stack() {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            let graph = deep_declarations(&deep_source("sum"))
                .check(
                    CheckingLimits::new(u64::MAX)
                        .with_input_bytes(u64::MAX)
                        .with_work_budget(u64::MAX),
                )
                .unwrap_or_else(|refusals| panic!("checks: {refusals:?}"));
            let package = CheckedPackage::link(graph);
            let emission =
                emit_checked(&package).unwrap_or_else(|refusal| panic!("emits: {refusal:?}"));
            let bytes = emission.package.bytes();
            let limits = crate::checked_v2::V2ReadLimits {
                artifact_bytes: bytes.len(),
                nodes: u64::MAX,
                edges: u64::MAX,
                occurrences: u64::MAX,
                diagnostics: u64::MAX,
                work: u64::MAX,
            };
            let pinned: PinnedRequest = qsl_semantics::library::fixtures::single_pin(
                library(),
                emission.package.package_id(),
            );
            let read = crate::checked_v2::read_checked_package_v2(
                bytes,
                library(),
                limits,
                &read_evidence(&emission),
                &pinned,
            )
            .unwrap_or_else(|failure| panic!("verifies at its package_id: {failure:?}"))
            .into_value();
            let clone = read.clone();
            assert!(read == clone, "the read equals its clone");
            assert!(!format!("{read:?}").is_empty());
            drop(clone);
            drop(read);
        })
        .expect("the thread spawns")
        .join()
        .expect("the read-back completes on a 512 KiB stack");
}

/// FR-264-AC-2: every body of every package emitted for each TC-415 nested
/// expression form, at 2 and at 1,000 levels, is in the stratified grammar.
/// An unguarded `value` chain refuses to check at both depths and emits
/// nothing.
#[trace("TC-738", "FR-264-AC-2")]
#[test]
fn every_nested_form_emits_in_the_stratified_grammar_at_2_and_1000_levels() {
    use qsl_semantics::check::depth_forms::{declarations, Form, FORMS};
    for form in FORMS {
        for levels in [2, 1_000] {
            std::thread::Builder::new()
                .stack_size(STACK)
                .spawn(move || {
                    let checked = declarations(form, levels).check(CheckingLimits::default());
                    if matches!(form, Form::Field) {
                        assert!(checked.is_err(), "an unguarded `value` is unproved");
                        return;
                    }
                    let graph = checked.unwrap_or_else(|refusals| {
                        panic!("{form:?} x{levels} checks: {refusals:?}")
                    });
                    let package = CheckedPackage::link(graph);
                    // The forms are built in code, not read from a source
                    // unit, so every occurrence is placed at the fixture unit.
                    let emission = crate::emit::emit_package(&package, whole_unit)
                        .unwrap_or_else(|refusal| panic!("{form:?} x{levels} emits: {refusal:?}"));
                    let written = wire(&emission);
                    for node in nodes(&written) {
                        assert!(
                            qsl_semantics::check::stratum::is_stratified_body(&node["body"]),
                            "{form:?} x{levels}: a written body is outside the grammar: {}",
                            node["body"]
                        );
                    }
                })
                .expect("the thread spawns")
                .join()
                .unwrap_or_else(|_| panic!("{form:?} x{levels} completes on a 512 KiB stack"));
        }
    }
}

/// `function f using v(a: Boolean): Boolean pure { a and (a and (… a)) }`
/// with `levels` connectives, through S1 at its default limits, S2 and the
/// assembler, or `None` when S1's defaults do not admit it.
fn and_chain(levels: usize) -> Option<PackageDeclarations> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(a: Boolean): Boolean pure {{ {}a{} }}\n",
        "a and (".repeat(levels),
        ")".repeat(levels),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .ok()
    .filter(qsl_cst::ParsedSource::is_admissible)?;
    let forms = qsl_forms::build_unit(&parsed).ok()?;
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        forms,
        qsl_semantics::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .ok()
}

/// TC-727 step 1 (FR-258-AC-3): the longest `a and (…)` chain S1's default
/// limits admit checks at the default checking limits and emits, on a
/// 512 KiB stack: no outcome names a depth.
#[trace("TC-727", "FR-258-AC-3")]
#[test]
fn the_longest_and_chain_s1_admits_checks_and_emits_at_the_defaults() {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(|| {
            // The longest chain S1 admits: `admitted` is, `refused` is not.
            let (mut admitted, mut refused) = (1, qsl_cst::Limits::default().nodes);
            assert!(and_chain(admitted).is_some() && and_chain(refused).is_none());
            while refused - admitted > 1 {
                let middle = admitted + (refused - admitted) / 2;
                match and_chain(middle) {
                    Some(_) => admitted = middle,
                    None => refused = middle,
                }
            }
            let unit = and_chain(admitted).expect("the longest admitted chain assembles");
            let graph = unit
                .check(CheckingLimits::default())
                .unwrap_or_else(|refusals| panic!("{admitted} levels check: {refusals:?}"));
            let package = CheckedPackage::link(graph);
            let emission = emit_checked(&package)
                .unwrap_or_else(|refusal| panic!("{admitted} levels emit: {refusal:?}"));
            assert!(nodes(&wire(&emission)).len() > admitted);
        })
        .expect("the thread spawns")
        .join()
        .expect("the chain completes on a 512 KiB stack");
}
