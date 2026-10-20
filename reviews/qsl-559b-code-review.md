---
id: SR-12225
title: "Code review (Rust lane) of QSL-559 E15 PR 2 ProtocolSystem<Sc>: channels, await/timeout, repeat, choice, check, event/commit (PR #1106)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@aaea08dd918f98407927d81c7afee7af6808fc21; qsl-eval/src/simulation/protocol{.rs,/channel.rs,/identity.rs,/program.rs,/settle.rs,/steps.rs}, qsl-eval/src/simulation/model/frames.rs, qsl-semantics/src/check/{mod.rs,protocol_clause/mod.rs,protocol_clause/types.rs}, qsl-replay/src/model_check/counterexample.rs, qsl-analyze/tests/it/{protocol_constructs.rs,protocol_system.rs,fill.rs,main.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-210
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: reviews
---
# SR-12225: Code review (Rust lane) of QSL-559 E15 PR 2 (PR #1106)

## Summary

Ticket: QSL-559. PR quire-spec-language#1106, branch qsl-559b-impl, head aaea08dd9. The review covers `git diff origin/main...HEAD` only: 15 files, +2525/-293. The rust-review lane is folded into this file. QSpec is cited by id only.

No gates were run. Three narrow probes ran as temporary tests in `qsl-analyze/tests/it/protocol_constructs.rs` through `locked-build.sh`. All three were reverted and `target/` was deleted.

1. **Ordinal reuse under a running grandchild (FND-001).** The protocol was `repeat max 2 { parallel Outer { branch p { event F (f: Boolean); parallel Inner { branch c attempt A {f}; branch d attempt Dd } join any [c, d] outstanding continue }; branch q attempt Qq } join all [p, q] }`. Exploring it stops with `runtime_invariant`/`established-invariant-broken`. With `max 1`, or with `outstanding cancel` on `Inner`, it explores 32 and 44 states.
2. **`lose` identity.** In a `fifo` queue `[true, false, true]` from one `send` under `repeat`, two `lose` steps share one identity and reach different states. TS-2 allows this: "Several successors of one identity are told apart by their post-state digest". It is not a finding.
3. The FR-206-AC-4/5, FR-205-AC-3, FR-210-AC-3 and FR-215-AC-4 tests were built, not run as a suite.

## Verdict

**Not mergeable: one major (FND-001).** Two mediums and five lows are also recorded below.

What is sound:

