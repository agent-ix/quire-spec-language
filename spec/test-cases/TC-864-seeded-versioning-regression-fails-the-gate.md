---
id: TC-864
title: "A seeded spec-versioning regression fails the gate naming exactly that case"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-340
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: verifies
---
# TC-864: A seeded spec-versioning regression fails the gate naming exactly that case

## Description

Verify the gate end to end over a test-only corpus that the gate's real run
does not read: one real pair whose superseding revision tightens a
ConfigVersion clause is a violation naming exactly the seeded case, while
the rest of the corpus holds. Verify also that the real corpus passes
`make refinement-versioning` and fails once the seed is added.

Scope: FR-340-AC-6, FR-343-AC-2 to FR-343-AC-4, FR-344-AC-5.

## Test Procedure

Build the test corpus from FR-108's ConfigVersion fixtures:

- **seed**: `prior` the ConfigVersion unit, `superseding` the tightened
  unit; cases healthy-parent (admitted by `prior`, refused by
  `superseding`) and violating-parent.
- **dropped**: `superseding` removes the selected clause (a `Clause` case)
  and the function `sameIdentity` (a `Function` case of it).
- **broken**: `superseding` has an ill-typed function body; cases
  healthy-parent and violating-parent.
- **identity**: both sides the ConfigVersion unit; cases healthy-parent and
  violating-parent.

1. Run the gate over the seed pair together with the identity pair.
2. Run the gate over the dropped pair.
3. Run the gate over the broken pair.
4. Run `make refinement-versioning` over
   `tests/fixtures/refinement/versioning/`; add the seed pair to a copy of
   that corpus and run `cargo run --package xtask -- refinement versioning`
   over the copy.

Each expected result is a literal in the test. Tag the tests
`#[trace("TC-864", "<AC>")]` with the AC each step backs.

## Expected Results

- Step 1: verdict violation, exit 10; exactly one `regression`, the seed
  pair's healthy-parent case, naming both `RawSourceRef`s, the selection,
  `admitted`, `refused` and the superseding disposition's codes; the seed's
  violating-parent case is `not applicable`; the identity pair's
  healthy-parent case `holds` and its violating-parent case is `not
  applicable`.
- Step 2: the `Clause` case `holds`; the `Function` case is a
  `regression`.
- Step 3: healthy-parent is a `regression`; violating-parent is `not
  applicable`.
- Step 4: the real corpus exits 0; the copy with the seed exits 10 and the
  step fails.
