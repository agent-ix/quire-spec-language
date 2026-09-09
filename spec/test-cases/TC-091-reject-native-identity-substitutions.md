---
id: TC-091
title: "Reject static identity and projection substitutions"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: verifies
---
# TC-091: Reject static identity and projection substitutions

## Description

Property, priority P1. Verifies FR-021-AC-3, FR-021-AC-4 and FR-021-AC-5.
Planned; no implementation or execution is claimed. Mutations have independent
expected canonical content and begin with a successfully checked native fixture.

## Test Procedure

Independently change each semantic-definition selection, native/formal source
binding, selected used/unused model declaration or role, authored clause,
resolution, location and runtime obligation in canonical fixture data. Compare
the expected hash with the original. For admissible source/model/authorship
changes also construct the changed real CheckedPackage and compare its emitted
identity. Unsupported definitions and forged derivations remain adverse raw
reader inputs; no caller API is invented to create them as checked packages.

Change only a clause's projection availability and recompute the raw artifact
selector, retaining the unchanged static identity. Substitute the raw package
digest, an actual independently selected IR canonical/bound digest and a digest
from a separately qualified JCS fixture. Mutate canonical domain/version/
algorithm, hex case/width and individual digest bytes. Run through the actual
reader with otherwise correct external bindings and record the first refusal.

## Expected Results

Every changed static dependency produces a different independent canonical
expectation. Available conforming producers agree with their changed expectation;
unsupported or forged inputs receive their specified refusal, not a manufactured
successful package. Projection-only mutation leaves the canonical expectation
unchanged yet fails full manifest comparison. Unknown canonical selections are
unknown_profile; malformed/wrong/foreign-role digests are invalid_package.
No digest match bypasses reconstruction or promotes an unlowered projection.
