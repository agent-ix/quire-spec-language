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
    "current-head-lane prepare --deps-root <path> --manifest <path>\n\
     current-head-lane revision-log --qsl <path> --manifest <path> --deps-root <path>\n\
     current-head-lane check-incompatible-fixture --manifest <path>"
        .to_owned()
}

const IR_URL: &str = "https://github.com/agent-ix/quire-contract-ir";
const RT_URL: &str = "https://github.com/agent-ix/quire-contract-runtime";

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

/// FR-058: refresh every locally cloned checkout this lane's `[patch]`
/// entries need, then re-resolve the lane's own manifest so its committed
/// `Cargo.lock` picks up each dependency's current head (#249 review,
/// HIGH-1) -- this is what stops the lane silently going stale between
/// `prepare` runs (RT/CG resolving from whatever was last committed, rather
/// than from a fresh `cargo update`). `cargo update --manifest-path
/// <lane manifest>` only ever rewrites *this* lane's own lock; it never
/// touches the root workspace's `Cargo.lock`.
fn run_prepare(deps_root: &Path, lane_manifest: &Path) -> Result<String, String> {
    std::fs::create_dir_all(deps_root)
        .map_err(|error| format!("cannot create {}: {error}", deps_root.display()))?;
    let ir_head = deps_root.join("quire-contract-ir");
    clone_or_refresh_branch(&ir_head, IR_URL, "main")?;
    let rt_head = deps_root.join("quire-contract-runtime");
    clone_or_refresh_branch(&rt_head, RT_URL, "main")?;
    run_cargo_update(lane_manifest)?;
    Ok(format!(
        "prepared {} (quire-contract-ir @ main), {} (quire-contract-runtime @ main); \
         refreshed {}'s own lock to each pinned dependency's current head",
        ir_head.display(),
        rt_head.display(),
        lane_manifest.display()
    ))
}

