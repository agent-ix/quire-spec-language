---
id: TC-1305
title: "Refuse a repeated operation role or frame entry at intake"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-510
    type: verifies
---
# TC-1305: Refuse a repeated operation role or frame entry at intake

## Description

Verify the operation and frame checks of FR-510. Scope: FR-510-AC-1 to
FR-510-AC-6.

## Test Procedure

Fixture `O0` is the TC-458 ConfigVersion package extended with object type
`Gauge` (fields `g`, `h`), object types `A` and `B`, and object `Counter`
with operation `bump`.

| Row | Mutation of O0 | Expected |
| --- | --- | --- |
| 1 | an operation whose owner is the population `config_history` | `invalid_model_binding`/`malformed-declaration` at the owner reference |
| 2 | second `Counter.bump`; then `Sub.bump` with `redefines Counter/bump` instead | `ambiguous_declaration`/`ambiguous-name` at the second, naming both; then admitted |
| 3 | parameters `a`, `a`; parameter `r` with result binder `r` | each `ambiguous_declaration`/`ambiguous-name` at the second |
| 4 | `modifies [Gauge/g, Gauge/g]` | `invalid_model_binding`/`malformed-declaration` at entry 1, naming positions 0 and 1 |
| 5 | `modifies [Gauge/g, Gauge/h]` | admitted |
| 6 | `creates [A, A]`; `deletes [B, B]` | each refused as row 4 |
| 7 | `creates [A]` with `deletes [A]` | admitted |
| 8 | repeated parameter plus repeated `modifies` | the parameter refusal only |
| 9 | repeated `modifies` plus repeated `creates` | the `modifies` refusal only |
| 10 | rows 2-4 with unrelated records reordered in the document | the same refusals naming the same entries; row 5's package equal under reordering |

Tag the test `#[trace("FR-510-AC-1", "FR-510-AC-2", "FR-510-AC-3", "FR-510-AC-4", "FR-510-AC-5", "FR-510-AC-6", "TC-1305")]`.

## Expected Results

Each row's Expected column; a refusal admits no declaration.
