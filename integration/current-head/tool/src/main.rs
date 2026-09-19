// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-058 (ADR-011 §7.1 T-12, #215): current-head integration lane tooling.
//!
//! `current-head-lane revision-log` records the exact commit this lane
//! resolved for QSL itself and for quire-contract-ir, quire-contract-runtime
//! and quire-contract-codegen (FR-058-AC-2).
//!
//! `current-head-lane check-incompatible-fixture` runs `cargo build` over the
//! intentionally incompatible fixture manifest and turns its result into one
//! stable diagnostic line (FR-058-AC-3): the fixture is expected to fail, so
//! this subcommand itself fails (non-zero exit) if the fixture unexpectedly
//! builds.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, ExitCode},
};

fn usage() -> String {
    "current-head-lane prepare --vendor-root <path>\n\
     current-head-lane revision-log --qsl <path> --manifest <path> --vendor-root <path>\n\
     current-head-lane check-incompatible-fixture --manifest <path>"
        .to_owned()
}

const IR_URL: &str = "https://github.com/agent-ix/quire-contract-ir";

fn run_git(args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|error| format!("cannot run git {}: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// Clones `url`'s default branch into `dest` if absent, otherwise fetches and
/// fast-forwards `dest` to that branch's current head. `dest` ends up on a
/// detached `FETCH_HEAD`, which is enough to build against; it is never a
/// working tree anyone edits.
fn clone_or_refresh_branch(dest: &Path, url: &str, branch: &str) -> Result<(), String> {
    if !dest.exists() {
        return run_git(&[
            "clone",
            "--branch",
            branch,
            "--single-branch",
            url,
            &dest.to_string_lossy(),
        ]);
    }
    let dest_str = dest.to_string_lossy();
    run_git(&["-C", &dest_str, "fetch", "origin", branch])?;
    run_git(&["-C", &dest_str, "checkout", "--detach", "FETCH_HEAD"])
}

fn run_prepare(vendor_root: &Path) -> Result<String, String> {
    std::fs::create_dir_all(vendor_root)
        .map_err(|error| format!("cannot create {}: {error}", vendor_root.display()))?;
    let head = vendor_root.join("quire-contract-ir");
    clone_or_refresh_branch(&head, IR_URL, "main")?;
    Ok(format!(
        "prepared {} (quire-contract-ir @ main)",
        head.display()
    ))
}

fn take_flag(args: &mut Vec<String>, name: &str) -> Option<String> {
    let index = args.iter().position(|arg| arg == name)?;
    if index + 1 >= args.len() {
        return None;
    }
    args.remove(index);
    Some(args.remove(index))
}

/// Extracts the trailing `#<sha>` a git dependency's resolved `source` string
/// carries, for example `git+https://.../quire-contract-ir?branch=main#<sha>`.
/// A source with no `#` at all (for example a plain registry source) has no
/// commit to extract and is `None`, never the whole string.
fn resolved_commit(source: &str) -> Option<&str> {
    let (_, sha) = source.rsplit_once('#')?;
    (!sha.is_empty()).then_some(sha)
}

fn git_head(root: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .map_err(|error| format!("cannot run git -C {}: {error}", root.display()))?;
    if !output.status.success() {
        return Err(format!(
            "git rev-parse HEAD failed for {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// One resolved ecosystem repository's commit, keyed by the package name(s)
/// `cargo metadata` reports for it. quire-contract-ir is not listed here: the
/// lane's `[patch]` resolves it to a local vendored path (see
/// `../README.md`), so `cargo metadata` reports no git `source` for it;
/// `run_revision_log` reads its commit directly from that vendored clone
/// instead.
struct Repo {
    label: &'static str,
    package_names: &'static [&'static str],
}

const REPOS: &[Repo] = &[
    Repo {
        label: "quire-contract-runtime",
        package_names: &["quire-contract-runtime"],
    },
    Repo {
        label: "quire-contract-codegen",
        package_names: &["quire-contract-codegen"],
    },
];

fn cargo_metadata(manifest: &Path) -> Result<Value, String> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .map_err(|error| format!("cannot run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("cargo metadata produced invalid JSON: {error}"))
}

fn run_revision_log(
    qsl_root: &Path,
    manifest: &Path,
    vendor_root: &Path,
) -> Result<String, String> {
    let document = cargo_metadata(manifest)?;
    let packages = document
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("unexpected cargo metadata shape: no packages array")?;

    let mut lines = vec![
        format!("quire-spec-language {}", git_head(qsl_root)?),
        format!(
            "quire-contract-ir {}",
            git_head(&vendor_root.join("quire-contract-ir"))?
        ),
    ];
    for repo in REPOS {
        let commit = packages
            .iter()
            .find(|package| {
                package
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| repo.package_names.contains(&name))
            })
            .and_then(|package| package.get("source"))
            .and_then(Value::as_str)
            .and_then(resolved_commit)
            .ok_or_else(|| format!("{}: no resolved git source found", repo.label))?;
        lines.push(format!("{} {commit}", repo.label));
    }
    Ok(lines.join("\n"))
}

/// FR-058-AC-3: the fixture manifest patches quire-contract-ir's
/// `quire-contract-model` package to a local, deliberately empty stub crate
/// (`fixtures/incompatible/stub-quire-contract-model/`) that declares none of
/// the types quire-spec-language's real source imports from it, so building
/// QSL's real source tree against it is expected to fail to compile with
/// unresolved-import errors.
const INCOMPATIBLE_FIXTURE_MARKER: &str = "FR-058-AC-3: quire-contract-ir patched to a deliberately empty stub crate is incompatible with quire-spec-language at head";

fn run_check_incompatible_fixture(manifest: &Path) -> Result<String, String> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .map_err(|error| format!("cannot run cargo build: {error}"))?;
    if output.status.success() {
        return Err(format!(
            "the incompatible fixture at {} built successfully; it is expected to fail \
             (FR-058-AC-3 no longer demonstrated -- the stub crate may now declare a type \
             quire-spec-language no longer imports)",
            manifest.display()
        ));
    }
    Ok(format!(
        "{INCOMPATIBLE_FIXTURE_MARKER}\n\
         cargo build exit status: {}\n\
         (raw compiler output follows, for local debugging only; it is not part of the stable diagnostic)\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    }
    let mode = args.remove(0);
    let result = match mode.as_str() {
        "prepare" => match take_flag(&mut args, "--vendor-root").map(PathBuf::from) {
            Some(vendor_root) => run_prepare(&vendor_root),
            None => {
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        },
        "revision-log" => {
            let qsl = take_flag(&mut args, "--qsl").map(PathBuf::from);
            let manifest = take_flag(&mut args, "--manifest").map(PathBuf::from);
            let vendor_root = take_flag(&mut args, "--vendor-root").map(PathBuf::from);
            match (qsl, manifest, vendor_root) {
                (Some(qsl), Some(manifest), Some(vendor_root)) => {
                    run_revision_log(&qsl, &manifest, &vendor_root)
                }
                _ => {
                    eprintln!("{}", usage());
                    return ExitCode::from(2);
                }
            }
        }
        "check-incompatible-fixture" => match take_flag(&mut args, "--manifest").map(PathBuf::from)
        {
            Some(manifest) => run_check_incompatible_fixture(&manifest),
            None => {
                eprintln!("{}", usage());
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("{}", usage());
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(summary) => {
            println!("{summary}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::resolved_commit;
    use ix_trace_rs::trace;

    /// A real git dependency's resolved `source` string carries the exact
    /// commit as a trailing `#<sha>`.
    #[trace("TC-159", "FR-058-AC-2")]
    #[test]
    fn resolved_commit_extracts_trailing_sha() {
        assert_eq!(
            resolved_commit(
                "git+https://github.com/agent-ix/quire-contract-runtime?branch=main#abc123"
            ),
            Some("abc123")
        );
    }

    /// A registry source has no `#<sha>` at all; `resolved_commit` must not
    /// invent one from a bare trailing fragment.
    #[trace("TC-159", "FR-058-AC-2")]
    #[test]
    fn resolved_commit_is_none_without_a_fragment() {
        assert_eq!(
            resolved_commit("registry+https://github.com/rust-lang/crates.io-index"),
            None
        );
    }

    /// A malformed source ending in a bare `#` (empty fragment) is not a sha.
    #[trace("TC-159", "FR-058-AC-2")]
    #[test]
    fn resolved_commit_is_none_for_an_empty_fragment() {
        assert_eq!(resolved_commit("git+https://example.com/repo#"), None);
    }
}
