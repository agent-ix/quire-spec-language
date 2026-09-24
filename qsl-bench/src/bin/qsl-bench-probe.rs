// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-196 probe: the counts, outcomes and one-shot measurements the
//! criterion benchmarks do not produce -- which inputs a layer admits or
//! refuses, input sizes in the unit each layer counts, CST identity bytes
//! hashed, and wall time plus peak RSS for inputs too slow to sample
//! repeatedly.
//!
//! ```text
//! qsl-bench-probe parse                     # nesting 0..=64, volume sizes
//! qsl-bench-probe cst                       # hashed bytes per source byte
//! qsl-bench-probe check <chain|independent> <N>   # one check, one process
//! qsl-bench-probe eval <N>                  # one f0 call on an N-chain
//! qsl-bench-probe model <types> <depth> <members> # record counts, outcomes
//! ```
//!
//! `check` measures one input per process because peak RSS is process-wide
//! and monotone (`qsl_bench::rss`); run it once per size.

use std::process::ExitCode;
use std::time::Instant;

use qsl_attrs::string_edge;
use qsl_bench::check::{self, call_head, completed_with_five, linked_chain};
use qsl_bench::model::{self, ModelShape};
use qsl_bench::parse::{self, ParseOutcome};
use qsl_bench::rss::peak_rss_kib;
use qsl_semantics::model::domain_package::DomainPackageRecord;
use qsl_semantics::model::normalize::object_universe_of;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::population::AdmissionOutcome;

/// The parser volume sizes the probe reports (functions per source).
const VOLUME_SIZES: [usize; 6] = [100, 500, 1_000, 2_000, 3_000, 4_000];

fn usage() -> ExitCode {
    eprintln!(
        "usage: qsl-bench-probe parse | cst | check <chain|independent> <N> | eval <N> | model <types> <depth> <members>"
    );
    ExitCode::from(2)
}

fn size(argument: Option<&String>) -> Option<usize> {
    argument?.replace('_', "").parse().ok()
}

fn rss() -> String {
    peak_rss_kib().map_or_else(|| "unavailable".to_owned(), |kib| kib.to_string())
}

fn describe(outcome: &ParseOutcome) -> String {
    match outcome {
        ParseOutcome::Admitted { tokens, nodes } => {
            format!("admitted tokens={tokens} nodes={nodes}")
        }
        ParseOutcome::Recovered { first_code } => {
            format!("recovered first_code={}", first_code.unwrap_or("none"))
        }
        ParseOutcome::Refused {
            code,
            cause,
            message,
        } => format!("refused code={code} cause={cause} message={message:?}"),
    }
}

fn probe_parse() {
    let mut max_admitted = None;
    for depth in 0..=64 {
        let text = parse::nested_source(depth);
        let outcome = ParseOutcome::of(&parse::parse(&text));
        println!(
            "parse.depth depth={depth} bytes={} {}",
            text.len(),
            describe(&outcome)
        );
        if outcome.is_admitted() {
            max_admitted = Some(depth);
        }
    }
    println!(
        "parse.depth.max_admitted {}",
        max_admitted.map_or_else(|| "none".to_owned(), |depth| depth.to_string())
    );
    for functions in VOLUME_SIZES {
        let text = parse::volume_source(functions);
        let outcome = ParseOutcome::of(&parse::parse(&text));
        println!(
            "parse.volume functions={functions} bytes={} {}",
            text.len(),
            describe(&outcome)
        );
    }
}

fn probe_cst() {
    let mut inputs: Vec<(String, String)> = VOLUME_SIZES
        .iter()
        .map(|&functions| {
            (
                format!("volume functions={functions}"),
                parse::volume_source(functions),
            )
        })
        .collect();
    inputs.extend(
        (0..=4).map(|depth| (format!("nested depth={depth}"), parse::nested_source(depth))),
    );
    for (label, text) in inputs {
        match parse::parse(&text) {
            Ok(parsed) if parsed.is_admissible() => {
                let hashed = parsed.cst().identity_preimage_bytes();
                let source = text.len();
                // Display only: both counts are far below 2^52, so the
                // float division loses nothing that two decimals show.
                let ratio = hashed.total as f64 / source as f64;
                println!(
                    "cst.hash {label} source_bytes={source} nodes={} hashed_bytes={} of_which_source_slices={} of_which_ancestor_paths={} ratio={ratio:.2} (counted by qsl-cst's preimage builder)",
                    parsed.cst().nodes().len(),
                    hashed.total,
                    hashed.source_slices,
                    hashed.ancestor_paths,
                );
            }
            other => println!("cst.hash {label} {}", describe(&ParseOutcome::of(&other))),
        }
    }
}

