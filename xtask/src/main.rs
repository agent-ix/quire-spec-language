// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL #138: `cargo xtask revendor` / `cargo xtask revendor-check`.
//!
//! `revendor` takes an explicit pinned commit (already recorded in
//! `resources/<tree>/VENDOR.json`) and an optional local clone to read it
//! from; it never resolves "latest" and never fetches over the network.
//! `revendor-check` compares the vendored tree against that same manifest
//! offline, with no clone required.
#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use xtask::{
    error::{Error, Result},
    manifest::Manifest,
    revendor, revendor_check, Sources, Tree,
};

const USAGE: &str = "usage: cargo xtask revendor --tree <native-v1|complete-value|all> [--qspec-clone <path>]\n       cargo xtask revendor-check [--tree <native-v1|complete-value|all>]";

struct Args {
    tree: Option<Tree>,
    qspec_clone: Option<PathBuf>,
}

fn parse_flags(operands: &[OsString]) -> Result<Args> {
    let mut tree = None;
    let mut qspec_clone = None;
    let mut iter = operands.iter();
    while let Some(flag) = iter.next() {
        let flag = flag.to_str().ok_or(Error::Usage(USAGE))?;
        match flag {
            "--tree" => {
                let value = iter.next().ok_or(Error::Usage(USAGE))?;
                let value = value.to_str().ok_or(Error::Usage(USAGE))?;
                tree = if value == "all" {
                    None
                } else {
                    Some(Tree::from_arg(value)?)
                };
            }
            "--qspec-clone" => {
                let value = iter.next().ok_or(Error::Usage(USAGE))?;
                qspec_clone = Some(PathBuf::from(value));
            }
            _ => return Err(Error::Usage(USAGE)),
        }
    }
    Ok(Args { tree, qspec_clone })
}

fn trees(selected: Option<Tree>) -> Vec<Tree> {
    match selected {
        Some(tree) => vec![tree],
        None => Tree::ALL.to_vec(),
    }
}

fn run_revendor(workspace_root: &Path, args: &Args) -> Result<String> {
    let mut summary = String::new();
    for tree in trees(args.tree) {
        let manifest_path = tree.manifest_path(workspace_root);
        let mut manifest = Manifest::load(&manifest_path)?;
        let sources = Sources {
            workspace_root,
            qspec_clone: args.qspec_clone.as_deref(),
        };
        let report = revendor(&mut manifest, &tree.root(workspace_root), &sources)?;
        manifest.save(&manifest_path)?;
        summary.push_str(&format!(
            "{}: {} written, {} unchanged, {} external verified, {} removed\n",
            tree.dir_name(),
            report.written.len(),
            report.unchanged.len(),
            report.verified_external.len(),
            report.removed.len()
        ));
        for dest in &report.written {
            summary.push_str(&format!("  wrote {dest}\n"));
        }
        for dest in &report.removed {
            summary.push_str(&format!("  removed {dest} (no longer in VENDOR.json)\n"));
        }
    }
    Ok(summary)
}

fn run_check(workspace_root: &Path, args: &Args) -> Result<String> {
    if args.qspec_clone.is_some() {
        // revendor-check is deliberately offline: it verifies vendored bytes
        // against the manifest's own recorded digest, never against a live
        // clone, so it stays safe to run from `cargo test` with no clone
        // present. `--qspec-clone` has no effect here.
        return Err(Error::CheckRefusesQspecClone);
    }
    let mut summary = String::new();
    let mut clean = true;
    for tree in trees(args.tree) {
        let manifest_path = tree.manifest_path(workspace_root);
        let manifest = Manifest::load(&manifest_path)?;
        let report = revendor_check(&manifest, &tree.root(workspace_root))?;
        summary.push_str(&format!(
            "{}: {} matched, {} drifted, {} stray\n",
            tree.dir_name(),
            report.matched.len(),
            report.drifted.len(),
            report.stray.len()
        ));
        for drift in &report.drifted {
            clean = false;
            summary.push_str(&format!(
                "  DRIFT {}: expected {}, found {}\n",
                drift.dest, drift.expected_sha256, drift.actual_sha256
            ));
        }
        for stray in &report.stray {
            clean = false;
            summary.push_str(&format!("  STRAY {stray} (not in VENDOR.json)\n"));
        }
    }
    if clean {
        Ok(summary)
    } else {
        Err(Error::CheckFailed { summary })
    }
}

fn run(arguments: &[OsString]) -> Result<String> {
    let Some((command, operands)) = arguments.split_first() else {
        return Err(Error::Usage(USAGE));
    };
    let command = command.to_str().ok_or(Error::Usage(USAGE))?;
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is one level under the workspace root")
        .to_path_buf();
    let args = parse_flags(operands)?;
    match command {
        "revendor" => run_revendor(&workspace_root, &args),
        "revendor-check" => run_check(&workspace_root, &args),
        _ => Err(Error::Usage(USAGE)),
    }
}

fn main() -> ExitCode {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match run(&arguments) {
        Ok(summary) => match write!(io::stdout().lock(), "{summary}") {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                let _ = writeln!(io::stderr().lock(), "io: cannot write summary: {error}");
                ExitCode::from(2)
            }
        },
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

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn tc_149_usage_text_never_offers_qspec_clone_for_revendor_check() {
        let check_line = USAGE
            .lines()
            .find(|line| line.contains("revendor-check"))
            .expect("USAGE documents revendor-check");
        assert!(
            !check_line.contains("--qspec-clone"),
            "revendor-check refuses --qspec-clone at runtime; USAGE must not advertise it: {check_line}"
        );
    }

    /// Tracing: TC-149.
    #[trace("TC-149", "NFR-011-AC-1")]
    #[test]
    fn tc_149_check_refuses_a_qspec_clone_flag_with_its_own_error_variant() {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let args = Args {
            tree: None,
            qspec_clone: Some(PathBuf::from("/nonexistent")),
        };
        let error = run_check(&workspace_root, &args).unwrap_err();
        assert!(matches!(error, Error::CheckRefusesQspecClone));
    }
}
