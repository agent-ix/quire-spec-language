---
id: SR-1483003
title: "Code review of PR #1590: ra race summary, race-freedom item and race replay, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a307ab9ef; git diff origin/main...a307ab9ef (PR #1590, branch qsl-932-ra-race-summary): qsl-eval/src/simulation/protocol/{race,ra,memory,steps,identity,access,footprint}.rs, qsl-eval/src/simulation/subject.rs, qsl-eval/src/automaton/key.rs, qsl-eval/src/smt/query.rs, qsl-analyze/src/model_check/check.rs, qsl-analyze/src/analyze{,/items}.rs, qsl-replay/src/model_check/{race_replay,replay,counterexample,item,smt_query}.rs, qsl-replay/src/certificate/closure.rs, qsl-route/src/deadlock.rs, qsl-analyze/tests/it/{race_freedom,litmus,weak_memory}.rs; ticket QSL-932"
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

Ticket: QSL-932. PR: quire-spec-language#1590, head a307ab9ef. Method: code-review with the rust-review lane. The PR adds the FR-224 race summary under `ra`, the derived race-freedom item, the `Race` counterexample kind and the FR-227 vector-clock race replay. Spec read: FR-224, FR-227, TC-669, TC-672, ADR-025 DR-1 to DR-8 and MR-1, and QSpec FR-438 at quire-specification 23dc1ce3.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-224-AC-1 (ra half) | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-2 (ra half) | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-224-AC-4 | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md | examined |
| FR-227-AC-3 | spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md | examined |
| ADR-025 DR-2, DR-3, DR-5, DR-6, MR-1 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md | examined |
| QSpec FR-438 typed form, updates, race replay | quire-specification spec/functional/memory/FR-438-report-data-races.md | context_only |
| race summary | qsl-eval/src/simulation/protocol/race.rs | examined |
| ra carriers, note, race_of, footprint | qsl-eval/src/simulation/protocol/ra.rs | examined |
| queued-send carriers | qsl-eval/src/simulation/protocol/steps.rs:1295-1335 | examined |
| EN-1 race item, race_found | qsl-analyze/src/model_check/check.rs | examined |
| race replay | qsl-replay/src/model_check/race_replay.rs | examined |
| item resolution | qsl-replay/src/model_check/replay.rs | examined |
| closure certificate | qsl-replay/src/certificate/closure.rs | examined |
| request writer | qsl-route/src/deadlock.rs | examined |
| SMT facade | qsl-eval/src/smt/query.rs | examined |
| tests and litmus fixture | qsl-analyze/tests/it/{race_freedom,litmus,weak_memory}.rs | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Race replay accepts a forged race through a release fence. A release fence sets `rel(l)` only for locations already in `messages`, so a relaxed store to a location first touched after the fence gives its message an empty clock. In fence-based message passing (`data := 1; fence release; flag := 1 relaxed` against `r1 := flag relaxed; fence acquire; r2 := data`), the reader gets no happens-before from the writer, and a forged race on `data` settles `reproduced`. | qsl-replay/src/model_check/race_replay.rs:184-198, 253-259 |
| FND-002 | high | The race-freedom item is proved under `tso` without being checked. `Tso` carries no race summary, so every step's `race()` is `None`, and EN-1 returns `Holds`. The closure certificate accepts that result. `race_freedom_items` still derives the item for `tso` subjects (`is_weak()`), so a racy `tso` subject is proved race-free. | qsl-analyze/src/model_check/check.rs:1346-1356; qsl-route/src/deadlock.rs:272-290; qsl-eval/src/simulation/protocol/memory.rs:223-229 |
| FND-003 | medium | Race replay does not model channel loss, duplication or displacement. It ignores `Removal::Lost` removals, duplicated entries and displaced entries, and it matches a `receive` to the first queued entry with the same send node. In a lossy or duplicating channel inside a loop, it joins an older (or no) clock, under-approximates happens-before, and accepts a forged race. | qsl-replay/src/model_check/race_replay.rs:151-179 |
| FND-004 | medium | The initial state's race maps do not follow FR-438's typed form. QSpec FR-438 says that entering an outermost weak `parallel` maps every carrier to `{"write":true,"reads":{}}` for each location. The code starts from an empty `RaceMap`, which encodes `"write":false`. Every state key before a location's first write therefore differs from FR-438. | qsl-eval/src/simulation/protocol/ra.rs:1043-1055; qsl-eval/src/simulation/protocol/race.rs:142 |
| FND-005 | high | The spec's race summary misses races when one location has both atomic and non-atomic accesses (ADR-025 DR-3, QSpec FR-438). Example: `a` runs `x := 1` (non-atomic), then `x := 2` (relaxed), then `y := 1` (relaxed); `b` runs `r1 := y` (relaxed, reads 1), then `r2 := x` (relaxed). RC11 races `b`'s load with `a`'s non-atomic store. The summary's `last_write` is `(a, atomic)`, so `conflicts` reports nothing and the item settles `proved`. The code conforms to the spec, so this is a spec defect. | qsl-eval/src/simulation/protocol/race.rs:202-228; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:589 |
| FND-006 | low | The `race(l)` footprint is narrower than ADR-025 MR-1 states. Only accesses to `l` touch it, whereas MR-1 says every access, fence, flush, send and receive in the weak `parallel` reads and writes `race(l)`. The narrower set is sound because clear, copy and or commute, but that argument is written nowhere. | qsl-eval/src/simulation/protocol/ra.rs:1753-1760 |
| FND-007 | low | The SMT facade refuses the race-freedom item with `item-form`, but SMT subjects are model subjects, for which EN-1 refuses the same request as `RaceOverModel` (`invalid-value`). One request defect gets two codes, and the SMT branch is untested. | qsl-eval/src/smt/query.rs:208-213; qsl-analyze/src/analyze/items.rs:207-212; qsl-analyze/src/model_check/check.rs:1042 |
| FND-008 | low | `disagreement` accepts any `Inconclusive`, not the `ReplayParity` verdict disagreement. A forged prefix that failed to replay (`no_value`) would still pass FR-227-AC-3's forged-race test. | qsl-analyze/tests/it/race_freedom.rs:186-193 |
| FND-009 | medium | Race replay clears `messages` at an outermost `join` only when `threads.len() == 1`. A thread outside the `parallel` that stepped (for example a sender), or another instance's threads, keeps the count above 1, so a second outermost `parallel` indexes depths into the old modification order. | qsl-replay/src/model_check/race_replay.rs:144-148 |
| FND-010 | low | Race replay reads fence orderings as strings (`fence_ordering() -> Option<&'static str>`, `matches!(.., "acquire" \| ..)`), not as the typed `AccessOrdering`. A spelling change would silently drop fence synchronisation. | qsl-eval/src/simulation/protocol/identity.rs:1039-1044; qsl-replay/src/model_check/race_replay.rs:184-186 |
| FND-011 | low | ADR-025 DR-2 calls its happens-before "RC11's definition", yet it includes RA-5's `S` join, which orders every pair of `seq_cst` fences. That is stronger than RC11's happens-before. The engine and replay share it, so replay cannot detect a race that the join masks. | qsl-replay/src/model_check/race_replay.rs:200-205; spec/decisions/ADR-025-weak-memory-models-for-parallel.md:588 |

