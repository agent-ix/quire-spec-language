---
id: SR-60027
title: "Re-verification of QSL PR #1588 after the SR-60023 fix cycle (QSL-1098)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1b244db028af7cff160740287fdad147fef1036a merged with origin/main (qsl-eval/src/automaton, qsl-replay closure and item, qsl-analyze check.rs; FR-338-AC-23 to AC-25, TC-525)"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
---
## Summary

Ticket: QSL-1098. Verdict: PASS. Head 1b244db02 was behind main; origin/main merged
into it in a fresh worktree with no conflicts. On the merged tree `make ci`
exits 0 (includes the default-workspace suites, so qsl-analyze tests ran and pass).

Re-verification by my own mutations (each reverted):
- M1 the closure checker ignores the bridge key (every state keyed as the default
  elementary state): all three TP-4 tests fail.
- M2 membership compares model digest only: the accept and the tamper tests fail.
- M3 `Translation::admits` returns true for Liveness: the foreign-context test fails.
So the key-tamper tests (replace, swap, rotate, drop) do fail when the key is ignored, and
the fixture reaches at least two distinct keys, checked against an independent engine replay.

Finding disposition:
- FND-001 fixed: the `Occurrences` struct; clippy is clean in `make ci`.
- FND-002 fixed: new multi-key fixture, independent key replay, mutation-sensitive tests.
- FND-003 fixed: the `.ok()` swallow is gone (key-context refusal propagates as
  `ItemRefusal::Translation`); one `Instance`/`Translation`, with the TP-4 request an
  `Option<Tp4>` private to `build_instances`. The producer calls `instances()`, which passes
  `None`, so a Liveness translation can never reach `certificate()`; the one remaining
  typed-fault arm in qsl-analyze check.rs is unreachable by construction and is a justified
  exhaustiveness arm, not a shim.
- FND-004 fixed: AC-23, AC-24, AC-25 split, each tagged on the test that asserts it;
  TC-525 scope and steps 12 to 14 match.
- FND-005 and FND-006 (low) accepted.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | The accepted certificate in the TP-4 tests is still grown from the checker's own named product states (the model-state digests are not recomputed independently); only the automaton keys are cross-checked against the independent replay. Acceptable for this slice; slice C may add a model-digest oracle. | qsl-replay/tests/it/vault_world.rs | correct-requirement-no-evidence |

## Verdict

PASS. No blocking or medium finding remains.
