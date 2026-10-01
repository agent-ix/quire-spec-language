---
id: TC-535
title: "Candidates carry and are filtered by the fairness kinds their backends advertise"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-134
    type: verifies
---
# TC-535: Candidates carry and are filtered by the fairness kinds their backends advertise

## Description

Verify that EN-1 registers advertising weak and strong fairness, that the
registry carries each backend's fairness kinds to its candidates, and that
it leaves out a backend lacking a fairness kind the item uses.

Scope: FR-134-AC-1, FR-134-AC-2.

## Test Procedure

1. Register EN-1 and a test temporal backend advertising only `Weak`;
   compute the candidates for an (`temporal-satisfaction`, `unbounded`)
   item with a weak constraint, then with a `strong` constraint.
2. Register only the `Weak` test backend; compute the candidates for the
   item with a `strong` constraint and settle it.

Tag the tests `#[trace("TC-535", "FR-134-AC-n")]`.

## Expected Results

- Step 1: EN-1 with `{Weak, Strong}` and the test backend with `{Weak}`;
  then EN-1 alone.
- Step 2: an empty candidate set reported as FR-075 reports one, naming the
  missing `strong` capability; `Unsupported`,
  `unsupported-requested-capability`.
