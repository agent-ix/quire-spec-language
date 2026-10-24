---
id: SR-12240
title: "Gap analysis of quire-spec-language PR #1120 (QSL-910) against FR-219, FR-220, FR-221, FR-225, FR-227, FR-229"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8b46777865c545628c49d8a013a156d110e098c7; PR #1120 diff origin/main...HEAD; FR-219 (TC-664), FR-220 (TC-665), FR-221 (TC-666), FR-225 (TC-670), FR-227 (TC-672), FR-229 (TC-674), ADR-025 MM/MA/TSO/MB/MS/MR/MK, ADR-027 SE-1 to SE-5, FT-1 to FT-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: reviews
---
# Gap analysis of quire-spec-language PR #1120

## Summary

Ticket: QSL-910, the remainder of QSL-565. Ticket text was read as data. The
analysis is planless and scoped to the PR diff. Plan completion: not assessed.

**Tagged ACs.** Each AC tag is checked against its test in
`qsl-analyze/tests/it/weak_memory.rs`:

- FR-219-AC-2 is fully covered. The test covers no member giving `sc`, a
  `tso` entry reaching the nested `parallel`, a nested entry and an attempt
  entry refusing with the entry named, and duplicates (`tso`+`ra`,
  `tso`+`tso`) naming both entries.
- FR-219-AC-4 is fully covered: `JoinPolicy` is settled with 0 product
  states, and `sc` without a member checks.
- FR-220-AC-5 is fully covered: both shapes and the wire text.
- FR-220-AC-11 is fully covered under `tso` and `ra`. The fixture's
  registers are `core.r1` and `core.r2`, not (`c1`, `r`) and (`c2`, `r`).
- FR-220-AC-12 and AC-13 are fully covered.
- FR-221-AC-3 is fully covered: FIFO `[x, y]`, the first flush writes `x`,
  the thread's own load reads 1, another thread's load reads 0, and
  `fetch_add` waits for both flushes.
- FR-229-AC-5 is fully covered.
- FR-220-AC-4 is covered except for "the same 16 states" (FND-001).

Partially covered ACs carry only their TC tag, and the PR body states which
half is missing. That is correct tagging.

**Code with no owner.** None. Every new module cites its FR/ADR clauses.
No stubs. No coverage inflation: the TC-only tags on partial tests do not
claim the ACs.

## Verdict

