---
id: TC-769
title: "The exit function maps every category and multi-item outcome, with undefined at 10"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: verifies
---
# TC-769: The exit function maps every category and multi-item outcome, with undefined at 10

## Description

Verify the one exit function over every O-16 category, the severity order, the failure variants and the undefined case.

Scope: FR-285-AC-1 to FR-285-AC-12.

## Test Procedure

1. Map each of the ten table rows: success, violation, undefined, refusal, unsupported, incomplete, an inconclusive `analyze` item with cause `CertificateRejected`, FR-283-AC-1's pending clause, FR-501-AC-4's `Indeterminate` clause, and internal-failure; then map the category of FR-281-AC-2's `refuted` terminal record.
2. Map the item-code sets {10, 22}, {0, 30, 20}, {21, 22}, {20, 21}, {0, 10}, {10, 11}, {11, 22}, {0, 11} and {0}.
3. Map `StageFailure::Refused` with a profile-gated cause and with `ill_typed`, `StageFailure::Limit`, `StageFailure::Cancelled`, `StageFailure::Fault`, and `CallFailure::Input`, `Cancelled` and `Fault`.
4. Render FR-100-AC-10's `sum-out-of-domain` evaluation through FR-100's outcome mapping.
5. Run a source with the source byte limit set to 1, and with `s3.nodes` set to 1.
6. Map `CallFailure::Unsupported` with cause `NotSupplied`, `CallFailure::Unsupported` with cause `Untranslatable`, and `CallFailure::Limit` naming `prepare.compile_work_units`.
7. Map the three payload-carrying failures of FR-285-AC-8 with and without their `protocol_results`.
8. Render a clause pending at the end of its supplied trace and a run whose clauses are all discharged; read each machine-output record.

9. Run FR-501-AC-3's `Pending` case and FR-501-AC-4's `Indeterminate` case, FR-501-AC-1's `Completed(true)` and `Completed(false)` cases and a work-stopped `Incomplete` case through `run_clause`, and read each label and exit from the report and from the multi-item ranking.
10. Read the label and exit of the `Pending` and `Indeterminate` cases and compare them with the catalog's `Category::Pending` and `Category::Indeterminate`; compare `trace_exit_code` with `exit_code` for each of the ten categories; search QSL for a local outcome-to-label or outcome-to-exit mapping.

Tag the tests `#[trace("TC-769", "<AC id>")]`.

## Expected Results

- Step 1: 0, 10, 10, 20, 21, 22, 22, 0, 11, 30, in that order; the terminal record's category is `Category::Violation` and maps to 10.
- Step 2: 22, 30, 21, 20, 10, 10, 22, 11, 0.
- Step 3: 21, 20, 22, 22, 30, 20, 22, 30.
- Step 4: outcome `{"kind": "undefined", "reason": "sum-out-of-domain"}` with exit 10.
- Step 5: each run refuses `stage_limit_exceeded` in category `incomplete`, exits 22, and names its limits field and configured bound.
- Step 6: 21, 21, 22.
- Step 7: 20, 21, 22, each equal with and without the payload.
- Step 8: category `pending`, exit 0, status `pending` and the pending clause ids; the discharged run has neither.
- Step 9: `pending`/0, `indeterminate`/11, `success`/0, `violation`/10 and `incomplete`/22 from the report, each equal to the ranking's.
- Step 10: the catalog's label and exit for each (`pending`/0, `indeterminate`/11); equal trace and plain exits for all ten; no local mapping in QSL.
