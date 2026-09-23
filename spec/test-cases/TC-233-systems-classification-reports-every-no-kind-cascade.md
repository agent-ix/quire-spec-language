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
cascades (an endpoint missing a direction, a second endpoint whose owning
component key names no declared node, plus one genuinely `Interface`-classified
type) reports both cascades and the correct `Interface` classification, all
three, not only the first cascade encountered. Scope: FR-086-AC-1.

Every fixture record here is a state real intake can produce: a component's
`has_part_signature` is set unconditionally at intake from the construct's
own Quire meaning id (`qsl-semantics/src/model/intake.rs`'s `read_component`), so no fixture
in this test constructs a component that intake could never emit; the two
no-kind cascades instead come from endpoint-level conditions intake genuinely
leaves open (an absent `direction` field, a producer-emitted `owner`
reference to a node the package never declares).

Catches an implementation that stops classification at the first no-kind
cascade instead of continuing through every component, then every endpoint,
then every relationship, then every allocation — a defect a test with only
one failing record cannot distinguish from correct exhaustive behavior.

## Test Procedure

1. Declare a domain package with: one component `C2` that is `Part`-classified
   (as every admitted component is); one endpoint `E1` (owned by `C2`) that
   declares no port direction; one endpoint `E2` whose declared `owner` names
   a component key `C3` that is not declared anywhere in this domain package;
   and one object-type export `I1` that supplies interface features.
2. Run systems classification over this domain package.
3. Inspect the classification's kind map and its refusal list.

## Expected Results

The kind map assigns `E1` no kind and `E2` no kind, and assigns `I1` kind
`Interface`. The refusal list contains two entries: one naming `E1`'s
missing direction, one naming `E2`'s missing owner `C3`, both present
regardless of which record was processed first. A mutant that stops at the
first no-kind cascade produces a refusal list with only one entry, failing
the completeness assertion.
