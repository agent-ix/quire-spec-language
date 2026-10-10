---
id: TC-427
title: "A stage limit names its kind, bound, actual counter and locus"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-427: A stage limit names its kind, bound, actual counter and locus

## Description

Verify FR-096's `LimitExceeded`: each kind has its `stage_limit_exceeded`
cause, and S2 and the family `check` each locate the limit where the charge
failed. This catches a limit with no locus where one is known, and a locus
invented for a synthesized function.

Scope: FR-096-AC-2, FR-096-AC-3, FR-096-AC-4, FR-096-AC-5.

## Test Procedure

1. Read `catalog_code()` of each `LimitKind` and of a `LimitExceeded` of
   each kind.
2. Run S1 over a body of `not`×8 `a` with `s1.nodes` one below the unit's
   syntax-node count; then run S1 at the default limits and S2 over the
   result.
3. Check a declaration whose preimage input bytes exceed bound `B`; then
   reach the same limit for an FR-151 synthesized function.
4. Check a declaration whose work charge a work budget `W` denies.

Tag the tests `#[trace("TC-427", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: `stage_limit_exceeded` with `input-bytes-exceeded`,
  `token-count-exceeded`, `node-count-exceeded`, `edge-count-exceeded`,
  `occurrence-count-exceeded`, `diagnostic-count-exceeded` and
  `work-budget-exceeded` for the seven kinds, and each `LimitExceeded`
  gives its kind's code and carries its setting name.
- Step 2: `stage_limit_exceeded`/`node-count-exceeded` with the configured
  bound, the count reached and setting `s1.nodes`, at the region S1's
  diagnostic names under the unit's `RawSourceRef`; at the defaults S2
  builds the form.
- Step 3: kind input bytes, bound `B`, actual the measured bytes, the
  declaration's region; the synthesized function's limit has no locus.
- Step 4: kind work budget, bound `W`, actual the spend the denied charge
  would have reached, the declaration's region.

