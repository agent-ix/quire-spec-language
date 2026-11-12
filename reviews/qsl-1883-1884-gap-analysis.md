---
id: SR-2026111203542700000002
title: "gap analysis of PR 2683 finite-field record binders and send-ordered loss queues"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-206-AC-12, TC-651 steps 12-14, FR-215-AC-4, FR-215-AC-5, TC-660 step 5, qsl-eval/src/simulation/protocol/program.rs, qsl-eval/src/simulation/protocol/channel.rs, qsl-analyze/tests/it/protocol_constructs.rs"
review_set: subset
---

## Summary

Tickets: QSL-1883 and QSL-1884. PR agent-ix/quire-spec-language#2683 at head c35db548a, scoped to the diff. Plan completion: not assessed. Requirement-to-test-to-code check, done by hand against the diff. FR-206-AC-12 is tagged on two new tests with TC-651. The finite-field record test is tagged FR-215-AC-4, which is wrong: see code-review SR-2026111203542700000001 FND-004, not repeated here. Every changed code path is owned by ADR-027 PS-7/ST-3 to ST-5 (FR-206) or FR-215 "Model checking a protocol subject". There is no stub or placeholder code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two clauses of FR-206-AC-12 have no assertion. (a) The AC says "each queued entry is still deliverable to a matching `receive`", and TC-651 step 12 says "both remaining entries are each receivable". The displacement test takes no `receive` after the third send. The `[A, B, A]` test's receive constraint `{ r }` admits only `A`, so a queued `B` is never shown to be deliverable. (b) The AC requires "the same sends made in the order `P2` then `P1`" to displace `P2` under `loss head` and `P1` under `loss tail`. The test keeps the send order `P1`, `P2` and swaps the payloads. That shows the order is not byte order, but it would also pass for a selector that picks a fixed node. Fix: add a `receive` that accepts any payload after the third send and assert both survivors are taken, and add the `P2`-then-`P1` send order as the AC writes it. | qsl-analyze/tests/it/protocol_constructs.rs:381-437 |
| FND-002 | low | PS-7 and ST-5 make `lose` give one step per position on a send-ordered queue. The code does that through post-state dedup, but no test has a `lose` over `[A, B, A]`, and AC-12 does not ask for one. A regression to the bag-wide dedup for `lose` would pass every test. Fix (optional): add the `lose` half to the `[A, B, A]` test under an `at-most-once` delivery. | qsl-eval/src/simulation/protocol/channel.rs:786-800 |

## Verdict

PASS WITH FINDINGS for coverage. Every changed path has an owning requirement, and both PS-7 behaviours the tickets name have tagged tests that fail without the code. FND-001 leaves part of FR-206-AC-12 unasserted. FR-215-AC-5 has no test at all (code-review FND-004).

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in 70ff900db. The displacement test now runs both send orders, `P1`-then-`P2` and `P2`-then-`P1`. For each selector and order it adds `receive R via ch of <survivor> { true }` for both the kept entry and `N`, and asserts exactly one receive is enabled. |
| FND-002 | fixed | Verified in 70ff900db. The `[A, B, A]` test adds an any-payload `receive` and an `at-most-once` `lose`, and asserts each gives the three states `[B, A]`, `[A, A]` and `[A, B]`. |
