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
7. Stub the lint gate's target list to include `xtask string-edge`, then
   simulate a non-zero exit from the tool and observe the gate's own exit
   code.

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
  is accepted into the allow-list under any submitted reason text.
- Step 7: the lint gate exits non-zero when `xtask string-edge` does.