- **Channels (ST-3 to ST-5, PS-7).** There is one queue per (channel, FIFO key) inside an instance, so the key's `queues` member is keyed (instance, channel, FIFO key). A `fifo` queue is in delivery order. An `unordered` queue is a bag kept in ascending entry JCS bytes, so equal bags key equal. `block` drops the send at capacity. `reject` leaves the queue unchanged and records `rejected`. `loss head|tail|new` drops the right entry, and capacity 0 loses the arriving one. `receive` takes the head of each `fifo` key, or any distinct entry of a bag, from the `of` send only, which is ST-4. `duplicate` respects capacity and `lose` removes the entry. Both follow the delivery premise (`unknown` admits both). Empty queues leave the state.
- **Settling (FO-1, FO-2, PS-6).** A `choice` enters exactly one case, and a non-exclusive or non-exhaustive guard is a fault, never a silent pick. A `repeat` resets its counter on entry and counts 1..max in the body. At max it enters `exhausted`, or leaves when there is none. A false guard leaves and clears the counter. A false `check` rests. An undecided guard drops the step and records its finding. Settling is a loop bounded by 6×(controls+1), and S3's progress rule (graph.rs, PS-6) keeps every body cycle through a step node, so the bound is reached only on a broken invariant.
- **Binder visibility (PS-5) in the paths that reuse no ordinal.** `binding` takes the nearest binding by the reader or an ancestor, else the one binding below it. `bind` copies the old binding to each running forked reader's branch below the rebinder, so a branch reads its parent's binder as it stood at the fork. `reads_from` stops at the end of the innermost `repeat` body that holds the binder, follows back edges inside the thread's scope, and returns false at or before the binder (FR-466: readers follow the binder). Retention keeps exactly what `read_by` names for a running reader that can still read. The two-bindings-below fault is unreachable because only an `all` join exports branch binders (protocol_slots.rs `Out(parallel)`), and that requires every branch completed.
- **Key determinism.** `counters` and `queues` are `BTreeMap`s, `map_enc` sorts by JCS, `FifoKey` orders by its JCS bytes, and the stale clear on ordinal reuse removes the old paths' binders and counters. FR-205-AC-3's two completion orders reach one state, which the test asserts.
- **Panic and overflow surface.** No `unwrap`, `expect`, `panic!`, unchecked indexing or `as` casts were added in production code. Every `remove(at)` is bounds-checked first. Capacity is `usize::try_from(..).unwrap_or(usize::MAX)`. Counters use `saturating_add`. Every walk (settle, `reads_from`, `enclosing_loops`, spans) is iterative, so deep nesting cannot overflow the stack. Fan-out runs through the existing budget, meter and state limit.
- **Requires-bound.** `protocol_domains` covers every binder root, `send`/`receive`/`event` payloads included, before exploration. The `values_of(..).ok_or(fault)` paths can only fault when the pre-check is bypassed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Ordinal reuse ignores running descendants of a branch thread that has left the state. `fork` takes the smallest ordinal with no live `ParallelKey` sibling. It then clears every binder and counter under the new branch paths. Take a branch thread `p` that forks `Inner` (`join any … outstanding continue`), completes, and is joined by its parent's `all` join. `p` leaves and its instance leaves. `p`'s child `c` keeps running. The parent's next `repeat` iteration reuses ordinal 0, so it recreates `p`'s path and clears `p`'s binding of `f`, which `c` still reads. The probe in Summary item 1 stops the whole exploration with `runtime_invariant`/`established-invariant-broken`, because `evaluate` fails on the missing binder. An admitted protocol therefore gets an internal error, not a verdict. If the reuse is fixed by clearing less, `c` would instead read the new `p`'s binding, because the new `p` has the same path and is `c`'s nearest ancestor. ADR-027 PS-2 ("no live sibling of the same node holds") does not say whether a sibling whose descendants still run is live. The code and the spec need one reading. The natural one: an ordinal is free only when no live thread's path extends one of its branch paths. | qsl-eval/src/simulation/protocol/steps.rs:430-455, qsl-eval/src/simulation/protocol/steps.rs:800-806 |
| FND-002 | medium | `qsl-eval/src/simulation/model/frames.rs` adds `#[allow(clippy::too_many_arguments)]` on `apply`. Hotfix #1105, still open, replaces this with a `frames::Outputs` struct and is to land first. This PR must rebase onto #1105 and drop the allow. Otherwise the two conflict, or the allow survives the struct. | qsl-eval/src/simulation/model/frames.rs:139-142 |
| FND-003 | medium | `initial_states` maps every `settle` error to `fault("an-initial-state-settles")`. The real cause is discarded, whether that is an `ExpansionStop` from the evaluator (a meter or extent stop, or `broken()`) or a named fault such as `choice-guards-are-exclusive`. Steps after the initial state propagate it. The initial state should too. | qsl-eval/src/simulation/protocol.rs:626-647 |
| FND-004 | low | `send` under `overflow loss head|tail` records `outcome: "lost"` although its own entry was appended and an older one was dropped. Under `loss new` the same word means the send's own payload was lost. Readers of the binder read only the payload (spec-gap reading 1), so nothing reads the difference today, but the identity's `record` conflates the two outcomes. | qsl-eval/src/simulation/protocol/channel.rs:202-216 |
| FND-005 | low | `wait_cause` reports every waiting `receive` as `empty-queue`. That includes one whose queue holds entries that are not deliverable to it: the head is from another `send` of the same channel, or the head fails the receive's constraint. FR-429's table defines `empty-queue` as "a receive with no deliverable entry". ST-4's "deliverable" includes the constraint, so this is defensible, but the name misleads a reader of a deadlock counterexample. Worth a spec-gap note alongside reading 2. | qsl-eval/src/simulation/protocol/steps.rs:942-950 |
| FND-006 | low | A second reasoned `too_many_arguments` allow is added on `admits` (8 parameters). Reasoned allows are repo practice, but `instance`, `path` and `node` are the reader triple that `frame` also takes. One `Reader` struct would remove this allow and shrink the four call sites. | qsl-eval/src/simulation/protocol/steps.rs:295-309 |
| FND-007 | low | The channel qualified name is `format!("{}::{}", protocol.name, channel.name)`, which is the bare protocol name with no package qualifier (spec-gap reading 6). Two packages' protocols of one name give equal `channel` members in `full-queue`/`empty-queue` causes and in `duplicate`/`lose` identities. Within one subject this is unambiguous. | qsl-eval/src/simulation/protocol/program.rs:505-509 |