## Verdict

Not mergeable as is: FND-001 and FND-002 give wrong results through APIs this PR adds. FND-005 is a spec defect, owned by QSpec FR-438 and ADR-025 DR-3. It needs a spec ticket and does not block this PR's code.

Judgement of the coder's stated decisions:

1. `earlier` as the last matching access on the canonical path is sound. The state key holds every race Boolean and the summary, so any path to the state carries a latest access by the named thread, and replay checks it against happens-before.
2. The typed layout matches FR-438-AC-6's post-write values, but the entry state diverges (FND-004).
3. Clearing a queued send's view on later accesses is required by FR-438, which makes the queued view a carrier that is not the writer's own.
4. Yes, this is a soundness bug for the forged-race refusal (FND-003).
5. The `S` join follows DR-2's "as RA-2 to RA-5 join views" and matches the engine. The defect is in the spec wording (FND-011).
6. Refusing is acceptable, but the code differs from EN-1's (FND-007).

Other checks:

- Tests are not tautological. Each AC-1/AC-2/AC-4 assertion fails if the summary or the item is wrong, and the reproduced and inconclusive replay tests pin each other.
- Trace tags match what each test asserts.
- The litmus `Core::look` change adds one operation and two contracts. Every litmus consumer still passes.
- An edge into an already-known state is still checked for a race, because `advance` runs before the `known` lookup.

