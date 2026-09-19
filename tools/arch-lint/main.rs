// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-059/FR-060/FR-061 (ADR-011 §7.1 T-12, #215 scope amendment):
//! architecture-conformance checks over the QSL/IR/RT/CG ecosystem.
//!
//! `arch-lint direction` is FR-059 (backend direction, FB-05/FB-11).
//! `arch-lint api-surface` is FR-060 (the reusable API-surface check).
//! `arch-lint duplicate-revisions` is FR-061 (one revision per quire crate
//! in QSL's own `Cargo.lock`).
#![forbid(unsafe_code)]

mod api_surface;
mod duplicate_revisions;
mod error;
mod graph;
mod metadata;

use std::{
    ffi::OsString,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use error::{Code, Error, Result};

fn usage() -> Error {
    Error::new(
        Code::Usage,
        "arch-lint direction --qsl <path> --ir <path> --rt <path> --cg <path> [--offline]\n\
         arch-lint api-surface --qsl <path> [--cg <path>]\n\
         arch-lint duplicate-revisions --lockfile <path>",
    )
}

fn take_flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    if index + 1 >= args.len() {
        return Err(usage());
    }
    args.remove(index);
    Ok(Some(args.remove(index)))
}

fn take_bool(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|arg| arg == name) {
        Some(index) => {
            args.remove(index);
            true
        }
        None => false,
    }
}

fn require(args: &mut Vec<String>, name: &str) -> Result<PathBuf> {
    take_flag(args, name)?.map(PathBuf::from).ok_or_else(usage)
}

fn run_direction(mut args: Vec<String>) -> Result<(String, bool)> {
    let offline = take_bool(&mut args, "--offline");
    let qsl = require(&mut args, "--qsl")?;
    let ir = require(&mut args, "--ir")?;
    let rt = require(&mut args, "--rt")?;
    let cg = require(&mut args, "--cg")?;
    if !args.is_empty() {
        return Err(usage());
    }
    let mut edges = Vec::new();
    for root in [&qsl, &ir, &rt, &cg] {
        edges.extend(metadata::edges_for_manifest(
            &root.join("Cargo.toml"),
            offline,
        )?);
    }
    let report = graph::check(&edges);
    let mut summary = String::new();
    summary.push_str("FR-059 backend direction check (ADR-011 FB-05/FB-11)\n");
    if report.fb05.is_empty() {
        summary.push_str("  FB-05: PASS (no unapproved edge into QSL)\n");
    } else {
        summary.push_str("  FB-05: FAIL\n");
        for violation in &report.fb05 {
            summary.push_str(&format!(
                "    {} -> {} ({}, via {})\n",
                violation.edge.from,
                violation.edge.to,
                violation.edge.kind.as_str(),
                violation.edge.via_crate
            ));
        }
    }
    if report.fb11.is_empty() {
        summary.push_str("  FB-11: PASS (no cycle among QSL/IR/RT/CG)\n");
    } else {
        summary.push_str("  FB-11: FAIL\n");
        for cycle in &report.fb11 {
            let path: Vec<String> = cycle.path.iter().map(ToString::to_string).collect();
            summary.push_str(&format!("    {}\n", path.join(" -> ")));
        }
    }
    Ok((summary, report.is_clean()))
}

fn run_api_surface(mut args: Vec<String>) -> Result<(String, bool)> {
    let qsl = require(&mut args, "--qsl")?;
    let cg = take_flag(&mut args, "--cg")?.map(PathBuf::from);
    if !args.is_empty() {
        return Err(usage());
    }
    let mut summary = String::new();
    summary.push_str("FR-060 API-surface check (ADR-011 T-12)\n");
    summary.push_str(
        "  Note: this is a textual scan. It does not resolve `use ... as` renamed \
         imports or macro-expanded call sites, and it does not skip a call pattern \
         found inside a comment or string literal -- both are stated limitations of \
         this check, not silent gaps.\n",
    );
    let mut all_passed = true;
    for rule in api_surface::RULES {
        let scan_root = match rule.role {
            api_surface::Role::Qsl => Some(qsl.as_path()),
            api_surface::Role::Cg => cg.as_deref(),
        };
        let outcome = api_surface::evaluate(rule, &qsl, scan_root)?;
        let passed = outcome.passed();
        all_passed &= passed;
        match &outcome.status {
            api_surface::RuleStatus::Pending(reason) => {
                summary.push_str(&format!(
                    "  {} [{}]: PENDING -- {reason}\n",
                    outcome.rule_id, rule.description
                ));
            }
            api_surface::RuleStatus::Live if outcome.violations.is_empty() => {
                summary.push_str(&format!(
                    "  {} [{}]: PASS\n",
                    outcome.rule_id, rule.description
                ));
            }
            api_surface::RuleStatus::Live => {
                summary.push_str(&format!(
                    "  {} [{}]: FAIL\n",
                    outcome.rule_id, rule.description
                ));
                for site in &outcome.violations {
                    summary.push_str(&format!(
                        "    {}:{} (module {})\n",
                        site.file.display(),
                        site.line,
                        site.module
                    ));
                }
            }
        }
    }
    Ok((summary, all_passed))
}

fn run_duplicate_revisions(mut args: Vec<String>) -> Result<(String, bool)> {
    let lockfile = require(&mut args, "--lockfile")?;
    if !args.is_empty() {
        return Err(usage());
    }
    let packages = duplicate_revisions::read_lockfile(&lockfile)?;
    let duplicates = duplicate_revisions::check(&packages);
    let mut summary = String::new();
    summary.push_str("FR-061 duplicate-revision check (ADR-011 §7.1)\n");
    if duplicates.is_empty() {
        summary.push_str("  PASS: one revision per quire-ecosystem crate\n");
    } else {
        summary.push_str("  FAIL\n");
        for duplicate in &duplicates {
            summary.push_str(&format!("    {}:\n", duplicate.repo));
            for source in &duplicate.sources {
                summary.push_str(&format!(
                    "      {}\n",
                    source
                        .as_deref()
                        .unwrap_or("<no source: path or workspace member>")
                ));
            }
        }
    }
    Ok((summary, duplicates.is_empty()))
}

fn run(arguments: &[OsString]) -> Result<(String, bool)> {
    let mut args: Vec<String> = arguments
        .iter()
        .map(|arg| arg.to_str().map(str::to_owned))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(usage)?;
    if args.is_empty() {
        return Err(usage());
    }
    let mode = args.remove(0);
    match mode.as_str() {
        "direction" => run_direction(args),
        "api-surface" => run_api_surface(args),
        "duplicate-revisions" => run_duplicate_revisions(args),
        _ => Err(usage()),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments) {
        Ok((summary, passed)) => {
            if let Err(error) = writeln!(io::stdout().lock(), "{summary}") {
                let _ = writeln!(io::stderr().lock(), "io: cannot write report: {error}");
                return ExitCode::from(3);
            }
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            let _ = writeln!(io::stderr().lock(), "{error}");
            ExitCode::from(error.exit_code())
        }
    }
}
