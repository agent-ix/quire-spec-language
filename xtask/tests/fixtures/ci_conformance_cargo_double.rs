// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rust executable double for Cargo at the Make recipe process boundary.
#![forbid(unsafe_code)]

use std::fs;

mod checks;
use checks::CHECKS;

fn main() {
    let log = std::env::var("CONFORMANCE_DOUBLE_LOG").expect("process-double log");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    assert!(words.starts_with(&["test", "--locked", "-p"]));
    assert!(words.windows(2).any(|words| words == ["--", "--exact"]));
    assert!(words.ends_with(&["--format", "terse", "--nocapture"]));
    let mut words = words.into_iter();
    words.find(|word| *word == "--exact").expect("exact flag");
    let selection = words.next().expect("exact selection");
    let index = CHECKS
        .iter()
        .position(|(name, _)| *name == selection)
        .expect("required selection");
    let mut calls = fs::read_to_string(&log).unwrap_or_default();
    calls.push_str(&format!(
        "{}\t{}\n",
        selection,
        std::env::var("QSPEC_DIR").unwrap()
    ));
    fs::write(log, calls).unwrap();
    println!("selected: {selection}");
    let fail_at = std::env::var("CONFORMANCE_DOUBLE_FAIL_AT").unwrap();
    let mode = if fail_at == index.to_string() {
        std::env::var("CONFORMANCE_DOUBLE_MODE").unwrap()
    } else {
        "positive".to_owned()
    };
    match mode.as_str() {
        "no-summary" => {}
        "zero-vectors" => {
            println!("conformance: 0 of 0 QSpec operation vectors match (process double)")
        }
        _ => println!("{}", CHECKS[index].1),
    }
    match mode.as_str() {
        "zero-tests" => println!(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 1 filtered out; finished in 0.00s"
        ),
        "ignored" => println!(
            "test result: ok. 0 passed; 0 failed; 1 ignored; 0 filtered out; finished in 0.00s"
        ),
        _ => println!(
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out; finished in 0.00s"
        ),
    }
    if mode == "skip" {
        println!("skipped: QSPEC_DIR not set");
    }
    std::process::exit(if mode == "nonzero" { 23 } else { 0 });
}
