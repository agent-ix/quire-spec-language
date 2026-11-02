---
id: TC-1287
title: "Carried quantities check, lower and convert"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-497
    type: verifies
---
# TC-1287: Carried quantities check, lower and convert

## Description

Verify that the `Quantity<u, C>` source form checks and lowers to the
`carried_quantity` node, that conversion into it lowers with the node's
rounding, and that the kernel gives the outcomes of QSpec FR-142-AC-28 to
AC-30, including the `inexact_conversion` refusal. Scope: FR-497-AC-1 to
FR-497-AC-7.

## Test Procedure

1. Check and lower the type forms of QSpec `CQ-W01` to `CQ-W04` and the
   ineligible carriers and operand uses of `CQ-W05` to `CQ-W08`, `CQ-W13` and
   `CQ-W14`, by reference, and compare the nodes with the vectors.
2. Lower the conversions of `CQ-W09` to `CQ-W12` and `CQ-W15` and compare the
   `result_type` node and `mode`.
3. Evaluate the rows `CQ-E01` to `CQ-E20` under `CheckMode::Kernel` with the
   operands, work limits and `ScalarLimitsV1` tuples each states, and compare
   outcome, cause, `expected`, charge point and, for non-`exact` modes, the
   `DecimalLoss` record. Include TC-187 U07, U08, U16 to U18, U29 and U44.
4. Build the `RefusalRecord` of an `InexactConversion` evaluation and render it
   through the FR-100 kernel-record table.

Tag the test `#[trace("FR-497-AC-1", "FR-497-AC-2", "FR-497-AC-3", "FR-497-AC-4", "FR-497-AC-5", "FR-497-AC-6", "FR-497-AC-7", "TC-1287")]`.
Vectors are referenced by QSpec row ID, never copied.

## Expected Results

- Step 1: the three carried forms are three nodes with `unit`, `carrier` and
  `rounding` bindings; every ineligible carrier and operand use refuses as
  FR-497-AC-1 and AC-2 state.
- Step 2: each conversion's `mode` equals the result node's `rounding`; none is
  `null` or another mode.
- Step 3: every row gives the outcome its QSpec row states; an inexact exact-mode
  value refuses `inexact_conversion` before `unit.target-domain`, also under a
  work limit that denies later charges.
- Step 4: code `inexact_conversion`, cause `inexact-conversion`, `expected` the
  carrier type, exit 20.
