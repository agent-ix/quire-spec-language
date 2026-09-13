---
id: SR-413
title: "Failure-domain review of ConfigVersion numeric backends"
type: SpecReview
analysis: failure-domain
scope: "IT-010 failure, identity, purity and graph boundaries"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: reviews
---

## Summary

The review examined tool and pin failures, model-domain edges, native/Kani state transitions,
generated-code execution, playback decoding and unrepresentable object/graph forms. All identified
failure domains now have explicit fail-closed outcomes and no open finding remains.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Closed: `-1` and `1001` cannot be evaluated as ordinary Boolean oracle inputs; both paths retain invalid-domain classification and no Boolean truth. | IT-010-SC-03 |
| FND-002 | high | Closed: Kani playback must decode the exact printed counterexample, keep its result in-domain and replay that same pre/post pair; substituting a convenient fixture is forbidden. | IT-010-SC-05 |
| FND-003 | medium | Closed: malformed playback, missing/mismatched cargo-kani, proof timeout/failure and generated compilation failure cannot be reported as a proof or parity result. | IT-010-SC-01, IT-010-SC-05 |
| FND-004 | medium | Closed: ParentOrder and NoCycle terminate at the earliest representational boundary with clause/span identity and no partial artifact; cycles are not traversed or approximated in this backend slice. | IT-010-SC-06 |

## Failure Controls

Generated oracles are pure functions over copied primitives. Native runtime inputs remain immutable,
and Kani subjects are isolated compiled fixtures. Model, clause, source, snapshot and invocation
identities are compared in their own domains. Every unsupported or unavailable stage stops before a
downstream verdict can be claimed.
