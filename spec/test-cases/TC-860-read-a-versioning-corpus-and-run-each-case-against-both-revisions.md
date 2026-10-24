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
   as the same bytes, with two `Clause` cases (healthy-parent
   and violating-parent). Run `xtask refinement versioning`.
2. Add eight malformed pairs beside step 1's: a `pair.json` holding `{`; one
   with no `superseding`; one with an extra member `expected`; one whose
   `prior.path` is `../outside.qsl`; one whose `prior.path` is absolute;
   one whose `superseding.identity` differs from `prior.identity`; one
   whose `packages` names `repo:../outside`; one naming `repo:/etc/hostname`. Run the
   gate. Add to step 1's pair a case naming a snapshot file that does not
   exist. Then run the gate over an empty corpus directory.
3. One pair whose `prior` is the ConfigVersion unit and whose `superseding`
   is the tightened unit, with the healthy-parent case. Run the gate. Name
   the two units the other way round and run again.
4. Step 1's pair with the healthy-parent case stating no limit sets; then
   with `accounting` holding a work budget of one unit.
5. Copy step 1's corpus to a second temporary directory and run the gate
   in each.

Tag the tests `#[trace("TC-860", "FR-340-AC-n")]`.

## Expected Results

- Step 1: two case results, each naming both `RawSourceRef`s and its
  selection; neither run was given an expected `package_id`.
- Step 2: exactly eight pair-level tool-failure results, each naming its
  `pair.json` path and its defect (not JSON; missing `superseding`; unknown
  member `expected`; path outside the pair directory; absolute path;
  identity mismatch naming both label pairs; `repo:` path outside the
  repository; `repo:` absolute path). None of their cases ran.
  The missing-file case is one tool-failure case result naming the case
  and the defect; step 1's two case results are present. The empty corpus gives one
  tool-failure result naming the corpus path; verdict tool failure, exit 30.
- Step 3: the healthy-parent case is a `regression` (prior `admitted`,
  superseding `refused`). Named the other way round, the case is `not
  applicable`.
- Step 4: the first run's two classes are `admitted`; in the second both
  are `incomplete`, naming `work_units`, and the case is `unresolved
  (incomplete)`.
- Step 5: the two reports are byte-equal.
