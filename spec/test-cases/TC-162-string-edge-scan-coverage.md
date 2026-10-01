---
id: TC-162
title: "The string-edge scan reports every unmarked string dispatch"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: verifies
---
# TC-162: The string-edge scan reports every unmarked string dispatch

## Description

Verify that `xtask string-edge` reports every string comparison or string
`match` outside a `#[string_edge]`-marked function in non-test code, respects
the checked-in allow-list, excludes test code, exits non-zero exactly when
its report is non-empty, rejects an allow-list entry that gates a branch
(including each of ADR-010 §4.3's five named production sites), and that the
lint gate fails when the tool does. Scope: FR-064-AC-1 through FR-064-AC-6.

**Status.** `xtask string-edge` is clean over the whole
workspace and `string-edge` is a prerequisite of `make ci` (step 7, backed
by `the_real_makefile_wires_string_edge_into_ci` and
`a_failed_prerequisite_fails_the_aggregate_target`). The scan skips a
`tests/` directory, `#[cfg(test)]` items and files declared by
`#[cfg(test)] mod x;`, never a file by name alone. The detector treats a
comparison that is a term of a `&&`/`||` chain, a match arm's own value, or a
`strip_prefix` call as branch-gating. Step 6's real-site half runs the raw
scan (marks ignored) over the ADR-010 §4.3 dispatch strings still compared
in the tree -- `CanonicalizationDomain::from_str`,
`AdapterArtifact::try_from`, `clock_binding_name` -- and asserts each is
branch-gating and rejected as an allow-list entry
(`real_adr010_sites_are_flagged_branch_gating_by_the_structural_detector`,
`FR-064-AC-5`). The scan reads string literals only; a comparison against a
named `const NAME: &str` or another string value is not detected (FR-064
Status).

**Correction: steps 2 and 6 (PR #262 review, finding F11).** An earlier
draft of `xtask/src/string_edge.rs` checked the allow-list only through
`allow_list_branch_gating_check`, which reads the real, checked-in
`allow_list()` directly -- and `allow_list()` returns an empty `Vec` today
(nothing has yet been reviewed and admitted). Steps 2 and 6 need a
*non-empty* allow-list to add, remove and submit entries against, so no
test could exercise them through that one entry point: the rejection this
requirement's own Behavior section describes was asserted in prose but
unreachable by anything the test suite could actually run, the same
"claims a behavior the code cannot exhibit" shape F7 found elsewhere in
this PR. `branch_gating_entries` is now a separate, pure function taking
the candidate allow-list as a parameter (`allow_list_branch_gating_check`
is a thin wrapper that scans the crate and calls it with the real
`allow_list()`); `xtask/src/string_edge.rs`'s own test module now
constructs non-empty allow-list fixtures directly and exercises steps 2's
rejection shape and step 6's five ADR-010 §4.3 sites through it
(`allow_list_entry_at_a_branch_gating_occurrence_is_rejected`,
`none_of_the_five_adr010_production_sites_can_be_allow_listed`). Step 6's
production sites are fixtures shaped like each named site, not the real
crate's own occurrences of them (the real-site half is
`real_adr010_sites_are_flagged_branch_gating_by_the_structural_detector`); the fixtures demonstrate the
rejection rule works, not that the real crate's five sites are converted.
Step 2's own dedicated test lands the same way: `run`'s allow-list
filtering is split into a separate, pure `unreported_occurrences`, and
`allow_listed_occurrence_is_silent_removing_the_entry_reports_it_again`
exercises the real add-then-remove-reappears sequence through it.

## Test Procedure

1. Build a fixture QSL-shaped source tree with one string comparison inside
   a `#[string_edge]`-marked function and one string `match` inside an
   unmarked function; run `xtask string-edge` against it.
2. Add an allow-list entry naming the unmarked occurrence's file and line;
   re-run the scan. Remove the entry and re-run again.
3. Move the unmarked occurrence from step 1 into a `#[cfg(test)]` module,
   into a file under a `tests/` directory and into a file declared by
   `#[cfg(test)] mod x;` (separate fixtures); run the scan against each.
   Separately, a file named `tests.rs` or `*_tests.rs` that is not declared
   `#[cfg(test)]` is scanned.
4. Run the scan against the empty-report fixture from step 2 (with the
   allow-list entry present) and against the non-empty-report fixture (entry
   removed); capture both exit codes.
5. Construct one allow-list entry whose comparison result feeds an `if`
   condition selecting between two code paths in the fixture, and one entry
   whose comparison result feeds only a logged message; submit both to
   `xtask string-edge`.
6. For each of the five sites ADR-010 §4.3 names (the `"allocation"`
   relationship-category string, `"quire.protocol.finite-global/v1"`,
   `"filament-canonical-json-1"`, `"quire.state.authority-adapter"`, and the
   `clock:` prefix), construct a fixture occurrence shaped like that site
   (a string comparison whose result selects one of two branches) and an
   allow-list entry naming it; submit each to `xtask string-edge` in turn.
   Then run the real scan with `#[string_edge]` marks ignored over the
   workspace, and for each ADR-010 §4.3 dispatch string still compared in
   the tree (`CanonicalizationDomain::from_str`, `AdapterArtifact::try_from`,
   `clock_binding_name`) submit an allow-list entry at its real file and
   item.
7. Confirm that the real `Makefile`'s `ci` target lists `string-edge` as a
   prerequisite and that `string-edge`'s recipe runs `cargo xtask
   string-edge` (a grep-shaped check over the real file, not a stub of an
   abstraction that does not exist). Separately, on
   a minimal fixture `Makefile` of an aggregate-target/prerequisite shape,
   confirm a failed prerequisite fails the aggregate target and a
   succeeding one does not (the same mechanism FR-063-AC-5/TC-161 step 7
   demonstrates).

## Expected Results

- Step 1: the report names exactly the unmarked occurrence, with its file
  and line; the marked occurrence is absent from the report.
- Step 2: with the allow-list entry present, the report omits the
  occurrence; with the entry removed, the report includes it again, with no
  source change.
- Step 3: none of the `#[cfg(test)]`-module, `tests/`-directory or
  `#[cfg(test)] mod x;`-file occurrences is reported, marked or not,
  allow-listed or not; an undeclared `tests.rs`/`*_tests.rs` file is scanned.
- Step 4: the empty-report run exits zero; the non-empty-report run exits
  non-zero.
- Step 5: the branch-gating entry is rejected, naming its file and line; the
  display-only entry is accepted.
- Step 6: each of the five constructed entries is rejected; none of the five
  is accepted into the allow-list under any submitted reason text. In the
  real scan, each of the three named sites above is flagged branch-gating and
  an allow-list entry at it is rejected. The `"allocation"` site is gone from
  the tree and the profile string is now only registry data
  (`RegisteredDefinition::identity`), so neither is a real site.
- Step 7: `ci` lists `string-edge` and its recipe runs `cargo xtask
  string-edge`; the fixture `Makefile` fails its aggregate target exactly
  when the prerequisite's recipe fails, and not otherwise.