---
id: TC-426
title: "A stage limit names its kind, bound, actual counter and locus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
---
# TC-426: A stage limit names its kind, bound, actual counter and locus

## Description

Verify FR-096's `LimitExceeded`: each kind has its `stage_limit_exceeded`
cause, and S1, `Typer` and the family `check` each locate the limit where
the charge failed. It also backs FR-062-AC-7: holding a nested body fixed
and varying only `Typer`'s depth bound by one shows the bound to be the
proximate cause of a `Limit` outcome. This catches a limit reported as
`resource_exhausted`, a limit with no locus where one is known, and a locus
invented for a synthesized tree.

Scope: FR-096-AC-2, FR-096-AC-3, FR-096-AC-4, FR-096-AC-5; FR-062-AC-7.

## Test Procedure

1. Read `catalog_code()` of each `LimitKind` and of a `LimitExceeded` of
   each kind.
2. Check a function whose body is `not not not true` through
   `ValueFunctionFamily::check` with `CheckingLimits.depth` 3, then 4.
3. Check a declaration whose preimage input bytes exceed bound `B`; then
   reach the same limit inside an FR-151 synthesized function.
4. Run S1 over a source nested one bracket pair past its nesting bound `N`.

Tag the tests `#[trace("TC-426", "FR-096-AC-n")]` with the AC each backs;
step 2's test also carries `#[trace("TC-378", "FR-062-AC-7")]`.

## Expected Results

- Step 1: `stage_limit_exceeded` with `input-bytes-exceeded`,
  `nesting-depth-exceeded`, `node-count-exceeded` and
  `work-budget-exceeded`, and each `LimitExceeded` gives its kind's code.
- Step 2: at 3, `StageFailure::Limit` with kind nesting depth, bound 3,
  actual 4, the region of `true`, and code
  `stage_limit_exceeded`/`nesting-depth-exceeded`; at 4, no nesting-depth
  limit.
- Step 3: kind input bytes, bound `B`, actual the measured bytes, the
  declaration's region; the synthesized function's limit has no locus.
- Step 4: kind nesting depth, bound `N`, actual `N + 1`, the region of the
  bracket that opened the pair past the bound, under the source's
  `RawSourceRef`.

## Status

Planned. ADR-013 §7 slice S-5b (QSL-160), after S-4b and FR-091-AC-10.
