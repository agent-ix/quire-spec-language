---
id: TC-542
title: "S3 checks a refinement's assume and ensure rows into F_C and F_A"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: verifies
---
# TC-542: S3 checks a refinement's assume and ensure rows into F_C and F_A

## Description

Verify that `assume` rows check into `F_C` over the concrete model and
`ensure` rows into `F_A` over the abstract model with FR-123's kind and
granularity rules, and that a concrete protocol subject adds its scheduler
constraints to `F_C`.

Scope: FR-137-AC-1 to FR-137-AC-3.

## Test Procedure

Fixtures: `CasRefinesCounter` (ADR-020 §8); FR-136-AC-4's protocol subject.

1. Check `CasRefinesCounter`; with the `ensure` row unmarked; with it
   removed.
2. Check each row variant of FR-137-AC-2.
3. Check the protocol subject with no `assume` row; with `assume fair weak
   each incA`; with `scheduling adversarial`.

Tag the tests `#[trace("TC-542", "FR-137-AC-n")]`.

## Expected Results

- Step 1: six `Weak Each` constraints in `F_C` and `[Weak Each inc]` in
  `F_A`; `[Weak Whole inc]` with a different node identity; empty `F_A`.
- Step 2: `missing_declaration`/`missing-name` naming `Spec`; the
  same naming `Impl`; `missing_declaration`/`missing-name`; one constraint.
- Step 3: three `Scheduler` constraints (branch `A`, branch `B`, root);
  those three and the authored one; the authored one only.
