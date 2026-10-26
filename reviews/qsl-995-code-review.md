---
id: SR-1596003
title: "Code review of PR #1604: tso race summary and race-freedom under tso, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@012638084; git diff origin/qsl-932-fix-round-2...012638084 (PR #1604, branch qsl-995-tso-race): qsl-eval/src/simulation/protocol/{tso,ra}.rs, qsl-replay/src/model_check/race_replay.rs, qsl-replay/src/certificate/closure.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/analyze/items.rs, qsl-analyze/tests/it/race_freedom.rs; ticket QSL-995"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: reviews
---

## Summary

Ticket: QSL-995. PR: quire-spec-language#1604, head 012638084, base qsl-932-fix-round-2 (#1600). Method: code-review with the rust-review lane.

The PR adds the `tso` race summary: per-location summaries, per-thread `cur`/`acq`/`rel` carriers, buffered-element and memory-value carriers, and queued-send carriers. It also adds the `tso` state-key entries and a store-buffer happens-before in race replay, and it deletes both `TsoRace` refusals.

Spec read: FR-224, FR-227, ADR-025 DR-2, DR-3 and MR-1, and QSpec FR-438 at quire-specification 23dc1ce3.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-224-AC-2 (tso half) | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-1 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-4 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-6 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | context_only |
| FR-227-AC-3 | spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md | examined |
| ADR-025 DR-3 (tso carriers) | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| QSpec FR-438 tso carrier rules and typed form | quire-specification spec/functional/memory/FR-438-report-data-races.md | context_only |
| tso race summary | qsl-eval/src/simulation/protocol/tso.rs | examined |
| tso race replay | qsl-replay/src/model_check/race_replay.rs | examined |
| refusal deletions | qsl-analyze/src/model_check/check.rs; qsl-replay/src/certificate/closure.rs | examined |
| tests | qsl-analyze/tests/it/race_freedom.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A genuine `tso` race settles `inconclusive` because replay orders `seq_cst` fences under `tso`. Replay's `fence` joins the global SC fence clock (`S`) for every `seq_cst` fence, but `tso` has no counterpart of `S` (DR-3), and the engine's `tso` fence is acquire plus release only. Reproduced in a reviewer probe (not committed): `a: x := 1 (non-atomic); fence seq_cst; y := 1 relaxed` against `b: fence seq_cst; r1 := y relaxed (reads 1); r2 := x`. Under `tso` the engine refutes with the canonical prefix `fork, Sx, flush, F1, Sy, flush, F2, Ly, Lx`, and replay returns `Inconclusive(Verdicts{Violation, Success})`. The same program under `ra` replays to `reproduced`. | qsl-replay/src/model_check/race_replay.rs:311-336 |
| FND-002 | medium | A `tso` queue entry's race map is emitted under the member `"view"`, as `ra`'s carried view is. QSpec FR-438 names the `tso` queue entry's member `"race"`. The generic encoding writes `("view", carried_key(..))` for every model. | qsl-eval/src/simulation/protocol.rs:1385-1390; qsl-eval/src/simulation/protocol/tso.rs:690-692 |
| FND-003 | medium | `a_race_changes_no_authored_claims_verdict_under_tso` is tagged FR-224-AC-4, but AC-4 is stated under `ra` only ("settles `refuted` with ... `Formula`"). The test asserts that the claim holds under `tso`, a different verdict, and it cannot fail on the race summary, which never reaches an authored claim. The tag is false. | qsl-analyze/tests/it/race_freedom.rs:903-915 |
| FND-004 | low | `a_release_acquire_flag_proves_race_freedom_under_tso` is tagged FR-224-AC-1, which is stated under `ra` only. The `tso` `proved` verdict is in FR-224-AC-6 ("settles `proved` with the `release` store and `acquire` load"). The test is still worth keeping: with the summary present it fails on a false race, although it passes with the summary disabled. It should carry the AC-6 tag (as a partial binding) or the "asserts no acceptance criterion" line. | qsl-analyze/tests/it/race_freedom.rs:858-870 |
| FND-005 | low | `a_forged_race_settles_inconclusive_under_tso` is tagged FR-227-AC-3, whose shapes are all `ra`. The guard is legitimate under FR-227 "For `kind: Race`", but the tag claims more than AC-3 states. | qsl-analyze/tests/it/race_freedom.rs:917-937 |

## Verdict

Not mergeable: FND-001 settles a genuine `tso` race `inconclusive`.

The `tso` carrier rules match DR-3 and FR-438 for `tso`. `tso` has no RA views, so the summaries `cur`/`acq`/`rel` play their part. The rules as implemented:

- A store gives `cur` (release or `seq_cst`) or `rel(l)` (otherwise) to its buffered element, or to memory for a locked access.
- A load joins its own newest buffered element, else the memory value, into `acq`, and into `cur` when it acquires.
- An RMW joins the read map into its written value, and decides its write part against the pre-state memory map.
- `flush` copies the element's map to memory.
- A fence joins `acq` into `cur` and/or sets every `rel(l)`, with no `S`.
- Fork copies the parent's summaries, `join` joins the branches', and an outermost `join` clears them.
- `send` carries `cur`, and `receive` joins it into `cur` and `acq`.

The order is decide, then update, then apply the rule. Every carrier starts at `write:true`. The state key gains `race-summary`, `race-summaries` and `race-memory` entries and the element `race` member only when some location has a non-atomic access.

Other checks:

- Both `TsoRace` refusals and their two tests are deleted.
- The `tso` `carried_choice_key` is the carried race map. A `tso` carried view holds nothing else, and a `send` waits for an empty buffer, so it is the whole carried view.
- Replay's buffered happens-before follows TSO-2 to TSO-5 (buffer, `flush`, locked RMW and `seq_cst` store), apart from FND-001.

Mutation check: with `TsoRace::enter` disabled (no summary), `a_relaxed_flag_refutes_race_freedom_under_tso_and_the_race_replays` (AC-2) and the two-receiver `tso` test fail. The AC-1, AC-4 and forged-`tso` tests still pass.

Evidence: `cargo test -p qsl-analyze --test it -- race_freedom:: weak_memory:: tso_fairness:: protocol_system:: release_acquire::` gave 113 passed and 0 failed. `make ci` was not run, by brief.
