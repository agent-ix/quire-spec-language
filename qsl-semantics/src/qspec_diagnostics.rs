// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test support: the (code, cause) pairs QSpec's
//! `quire.native.diagnostics/v1` catalog lists, read at run time from the
//! quire-specification checkout `QSPEC_DIR` names. Nothing of QSpec is
//! copied into this repository.

use std::collections::BTreeSet;
use std::path::Path;

/// The catalog's path inside a quire-specification checkout.
const CATALOG: &str = "proposals/quire-v1/definitions/native-diagnostics.md";

/// Every `(code, cause)` pair the catalog's "Required distinguishing causes"
/// table lists: each code of a row's first cell with each cause its second
/// cell names before the first `;`. `None` when `QSPEC_DIR` is unset.
pub(crate) fn listed_cause_pairs() -> Option<BTreeSet<(String, String)>> {
    let qspec = std::env::var_os("QSPEC_DIR")?;
    let path = Path::new(&qspec).join(CATALOG);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let table = text
        .split_once("## Required distinguishing causes")
        .unwrap_or_else(|| panic!("{} has no required-causes section", path.display()))
        .1;
    let mut pairs = BTreeSet::new();
    for row in table
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
    {
        let mut cells = row.split('|').skip(1);
        let (Some(codes), Some(causes)) = (cells.next(), cells.next()) else {
            continue;
        };
        let causes = causes.split(';').next().unwrap_or_default();
        for code in quoted(codes) {
            for cause in quoted(causes) {
                pairs.insert((code.to_owned(), cause.to_owned()));
            }
        }
    }
    assert!(!pairs.is_empty(), "{} lists no causes", path.display());
    Some(pairs)
}

/// The backtick-quoted spellings of `cell`.
fn quoted(cell: &str) -> impl Iterator<Item = &str> {
    cell.split('`').skip(1).step_by(2)
}
