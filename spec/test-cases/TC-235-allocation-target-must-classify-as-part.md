---
id: TC-235
title: "An allocation whose target does not classify as Part refuses wrong-export"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: verifies
---
# TC-235: An allocation whose target does not classify as Part refuses wrong-export

## Description

Verify that `check_allocation` refuses an allocation whose declared target
element classifies as `Port` (or any kind other than `Part`) with a
wrong-export cause naming both the required kind (`Part`) and the target's
actual kind, and admits an allocation whose target correctly classifies as
`Part`. Scope: FR-086-AC-3.

Catches an implementation that checks only that the target element *exists*
in the domain package (a dangling-reference check) without checking that it
resolves to the specific required kind — a defect invisible to a test that
only supplies a target absent from the package entirely, never one that is
present but of the wrong kind.

## Test Procedure

1. Declare an allocation whose target element names a declared endpoint
   (classified `Port`), not a component.
2. Classify the domain package, confirm the allocation itself classifies as
   `Allocation`, then run `check_allocation` over it.
3. Declare a second allocation whose target element names a declared,
   correctly `Part`-classified component, and run `check_allocation` over
   it.

## Expected Results

Step 2 refuses wrong-export, naming required kind `Part` and actual kind
`Port`. Step 3 admits. A mutant that only checks the target element's
presence in the package (not its resolved kind) admits step 2, failing the
refusal assertion.