Evidence: `cargo test -p qsl-analyze --test it -- race_freedom:: load_buffering:: rc11_oracle:: release_acquire:: tso_fairness::` gave 43 passed and 0 failed. A second filter, `race_freedom litmus weak_memory`, gave 44 passed. `cargo test -p qsl-replay -p qsl-route --lib` gave 877 and 18 passed, 0 failed. `make ci` was not run, by brief.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | EN-1 can wrongly prove race-freedom over a channel. It treats queued entries with the same bytes (send node, thread, payload) as one choice, but those bytes leave out the carried view. With `repeat` sending the same payload before and after `data := 1`, a `lose` drops only the older entry and a `receive` takes only the oldest. The path that loses the newer entry and receives the older one is never explored, so the race on `data` is missed and the item settles `proved`. The dedup predates this PR, but this PR's race item now depends on it. | qsl-eval/src/simulation/protocol/channel.rs:352-354, 482 |
| FND-013 | low | Race replay folds a channel's per-key sub-queues into one queue per (channel, instance) and takes the first entry with matching bytes. With a keyed `fifo` channel whose key reads model state, equal-byte entries in different sub-queues can be matched differently from the engine. | qsl-replay/src/model_check/race_replay.rs:122, 146-155 |
| FND-014 | low | Four new tests have no trace tag, in a repo that traces its tests: `a_store_after_the_send_races_with_the_receivers_load`, `the_race_freedom_item_over_a_tso_parallel_is_refused`, `the_race_freedom_item_over_a_model_subject_is_refused` and `a_closure_certificate_over_a_racy_edge_is_rejected_bad_state`. | qsl-analyze/tests/it/race_freedom.rs:515, 536, 559, 601 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | b0015317e: a release fence records `fence = cur` and clears `rel`. `released(l)` falls back to `fence` for any location the thread has not released since. `Clocks::join` joins per location and joins the fence clocks too. This is sound: `cur` only grows, so `fence` covers every earlier `rel(l)`. `a_forged_race_through_release_and_acquire_fences_is_refused` fails on the old rule. |
| FND-002 | still-open | EN-1 now refuses: `TsoRace` at `check_protocol` entry maps to `unsupported_construct`/`not-yet-implemented`, which is the right code and place, like `WeakLiveness`. But `check_protocol_closure` (closure.rs:388) still checks a race-freedom certificate under `tso`, where no step carries a race, so it would accept a forged closure certificate. It needs the same refusal. |
| FND-003 | still-open | Lose, displace and receive-by-bytes now mirror the engine, which also takes the first entry with equal bytes. `Duplicate` does not: it removes the original and appends two copies (race_replay.rs:165-169), which moves the original behind later equal-byte entries. The engine leaves the original in place and appends the copy (`fifo`) or inserts it after the equal entries (unordered). Take [e1, e2] with equal bytes and duplicate e1: the engine has [e1, e2, e1'] and replay has [e2, e1, e1]. The second receive joins e1's older clock where the engine joins e2's, so a forged race is accepted. Also, no test fails on the old rule. |
| FND-004 | fixed | b0015317e: `enter` sets the write Boolean on `tip` for every race location, so every carrier starts at `{"write":true,"reads":{}}` as FR-438 states. Verdicts are unchanged because `last_write` is null. |
| FND-005 | deferred | Filed upstream as STD-491 (QSpec FR-438 / ADR-025 DR-3 spec gap). |
| FND-006 | fixed | b0015317e adds the commutation argument to the MR-1 footprint doc at ra.rs:1707-1716. |
| FND-007 | fixed | b0015317e: the facade refuses `Refusal::RaceOverModel` with code `invalid-value`, matching EN-1. Asserted by `the_race_freedom_item_over_a_model_subject_is_refused`. |
| FND-008 | fixed | b0015317e: `disagreement` now requires `DisagreementCause::Verdicts` with `Violation`/`Success`. |
| FND-009 | fixed | b0015317e: replay clears `messages` only when the `live` view-holding set is just the joining thread. This mirrors the engine's `merge`, which clears when `threads.keys().all(== parent)`. |
| FND-010 | fixed | b0015317e: `Members::Fence` holds `AccessOrdering` and replay matches the typed variants. The encoded spelling is `ordering.label()`, as before. |
| FND-011 | deferred | Filed upstream as STD-491. |