fn run_cargo_update(manifest: &Path) -> Result<(), String> {
    let output = Command::new("cargo")
        .arg("update")
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .map_err(|error| format!("cannot run cargo update: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo update failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// The sha `refs/heads/<branch>` currently points to on `url`'s remote --
/// used by `revision-log` to refuse to report a resolved commit as "current
/// head" when it is actually stale (FR-058's "cannot silently substitute",
/// #249 review HIGH-1).
fn git_ls_remote_head(url: &str, branch: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["ls-remote", url, branch])
        .output()
        .map_err(|error| format!("cannot run git ls-remote {url} {branch}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-remote {url} {branch} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let sha = text
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("git ls-remote {url} {branch} returned no ref"))?;
    Ok(sha.to_owned())
}

/// Fails loudly if `resolved` (the commit the lane actually resolved for
/// `label`) is not `url`'s current `main` head -- a mismatch means the lane
/// is running against a stale local snapshot, not current head, and must not
/// silently report itself as current-head anyway.
fn require_current_head(label: &str, url: &str, resolved: &str) -> Result<(), String> {
    let remote_head = git_ls_remote_head(url, "main")?;
    if remote_head != resolved {
        return Err(format!(
            "{label}: resolved {resolved} does not match {url}'s current main head \
             {remote_head} -- the lane is running against a stale snapshot, not current \
             head; run `make integration-current-head-prepare` to refresh it"
        ));
    }
    Ok(())
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
/// `cargo metadata` reports for it. quire-contract-ir and quire-contract-runtime
/// are not listed here: the lane's `[patch]` resolves both to a local
/// path (see `../README.md`), so `cargo metadata` reports no git `source` for
/// either; `run_revision_log` reads their commits directly from those
/// local clones instead (`REPOS_VIA_LOCAL_CLONE`).
struct Repo {
    label: &'static str,
    package_names: &'static [&'static str],
}

const REPOS: &[Repo] = &[Repo {
    label: "quire-contract-codegen",
    package_names: &["quire-contract-codegen"],
}];

/// `(label, local clone directory name, remote URL)` for every repository
/// this lane resolves through a local `[patch]`-ed clone rather than a live
/// git dependency `cargo metadata` can read a `source` for.
const REPOS_VIA_LOCAL_CLONE: &[(&str, &str, &str)] = &[
    ("quire-contract-ir", "quire-contract-ir", IR_URL),
    ("quire-contract-runtime", "quire-contract-runtime", RT_URL),
];

fn cargo_metadata(manifest: &Path) -> Result<Value, String> {
    let output = Command::new("cargo")
        .arg("metadata")
        .arg("--format-version=1")
        .arg("--locked")
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

/// FR-058-AC-2/AC-4: record the exact commit resolved for QSL itself and for
/// each of IR/RT/CG, and refuse to report a resolved commit for IR/RT/CG as
/// "current head" when it no longer matches that repository's real remote
/// `main` (#249 review, HIGH-1) -- otherwise a stale local clone or an
/// un-refreshed lane lock would silently report itself as current-head,
/// exactly the silent substitution FR-058 exists to rule out.
fn run_revision_log(qsl_root: &Path, manifest: &Path, deps_root: &Path) -> Result<String, String> {
    let document = cargo_metadata(manifest)?;
    let packages = document
        .get("packages")
        .and_then(Value::as_array)
        .ok_or("unexpected cargo metadata shape: no packages array")?;

    let mut lines = vec![format!("quire-spec-language {}", git_head(qsl_root)?)];
    for (label, dir_name, url) in REPOS_VIA_LOCAL_CLONE {
        let commit = git_head(&deps_root.join(dir_name))?;
        require_current_head(label, url, &commit)?;
        lines.push(format!("{label} {commit}"));
    }
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
        require_current_head(
            repo.label,
            "https://github.com/agent-ix/quire-contract-codegen",
            commit,
        )?;
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

/// The compiler diagnostics TC-159 step 3 requires: an unresolved-import
/// error, at minimum any `error[E0` diagnostic. A build failure with none of
/// these (a missing manifest, a toolchain error, a network failure fetching a
/// dependency) is not the incompatibility this fixture demonstrates, and must
/// not be reported as though it were (#249 review, HIGH-3).
const EXPECTED_DIAGNOSTIC_MARKERS: &[&str] = &["error[E0432", "error[E0433", "error[E0"];

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
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !EXPECTED_DIAGNOSTIC_MARKERS
        .iter()
        .any(|marker| stderr.contains(marker))
    {
        return Err(format!(
            "the incompatible fixture at {} failed to build, but not with the expected \
             unresolved-import compiler diagnostic (E0432/E0433, at minimum any `error[E0`); \
             this is not FR-058-AC-3's demonstrated incompatibility -- it looks like a \
             different failure (a missing manifest, a toolchain or network error). Raw stderr:\n{stderr}",
            manifest.display()
        ));
    }
    Ok(format!(
        "{INCOMPATIBLE_FIXTURE_MARKER}\n\
         cargo build exit status: {}\n\
         (raw compiler output follows, for local debugging only; it is not part of the stable diagnostic)\n{stderr}",
        output.status,
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
        "prepare" => {
            let deps_root = take_flag(&mut args, "--deps-root").map(PathBuf::from);
            let manifest = take_flag(&mut args, "--manifest").map(PathBuf::from);
            match (deps_root, manifest) {
                (Some(deps_root), Some(manifest)) => run_prepare(&deps_root, &manifest),
                _ => {
                    eprintln!("{}", usage());
                    return ExitCode::from(2);
                }
            }
        }
        "revision-log" => {
            let qsl = take_flag(&mut args, "--qsl").map(PathBuf::from);
            let manifest = take_flag(&mut args, "--manifest").map(PathBuf::from);
            let deps_root = take_flag(&mut args, "--deps-root").map(PathBuf::from);
            match (qsl, manifest, deps_root) {
                (Some(qsl), Some(manifest), Some(deps_root)) => {
                    run_revision_log(&qsl, &manifest, &deps_root)
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
    use super::{resolved_commit, run_check_incompatible_fixture};
    use ix_trace_rs::trace;
    use std::path::Path;

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

    /// tc_current_head_lane_check_incompatible_fixture_missing_manifest
    /// (negative control, #249 review HIGH-3): a `cargo build` failure with
    /// no manifest at all is a different failure than TC-159 step 3's
    /// unresolved-import compiler diagnostic, and must not be reported as
    /// though FR-058-AC-3's incompatibility had been demonstrated.
    #[trace("TC-159", "FR-058-AC-3")]
    #[test]
    fn tc_current_head_lane_check_incompatible_fixture_missing_manifest() {
        let error =
            run_check_incompatible_fixture(Path::new("/nonexistent/does-not-exist/Cargo.toml"))
                .expect_err("a missing manifest must not report the marker as healthy");
        assert!(
            error.contains("not with the expected unresolved-import compiler diagnostic"),
            "{error}"
        );
    }
}