## Spec-gap readings in the PR body

1. Send record shape: acceptable, with FND-004's nit.
2. `full-queue` cause: needed, since FR-429 has no cause for `block`. It should go to QSpec FR-429 as a table row.
3. Capacity per FIFO key: consistent with PS-7 ("per FIFO key") and FR-425.
4. Counter 1..max, max in `exhausted`, leave at max with no `exhausted`: consistent with PS-6 ("from 0 to the maximum", where 0 holds only during settling) and FO-1.
5. `duplicate` appends at the `fifo` tail: a reading, since FR-170 says "add a copy". It is acceptable.
6. `<protocol>::<channel>`: see FND-007.

## Dispositions

Disposition pass 1 at head b81593a65. The fix commits are ca6608aef and 0e421f2b5, and the branch merges #1105 and main. A probe re-ran the review's FND-001 protocol along both paths, reverted afterwards. The first path is `f = false`, then the second `Outer` fork, then `f = true`: the first `c` takes no `A`. The second path is `f = true`, then the second fork, then `f = false`: the first `c` still takes `A`. The system explores 178 states. The tagged tests and `an_ordinal_stays_held_while_a_descendant_of_its_branch_runs` pass. No new findings.

The ordinal-hold rule is sound and complete. The ordinal is a function of the state (its live `ParallelKey`s and thread paths), so equal keys still choose equal ordinals. Choosing an ordinal enables or disables no step, so no interleaving is lost. A held ordinal stays held only while a descendant runs, so ordinals stay bounded by the live threads and the state space does not grow. With the hold in place, the stale clear only removes paths with no live thread below them. The new wire values `displaced` and `head-blocked`, beside `full-queue`, are spec gaps against QSpec FR-170 and FR-429 and should go to QSpec. They do not block.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | ca6608aef: in `fork`, an ordinal is free only when no `ParallelKey` holds it and no thread's path extends one of its branch paths. |
| FND-002 | fixed | 0e421f2b5: the allow is dropped and #1105's `frames::Outputs` is merged. frames.rs no longer differs from main. |
| FND-003 | fixed | ca6608aef: `settle` returns `SettleError`. An initial state reports a refusal as itself and an evaluation stop as `ProtocolRefusal::InitialStop(stop)`, with the stop's own code and cause. |
| FND-004 | fixed | ca6608aef: `loss head/tail` records `displaced`, and `loss new` records `lost`. |
| FND-005 | fixed | ca6608aef: `receive_cause` gives `constraint` when an entry is deliverable but the constraint fails, `head-blocked` when its send's entries wait behind another send's `fifo` head, and `empty-queue` otherwise. The test is `a_waiting_receive_names_why_it_waits`. |
| FND-006 | fixed | ca6608aef: a `Reader { instance, path, node }` struct replaces the three parameters of `admits`, and the allow is gone. |
| FND-007 | accepted-no-change | The bare `<protocol>::<channel>` name is unambiguous within one subject, and the PR body states it as spec-gap reading 6. |
