---
id: TC-161
title: "The seam probe demonstrates exhaustiveness at every S1-S4 seam"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: verifies
---
# TC-161: The seam probe demonstrates exhaustiveness at every S1-S4 seam

## Description

Verify that a build under the `seam-probe` cargo feature fails to compile
with exactly the checked-in set of seam-function locations, that removing a
checked-in entry or removing an actual compiler error each causes
`xtask seam-probe` to report a mismatch, that the feature is unreachable
under default features, that the tool's exit code reflects set equality,
that the full gate fails when the tool does, that the checked-in list covers
every named category, and that a wildcard-arm escape or a
`#[non_exhaustive]` enum is caught by the lint gate rather than silently
passing the probe. Scope: FR-063-AC-1 through FR-063-AC-7.

## Test Procedure

1. Run `cargo build --features seam-probe` against the QSL crate and collect
   every `E0004` diagnostic location from the build's stderr.
2. Compare the collected location set against the checked-in seam-function
   list; run `xtask seam-probe` and capture its exit code and report.
3. Remove one entry from the checked-in list (leaving the source unchanged)
   and re-run `xtask seam-probe`.
4. Add a match arm for the probe variant at exactly one seam function in the
   source (leaving the checked-in list unchanged) and re-run
   `xtask seam-probe`.
5. Build the QSL crate with default features (no `seam-probe`) and inspect
   whether the probe variant is reachable from any non-probe code path;
   inspect `Cargo.toml` for the feature's membership in `default` and in
   every other feature's dependency list.
6. Construct a checked-in list missing one real entry and run
   `xtask seam-probe` against the real build; separately, run it against a
   correct list.
7. Stub the full gate's target list to include `xtask seam-probe`, then
   simulate a non-zero exit from the tool and observe the gate's own exit
   code.
8. Inspect the checked-in seam-function list and confirm it names at least
   one entry for each of the five categories FR-063-AC-6 lists; remove the
   entry for one category at a time (five runs) and re-run `xtask seam-probe`
   against the real build.
9. In a fixture seam module, replace one seam's exhaustive `match` with one
   carrying a `_ => unsupported(...)` fallback arm. Build the fixture with
   `--features seam-probe` and collect its `E0004` locations; separately run
   `cargo clippy` over the fixture module.
10. Inspect the definitions of `FamilyKind`, the parsed form enum, the
    checked node enum and each family `Cause` enum for `#[non_exhaustive]`.

## Expected Results

- Step 1: the collected `E0004` location set matches the checked-in list
  from ADR-012 §5.1's named seams (the `FamilyKind` prefix and
  stage-participation matches, the parser entry table and check seam, the
  checked-node-enum evaluator/emitter/requirement-derivation matches, each
  family `Cause` enum's `catalog_code()`).
- Step 2: `xtask seam-probe` exits zero and reports no mismatch when the sets
  are equal.
- Step 3: `xtask seam-probe` exits non-zero, naming the removed entry's
  corresponding location as unexpected-but-present.
- Step 4: `xtask seam-probe` exits non-zero, naming that seam's location as
  expected-but-missing.
- Step 5: the default-feature build compiles cleanly with the probe variant
  unreachable from any non-probe path; `seam-probe` is absent from `default`
  and from every other feature's dependency list.
- Step 6: the wrong list produces a non-zero exit naming the missing entry;
  the correct list produces a zero exit.
- Step 7: the full gate exits non-zero when `xtask seam-probe` does.
- Step 8: each of the five removals causes `xtask seam-probe` to fail,
  naming that category's location as expected-but-absent from the list; the
  list as checked in (before any removal) contains all five.
- Step 9: the fixture build with the fallback arm produces no `E0004` at
  that seam (the fallback arm compiles, so `xtask seam-probe` alone would
  report the sets as equal and exit 0 against a list that omits it); the
  `cargo clippy` run over the fixture module fails on
  `clippy::wildcard_enum_match_arm` or `clippy::match_wildcard_for_single_variants`.
- Step 10: none of the four enums carries `#[non_exhaustive]`.
