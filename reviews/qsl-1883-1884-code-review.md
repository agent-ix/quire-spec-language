---
id: SR-2026111203542700000001
title: "code review of PR 2683 finite-field record binders and send-ordered loss queues"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/protocol/program.rs, qsl-eval/src/simulation/protocol/channel.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/protocol_constructs.rs, qsl-analyze/tests/it/protocol_system.rs; consequence read of qsl-replay/src/model_check/race_replay.rs"
review_set: subset
---

## Summary

Tickets: QSL-1883 and QSL-1884. PR agent-ix/quire-spec-language#2683 at head c35db548a, diff origin/main...HEAD (merge base 085738cfc) over 6 files. The rust-review lane is folded in. Spec is already on main: ADR-027 PS-7, ST-3 to ST-5, FR-206-AC-12, TC-651, FR-215-AC-4/AC-5, TC-660.

QSL-1883: `Program::new` and `refuse_unsupported` now take the `ModelSystem` and call `model.enumerates` (`enumerable_in` with the type environment), so a record of finite fields is stepped. All three `Program::new` callers are updated; the crate-level re-export of `enumerable` is dropped and has no other user.

QSL-1884: `Channel::send_ordered` is set exactly for `unordered` + `loss head|tail`. Sends (single and interval), displacement and `duplicate` append on such a queue; `receive` takes any position and suppresses only a candidate equal to the one before it; `lose`/`duplicate` dedup by post-state. `Unsupported::UnorderedLossSelector` is removed with no remaining reference in the workspace.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The independent race replay still models every `unordered` queue as sorted. `race_replay::insert` places a non-displacing send or a `duplicate` copy with `partition_point` whenever `flow.key` is `None`, and pushes a displacing send at the end. The engine now appends on a send-ordered queue, and `QueueFlow` carries no send-ordered flag, so the replay cannot know. After a displacing push the replay queue is no longer sorted, so a later `partition_point` can land before an equal entry. `nth_equal(entry, before)` then takes another entry's clock, and replayed happens-before for a `kind: Race` counterexample over a weak `parallel` can differ from the engine's. Before this PR the case could not arise, because the channel was refused. Fix: tell the replay the queue is send-ordered (a flag on `QueueFlow`, or the channel's order) and append there as the engine does, with a race-replay test over a `loss head` unordered channel. | qsl-replay/src/model_check/race_replay.rs:166-176; qsl-replay/src/model_check/race_replay.rs:206; qsl-replay/src/model_check/race_replay.rs:286-292; qsl-eval/src/simulation/protocol/channel.rs:254 |
| FND-002 | medium | A send-ordered queue is written into the state key as a `bag` whose elements are in send order. The queue encoder chooses `sequence` only when there is a FIFO key, so `[B, A]` and `[A, B]` are both written as `{"type":"bag", ...}`. ADR-027 PS-7 says the state key of such a queue "carries the sequence", and FR-181 orders a bag's elements by their JCS bytes. A reader that normalizes a bag, as FR-181 says a reader does, collapses two distinct states. Fix: write `sequence` for a `send_ordered` queue, with an assertion of the kind in the AC-12 test. | qsl-eval/src/simulation/protocol.rs:1553-1556 |
| FND-003 | medium | The deleted test `check_protocol_refuses_an_unstepped_construct` was the only test that a `ProtocolRefusal::Unsupported` raised when the system is built reaches `check_protocol` as `ModelCheckRefusal::Protocol` with `unsupported_construct`/`not-yet-implemented`. The other tests of that code (weak_memory.rs:2708, hyper_projected.rs:457) go through `case.system` or other entry points, not `check_protocol`. Other unsupported constructs still refuse, for example `SymbolicCapacity` and `Capture`. Fix: point the test at a construct that is still refused, for example `capacity symbolic(..)`, instead of deleting it. | qsl-analyze/tests/it/protocol_system.rs:1836-1872 (origin/main) |
| FND-004 | medium | The finite-field record test is tagged `#[trace("TC-660", "FR-215-AC-4")]`, but FR-215-AC-4 covers an unbounded `Int` field that returns `RequiresBound`, plus `Fill`. The finite-field case already has its own criterion, FR-215-AC-5 (TC-660 step 5): `Msg {n: Int[0,1]; m: Mode; i: Inner}` gives exactly eight `event` steps in domain order and `Wide` returns `RequiresBound` naming the binder, `Wide` and `t`. No test is tagged FR-215-AC-5. This test, two `Boolean` fields and a count of 4, does not meet AC-5 either: no enum, no nested record, no order, no `Wide`. Fix: write AC-5's scenario and tag it `#[trace("TC-660", "FR-215-AC-5")]`. No new AC is needed. | qsl-analyze/tests/it/protocol_constructs.rs:531 |
| FND-005 | low | The `Unsupported::UnenumerableRecord` doc still says it covers "a record, an enum or a bounded text type". A record or enum of finite fields is now stepped, so only an unenumerable one is refused. | qsl-eval/src/simulation/protocol.rs:143-145 |
| FND-006 | low | `deliverable` reads `send_ordered` with `channels.get(key.channel).is_some_and(..)`, so a queue with no channel is silently treated as a bag. Its neighbour `channel_steps` returns `ProtocolRefusal::fault("a-queue-names-a-channel")` for the same broken invariant. Fix: use the same fault. | qsl-eval/src/simulation/protocol/channel.rs:650-654; qsl-eval/src/simulation/protocol/channel.rs:770-774 |

## Verdict

FAIL. FND-001 to FND-004 should be fixed before merge. FND-001 is a real divergence between engine and replay that this PR introduces. FND-004 is a dishonest trace, and the finite-field criterion is still untested.

PS-7 conformance otherwise holds. Each queue position is its own `receive`/`lose` candidate. Suppressing a candidate that is equal to the one before it is exactly the "same post-state is one transition" rule, because removing either of two positions of equal entries gives the same sequence only when every entry between them is equal too. So `[A, B, A]` gives `[B, A]` and `[A, B]`. `lose` dedups by post-state bytes, so it gives the same two states. `duplicate` appends the copy as the newest entry (ST-5), and the copy of either `A` in `[A, B, A]` is one transition. `loss head` displaces index 0 and `loss tail` the last index, the oldest and newest sent, on both the single-send and the interval-send paths. Footprints write the channel's queue for every send and receive, so partial-order reduction cannot treat two sends to the channel as commuting. No panics, no unsafe, no new dependencies.

Do the new tests fail without the code? Yes. Without the PR, both AC-12 tests fail at `sc_system`, because the refusal is raised there. With the old bag-wide dedup, `[A, B, A]` would give one receive (count assertion). With `insert_sorted`, `[true, false, true]` is never reached (`expect`). Without the `duplicate` append change, the duplicate `find` fails. The record test fails on main because `enumerable` refuses every record.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in 70ff900db. `QueueFlow` gained `send_ordered`, set on every send, receive and channel step, and `race_replay` now appends when `flow.key.is_some() \|\| flow.send_ordered`. The special-case push for a displacing send is gone; it is equivalent, because displacement implies fifo or send-ordered. Coverage is weak: the new race test `a_forged_race_after_a_displaced_send_on_a_send_ordered_queue_is_refused` sends entries of equal bytes, where a sorted insert and an append agree, so it would not catch a return of the bug. Accepted, because the replay change is a one-flag mirror of the engine, and the engine's order is pinned by the FR-206-AC-12 tests. |
| FND-002 | fixed | Verified in 70ff900db. The queue encoder writes `sequence` for a `send_ordered` queue, and the AC-12 displacement test asserts the kind `sequence` in the state key. |
| FND-003 | fixed | Verified in 70ff900db. `check_protocol_refuses_an_unstepped_construct` is restored, retargeted at `capacity symbolic(k)`, and asserts `ModelCheckRefusal::Protocol` with `unsupported_construct`/`not-yet-implemented`. |
| FND-004 | deferred | QSL-1892. The test is now tagged `#[trace("TC-660")]` only, with no FR-215-AC-5 claim. It is named `a_finite_field_record_binder_steps_in_domain_order` and covers `Msg {n: Int[0,1]; i: Inner}` (4 steps in order) and `Wide` returning `RequiresBound` at field path [1]. Its doc names the gap left: an event record with a native `enum` field cannot be explored. That engine gap is ticketed as QSL-1892, so the AC-5 tag waits on it. The tag is honest. |
| FND-005 | fixed | Verified in 70ff900db. The doc now reads "a type with a field of no finite domain, such as a text type or a map". |
| FND-006 | fixed | Verified in 70ff900db. `deliverable` returns `ProtocolRefusal::fault("a-queue-names-a-channel")` when a queue names no channel. |
