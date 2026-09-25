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
2. Run S2 with nesting-depth bound 8 over a body of `not`×8 `a`.
3. Check a declaration whose preimage input bytes exceed bound `B`; then
   reach the same limit for an FR-151 synthesized function.
4. Check a declaration whose work charge a work budget `W` denies.

Tag the tests `#[trace("TC-427", "FR-096-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: `stage_limit_exceeded` with `input-bytes-exceeded`,
  `nesting-depth-exceeded`, `token-count-exceeded`, `node-count-exceeded`,
  `edge-count-exceeded`, `occurrence-count-exceeded`,
  `diagnostic-count-exceeded` and `work-budget-exceeded` (revision
  `1-draft.7`), and each `LimitExceeded` gives its kind's code.
- Step 2: kind nesting depth, bound 8, actual 9, the region of the node at
  depth 9 under the unit's `RawSourceRef`.
- Step 3: kind input bytes, bound `B`, actual the measured bytes, the
  declaration's region; the synthesized function's limit has no locus.
- Step 4: kind work budget, bound `W`, actual the spend the denied charge
  would have reached, the declaration's region.

## Status

Backed (QSL-160). Step 1:
`limit_exceeded_reports_stage_limit_exceeded_per_kind`
(`qsl-foundation/src/diagnostic/stage.rs`). Step 2:
`the_s2_depth_limit_is_located_at_the_first_node_past_the_bound`
(`qsl-forms/tests/it/value_forms.rs`). Steps 3 and 4:
`a_declaration_input_bytes_limit_is_located_at_the_declaration` and
`a_denied_work_charge_is_located_at_the_declaration`
(`qsl-semantics/src/check/family.rs`, `locus_tests`), over declarations
read from real source through S1, S2 and the assembler.
