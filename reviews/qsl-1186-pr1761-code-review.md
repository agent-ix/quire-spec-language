---
id: SR-1761001
title: "PR #1761 (QSL-1186d) code review: the predicate-join parallel"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff origin/main...44c004e6a (qsl-semantics lowering/protocol.rs, lowering/protocol/controls.rs, check/mod.rs, protocol_clause/mod.rs, protocol_clause/types.rs; qsl-package emit/tests/protocol_graph.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---

# PR #1761 code review (Rust)

## Summary

Ticket: QSL-1186 (1186d). Head 44c004e6a, on main.

**What changed:**
- **Signature index.** `PendingPredicateJoin` carries the function's signature index, which indexes both `signatures` and lowering's `functions`, so the S3 identity and the written reference are the same key.
- **Dependency order.** `dependencies()` adds each predicate join's function, so the function is keyed before the protocol (#1714).
- **The join Group.** It writes `policy predicate` and `predicate`, a reference to that `function` node, from the same pending join. The identity binding in `check/mod.rs` takes the pending joins only after lowering.
- **Refusal removed.** The not-yet-implemented refusal for a predicate parallel is gone.

**Soundness probes** (probe_1761.diff, reverted):

| Case | Result | Assessment |
| --- | --- | --- |
| **Parameter order vs labels:** `join predicate(leftonly) [b, a]` with `leftonly(x, y) = x and not y` | Writes `joined` `[a, b]` and the two-parameter function; IR reads it back verified. | The `k`-th parameter binds the `k`-th label in ascending order, as FR-466 "Control layouts" and QSL FR-206-AC-11 (`[c, a]`: `f` receives `a` first) both state. Declared order never binds. QSL's evaluator still refuses predicate joins (`Unsupported::JoinPredicate`), so no evaluator disagrees. |
| **Duplicate labels:** `[a, a, b]` | Dedups to `[a, b]` with a two-parameter function and reads back. A three-parameter function over `[a, a, b]` refuses `JoinFunctionType` (expected two). | As FR-218-AC-31 states. |
| **A function from another package:** `predicate(M::both)` | Rejected at parse: a predicate names an identifier, so it can only be a unit function. | — |
| **Recursion through the protocol:** `both` reads `protocol Flow`, which joins on `predicate(both)` | Refuses `ReferenceCycle` (not-emitted, QSL-844) cleanly: no fault, no panic. | — |
| **Helper call:** a predicate function that calls another function | Reads back. | — |

**Gates at 44c004e6a** (`--all-features`, debug info off):

| Crate or check | Result |
| --- | --- |
| qsl-semantics | it 1339, all passing |
| qsl-package lib | 411 passed, 5 ignored; the two known C14/i2 conformance failures (QSL-1190) |
| qsl-analyze `protocol` | 104 |
| Workspace clippy `-D warnings` | exit 0 |
| fmt | exit 0 |

## Verdict

**Clean. Mergeable.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
