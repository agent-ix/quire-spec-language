---
id: SR-1928001
title: "code review (with rust-review) of PR #1928 (QSL-1343): linear FR-096 anchor walk and open-composite set"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1928 git diff origin/main...f6447257 (qsl-semantics/src/check/lowering.rs, qsl-replay/tests/it/admitted_record_chain.rs, qsl-replay/tests/it/main.rs)"
review_set: subset
---
# SR-1928001: code review of PR #1928 (QSL-1343)

## Summary

Ticket: QSL-1343. One code-review pass with the rust-review lane folded in, over `git diff origin/main...f6447257` only. Major bugs only.

Examined, clean:

- `relax` equivalence (lowering.rs:5763-5795). The old label-correcting worklist reaches the fixpoint where each node not in `placed` takes the least anchor over the roots that reach it along `reached_nodes` without entering a `placed` node, and each root keeps its own anchor unless a lesser root reaches it. The new walk visits roots in ascending `Location` order and anchors each node once, from the first root to reach it. By induction on that order, when a walk stops at an already-anchored node, every node beyond it was already walked by a lesser-or-equal root, so the result is the same map. Cases checked: a node reached from roots with different anchors (least wins in both); a root reached by a lesser root (skipped by the vacant-entry check, same as old lowering); `placed` nodes (never entered in either; in the second call the roots are pre-filtered against `placed`, and in the first `placed` is empty); roots missing from `graph.nodes` (anchored, not expanded, in both); cycles (the vacant-entry check ends them); roots with equal `Location`s (`Location: Ord` is a total order consistent with `Eq`, so a tie yields an equal value whichever root wins); anchors are copied unchanged along every edge, with no transform; `named_nodes` iteration order does not affect a map result.
- `composites_open` sync. `composites_in_progress` is mutated in three places only: push in `open_composite` (paired insert, after the only `?` there), pop in `close_composite` (paired remove, before `insert_ref(..)?`), and `drain(open..)` in `type_ref`'s refusal path (paired removes). The `contains` guard stops a declaration from being pushed twice, so set and stack stay one-to-one. `drain(open..)` would panic if the stack were shorter than `open`, where the old `truncate` was a no-op, but `build_type_node` pops only composites pushed by its own frames, so the length never falls below the entry length.
- Built and ran `cargo test -p qsl-semantics --all-features --lib -- lowering` (with the brief's skips): 84 passed, 0 failed; and `-- check::region::` (the FR-096 anchor tests, TC-426/TC-427): 8 passed, 0 failed.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

**Clean.** The new `relax` computes the same fixpoint as the old one, and the set stays in sync with the stack on every path.
