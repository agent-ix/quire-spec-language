---
id: TC-848
title: "A fairness premise over a supplied trace is missing, and the clause settles unsupported"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
---
# TC-848: A fairness premise over a supplied trace is missing, and the clause settles unsupported

## Description

Verify that an infinite-trace clause with a non-empty fairness set,
evaluated over a supplied trace with no model subject behind it, returns
`MissingFairnessPremise` naming the first constraint of its fairness set,
evaluates no formula and maps to O-16 unsupported, over a finite prefix,
over an observed lasso and through `run_clause`; and that the same clause
with an empty fairness set evaluates.

Scope: FR-328-AC-6, FR-329-AC-7, FR-330-AC-6.

## Test Procedure

Fixture: the `Counter` unit (FR-124-AC-1), the prefix with `c.value` 0, 1,
2, the observed `Counter` lasso with an empty prefix and loop 0, 1, 2, and
ADR-018 §6's `upd(a)` lasso as `Lasso::Observed`. Each evaluation runs on a
meter that records every position visit.

1. Evaluate FR-328-AC-6's clause over the prefix with `fair weak inc`, then
   with an empty fairness set.
2. Evaluate FR-329-AC-7's clause over the observed `upd(a)` lasso with `fair
   weak attemptUpdate`, then with an empty fairness set.
3. Run FR-330-AC-6's request from source.

Tag the tests `#[trace("TC-848", "<AC id>")]`.

## Expected Results

- Step 1: `MissingFairnessPremise` naming `fair weak whole inc`, O-16
  unsupported, no position visited; then `Completed(false)` at position 0.
- Step 2: `MissingFairnessPremise` naming `fair weak whole attemptUpdate`,
  O-16 unsupported, no position visited; then `Completed(false)` at
  position 0.
- Step 3: stage `evaluate`, `unsupported`, exit 21, cause
  `unsupported_projection`/`missing-fairness-premise` naming `fair weak
  whole inc`.
