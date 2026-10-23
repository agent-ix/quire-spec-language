---
id: SR-16216
title: "spec-review of the QSL race-summary half (quire-spec-language PR 1328)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@72edb781; spec/decisions/ADR-025-weak-memory-models-for-parallel.md; spec/functional/FR-221-explore-the-x86-tso-memory-model.md; spec/functional/FR-222-explore-the-release-acquire-memory-model.md; spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md; spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md; spec/test-cases/TC-669-race-freedom-item.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: references
---

# Spec review of the QSL race-summary half (quire-spec-language PR 1328)

## Summary

Ticket: STD-360 (spec gap behind QSL-932), QSL half of quire-specification
#396 (a7a49309). One spec-review pass over `git diff origin/main...72edb781`
(6 files, +47/-8). origin/main is an ancestor of the head.

Gates run by the reviewer: `make check-spec-validation` with `QSPEC_DIR` at
QSpec #396's head exits 0.

Examined, clean:

- ADR-025 DR-3 and the FR-224 bullets match QSpec FR-438's carriers, own
  carriers (`cur_b`, `acq_b`), copy-on-assignment, `or`-on-join and the
  decide/update/apply order. FR-224-AC-5 and AC-6 restate FR-438-AC-6 and
  AC-7 value for value. Those values were hand-checked in SR-16215 and hold.
  TC-669 steps 5 and 6 cover them, and step 6 uses the message-passing shape
  with `send`/`receive` that AC-6 states.
- Footprints (ADR-025 MR-1, FR-229). Every access, fence, `flush`, `send` and
  `receive` inside the weak `parallel` reads and writes `race(ℓ)` for every
  racy `ℓ`. That makes all such steps mutually dependent, which is
  conservative and sound. The steps it leaves out are fork, branch entry,
  `join`, and channel `duplicate`/`lose`, which copy or `or` carriers. Each
  of them commutes with a sibling's access: an access sets every non-own
  carrier false, so copy-then-clear and clear-then-copy agree, and so do
  `or`-then-clear and clear-then-`or`. No independence is claimed wrongly.
- MS-2 is consistent: race maps are memory-component data that `raced`,
  replay and MK-1 rendering read, and atoms do not.
- Ids: FR-224-AC-5, FR-224-AC-6 and TC-669 steps 5-6 are unused on main and
  in every open QSL PR.
- Behaviour-only wording; lead defaults D1 to D8 labelled. No follow-up,
  deferral or narrowing.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [soundness] DR-3's order reports a race on an RMW that C11 rules out: "decides `raced` from its pre-state, then updates the Booleans, then applies §3's rule", and FR-224's `Ra` bullet says the same. The QSpec side has the same problem (SR-16215 FND-001). Thread `a` reads `x` non-atomically and then stores `x` with `release`. Thread `b`'s `acquire` RMW of `x` reads that store, so `a`'s read happens before the RMW. But the write part is checked against `b`'s pre-state `cur_b`, which has not joined the read message, so a race is reported. Replay then settles `inconclusive`, `replay-parity`, where the right verdict is `proved`. This holds under `ra` (RA-4) and `tso` (TSO-5). Fix it together with QSpec FR-438: an RMW decides its write-part races after joining the race map its read part reads, when that part is `acquire` or `seq_cst`. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:580; spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md:78-82 | wrong-requirement |
| FND-002 | medium | [serialization] The QSL serialization bullets do not name the queue-entry members that QSpec #396 adds. QSpec FR-437 and FR-425 give every queue entry whose `send` ran inside an `ra` weak `parallel` a `view` member, whether or not the subject races. Under `tso`, a racy subject's queue entry gets a `race` member. FR-222's `Ra` bullet names only "each view's race Booleans as the view's `race` member". FR-221's `Tso` bullet names the element `race` member and the `race-summaries` and `race-memory` entries, but not the queue entry's `race`. Neither FR-229 nor FR-224 names the `ra` queue-entry `view`. A QSL coder following FR-222 would leave the in-flight view out of the key, which is the coalescing defect QSpec D7 fixes. Fix: in FR-222, also serialize each in-flight `send`'s view as its queue entry's `view` member, and in FR-221 add the queue entry's `race` member. | spec/functional/FR-222-explore-the-release-acquire-memory-model.md:67-69; spec/functional/FR-221-explore-the-x86-tso-memory-model.md:55-59 | missing-requirement |
| FND-003 | low | [completeness] FR-224's `Tso` bullet says "SHALL apply FR-438's `tso` carrier rules:" and then lists them, but the list leaves out the RMW rule: the written value's race map also joins the read one's, so a release sequence continues through the RMW. A reader of QSL alone would take the list as complete. Fix: add the RMW clause to the list, or drop the list and cite FR-438 only. | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md:86-95 | missing-requirement |
| FND-004 | low | [consistency] ADR-025 MS-5 still costs the race summary as "Boolean per location and thread (DR-3)". After DR-3's change, every view, message view, buffered element, memory value and queue entry carries a race map (per location, plus a per-thread read map), so the stated state-space factor is too low. Fix: restate the MS-5 race term per carrier. | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:247 | wrong-requirement |