/// `#[string_edge]`: `shape` is a command-line word, matched here to pick
/// the generator; it selects no family semantics.
#[string_edge]
fn probe_check(shape: &str, functions: usize) -> ExitCode {
    let declarations = match shape {
        "chain" => check::call_chain(functions),
        "independent" => check::independent(functions),
        _ => return usage(),
    };
    let start = Instant::now();
    let result = check::check(declarations);
    let elapsed = start.elapsed();
    let verdict = match &result {
        Ok(_) => "checked".to_owned(),
        Err(refusals) => format!("refused refusals={}", refusals.len()),
    };
    println!(
        "check shape={shape} functions={functions} wall_ms={:.1} peak_rss_kib={} {verdict}",
        elapsed.as_secs_f64() * 1e3,
        rss()
    );
    ExitCode::SUCCESS
}

fn probe_eval(functions: usize) -> ExitCode {
    let package = linked_chain(functions);
    let objects = ObjectEnvironment::default();
    let start = Instant::now();
    let evaluation = call_head(&package, &objects);
    let elapsed = start.elapsed();
    println!(
        "eval frames={functions} wall_us={:.1} completed_with_five={} peak_rss_kib={}",
        elapsed.as_secs_f64() * 1e6,
        completed_with_five(&evaluation),
        rss()
    );
    ExitCode::SUCCESS
}

fn probe_model(shape: ModelShape, members: usize) -> ExitCode {
    let document = model::document(shape);
    let bytes = document.len();
    let offer = model::offer(document);
    let domain_package = match model::intake(&offer) {
        Ok(domain_package) => domain_package,
        Err(failure) => {
            println!("model intake refused: {failure:?}");
            return ExitCode::FAILURE;
        }
    };
    let count = |pick: fn(&DomainPackageRecord) -> bool| {
        domain_package
            .records
            .iter()
            .filter(|record| pick(record))
            .count()
    };
    println!(
        "model.intake types={} depth={} document_bytes={bytes} records={} object_types={} fields={} populations={}",
        shape.types,
        shape.depth,
        domain_package.records.len(),
        count(|record| matches!(record, DomainPackageRecord::ObjectType(_))),
        count(|record| matches!(record, DomainPackageRecord::FieldMember(_))),
        count(|record| matches!(record, DomainPackageRecord::Population(_))),
    );
    let view = model::view(&domain_package);
    println!(
        "model.normalize declarations={} type_identities={}",
        view.declarations().len(),
        view.type_identities().len()
    );
    let universe = object_universe_of(&domain_package, &model::key(&model::chain_type(0)))
        .map(|universe| universe.identity());
    println!("model.universe ok={}", universe.is_ok());
    let population = model::population_document(shape, members);
    match model::admit_population(&domain_package, &view, &population) {
        AdmissionOutcome::Admitted(binding) => {
            println!("model.admit members={}", binding.members().len());
        }
        other => println!("model.admit not admitted: {other:?}"),
    }
    ExitCode::SUCCESS
}

/// `#[string_edge]`: dispatches the probe's command-line subcommand words.
#[string_edge]
fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.first().map(String::as_str) {
        Some("parse") => {
            probe_parse();
            ExitCode::SUCCESS
        }
        Some("cst") => {
            probe_cst();
            ExitCode::SUCCESS
        }
        Some("check") => match (arguments.get(1), size(arguments.get(2))) {
            (Some(shape), Some(functions)) => probe_check(shape, functions),
            _ => usage(),
        },
        Some("eval") => match size(arguments.get(1)) {
            Some(functions) => probe_eval(functions),
            None => usage(),
        },
        Some("model") => match (
            size(arguments.get(1)),
            size(arguments.get(2)),
            size(arguments.get(3)),
        ) {
            (Some(types), Some(depth), Some(members)) => {
                probe_model(ModelShape { types, depth }, members)
            }
            _ => usage(),
        },
        _ => usage(),
    }
}