Mergeable on coverage. The tags claim only fully covered ACs. The gaps below
are honest remainders, and the PR body lists them. The one blocker for PR
#1120 is code-review SR-12239 FND-001 (the closure checker ignores bound
cuts), which this suite also misses (FND-002 here).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-220-AC-4 says `relaxed` and `seq_cst` "reach the same 16 states". The test checks (13, 3) for each and then only `keys[0].len() == keys[1].len()`, not that the sorted state-key lists are equal. Assert `keys[0] == keys[1]`. | qsl-analyze/tests/it/weak_memory.rs:700-719 |
| FND-002 | medium | No test covers the certificate path under a bound. The `MemoryBoundReached` test settles the engine's `Memory` outcome, and the `seq_cst` test certifies with no bound reached. No test checks the closure certificate under a `max_store_buffer` that cuts stores. A test for this would have caught SR-12239 FND-001. | qsl-analyze/tests/it/weak_memory.rs:841-852; qsl-analyze/tests/it/weak_memory.rs:965-1010 |
| FND-003 | medium | The FR-219-AC-3 and AC-5 identity halves are not delivered. The resolved models are a `ProtocolSubject` member, but no obligation-identity digest includes them, because protocol subjects have no request wire. So "pairwise different obligation identities" and "writes `SB::Both` with effective model `sc` into the request" are tested only as `ResolvedMemory` equality. FR-225-AC-1's "equal obligation identities across `max_store_buffer`" is not tested either. This is disclosed and deferred, and the ACs are correctly left untagged. | qsl-eval/src/model_subject.rs:50-53; qsl-analyze/tests/it/weak_memory.rs:450-474 |
| FND-004 | medium | FR-225's Description ("every terminal record ... SHALL state the bound used and whether it was reached") and FR-225-AC-1 are not implemented. Only `MemoryBoundReached` carries the bound. A `proved` or `refuted` record over a `tso` subject does not state `max_store_buffer` 4, not reached. | qsl-replay/src/model_check/settle.rs:260-283 |
| FND-005 | medium | The FR-221-AC-4 litmus table (MP, LB, IRIW, 2+2W, CoRR under `tso`, weak outcome unreachable) and the `send` gate are untested. These are the cheapest whole-model checks that TSO forbids what it should forbid, and only SB is exercised. The FR-221-AC-2 fence half waits on PR #1119. | spec/functional/FR-221-explore-the-x86-tso-memory-model.md:65-66 |
| FND-006 | medium | FR-220-AC-14 is not implemented: a `release` load, an `acquire` store, or an RMW with no ordering, under `tso`, explores instead of settling V-8. The PR cites an open QSpec FR-439 decision. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:174 |
| FND-007 | medium | FR-229-AC-2's `memory` anchor on a `flush` position, FR-229-AC-4 (MR-1 and MR-2 visibility and independence), and FR-227-AC-1's carried memory component per step are not delivered. See SR-12239 FND-002 for the footprint side. | spec/functional/FR-229-carry-the-memory-component-through-the-protocol-system.md:119-121; spec/functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md:93 |
| FND-008 | low | Spec question: FR-221-AC-1 names the counterexample `Sx, Ly, Sy, Lx`, but FR-181's canonical transition order yields `Sx, Sy, Lx, Ly`. The state counts (34 + 4) and the last-position memory and buffers match. Either the AC should be reworded to "a counterexample whose last position ...", or FR-181's order should be reconciled. | spec/functional/FR-221-explore-the-x86-tso-memory-model.md:63; qsl-analyze/tests/it/weak_memory.rs:758-835 |
| FND-009 | low | Spec question: MA-1 counts sharing over attempt footprints only. A model read by a non-attempt step in another branch (an `event`/`send` record enumeration, or a guard) does not make a location shared, so a store to it applies in place under `tso`. This matches the spec as written. ADR-025 should say whether such readers count. | qsl-eval/src/simulation/protocol/access.rs:347-391 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | `memory_bound.reached` is read only from a `MemoryBoundReached` value. A `refuted` run whose exploration cut stores before it found the counterexample states `reached: false`. FR-225 asks whether the bound was reached. The verdict is still sound (MB-4), but the stated flag can be false when stores were cut. | qsl-replay/src/model_check/settle.rs |

## Dispositions

Round 1. Reviewed at 7ff5a7d1acf4cd74e3e769029086101d62dc6fc3; the fix commit is acb5ad013.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed acb5ad013 | The FR-220-AC-4 test now compares the sorted state-key sets for equality. |
| FND-002 | fixed acb5ad013 | `a_closure_certificate_closed_only_under_a_cutting_bound_is_not_proved`: a bound-4 `Holds` certificate checked under bound 1 settles `MemoryBoundReached`, with the same state count as the engine's own run, and `proved` under the default bound. |
| FND-003 | deferred: protocol subjects have no request wire and no obligation-identity digest yet. The ACs are left untagged as remainder, and the PR body records it. |
| FND-004 | fixed acb5ad013 | `ModelCheckRecord.memory_bound: Option<StatedMemoryBound{bound, reached}>`. A `tso` record states `max_store_buffer` 4, not reached, and an `sc` record states none. The litmus test asserts both. See FND-010 for the `reached` flag on a refuted run. |
| FND-005 | fixed acb5ad013 | MP, LB, IRIW, 2+2W and CoRR under `tso` settle `proved`, and their terminal register outcomes equal the `sc` ones (more than one outcome each). The `send` gate is tested, with a non-vacuous `waited > 0`. FR-221-AC-4 is tagged, and the join half is covered by `the_join_waits_for_empty_buffers`. |
| FND-006 | deferred: FR-220-AC-14 waits on an open QSpec FR-439 decision. The PR records it as remainder. |
| FND-007 | deferred: the MR-1 and MR-2 footprints have landed and are tested (TC-666 only). The FR-229-AC-2 `memory` anchor on a flush position and the FR-227-AC-1 carried memory component per step are still remainder and untagged. |
| FND-008 | deferred: spec question for the spec owner (the FR-221-AC-1 step order against FR-181). |
| FND-009 | deferred: spec question for the ADR-025 owner (whether readers other than attempts count for MA-1 sharing). |