## Verdict

Not mergeable yet: two medium findings. The QSL half matches QSpec #396. Its
footprint rule is conservative and sound, and its ACs mirror the
hand-checked QSpec values. FND-001 needs the same RMW ordering fix as QSpec
FR-438. FND-002 needs one clause each in FR-222 and FR-221. Merge after
QSpec #396, with both halves fixed together.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-005 | high | [soundness] FR-224-AC-7 and TC-669 step 7 copy QSpec FR-438-AC-8 and inherit its defect (SR-16215 FND-004). The vector is racy, so `proved` is wrong under `acquire` too. `b`'s `fetch_add` is enabled from the initial state, so it can read `x`'s initial value instead of `a`'s store. On the path `a` loads `x`, then `b`'s RMW reads the initial value, the RMW's write and `a`'s non-atomic load are unordered: nothing synchronises `a` with `b`. That is a real C11 data race, and the summary reports it at the RMW (`a`'s read Boolean in `cur_b` is false; there is no join, because the initial message carries no release from `a`). On the path where the RMW runs first and `a` then loads, the race completes at `a`'s load. Both races are at depth 2, so the `acquire` variant settles `refuted`, not `proved`. In the `relaxed` variant, which of the two depth-2 races is canonical (and so which access is `earlier`) depends on the canonical breadth-first order, and the vector does not fix it. Fix: make the RMW unable to read the initial value. Give it the precondition `x = 1`, which FR-435-AC-5 makes enabled only when it reads 1. The only remaining path is then load, release store, RMW reading that store: `acquire` is `proved` through the new join, and `relaxed` is `refuted` with `earlier` `a`'s load and `later` the RMW, uniquely. | spec/functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md:129; spec/test-cases/TC-669-race-freedom-item.md:34-36 | wrong-requirement |

## Dispositions

Round 1, at head 7dc7e45d (fix diff checked against the original review head).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | fixed; after: An RMW whose read part is `acquire` or `seq_cst` SHALL decide its write part only after joining the race map of the message it reads into `cur`. |
| FND-002 | fixed | fixed; after: - `Ra` SHALL serialize its component, FR-223's graph and FR-224's race summary as the state key's memory member (FR-229), with each view's race Booleans as the view's `race` member (QSpec FR-438), and SHALL serialize the view each in-flight `send` carries as its queue entry's `view` member (QSpec FR… |
| FND-003 | fixed | fixed; after: an RMW reads as such a load and writes as such a store, its written value's map also joining the one it read, and decides its write part as `Ra` does; |
| FND-004 | fixed | fixed; after: \| MS-5 \| **State-space cost.** Under `tso`, a thread `b` that may store to `s_b` shared locations over value domains of size at most `d` has at most `Σ_{k=0}^{B} (s_b · d)^k` buffer contents with bound `B` (MB-2). Under `ra`, a location has at most `M` live messages (MB-2), each with a value and a… |

Round 2 (author).

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | The RMW now has the precondition `x.v = 1`, so it is enabled only when it reads `a`'s store (QSpec FR-435-AC-5) and no path reads the initial value. With `acquire` the item is `proved` under both models. With `relaxed`, the RMW against `a`'s load is the only race, with `earlier` set to `a`'s load and `later` set to the RMW. QSpec FR-438-AC-8 and DR-13 and QSL FR-224-AC-7 and TC-669 step 7 all say the same. |
