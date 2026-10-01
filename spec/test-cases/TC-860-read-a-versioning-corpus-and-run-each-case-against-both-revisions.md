---
id: TC-860
title: "The spec-versioning gate reads each pair, refuses a malformed one, and runs every case against both revisions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-340
    type: verifies
---
# TC-860: The spec-versioning gate reads each pair, refuses a malformed one, and runs every case against both revisions

## Description

Verify FR-340's corpus reading and case running: a well-formed pair runs
every case against both revisions with no expected `package_id`; each
malformed pair is one tool-failure result and the rest still run; the
direction is the pair's `prior`/`superseding` naming; limits default or
are taken from the case; and the report does not depend on where the corpus
sits.

Scope: FR-340-AC-1 to FR-340-AC-5.

## Test Procedure

Build corpora in temporary directories from FR-108's ConfigVersion unit,
package and snapshot fixtures. The tightened revision is the ConfigVersion
unit with one state clause strengthened so that the healthy-parent snapshot
violates it.

1. One pair, `prior` the ConfigVersion unit and `superseding` the same unit
   under another revision label, with two `Clause` cases (healthy-parent
   and violating-parent). Run `xtask refinement versioning`.
2. Add six malformed pairs beside step 1's: a `pair.json` holding `{`; one
   with no `superseding`; one with an extra member `expected`; one whose
   `prior.path` is `../outside.qsl`; one whose `prior.path` is absolute;
   one whose `superseding.identity` differs from `prior.identity`. Run the
   gate. Then run the gate over an empty corpus directory.
3. One pair whose `prior` is the ConfigVersion unit at revision `b` and
   whose `superseding` is the tightened unit at revision `a`, with the
   healthy-parent case. Run the gate. Swap only the two revision labels and
   run again.
4. Step 1's pair with the healthy-parent case stating no limit sets; then
   with `accounting` holding a work budget of one unit.
5. Copy step 1's corpus to a second temporary directory and run the gate
   in each.

Tag the tests `#[trace("TC-860", "FR-340-AC-n")]`.

## Expected Results

- Step 1: two case results, each naming both `RawSourceRef`s and its
  selection; neither run was given an expected `package_id`.
- Step 2: exactly six pair-level tool-failure results, each naming its
  `pair.json` path and its defect (not JSON; missing `superseding`; unknown
  member `expected`; path outside the pair directory; absolute path;
  identity mismatch naming both label pairs). None of their cases ran.
  Step 1's two case results are present. The empty corpus gives one
  tool-failure result naming the corpus path; verdict tool failure, exit 30.
- Step 3: the healthy-parent case is a `regression` (prior `admitted`,
  superseding `refused`). After the label swap the two classes and the
  result are unchanged.
- Step 4: the first run's two classes are `admitted`; in the second both
  are `incomplete`, naming `work_units`, and the case is `unresolved
  (incomplete)`.
- Step 5: the two reports are byte-equal.
