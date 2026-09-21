---
id: TC-232
title: "A relationship between two object types carries no systems-model kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: verifies
---
# TC-232: A relationship between two object types carries no systems-model kind

## Description

Verify that a relationship whose both ends resolve to declared object types
(a plain navigation relationship) resolves with no systems-model kind,
distinct from a relationship whose ends resolve to declared endpoints (a
Connection candidate under FR-086). Scope: FR-085-AC-3.

Catches an implementation that, having a single shared `RelationshipRecord`
shape for both plain and systems-model relationships, tags every resolved
relationship with a default kind label (for example, always `None` even for
a genuine Connection, or always attempting a Connection classification even
when both ends are plain object types) instead of leaving the kind
determination entirely to the classifier this requirement does not own.

## Test Procedure

1. Declare a relationship whose source and target ends both name declared
   object types (not endpoints), and resolve its ends under this
   requirement.
2. Separately declare a relationship whose source and target ends both name
   declared endpoints owned by Part-classified components, and run this
   requirement's end resolution over it (without running FR-086's
   classification).
3. Inspect the resolved value's carried kind information from step 1 and
   from step 2.

## Expected Results

Step 1's resolved relationship carries no systems-model kind. Step 2's
resolved relationship also carries no systems-model kind at this stage —
resolving endpoint-typed ends is this requirement's scope; classifying the
result as a Connection is FR-086's separate, later step, not something this
resolution step decides. A mutant that has this resolution step itself
assign `Connection` whenever both ends are endpoints (short-circuiting
FR-086's own three-condition check) produces a kind-bearing value in step 2,
failing the scope-boundary assertion.
