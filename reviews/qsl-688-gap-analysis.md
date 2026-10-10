---
id: SR-2450
title: "Gap analysis of quire-spec-language PR #688: exact numeric intake"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@233609db89f4fe7afe11560b6aa0105c325fc9f9; PR #688 changed paths qsl-semantics/src/model/intake.rs, qsl-semantics/tests/it/model_intake.rs, qsl-semantics/Cargo.toml, Cargo.lock"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: reviews }
  - { target: ix://agent-ix/quire-spec-language/FR-260, type: reviews }
---
# Gap analysis of quire-spec-language PR #688

## Summary

The corrected numeric reader has focused coverage for the safe effective range
and exclusive bounds. The changed dependency revision also changes the
architecture fixture's numeric wire spelling. The tagged `TC-730` corpus test
does not carry the new FCD revision's digest, leaving the end-to-end admission
criterion red.

## Verdict

FAIL. One high severity test-to-source gap is present in the changed scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `FR-260-AC-1` requires every corpus package to admit under the digest it had, but the FCD revision changes the architecture package content (`0`/`9` JSON operands become `"0"`/`"9"`) while `model_intake.rs:145` retains the old `aed361…` digest. The updated dependency and the recorded evidence therefore disagree, so the tagged test exercises a stale fixture identity and fails at admission. | qsl-semantics/tests/it/model_intake.rs:111-145; qsl-semantics/Cargo.toml:43-44 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | The changed reader's effective-range implementation contradicts retained `FR-056-AC-16` coverage: an explicit decimal-string `i128::MIN` bound is narrowed to the JCS-safe lower edge because all lower bounds are intersected with `-9_007_199_254_740_991`. FCD's effective-range implementation treats the safe range as the default and lets an explicit bound replace that default. | qsl-semantics/src/model/intake.rs:2273-2315,4735-4763 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 14fbc9c89ac4623e4032f9241f3ef91aaac62714 |
| FND-002 | still-open | The digest correction does not change the safe-range intersection; explicit i128-wide bound coverage remains inconsistent with the implementation. |
| FND-002 | fixed | 233609db89f4fe7afe11560b6aa0105c325fc9f9 |

## Coverage

- Changed production path: `read_value_type` safe-range intersection, exact bound parsing, inclusive/exclusive conversions, duplicate detection, and empty-range refusal examined.
- Changed tests: `reads_effective_integer_range_and_exclusive_bounds` and retained malformed-bound cases examined.
- Dependency evidence: fresh FCD `origin/main` fetch resolved c620d6be; its architecture expected fixture differs from the old 68ace480 fixture in the two integer constraint operands.
- Traceability: `FR-260-AC-1` / `TC-730`, `FR-056-AC-1`, and `FR-056-AC-16` examined. Full repository matrix output was still running at artifact capture time.
- Plan completion: not assessed

The focused locked test was started for this head but could not produce a result in
the shared build lane; the static contradiction in the retained i128 bound test is
independent of that unavailable execution result.

## Disposition pass 2

At `233609db89f4fe7afe11560b6aa0105c325fc9f9`, FND-002 is fixed. Explicit
inclusive and exclusive bounds now replace absent safe defaults and continue to
tighten when a second bound exists on the same side. No new code or gap findings
were identified. The focused model-intake gate is attributed to the dispatching
leader's queued run.
