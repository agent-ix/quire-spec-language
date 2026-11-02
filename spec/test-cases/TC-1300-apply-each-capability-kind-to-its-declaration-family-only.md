---
id: TC-1300
title: "Apply each capability kind to its declaration family only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: verifies
---
# TC-1300: Apply each capability kind to its declaration family only

## Description

Verify the kind-to-family applicability table of FR-505 on the spine `analyze`
operation. Scope: FR-505-AC-1 to FR-505-AC-6.

## Test Procedure

Fixture `C0` is a checked package with six claims: `V` (value function),
`S` (state clause), `T` (temporal clause), `P` (protocol), `Q` (probabilistic
claim) and `K` (task-set claim); one item is built per (kind, claim) pair with
an engine registered for every kind.

| Row | Items | Expected |
| --- | --- | --- |
| 1 | all 72 (kind, claim-family) pairs | exactly the 12 tabled pairs run; the other 60 settle `invalid-request`, `invalid_capability`/`inapplicable-kind`, naming the kind and the claim's family, no engine run |
| 2 | `operation-contract` required on `S`; `finite-replay` on `S` | first runs; second `invalid-request` naming `state-model`; `S`'s clause record unchanged against a run without the second item |
| 3 | each protocol kind on `P`, then on `S`, `V`, `T` | applicable on `P`; on `S`, `V`, `T` `invalid-request` naming `state-model`, `value`, `temporal-trace` |
| 4 | `probabilistic-satisfaction` on `Q`, `schedulability` on `K`; both on `S` | first two run; both on `S` `invalid-request` naming `state-model` |
| 5 | index 0 `operation-contract` on `S` (`supported`), index 1 `finite-replay` required on `S` | records at 0, 1 in order; aggregate unavailable; engine ran for index 0 only |
| 6 | compile with `--cfg seam_probe` a `Capability` variant absent from the table | compile error |

Tag the test `#[trace("FR-505-AC-1", "FR-505-AC-2", "FR-505-AC-3", "FR-505-AC-4", "FR-505-AC-5", "FR-505-AC-6", "TC-1300")]`.
TC-115 step 3 and its last sentence are superseded by this case for the
composed protocol family; TC-115 keeps its static-subject text until FR-508.

## Expected Results

Each row's Expected column.
