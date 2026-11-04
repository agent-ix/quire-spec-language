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
`analyze` operation. Scope: FR-507-AC-1 to FR-507-AC-11.

## Test Procedure

Fixture `C0` is a checked package with one claim per declaration family: `V`
(value function), `S` (state clause), `T` (temporal clause), `R` (the model
claim of a `relation` clause whose every operation has an implementation
binding), `P` (protocol), `Q` (probabilistic claim) and `K` (task-set claim).
`R'` is the same `relation` clause's code claim. `C0` also holds one model
refinement `M` (proof-eligible, FR-322-AC-91, with step rows `R0` and `R1`) with its whole-refinement item and its per-step and
initial-state obligations, a deadlock-freedom item over a model subject,
deadlock-freedom and race-freedom items anchored on `P`'s protocol clause
node, and a time-lock-freedom item over a timed subject. Every node of `C0`
is emitted by QSL's lowering except `X`, a `claim`/`verification_claim` node
that QSL never emits: the test adds `X` to the emitted package by hand, as a
well-formed node with a claim occurrence, before calling `analyze`. `engines` holds
`qsl.test-all`, advertising every (kind, mode), and `qsl.test-vv`,
advertising only (`value-validity`, `bounded`). Unless a row says otherwise,
each item reaches `analyze` settled `Supported(qsl.test-all)`.

| Row | Items | Expected |
| --- | --- | --- |
| 1 | all 84 (kind, family) pairs: each of the twelve kinds on `V`, `S`, `T`, `R`, `P`, `Q`, `K` | exactly 14 run: `value-validity` on `V`, `operation-contract` on `S`, `temporal-satisfaction` on `T`, `temporal-satisfaction` and `operation-contract` on `R`, the seven protocol kinds on `P`, `probabilistic-satisfaction` on `Q`, `schedulability` on `K`. The other 70 settle `invalid-request`, `invalid_capability`/`inapplicable-kind`, payload `request_index` (the item's index), `capability` (its kind label) and `family` (`value`, `state-model`, `temporal-trace`, `relation-claim`, `protocol`, `probabilistic` or `real-time`), no engine run |
| 2 | `operation-contract` required on `S`; `finite-replay` on `S` | first runs; second `invalid-request` with `family` `state-model`; `S`'s clause record unchanged against a run without the second item |
| 3 | each protocol kind on `P`, then on `S`, `V`, `T` | applicable on `P`; on `S`, `V`, `T` `invalid-request` with `family` `state-model`, `value`, `temporal-trace` |
| 4 | `probabilistic-satisfaction` on `Q`, `schedulability` on `K`; both on `S` | first two run; both on `S` `invalid-request` with `family` `state-model` |
| 5 | index 0 `operation-contract` on `S` (`supported`), index 1 `finite-replay` required on `S` | records at 0, 1 in order; aggregate unavailable; engine ran for index 0 only |
| 6 | compile with `--cfg seam_probe` a `Capability` variant absent from the table | compile error |
| 7 | at indices 0 and 1, `temporal-satisfaction` on `R` and `operation-contract` on `R'`; at indices 2 to 5, `M`'s whole item (`declaration` occurrence, `temporal-satisfaction`) and its initial-state, `R0` and `R1` obligations (`generated` occurrences at ordinals 0, 1, 2, `operation-contract`); `global-conformance` on `R`; `schedulability` on `R` (one item, since `R` and `R'` share a claim occurrence) and on each of `M`'s four occurrences | the six records at indices 0 to 5 run, are settled by an engine and read `relation-claim`; the `global-conformance` item settles `invalid-request`, `settled_by` `disposition`, `inapplicable-kind` with `family` `relation-claim` and no engine run; each of the five `schedulability` items settles `inapplicable-kind` with `family` `relation-claim` |
| 8 | `temporal-satisfaction` on the model deadlock-freedom item, the time-lock-freedom item, and the deadlock-freedom and race-freedom items anchored on `P`'s protocol clause node (each carrying its `item_kind`); `global-conformance` on `P` (`item_kind` `formula`) | all run; the four derived items read `temporal-trace` from their `item_kind`, not `protocol` from their anchor, so the two anchored on `P` are not `inapplicable-kind`; `global-conformance` on `P` reads `protocol` |
| 9 | each of the twelve kinds on a claim naming a node absent from `C0`, and on a claim naming the hand-built node `X` | all 24 settle `inapplicable-kind`; each payload has `request_index` and `capability` and no `family` member; no engine run |
| 10 | `finite-replay` on `S` handed to `analyze` settled `Supported(qsl.test-all)`, `Supported(qsl.test-missing)` (a candidate `engines` does not hold), `Supported(qsl.test-vv)` (a held engine advertising only (`value-validity`, `bounded`)), `RequiresBound`, `Unsupported`, `invalid-request` with `absent-extent`, and `invalid-request` with `ambiguous-backend` | each of the seven settles `invalid-request` with `inapplicable-kind`, `family` `state-model`, never `unknown-backend`, `inconsistent-candidates` or the handed disposition; no engine run |
| 11 | the required kind of each row of FR-507's claim-form table on a claim of that row: `value-validity` on a Boolean value clause with occurrence role `claim` and one with role `declaration`, and on a scalar operation application in `V`'s body; `operation-contract` on `S` and on a frame obligation (`state`/`operation_anchor`, role `anchor`); `temporal-satisfaction` on `T`, on a state-graph claim (`claim`/`state_graph`), on a `hyper` clause (`claim`/`hyperproperty`), on a temporal claim over a timed subject and on one over a hybrid subject (each `temporal`/`temporal_clause`); `temporal-satisfaction` on the whole item of a refinement whose abstract side is a protocol (FR-147, a `relation`/`refinement` node); `finite-replay` and `refinement` on `P`; `probabilistic-satisfaction` on `Q`; `schedulability` on `K`; then `schedulability` on each of the claims of the non-`real-time` rows above | none of the required-kind items settles `inapplicable-kind`, and each is settled by an engine; each `schedulability` item settles `inapplicable-kind` with `family` the row's family (`value`, `state-model`, `temporal-trace` or `relation-claim`) |

Tag the test `#[trace("FR-507-AC-1", "FR-507-AC-2", "FR-507-AC-3", "FR-507-AC-4", "FR-507-AC-5", "FR-507-AC-6", "FR-507-AC-7", "FR-507-AC-8", "FR-507-AC-9", "FR-507-AC-10", "FR-507-AC-11", "TC-1302")]`.
TC-115 step 3 and its last sentence are superseded by this case for the
composed protocol family; TC-115 keeps its static-subject text until FR-510.

## Expected Results

Each row's Expected column.
