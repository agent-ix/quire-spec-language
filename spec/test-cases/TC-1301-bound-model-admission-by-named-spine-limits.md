---
id: TC-1301
title: "Bound model admission by named spine limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-506
    type: verifies
---
# TC-1301: Bound model admission by named spine limits

## Description

Verify that each legacy native-model limit is bounded on the spine by the
setting FR-506 names, that the collection maximum is a raisable setting, and
that nesting depth is no limit. Scope: FR-506-AC-1 to FR-506-AC-8.

## Test Procedure

Fixture `L0`: domain package with types `A` (field `a`) and `B` (field `b`),
object `Gauge` with operation `adjust` (parameter `p`, a result, `modifies
[Gauge/g]`, `creates [A]`, `deletes [B]`). Its declared types, fields,
operations, parameters, results and frame entries are 2 + 2 + 1 + 1 + 1 + 3 =
10 plus object `Gauge` and field `g` = 12.

| Row | Setting and input | Expected |
| --- | --- | --- |
| 1 | `model.declaration_records` 12, L0 | admitted |
| 2 | `model.declaration_records` 11, L0 | `resource_exhausted` at `model.declaration_records`, no declaration published |
| 3 | `model.declaration_records` 0, L0 | `resource_exhausted` |
| 4 | defaults; field maximum 10000; option of sequence maximum 10000; unused declaration maximum 10000 | all admitted |
| 5 | defaults; the three of row 4 at 10001, one package each | each `resource_exhausted` naming `model.sequence_maximum`, 10000, 10001 (unused one included), `resource_exhausted`/`intake-limit-exceeded`, exit 22 |
| 6 | `model.sequence_maximum` 10001, the three of row 5 | all admitted |
| 7 | `model.sequence_maximum` 0, a package with any collection | `resource_exhausted` |
| 8 | `model.sequence_maximum` 3 with maximum 3, then 4 | 3 admitted, 4 refused |
| 9 | Text `maxLength` 1048576, then 1048577 | admitted, then `unsupported_construct`/`declaration-form` naming the bound |
| 10 | `intake.input_bytes` equal to L0's document bytes, +1 short, 0 | admitted, `resource_exhausted` naming it, `resource_exhausted` |
| 11 | `s3.nodes` and `identity.input_bytes` at exact usage, one short, 0 | admitted, refused naming the setting, refused |
| 12 | row 2, 5, 10, 11 refusals | none contains `model roles limit exceeded`, `model entries limit exceeded` or `model nodes limit exceeded` |
| 14 | `model.sequence_maximum` 3 through the builder, `stage_limits` and the settings operand; maximum 4, then 3 | `resource_exhausted`/`intake-limit-exceeded` with `kind`, `bound` 3, `actual` 4, `setting`; then admitted |
| 15 | `model.sequence_maximum` 4294967295 with a declared maximum 4294967295; operand `model.sequence_maximum=4294967296` | admitted; usage refusal naming the operand, cause `not an integer that fits a bound`; no limit changes |
| 13 | defaults; model 200 levels deep of option and sequence value types | admitted |

Tag the test `#[trace("FR-506-AC-1", "FR-506-AC-2", "FR-506-AC-3", "FR-506-AC-4", "FR-506-AC-5", "FR-506-AC-6", "FR-506-AC-7", "FR-506-AC-8", "TC-1301")]`.

## Expected Results

Each row's Expected column; a refusal publishes nothing.
