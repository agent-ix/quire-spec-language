---
id: TC-791
title: "The seam probe reports the StateModel arms at S1, S2 and S3"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-300
    type: verifies
---
# TC-791: The seam probe reports the StateModel arms at S1, S2 and S3

## Description

Verify that the FR-063 seam probe covers `StateModel`'s hook arms.
Scope: FR-300-AC-3.

## Test Procedure

1. Run `xtask seam-probe`.
2. Remove the `StateModel` S3 hook arm's entry from the probe's expected
   seam list in a scratch copy and run the probe against it.

## Expected Results

1. The probe passes, and its reported locations include the `StateModel`
   arm of the `catalog_code()` prefix match, of the S2 hook match and of the
   S3 hook match.
2. The probe fails and names the S3 arm location as reported but not
   expected.
