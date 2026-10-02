---
id: TC-567
title: "Symmetry declarations admit on an annotated population and refuse malformed forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: verifies
---
# TC-567: Symmetry declarations admit on an annotated population and refuse malformed forms

## Description

Verify admission of `[[a, b, c]]`, its generators and the one `over` instance, and each form refusal before any expansion.

Scope: FR-151-AC-1 to FR-151-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.1's subject: annotated and unannotated units, universe `{a, b, c}`, one all-zero initial state, claim `ReachesTwo`.

1. Pre-check `[[a, b, c]]` over the annotated unit; read the generators and the instance list.
2. Pre-check `[[a, b, c]]` over the unannotated unit, `[[a, b, d]]`, `[[a, b], [b, c]]`, and a declaration for a population without a universe.

Tag the tests `#[trace("TC-567", "FR-151-AC-n")]`.

## Expected Results

- Step 1: generators `(a b)` and `(a b c)`; one instance bound to `a`, orbit `[a, b, c]`, group the class `[b, c]`.
- Step 2: `invalid_runtime_input`/`invalid-value` naming, in turn, the population, `d`, `b` and the population; no state expanded.
