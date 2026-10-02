---
id: TC-588
title: "An undefined claim evaluation found on a reduced run refutes with a concrete prefix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: verifies
---
# TC-588: An undefined claim evaluation found on a reduced run refutes with a concrete prefix

## Description

Verify that a symmetry-reduced run finds an undefined claim evaluation the
unreduced run finds, and that it settles `refuted` with cause
`UndefinedEvaluation` through a concrete, replayed prefix whose record names
no reduction.

Scope: FR-160-AC-5.

## Test Procedure

Fixture: ADR-021 §7.1's subject with its symmetry declaration, and
`always holds(2 / (2 - c.versionNumber) >= 1)` for `c = a`.

1. Run the item with symmetry and settle it after replay.
2. Run the item unreduced.

Tag the test `#[trace("TC-588", "FR-160-AC-5")]`.

## Expected Results

- Step 1: `refuted`, `decisive-counterexample`, cause `UndefinedEvaluation`
  with `division-by-zero` at a state where `a.versionNumber = 2`; the record
  names no reduction; the prefix has two steps.
- Step 2: `refuted` with an `UndefinedEvaluation` prefix of two steps.
