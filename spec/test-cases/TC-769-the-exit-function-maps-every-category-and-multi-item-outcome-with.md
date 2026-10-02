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

Scope: FR-285-AC-1 to FR-285-AC-4.

## Test Procedure

1. Map each of the nine table rows: success, violation, undefined, refusal, unsupported, incomplete, an inconclusive `analyze` item with cause `CertificateRejected`, FR-283-AC-1's pending clause, and internal failure; then map the category of FR-281-AC-2's `refuted` terminal record.
2. Map the item-code sets {10, 22}, {0, 30, 20}, {21, 22}, {20, 21}, {0, 10} and {0}.
3. Map `StageFailure::Refused` with a profile-gated cause and with `ill_typed`, `StageFailure::Limit`, `StageFailure::Cancelled`, `StageFailure::Fault`, and `CallFailure::Input`, `Cancelled` and `Fault`.
4. Render FR-100-AC-10's `sum-out-of-domain` evaluation through FR-100's outcome mapping.

Tag the tests `#[trace("TC-769", "<AC id>")]`.

## Expected Results

- Step 1: 0, 10, 10, 20, 21, 22, 22, 0, 30, in that order; the terminal record's category is `Category::Violation` and maps to 10.
- Step 2: 22, 30, 21, 20, 10, 0.
- Step 3: 21, 20, 22, 22, 30, 20, 22, 30.
- Step 4: outcome `{"kind": "undefined", "reason": "sum-out-of-domain"}` with exit 10.
