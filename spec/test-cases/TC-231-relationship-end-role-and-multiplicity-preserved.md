---
id: TC-231
title: "A resolved relationship end retains its declared role and multiplicity exactly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: verifies
---
# TC-231: A resolved relationship end retains its declared role and multiplicity exactly

## Description

Verify that resolving a relationship end retains its declared role and
typed multiplicity unchanged, with no default or inferred multiplicity
substituted for an end that declares none. Scope: FR-085-AC-2.

Catches an implementation that normalizes an absent declared multiplicity to
a convenient default (for example, `[0, unbounded]` or `[1, 1]`) instead of
retaining "no declared multiplicity" as its own distinct, absent state — a
defect invisible to any test that only supplies ends with an explicit
multiplicity.

## Test Procedure

1. Declare a relationship whose source end declares role `"placed-by"` and
   multiplicity `[1, 1]`, and whose target end declares no role and no
   multiplicity at all.
2. Resolve the relationship's ends.
3. Inspect the resolved source end's role and multiplicity, and the resolved
   target end's role and multiplicity.

## Expected Results

The resolved source end reports role `"placed-by"` and multiplicity exactly
`[1, 1]`. The resolved target end reports no role and an absent
multiplicity — not `[0, unbounded]`, not `[1, 1]`, and not any other
synthesized value. A mutant that defaults an absent multiplicity to
`[0, unbounded]` produces a present multiplicity value on the target end,
failing the absence assertion.
