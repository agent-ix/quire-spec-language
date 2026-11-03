---
id: TC-1302
title: "Apply each capability kind to its declaration family only"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-507
    type: verifies
---
# TC-1302: Apply each capability kind to its declaration family only

## Description

Verify the declaration-family applicability table of FR-507 on the spine
`analyze` operation. Scope: FR-507-AC-1 to FR-507-AC-10.

## Test Procedure

Fixture `C0` is a checked package with one claim per declaration family: `V`
(value function), `S` (state clause), `T` (temporal clause), `R` (the model
claim of a `relation` clause whose every operation has an implementation
binding), `P` (protocol), `Q` (probabilistic claim) and `K` (task-set claim).
`R'` is the same `relation` clause's code claim. `C0` also holds one model
refinement `M` with its whole-refinement item and its per-step and
initial-state obligations, a deadlock-freedom item over a model subject,
deadlock-freedom and race-freedom items anchored on `P`'s protocol clause
node, and a time-lock-freedom item over a timed subject. An engine is
registered for every kind. Unless a row says otherwise, each item reaches
`analyze` settled `Supported` on that engine.

| Row | Items | Expected |
| --- | --- | --- |
| 1 | all 84 (kind, family) pairs: each of the twelve kinds on `V`, `S`, `T`, `R`, `P`, `Q`, `K` | exactly 14 run: `value-validity` on `V`, `operation-contract` on `S`, `temporal-satisfaction` on `T`, `temporal-satisfaction` and `operation-contract` on `R`, the seven protocol kinds on `P`, `probabilistic-satisfaction` on `Q`, `schedulability` on `K`. The other 70 settle `invalid-request`, `invalid_capability`/`inapplicable-kind`, payload `request_index` (the item's index), `capability` (its kind label) and `family` (`value`, `state-model`, `temporal-trace`, `relation-claim`, `protocol`, `probabilistic` or `real-time`), no engine run |
| 2 | `operation-contract` required on `S`; `finite-replay` on `S` | first runs; second `invalid-request` with `family` `state-model`; `S`'s clause record unchanged against a run without the second item |
| 3 | each protocol kind on `P`, then on `S`, `V`, `T` | applicable on `P`; on `S`, `V`, `T` `invalid-request` with `family` `state-model`, `value`, `temporal-trace` |
| 4 | `probabilistic-satisfaction` on `Q`, `schedulability` on `K`; both on `S` | first two run; both on `S` `invalid-request` with `family` `state-model` |
| 5 | index 0 `operation-contract` on `S` (`supported`), index 1 `finite-replay` required on `S` | records at 0, 1 in order; aggregate unavailable; engine ran for index 0 only |
| 6 | compile with `--cfg seam_probe` a `Capability` variant absent from the table | compile error |
| 7 | `temporal-satisfaction` on `R`, `operation-contract` on `R'`; `M`'s whole item (`item_kind` `refinement`, `temporal-satisfaction`) and each of its per-step and initial-state obligations (`operation-contract`); `global-conformance` on `R` | all but the last run, each read as family `relation-claim`; the last settles `inapplicable-kind` with `family` `relation-claim` |
| 8 | `temporal-satisfaction` on the model deadlock-freedom item, the time-lock-freedom item, and the deadlock-freedom and race-freedom items anchored on `P`'s protocol clause node; `global-conformance` on `P` | all run; the four derived items read `temporal-trace` from their `item_kind`, not `protocol` from their anchor; `global-conformance` on `P` reads `protocol` |
| 9 | each of the twelve kinds on a claim naming a node absent from `C0`, and on a claim naming a `claim`/`verification_claim` node | all 24 settle `inapplicable-kind`; each payload has `request_index` and `capability` and no `family` member; no engine run |
| 10 | `finite-replay` on `S` handed to `analyze` settled `Supported`, then `RequiresBound`, then `Unsupported`, then `invalid-request` with `absent-extent` | each settles `invalid-request` with `inapplicable-kind`, `family` `state-model`; no engine run |

Tag the test `#[trace("FR-507-AC-1", "FR-507-AC-2", "FR-507-AC-3", "FR-507-AC-4", "FR-507-AC-5", "FR-507-AC-6", "FR-507-AC-7", "FR-507-AC-8", "FR-507-AC-9", "FR-507-AC-10", "TC-1302")]`.
TC-115 step 3 and its last sentence are superseded by this case for the
composed protocol family; TC-115 keeps its static-subject text until FR-510.

## Expected Results

Each row's Expected column.
