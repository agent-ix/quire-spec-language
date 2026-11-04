---
id: SR-2339002
title: "Gap analysis of PR #2339: FreeDelays action (QSL-1667), gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e2e8126e5; PR #2339 diff against FR-204-AC-6..12, FR-196-AC-14, FR-202-AC-8; ticket QSL-1667"
review_set: subset
date: "2026-11-04"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: references
---

## Summary

Ticket: QSL-1667. This is a manual AC-to-test check scoped to the PR diff. Quoin's matrix was not run, because the review was tiered to focused checks. Plan completion: not assessed.

Each changed AC in the PR's stated scope has a tagged test in `qsl-analyze/tests/it/free_delays.rs` that asserts the spec's values: FR-204-AC-6, 7, 8, 9, 10, 11 and 12, and FR-196-AC-14. Hand-checked values and mutations are recorded in SR-2339001. The PR body declares a cut line: WitnessScheduler/WitnessEntry production, replay (FR-202-AC-8, the replay halves of AC-10, 11 and 12) and the FR-196-AC-14 malformed-body refusals are out of scope. The follow-up ticket for that work is "to be filed".

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-204-AC-6 | qsl-analyze/tests/it/free_delays.rs:213 | examined |
| FR-204-AC-7 | qsl-analyze/tests/it/free_delays.rs:258 | examined |
| FR-204-AC-8 / AC-9 | qsl-analyze/tests/it/free_delays.rs:282-283 | examined |
| FR-204-AC-10 | qsl-analyze/tests/it/free_delays.rs:349 | examined |
| FR-204-AC-11 | qsl-analyze/tests/it/free_delays.rs:426 | examined |
| FR-204-AC-12 | qsl-analyze/tests/it/free_delays.rs:457 | examined |
| FR-196-AC-14 | qsl-analyze/tests/it/free_delays.rs:534 | examined |
| FR-202-AC-8 | spec/functional/FR-202-replay-a-probabilistic-witness.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-204-AC-12's text says the row given `[2]` to `B` delay 2 splits "`A` wins 1/2, `B` returns 1/2". Under ADR-026 SD-3, tied identities fire in order while still enabled. With `B` first, `B` resets `x` and keeps `Idle`, so `A` is still enabled and sets `b`. With `A` first, `A` sets `b`. Both orders reach `b`, and the test correctly asserts this. The AC's conclusions are unaffected (max 1, witness delay 2 as the MEC exit, delay-1 chain value 0). Fix in a spec PR: "`B` delay 2 (tie: either order reaches `b`: `A` first sets `b`; `B` first resets `x` and `A`, still enabled, sets `b`)". | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md (AC-12) |
| FND-002 | medium | The trace tags claim whole ACs whose replay or refusal halves are cut: FR-204-AC-10 (path-set replay), AC-11 (subset-row replay refusal), AC-12 (replay `refuted`) and FR-196-AC-14 (byte-order, missing/repeated-row and out-of-window body refusals). FR-204-AC-7's every-scheduler `DelayDistribution` naming `A` has no test in the PR either. The matrix will show them as covered. Fix: file the follow-up ticket before merge with these clauses listed, or add the AC-7 every-scheduler assertion (a one-line `refusal` call). | qsl-analyze/tests/it/free_delays.rs:258,349,426,457,534 |

## Verdict

**CONDITIONAL.** The tests assert the spec's numbers and fail under real mutation of rows.rs and digital.rs. The open items are FND-001 (the spec text is wrong, not the code) and the coverage bookkeeping in FND-002.

## Dispositions (pass 1)

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | FR-204-AC-12 now reads "`B` delay 2 (tie: either order reaches `b`: `A` first sets `b`; `B` first resets `x` and `A`, still enabled, sets `b`)". TC-642 step 8's expected result states the same. `quire validate` exits 0 on both files. |
| FND-002 | fixed | Each test's doc comment in `free_delays.rs` names the clauses it verifies and the cut clauses: the AC-10 path set and replay, the AC-11 B-only replay refusal, the AC-12 replay, the FR-196-AC-14 canonical body and malformed-body refusals, and settled outcome items in general. The cut clauses are listed as remaining work in QSL-1708. The FR-204-AC-7 every-scheduler clause is now asserted: `unsupported`, `DelayDistribution` naming `A`. |
