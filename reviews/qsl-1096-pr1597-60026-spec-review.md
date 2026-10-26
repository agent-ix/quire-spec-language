---
id: SR-60026
title: "spec-review of FR-128 letter read order and the FR-337 CaptureRead standing decision (quire-spec-language PR 1597)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@325f2f32b27f1ae22554d332504353f4ca270be6; spec/functional/FR-128-replay-a-model-counterexample.md; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md; spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: references
---

# Spec review of QSL-1096 and QSL-1092 text (PR 1597)

## Summary

One spec-review pass over the FR-128, FR-337 and TC-1267 edits, checked against FR-125 (letters, "Instancing", the activation guard), FR-126, ADR-018 UE-1 to UE-6, FR-338 and the engine (`check.rs` `read`, `create`).

- FR-128 read order (guard, captures at an activation, atoms in clause-node order, then the expansion's contract conjunctions) matches ADR-018 UE-1 and UE-4 (EN-1 reads a position's letter when it creates the product state, before expansion) and the engine's `read`, and FR-128-AC-6/AC-7's contract-first statements for the deadlock item. FR-337-AC-13's new sentence and TC-1267 step 7 agree with each other and with the test.
- The `CaptureRead` text is a decision, not a deferral; FR-337-AC-12 names it and the existing tests assert it. See FND-001 and FND-002 for the reasoning and a leftover.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The "Capture observation" reasoning overreaches. It argues that a certificate over keys without the capture vector could not be sound, and concludes the engine returns no verdict. But a `Violated` verdict with its counterexample needs no FR-338 key (replay settles it), and the engine's product state is its own, not an `AutomatonStateKey`; the key argument justifies refusing only a `Holds` that carries a certificate. The deeper obstacle, unstated, is that a letter is one set of atoms per position while an atom that reads a capture is true or false per activation, so concurrent `on each` activations cannot share a letter. State both reasons, and say which verdicts the key argument covers. The claim that the sound key is a set of (state, vector) pairs is right for `on each` and a single pair for `on origin`. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:371-388 | wrong-requirement |
| FND-002 | low | FR-126 still says `CaptureRead` is a refusal "which the engine does not yet decide", contradicting the standing decision now in FR-337. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:119-121 | wrong-requirement |
| FND-003 | low | FR-128's read-order sentence places the state constraint nowhere relative to the letter, while the replay reads it first and the engine reads the letter first (see the code review). Say where the constraint falls. | spec/functional/FR-128-replay-a-model-counterexample.md:176-183, 201-216 | wrong-requirement |
| FND-004 | low | FR-337-AC-13 and TC-1267 step 7 now bundle five clauses (reproduce, closure accept, `BadState`, undefined replay with three altered payloads, the FR-339 wait) into one AC. Each clause is tested, but the AC reads as one criterion; consider splitting the undefined replay into its own AC. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:462 | wrong-requirement |

## Verdict

Mergeable after FND-001 (medium) is fixed; FND-002 and FND-003 are low and belong in the same round. `quire validate` is run on this file and the other two before push.
