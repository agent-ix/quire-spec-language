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
AC-31, including the `inexact_conversion` refusal and the exact `Rational[..]`
carrier. Scope: FR-497-AC-1 to FR-497-AC-8.

## Test Procedure

1. Check and lower the type forms of QSpec `CQ-W01` to `CQ-W04` and the
   `Rational[-3, 7; 1, 12]` carrier of `CQ-W16` by reference and compare the
   nodes with the vectors; check the ineligible carriers and
   carried-operand uses of FR-497-AC-2 from source, with `CQ-W13` as the
   dimension refusal. `CQ-W05` to `CQ-W08`, `CQ-W11`, `CQ-W12` and `CQ-W14`
   are strict-reader wire cases (IR leg) and out of scope here.
2. Lower the conversions of `CQ-W09`, `CQ-W10`, `CQ-W15` and `CQ-W17` to
   `CQ-W19` and compare the `result_type` node and `mode`.
3. Evaluate the rows `CQ-E01` to `CQ-E20` under `CheckMode::Kernel` with the
   operands, work limits and `ScalarLimitsV1` tuples each states, and compare
   outcome, cause, `expected`, charge point and, for non-`exact` modes, the
   `DecimalLoss` record. Include TC-187 U07, U08, U16 to U18, U29 and U44.
4. Evaluate the rows `CQ-E21` to `CQ-E26` (the `Rational[-3, 7; 1, 12]`
   carrier over `m`) under `CheckMode::Kernel` and compare outcome, cause,
   `expected` and charge point.
5. Build the `RefusalRecord` of an `InexactConversion` evaluation and render it
   through the FR-100 kernel-record table.

Tag the test `#[trace("FR-497-AC-1", "FR-497-AC-2", "FR-497-AC-3", "FR-497-AC-4", "FR-497-AC-5", "FR-497-AC-6", "FR-497-AC-7", "FR-497-AC-8", "TC-1287")]`.
Vectors are referenced by QSpec row ID, never copied.

## Expected Results

- Step 1: the four carried forms are four nodes with `unit`, `carrier` and
  `rounding` bindings, the `Rational` one with a `rational_range` carrier and
  `rounding` `exact`; every ineligible carrier and operand use refuses as
  FR-497-AC-1 and AC-2 state.
- Step 2: each conversion's `mode` equals the result node's `rounding`; none is
  `null` or another mode.
- Step 3: every row gives the outcome its QSpec row states; an inexact exact-mode
  value refuses `inexact_conversion` before `unit.target-domain`, also under a
  work limit that denies later charges.
- Step 4: `25 cm` gives `1/4 m` and `-3 m` gives `-3 m`; `1 in`, `8 m` and
  `-4 m` refuse `rational_out_of_domain`/`outside-domain` after
  `unit.target-domain` and before `unit.result-retain`, never
  `inexact_conversion`; a denied `unit.target-domain` charge is `incomplete`
  there; `1/4 m` converts back to `25 cm`.
- Step 5: code `inexact_conversion`, cause `inexact-conversion`, `expected` the
  carrier type, exit 20.
