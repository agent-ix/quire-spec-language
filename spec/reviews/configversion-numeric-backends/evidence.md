---
id: SR-415
title: "Evidence review of ConfigVersion numeric backends"
type: SpecReview
analysis: evidence
scope: "IT-010 plus FR-032/FR-033/FR-034 obligations"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: reviews
---

## Summary

Quoin's deterministic advisor reports no mismatch or inconclusive recommendation for FR-032,
FR-033 or FR-034. IT success criteria are not advisor obligations, so their concrete E2E, property
and model-checking evidence choices are recorded as review judgment rather than catalog verdicts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No scoped authored-method mismatch: each related FR criterion is `Test`, and the advisor recommends unit/BDD/E2E evidence, adding model-checking where applicable. | FR-032, FR-033, FR-034 |
| FND-002 | medium | Closed in the test plan: file generation alone is insufficient; IT-010 requires compiled generated Rust, actual proptest execution, actual cargo-kani execution and native replay. | IT-010-SC-02 through IT-010-SC-05 |
| FND-003 | medium | Closed at PR readiness: every IT-010 success token binds to an executing Rust integration test; outside-domain native classification and every generated strategy sample are observed rather than inferred. | IT-010-SC-01 through IT-010-SC-06 |

## Observed Evidence

- E2E/Integration: `tests/configversion_backends.rs` runs native source through package, lowering,
  strict reader, compiled generated Rust and `runtime::execute` comparison over all 16 shared-corpus
  state pairs; `tests/integer_lowering.rs` runs the plain integer oracle and produces its strategy
  and Kani bundles.
- Property: all four generated constructive populations execute; all 778 observed values are
  in-domain and replay against native execution with the same verdict, with explicit zero-discard
  rates.
- Analysis: actual cargo-kani 0.67.0 proves the identity subject and prints a failing changed-state
  playback; its executable digest and exact graph options are asserted.
- Negative controls: out-of-domain admission, changed-state counterexample, malformed playback,
  wrong tool identity and object/graph refusal with source loci.
