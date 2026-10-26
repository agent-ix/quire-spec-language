---
id: SR-60025
title: "gap analysis of QSL-1096 FR-128-AC-5 and FR-337-AC-13 authored undefined replay (quire-spec-language PR 1597)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@325f2f32b27f1ae22554d332504353f4ca270be6; spec/functional/FR-128-replay-a-model-counterexample.md; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md; spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-539
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1267
    type: reviews
---

# Gap analysis of QSL-1096 (PR 1597)

## Summary

Plan completion: not assessed. AC-to-test trace over the diff:

- FR-128-AC-5: `lasso_replay::an_authored_clauses_undefined_evaluation_replays` (TC-539, FR-128-AC-5): reproduces, `where` 1, prefix truncated to value 1, another locus, another cause.
- FR-337-AC-13 undefined-z half: `model_check_captures::replay_reproduces_an_undefined_capture_located_at_it` (TC-1267); the closure half and the guarded replay carry their own AC-13 tags, so the half-tag is the existing precedent.
- FR-337-AC-12: the standing refusal is asserted by the two existing AC-12 tests (no state charged, unbounded root precedence); only the AC text changed.
- `AuthoredUndefined`: no reference remains outside historical reviews (code, docs, ADRs, wire-code tables, spine mapping).

Gates: focused `cargo test -p qsl-analyze --test it -- lasso_replay model_check_captures model_settle`: 30 passed. The aggregate gate result is in the Verdict.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-128-AC-5 | spec/functional/FR-128-replay-a-model-counterexample.md | examined |
| FR-128-AC-6 | spec/functional/FR-128-replay-a-model-counterexample.md | examined |
| FR-337-AC-12 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-337-AC-13 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| TC-539 | spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md | examined |
| TC-1267 | spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md | examined |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-128-AC-6 (authored `always holds(true)` with the undefined `pre CanInc` reproducing at position 0, and inconclusive when located at the atom) and AC-8's authored claim became reachable when the refusal was removed, and AC-6's spec text says they reproduce, but no test carries FR-128-AC-6, AC-7 or AC-8 (the only TC-539 tests are the deadlock item and AC-5). The ordering claim FR-128 adds rests on AC-6, so it is unguarded (mutation: contracts before the letter, all tests pass). Add AC-6's two payloads. | spec/functional/FR-128-replay-a-model-counterexample.md:251; qsl-analyze/tests/it/model_settle.rs:526-533 | correct-requirement-no-evidence |
| FND-002 | low | FR-128-AC-5 is tagged as a whole, but the test uses a closure-driven atom undefined at value 2 with the evaluator's `UndefinedCause` unasserted and `trace_position` compared with the payload's own, where the AC names FR-126-AC-9's `division-by-zero` at `trace_position` 2. The reproduce, `where`, truncation and locus clauses are asserted; the stated value and cause are not. Assert position 2 and the cause, or reword the AC to the fixture. | qsl-analyze/tests/it/lasso_replay.rs:505-562; spec/functional/FR-128-replay-a-model-counterexample.md:250 | correct-requirement-no-evidence |
| FND-003 | low | The protocol replay of an authored clause's undefined evaluation, which the PR claims to serve, has no AC and no test. FR-128 says a protocol counterexample replays by the same rule (the constraint paragraph); add an AC or a test. | qsl-replay/src/model_check/replay.rs:713-731 | correct-requirement-no-evidence |
| FND-004 | low | Open-item text elsewhere still describes the refusal: analyze.rs "Open" notes for FR-281-AC-2 and FR-281-AC-7 say replay refuses an authored clause until FR-337, so those ACs are recorded open for a reason that no longer holds. Re-assess whether AC-2 and the replay of AC-7 can now be closed. | qsl-analyze/tests/it/analyze.rs:7-11, 22-23, 311-317 | correct-requirement-no-evidence |

## Verdict

PASS with findings: both claimed ACs have real, mutation-checked tests; the removed refusal leaves no dead code. Fix FND-001 in this PR; the others are low. Aggregate gate `make ci` (QSPEC_DIR=quire-specification): exit 0, including the string-edge, typestate and fmt/clippy lanes and the all-features test lane.
