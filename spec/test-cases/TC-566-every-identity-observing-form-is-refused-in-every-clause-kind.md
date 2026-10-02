---
id: TC-566
title: "Every identity-observing form is refused in every clause kind, and transparent forms check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: verifies
---
# TC-566: Every identity-observing form is refused in every clause kind, and transparent forms check

## Description

Verify the refusal in invariants, temporal atoms, `terminal when` predicates and state constraints, the source-order report of several refusals, and that transparent uses of the population still check.

Scope: FR-150-AC-3 to FR-150-AC-4.

## Test Procedure

Fixtures: The annotated ConfigVersion unit of TC-565.

1. Add, one at a time: an invariant comparing `self` to a reference literal for key `a`; a temporal atom converting `c` to text; a `terminal when` predicate with `reduce` over a `Bag` of references; a state constraint with the `fold`. Then add two of them together.
2. Add the invariant `forall p in config_history: p.parent = self implies p != self`, the atom `holds(c.parent = a.parent)` and a `fold` over a `Set<Int[0, 3]>` field.

Tag the tests `#[trace("TC-566", "FR-150-AC-n")]`.

## Expected Results

- Step 1: each refuses at its node's span with the node kind and `config_history`; the pair reports two refusals in source order.
- Step 2: the unit checks.
