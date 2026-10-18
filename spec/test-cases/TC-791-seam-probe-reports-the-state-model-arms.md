---
id: TC-791
title: "The seam probe reports the StateModel arms at S1 and S3 and only the Bound StateModel arm at S2"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-300
    type: verifies
---
# TC-791: The seam probe reports the StateModel arms at S1 and S3 and only the Bound StateModel arm at S2

## Description

Verify that the FR-063 seam probe covers `StateModel`'s S1 and S3 arms and
asserts that the S2 declaration dispatch has exactly one, the `Bound`
arm of FR-380 (ADR-016 FP-1, FP-3).
Scope: FR-300-AC-3.

## Test Procedure

1. Run `xtask seam-probe`.
2. In a scratch copy of the source, add a second `state_model::` call as an
   arm of the S2 declaration dispatch, then separately delete the `Bound`
   arm, and run the probe against each copy.
3. In a scratch copy of the source, delete the `StateModel` arm of the S3
   hook match and run the probe against that copy, its expected seam list
   unchanged.

## Expected Results

1. The probe passes, and its reported locations include the `StateModel`
   arm of the `catalog_code()` prefix match and of the S3 hook match, and
   it reports the S2 declaration dispatch as having the one `Bound` arm.
2. The probe fails and names the extra `state_model::` arm as unexpected in
   the declaration dispatch, and, for the copy without the `Bound` arm,
   names that arm as expected but missing.
3. The probe fails and names the `StateModel` S3 hook arm as expected but
   missing from its report.
