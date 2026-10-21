// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064: the standalone string-edge scanner. A caller names a base
//! directory, one or more crate source directories and edge rules; the scan
//! reports every string comparison or string `match` outside a
//! `#[string_edge]`-marked function in those crates' production source that
//! the rules do not allow.
//!
//! This is the one detector. QSL's `xtask string-edge` and every consumer
//! repository's CI call it with their own roots and rules; the crate names no
//! QSL crate and reads no QSL layout. The `qsl-string-edge` binary exposes
//! the same request on the command line (see `README.md`).
//!
//! Each crate source directory is one crate's constant-resolution scope: a
//! `const NAME: &str` is resolved by name within its own root only.
#![forbid(unsafe_code)]

mod detector;
pub mod error;
mod rules;
mod source;

use std::collections::BTreeSet;
use std::path::PathBuf;

pub use detector::Occurrence;
pub use error::{Error, Result};
pub use rules::{AllowListEntry, EdgeRules};

/// What to scan.
#[derive(Clone, Debug)]
pub struct ScanRequest {
    /// The directory source paths, rule entries and report locations resolve
    /// against. A relative path is relative to the current directory.
    pub base_dir: PathBuf,
    /// One crate source directory per crate (`src`, `crates/consumer/src`),
    /// not its `Cargo.toml`. Relative roots resolve against `base_dir`.
    pub crate_roots: Vec<PathBuf>,
    /// The allow-list.
    pub rules: EdgeRules,
}

/// A completed scan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    /// Every unmarked, un-allow-listed occurrence, sorted by file, line and
    /// item.
    pub occurrences: Vec<Occurrence>,
}

impl Report {
    /// Whether the scan found nothing to report.
    pub fn is_clean(&self) -> bool {
        self.occurrences.is_empty()
    }

    /// The human-facing summary: one line when clean, else a count and one
    /// `file:line` line per occurrence.
    pub fn render(&self) -> String {
        if self.is_clean() {
            return "string-edge: no unmarked, unlisted string comparison or string match found\n"
                .to_owned();
        }
        let mut summary = format!(
            "string-edge: {} unmarked, unlisted occurrence(s):\n",
            self.occurrences.len()
        );
        for occurrence in &self.occurrences {
            summary.push_str(&format!(
                "  {}:{}{}\n",
                occurrence.file,
                occurrence.line,
                if occurrence.branch_gating {
                    " (branch-gating)"
                } else {
                    ""
                }
            ));
        }
        summary
    }
}

/// Run the scan.
///
/// Refuses (FR-064-AC-5) any allow-list entry whose item has a branch-gating
/// occurrence, whether or not that item is marked, then reports every
/// occurrence that is neither inside a `#[string_edge]` function nor named by
/// an allow-list `(file, item)`. A request, rule, read or parse failure is an
/// [`Error`], never a clean report.
pub fn scan(request: &ScanRequest) -> Result<Report> {
    let base = source::resolve_base(&request.base_dir)?;
    let allow_list = rules::validated_keys(&request.rules, &base)?;
    let roots = source::resolve_roots(&base, &request.crate_roots)?;
    let mut found = Vec::new();
    for root in &roots {
        let files = source::load_root(&base, root)?;
        found.extend(detector::scan_parsed(&files));
    }
    if let Some((file, item)) = allow_list.iter().find(|(file, item)| {
        found.iter().any(|found| {
            found.occurrence.branch_gating
                && found.occurrence.file == *file
                && found.occurrence.item == *item
        })
    }) {
        return Err(Error::AllowListGatesABranch {
            file: file.clone(),
            item: item.clone(),
        });
    }
    let allowed: BTreeSet<(&str, &str)> = allow_list
        .iter()
        .map(|(file, item)| (file.as_str(), item.as_str()))
        .collect();
    let mut occurrences: Vec<Occurrence> = found
        .into_iter()
        .filter(|found| {
            !found.marked
                && !allowed.contains(&(
                    found.occurrence.file.as_str(),
                    found.occurrence.item.as_str(),
                ))
        })
        .map(|found| found.occurrence)
        .collect();
    occurrences.sort();
    Ok(Report { occurrences })
}

/// `Self`'s type name for an `impl` block, for the common case a syntax scan
/// needs (`impl PlainType { ... }`). One definition, shared by this detector
/// and QSL's `xtask` scans (PR #262 review, F8: the same one-fact-one-place
/// instinct that keyed `seam_probe`'s own checked-in locations on the
/// enclosing item's name instead of a line number).
///
/// **Known limitation (rust-review pre-handoff pass, carried over
/// unchanged).** A `Self` type that is not a plain path -- `impl &Foo`, a
/// tuple or reference type -- falls into the `"<impl>"` catch-all below,
/// which would silently collapse two *different* impl blocks' methods of the
/// same name into one indistinguishable key if two such impls ever both held
/// an occurrence in the same file. Widening this to handle every `syn::Type`
/// shape distinctly is speculative against today's real call sites; broaden
/// it if a future one actually needs a generic or otherwise non-`Type::Path`
/// `Self`.
pub fn impl_self_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_else(|| "<impl>".to_owned()),
        _ => "<impl>".to_owned(),
    }
}
