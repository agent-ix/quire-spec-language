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

**Scope correction.** `xtask string-edge` itself (steps 1-6 below) is fully
implemented and exercised by unit fixtures, and is clean over every file
#214 adds or touches (`xtask/src/*`). Step 7 (the lint-gate wiring) is
implemented only as a standalone `make string-edge` target, not as part of
`ci:`/the lint gate: the real QSL crate today has 60 pre-existing,
unmarked, mostly branch-gating occurrences outside `qsl-semantics/src/family/*` and
`qsl-eval/src/value/expression/*`, which the allow-list cannot admit and which
#214 does not own converting (see
[QSL-145](https://linear.app/agent-ix/issue/QSL-145) and FR-064's own
Status section). QSL-155 corrects step 7's original "stub the gate's target
list" wording (the same defect as FR-063-AC-5's, and the same fix -- see
below); the corrected version is still not satisfied, since no `Makefile`
target invokes `xtask string-edge` yet, which remains QSL-145's gap, not
this correction's.

**QSL-150's real-site investigation (step 6).** Running the real scan (not
a fixture) over the four of ADR-010 §4.3's five named sites still present in
the tree, at their real file and line, found every one comes back
`branch_gating: false`: each comparison is a term of a `&&`/`||` boolean
expression or a `match` arm body whose *result*, not the comparison itself,
is what an outer branch reads -- a shape this scanner's structural
(syntactic-nesting-only) detector does not see, by its own documented scope.
`branch_gating_entries` therefore does not reject an allow-list entry at any
of the four real sites today. This is recorded as a concrete, tested finding
(`xtask::string_edge::tests::real_adr010_sites_are_not_flagged_branch_gating_
by_the_structural_detector`) rather than left to the synthetic fixtures in
step 6 below implying more than they show.

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
crate's own occurrences of them (those 60 sites are QSL-145's, not #214's,
per this test's own earlier scope correction); the fixtures demonstrate the
rejection rule works, not that the real crate's five sites are converted.
QSL-150 lands step 2's own dedicated test the same way: `run`'s allow-list
filtering is split into a separate, pure `unreported_occurrences`, and
`allow_listed_occurrence_is_silent_removing_the_entry_reports_it_again`
exercises the real add-then-remove-reappears sequence through it.

## Test Procedure

1. Build a fixture QSL-shaped source tree with one string comparison inside
   a `#[string_edge]`-marked function and one string `match` inside an
   unmarked function; run `xtask string-edge` against it.
2. Add an allow-list entry naming the unmarked occurrence's file and line;
   re-run the scan. Remove the entry and re-run again.
3. Move the unmarked occurrence from step 1 into a `#[cfg(test)]` module and
   into a file under a `tests/` directory (two separate fixtures); run the
   scan against each.
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
7. Confirm whether any real `Makefile` target invokes `xtask string-edge`
   as part of a gate (QSL-155's correction to this criterion's original
   "stub the gate's target list" text: a grep-shaped check over the real
   file, not a stub of an abstraction that does not exist). Separately, on
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
- Step 3: neither the `#[cfg(test)]`-module occurrence nor the
  `tests/`-directory occurrence is reported, marked or not, allow-listed or
  not.
- Step 4: the empty-report run exits zero; the non-empty-report run exits
  non-zero.
- Step 5: the branch-gating entry is rejected, naming its file and line; the
  display-only entry is accepted.
- Step 6: each of the five constructed entries is rejected; none of the five
  is accepted into the allow-list under any submitted reason text. Separately
  (QSL-150's real-site investigation, above): none of the four real sites
  still present in the tree is rejected today, since none is flagged
  branch-gating by the structural detector.
- Step 7: no real `Makefile` target invokes `xtask string-edge` yet
  (QSL-145's own gap); the fixture `Makefile` fails its aggregate target
  exactly when the prerequisite's recipe fails, and not otherwise.
