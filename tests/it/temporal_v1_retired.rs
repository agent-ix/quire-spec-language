// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1293: the root crate has no `temporal` module, and no `.rs` file in any workspace source root (the root crate's `src/`,
//! `tests/`, `examples/`, and every workspace member's directory) names a
//! retired v1 entry point or result type.
//!
//! This guards reintroduction. It does not check the per-row COVERED status
//! FR-499-AC-1 asks of a deleting change; that is the deletion table's.
//!
//! The mapping classification (`Classification`, `mapping_support_v2`) and the
//! native owner-report route (its report, valuation evidence and refusal types)
//! are retired with the module; their successors are the evidence call's result
//! (FR-526) and the checked views (FR-527). Each retired name is built from two
//! separately-quoted halves so this file's own source needs no exemption from
//! its own scan.
//!
//! Names that other crates legitimately define (`Limits`, `Ledger`,
//! `Obligation`, `evaluate`, ...) are matched where the root crate's `temporal`
//! module is named: after `quire_spec_language::temporal::`, after
//! `crate::temporal::` in the root `src/`, and after `temporal::` in a file
//! that imports `quire_spec_language::temporal`, including inside a `{...}`
//! import group.
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

/// Retired names no other crate defines, matched anywhere.
fn unique_names() -> Vec<String> {
    vec![
        ["evaluate_", "artifact_v2"].concat(),
        ["mapping_support", "_checked"].concat(),
        ["Protocol", "Ledger"].concat(),
        ["Artifact", "Report"].concat(),
        ["Valuation", "Evidence"].concat(),
        ["Report", "Refusal"].concat(),
        ["Native", "ReportError"].concat(),
    ]
}

/// Retired names matched only as members of the root crate's `temporal` module.
fn module_members() -> Vec<String> {
    [
        ["evaluate", ""],
        ["evaluate_", "checked"],
        ["evaluate_checked_", "v2"],
        ["evaluate_", "with_progress"],
        ["evaluate_with_progress_", "v2"],
        ["evaluate_checked_with_", "progress"],
        ["evaluate_checked_with_progress_", "v2"],
        ["mapping_", "support"],
        ["Led", "ger"],
        ["Lim", "its"],
        ["Obliga", "tion"],
        ["Bind", "ing"],
        ["Prog", "ress"],
        ["Prem", "ises"],
        ["Assess", "ment"],
        ["Ba", "sis"],
        ["Tr", "uth"],
        ["Cap", "ture"],
        ["Activ", "ation"],
        ["Incomp", "lete"],
        ["Exhaus", "tion"],
        ["Usage", ""],
        ["LimitDimen", "sion"],
        ["Checked", "Classification"],
        ["ProtocolEval", "uation"],
        ["mapping_support_", "v2"],
        ["Classif", "ication"],
        ["Profi", "le"],
        ["Dimen", "sion"],
        ["Sub", "ject"],
    ]
    .into_iter()
    .map(|halves| halves.concat())
    .collect()
}

/// The retired module of the root crate.
fn banned_files() -> [&'static str; 2] {
    ["src/temporal.rs", "src/temporal"]
}

/// The identifiers that follow each `temporal::` in `contents`: the next
/// identifier, or the first segment of every top-level entry of a `{...}` group.
fn members_after_temporal(contents: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (at, _) in contents.match_indices("temporal::") {
        let rest = &contents[at + "temporal::".len()..];
        let identifier = |text: &str| -> String {
            text.trim_start()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect()
        };
        if let Some(group) = rest.strip_prefix('{') {
            let mut depth = 1_usize;
            let mut end = group.len();
            for (index, c) in group.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            end = index;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let mut level = 0_usize;
            let mut entry = String::new();
            let mut entries = Vec::new();
            for c in group[..end].chars() {
                match c {
                    '{' => level += 1,
                    '}' => level = level.saturating_sub(1),
                    ',' if level == 0 => entries.push(std::mem::take(&mut entry)),
                    _ => {}
                }
                if !(c == ',' && level == 0) {
                    entry.push(c);
                }
            }
            entries.push(entry);
            found.extend(entries.iter().map(|entry| identifier(entry)));
        } else {
            found.push(identifier(rest));
        }
    }
    found
}

/// Every regular `.rs` file under `root`, recursively, appended to `out`.
fn files_under(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                files_under(&path, out);
            } else if file_type.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            {
                out.push(path);
            }
        }
    }
}

#[trace("TC-1293")]
#[test]
fn the_root_crates_temporal_module_is_absent() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let present: Vec<&str> = banned_files()
        .into_iter()
        .filter(|file| manifest_dir.join(file).exists())
        .collect();
    assert!(present.is_empty(), "retired files still exist: {present:?}");
}

#[trace("TC-1293")]
#[test]
fn no_workspace_source_names_a_retired_v1_temporal_entry_point() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let unique = unique_names();
    let members = module_members();
    let roots: Vec<PathBuf> = ["src", "tests", "examples", "fuzz", "xtask"]
        .into_iter()
        .map(PathBuf::from)
        .chain(
            crate::support::workspace::member_dirs()
                .into_iter()
                .filter(|member| !member.as_os_str().is_empty()),
        )
        .collect();
    let mut offending = Vec::new();
    let mut scanned = 0_usize;
    for root in &roots {
        let mut files = Vec::new();
        files_under(&manifest_dir.join(root), &mut files);
        for path in files {
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            scanned += 1;
            for name in &unique {
                if contents.contains(name.as_str()) {
                    offending.push((path.display().to_string(), name.clone()));
                }
            }
            let in_root_src = path.starts_with(manifest_dir.join("src"));
            if in_root_src || contents.contains("quire_spec_language::temporal") {
                for member in members_after_temporal(&contents) {
                    if members.contains(&member) {
                        offending.push((path.display().to_string(), member));
                    }
                }
            }
        }
    }
    assert!(
        scanned > 0,
        "the scan found no `.rs` files in any source root; a root was renamed or moved"
    );
    assert!(
        offending.is_empty(),
        "retired v1 temporal names still have source references: {offending:?}"
    );
}
