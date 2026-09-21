---
id: TC-233
title: "Systems classification reports every no-kind cascade, not only the first"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: verifies
---
# TC-233: Systems classification reports every no-kind cascade, not only the first

## Description

Verify that classifying a domain package with several independent no-kind
cascades (a component missing part-signature, an endpoint missing a
direction, plus one genuinely `Interface`-classified type) reports both
cascades and the correct `Interface` classification, all three, not only the
first cascade encountered. Scope: FR-086-AC-1.

Catches an implementation that stops classification at the first no-kind
cascade instead of continuing through every component, then every endpoint,
then every relationship, then every allocation — a defect a test with only
one failing record cannot distinguish from correct exhaustive behavior.

## Test Procedure

1. Declare a domain package with: one component `C1` that does not supply
   the part-signature capability; one endpoint `E1` (owned by a different,
   correctly `Part`-classified component `C2`) that declares no port
   direction; and one object-type export `I1` that supplies interface
   features.
2. Run systems classification over this domain package.
3. Inspect the classification's kind map and its refusal list.

## Expected Results

The kind map assigns `C1` no kind and `E1` no kind, and assigns `I1` kind
`Interface`. The refusal list contains two entries: one naming `C1`'s
missing part-signature capability, one naming `E1`'s missing direction, both
present regardless of which record was processed first. A mutant that stops
at the first no-kind cascade produces a refusal list with only one entry,
failing the completeness assertion.
