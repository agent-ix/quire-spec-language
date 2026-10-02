---
id: TC-889
title: "S2 builds the refinement form with every row in source order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: verifies
---
# TC-889: S2 builds the refinement form with every row in source order

## Description

Verify that S2 builds one `RefinementForm` per `refinement` declaration with
every row as written, open arguments and population-valued forms included,
and resolves no name.

Scope: FR-310-AC-1 to FR-310-AC-3.

## Test Procedure

Fixtures: ADR-020 §8's `CasRefinesCounter`; FR-139's `RingIsQueue`;
FR-147's `CasTwice`.

1. Build the S2 form of `CasRefinesCounter`.
2. Build the forms of `RingIsQueue` and `CasTwice`.
3. Build the forms of a declaration whose abstract alias names no model and
   of one with a duplicate `population` row, then run S3 on them.

Tag the tests `#[trace("TC-889", "<AC id>")]`.

## Expected Results

- Step 1: abstract side `Model(Spec)`, concrete `Impl`, the rows in source
  order with spans; the `ensure` granularity `each`.
- Step 2: `seq` and `only` expression forms; `Apply` with `[self.ring,
  Open]`; `Protocol(Spec, Twice)` and an `Internal` row for `finish`.
- Step 3: both forms build; S3 refuses each with FR-135-AC-2's code.
