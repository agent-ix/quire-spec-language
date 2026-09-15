---
id: Task-041
title: "Evaluate initialized compensation captures"
type: Task
status: done
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-142
    type: verifies
---
## Scope

Fix issue #105 within the existing state-evaluation architecture. Resolve an
admitted compensation capture through its authored initializer and original
anchor, discover the initializer's external inputs, cache the immutable value
for the evaluation, and continue to refuse caller-supplied capture binders.

## Subtasks

- [x] Extend TC-142 across Full and Partial retry/recovery capture reads.
- [x] Resolve initialized captures without accepting a substituted `StateView` value.
- [x] Preserve exact missing-input, source-refusal, exhaustion and replay outcomes.
- [x] Run the required local Rust gates and PR-readiness reviews.
- [x] Record issue #105 closure and the dependent `quire-protocol` repin as
  post-merge delivery actions.

## Delivery

The shared evaluator now discovers an initialized capture's source inputs,
evaluates its admitted initializer at the original anchor, and retains that
immutable value for later reads in the same request. TC-142 exercises Full and
Partial retry/recovery expressions, exact replay and accounting, unavailable
sources, caller substitution, and malformed foreign/cyclic initializer graphs.
Issue #105 closes with the PR; `quire-protocol` #5 repins to the merge commit.
