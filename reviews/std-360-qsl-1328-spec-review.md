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

## Dispositions

Round 1. `make check-spec-validation` exits 0 with `QSPEC_DIR` set to the QSpec head (quire-specification#396), and origin/main is an ancestor of the head.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | ADR-025 DR-3 and FR-224's `Ra` bullet now say that an RMW whose read part is `acquire` or `seq_cst` decides its write part only after joining the race map it reads into `cur`. QSpec FR-438 says the same. New FR-224-AC-7 and TC-669 step 7 cover both models: the `acquire` RMW is `proved` and the `relaxed` RMW is `refuted` with `earlier` `a`'s load. The QSpec side is FR-438-AC-8 and DR-13. |
| FND-002 | fixed | FR-222 now serializes the view each in-flight `send` carries as the queue entry's `view` member, whether the subject races or not. FR-221 adds the queue entry's `race` member. FR-224-AC-5 and TC-669 step 5 gain the `ra` channel variant. |
| FND-003 | fixed | FR-224's `Tso` list now includes the RMW rule: it reads as a load and writes as a store, its written map joins the read one, and it decides its write part as `Ra` does. |
| FND-004 | fixed | MS-5 now costs the race summary per carrier: a race map of `N(1 + T)` Booleans for each view, message view, buffered store, memory value, thread summary and in-flight `send`, plus the per-location last-write and last-read record. |
