---
id: TC-865
title: "An unsupported or incomplete superseding run is unresolved, never holds, and names its limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: verifies
---
# TC-865: An unsupported or incomplete superseding run is unresolved, never holds, and names its limit

## Description

Verify the mutation controls: classifying `unsupported` as admitted, or
incomplete as holds, turns this test red. An incomplete case names the
limit it reached, its value and the case member that raises it.

Scope: FR-343-AC-5, FR-344-AC-4.

## Test Procedure

1. A pair whose `superseding` unit adds a protocol `compensate` construct
   (refused `unsupported_construct`/`not-yet-implemented`), with the
   healthy-parent case. Run the gate.
2. A pair whose `superseding` clause adds conjuncts to the prior's, so its
   evaluation charges more work. Run the healthy-parent case once with
   default limits and read each run's evaluation charge from its usage.
   Set the case's `accounting` work budget to the prior run's charge and
   run the gate.
3. The same pair with the case's work budget set to one unit less than the
   prior run's charge. Run the gate.

Each expected result is a literal in the test. Tag the tests
`#[trace("TC-865", "<AC>")]`.

## Expected Results

- Step 1: the case is `unresolved (unsupported)`; verdict unsupported,
  exit 21.
- Step 2: prior `admitted`, superseding `incomplete`; the case is
  `unresolved (incomplete)`, its entry naming `work_units`, the budget's
  value and the member `accounting`; verdict incomplete, exit 22.
- Step 3: prior `incomplete` at `evaluate`; the case is `unresolved
  (incomplete)`, not `not applicable`.
- No case is `holds`.
