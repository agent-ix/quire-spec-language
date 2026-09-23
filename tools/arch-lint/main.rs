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

/// `(repository, remote URL to check freshness against, `main` branch)`.
/// `--qsl` is excluded: it is this tool's own worktree, routinely checked
/// out on a feature branch rather than `main`, so it has no "current head"
/// to compare against; its resolved commit is still printed below.
const FRESHNESS_TARGETS: [(graph::Repo, &str); 3] = [
    (
        graph::Repo::Ir,
        "https://github.com/agent-ix/quire-contract-ir",
    ),
    (
        graph::Repo::Rt,
        "https://github.com/agent-ix/quire-contract-runtime",
    ),
    (
        graph::Repo::Cg,
        "https://github.com/agent-ix/quire-contract-codegen",
    ),
];

fn run_direction(mut args: Vec<String>) -> Result<(String, bool)> {
    let offline = take_bool(&mut args, "--offline");
    let qsl = require(&mut args, "--qsl")?;
    let ir = require(&mut args, "--ir")?;
    let rt = require(&mut args, "--rt")?;
    let cg = require(&mut args, "--cg")?;
    if !args.is_empty() {
        return Err(usage());
    }
    let mut summary = String::new();
    summary.push_str("FR-059 backend direction check (ADR-011 FB-05/FB-11)\n");
    summary.push_str("  Revisions used:\n");
    let roots = [
        (graph::Repo::Qsl, &qsl),
        (graph::Repo::Ir, &ir),
        (graph::Repo::Rt, &rt),
        (graph::Repo::Cg, &cg),
    ];
    let mut edges = Vec::new();
    // #249 review round 4: every root's resolved revision is printed
    // regardless of whether a later freshness check fails -- previously a
    // `Stale` error propagated via `?` before the summary built so far was
    // ever returned to `main`, so a stale-clone abort showed only the bare
    // diagnostic, not the "Revisions used" block FR-059-AC-7 promises
    // "regardless of outcome". The first `Stale` error found is still the
    // one this run fails with; it is folded into `summary` so the caller
    // never loses it.
    let mut stale: Option<Error> = None;
    for (repo, root) in &roots {
        let head = metadata::git_head(root)?;
        if let Some((_, url)) = FRESHNESS_TARGETS.iter().find(|(r, _)| r == repo) {
            if offline {
                summary.push_str(&format!(
                    "    {repo}: {head} (--offline: not checked against remote main)\n"
                ));
            } else {
                match metadata::require_current_head(repo.as_str(), url, &head) {
                    Ok(()) => {
                        summary.push_str(&format!("    {repo}: {head} (matches {url} main)\n"))
                    }
                    Err(error) => {
                        summary.push_str(&format!(
                            "    {repo}: {head} (STALE -- does not match {url} main)\n"
                        ));
                        if stale.is_none() {
                            stale = Some(error);
                        }
                    }
                }
            }
        } else {
            summary.push_str(&format!(
                "    {repo}: {head} (this worktree; no remote-main freshness check)\n"
            ));
        }
        if stale.is_none() {
            edges.extend(metadata::edges_for_manifest(
                &root.join("Cargo.toml"),
                offline,
            )?);
        }
    }
    if let Some(error) = stale {
        return Err(Error::new(
            error.code,
            format!("{summary}{}", error.message()),
        ));
    }
    let report = graph::check(&edges);
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
    api_surface::assert_is_qsl_root(&qsl)?;
    let mut summary = String::new();
    summary.push_str("FR-060 API-surface check (ADR-011 T-12)\n");
    summary.push_str(
        "  Note: T12-B, T12-C and T12-D match source tokens in shipped code: comments, \
         string literals and `#[cfg(test)]` items are excluded, and each mint's enclosing \
         function is resolved against a named, shrinking debt list. T12-A is a textual \
         scan of the CG tree, so a match inside a comment or string literal is reported. \
         No rule resolves a `use ... as` rename of the constructor's type -- a stated \
         limitation of this check, not a silent gap.\n",
    );
    let mut all_passed = true;
    // Flags a rule needed but was not given. Every rule is still evaluated,
    // so one rule's missing input never hides the others' results; the run
    // then fails as a usage error with the full report attached.
    let mut missing_flags: Vec<&'static str> = Vec::new();
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
            api_surface::RuleStatus::NeedsRoot(role) => {
                let flag = role.flag_name();
                summary.push_str(&format!(
                    "  {} [{}]: NOT EVALUATED -- its target exists; pass {flag} to give it a \
                     tree to scan\n",
                    outcome.rule_id, rule.description
                ));
                if !missing_flags.contains(&flag) {
                    missing_flags.push(flag);
                }
            }
            api_surface::RuleStatus::Live if passed => {
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
                        "    {}:{} (module {}{})\n",
                        site.file.display(),
                        site.line,
                        site.module,
                        if site.function.is_empty() {
                            String::new()
                        } else {
                            format!(", fn {}", site.function)
                        }
                    ));
                }
                for (module, function) in &outcome.stale_debt_entries {
                    summary.push_str(&format!(
                        "    stale debt entry, no remaining mint: {module}, fn {function}\n"
                    ));
                }
            }
        }
        for site in &outcome.debt {
            summary.push_str(&format!(
                "    debt: {}:{} (module {}, fn {})\n",
                site.file.display(),
                site.line,
                site.module,
                site.function
            ));
        }
        if let Some(note) = rule.scope_note {
            summary.push_str(&format!("    (not evaluated for every role: {note})\n"));
        }
    }
    if !missing_flags.is_empty() {
        return Err(Error::new(
            Code::Usage,
            format!(
                "{summary}missing input: pass {} to evaluate every rule",
                missing_flags.join(" and ")
            ),
        ));
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

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use std::{fs, path::Path};

    fn write(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// tc_arch_lint_api_surface_021 (negative control): with no `--cg`,
    /// T12-A reports that it needs `--cg` and T12-B, T12-C and T12-D are
    /// still evaluated -- one rule's missing input does not hide the others'
    /// results. The run fails as a usage error carrying the full report.
    #[trace("TC-157", "FR-060-AC-4")]
    #[test]
    fn tc_arch_lint_api_surface_021_missing_cg_still_runs_qsl_rules() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "Cargo.toml",
            "[package]\nname = \"quire-spec-language\"\nversion = \"0.2.0\"\n",
        );
        for relative in [
            "qsl-foundation/src",
            "qsl-cst/src",
            "qsl-source/src",
            "qsl-forms/src",
        ] {
            fs::create_dir_all(root.join(relative)).unwrap();
        }
        write(root, "qsl-replay/src/lib.rs", "pub fn run() {}\n");
        write(root, "src/check/mod.rs", "pub struct NodeKey;\n");
        write(root, "src/model/key.rs", "pub struct EffectiveId;\n");
        write(
            root,
            "src/model/population.rs",
            "pub struct PopulationId;\n",
        );
        write(
            root,
            "src/value/model_query.rs",
            "fn f() {\n    NodeKey::from_digest(k);\n    EffectiveId::from_digest(d);\n}\n",
        );

        let arguments: Vec<OsString> = ["api-surface", "--qsl"]
            .into_iter()
            .map(OsString::from)
            .chain([root.as_os_str().to_owned()])
            .collect();
        let error = run(&arguments).unwrap_err();
        assert_eq!(error.code, Code::Usage);
        let report = error.message();
        let line_of = |id: &str| {
            report
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("{id} [")))
                .unwrap_or_else(|| panic!("no {id} line in report:\n{report}"))
        };
        assert!(line_of("T12-A").ends_with("pass --cg to give it a tree to scan"));
        assert!(line_of("T12-B").ends_with(": FAIL"), "{report}");
        assert!(line_of("T12-C").ends_with(": FAIL"), "{report}");
        assert!(line_of("T12-D").ends_with(": PASS"), "{report}");
        assert!(report.contains("model_query.rs:2 (module value::model_query, fn f)"));
        assert!(report.contains("model_query.rs:3 (module value::model_query, fn f)"));
        assert!(report.ends_with("missing input: pass --cg to evaluate every rule"));
    }
}
