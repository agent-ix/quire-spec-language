---
id: SR-20261105101066
title: "QSL-1737 Rust code review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1737; changed checking/lowering, evaluator/emitter tests, FR-092, FR-093, TC-413"
review_set: subset
---
## Summary

Reviewed all changed production guards, lowering, structural identity and real typed-API evaluator tests. No production correctness defect, unsafe code, dependency change, vendoring or gate weakening was found.

## Scope

- FR-092-AC-29 — examined: "An alias and equivalent reduced endpoints share its identity; changing `m` to `cm` or either endpoint gives a distinct node."
- FR-093-AC-61 — examined: "Bounded nominal quantities lower addition, conversion and less-than to their existing quantity operation identities."
- TC-413 steps 10–12 and implementation status — examined: enabled typed-API tests are distinguished from deferred native and bounded-result conversion evidence.
- TC-413 verifies FR-093 — examined: the added relationship targets the operation requirement.
- QSpec FR-322-AC-384 and FR-097-AC-6 — context_only: upstream representation and retained bare-unit exact-ID controls.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The native test changes endpoint bodies without changing their structural paths, so the unreduced and reversed cases can pass solely on path/body disagreement. Make those cases otherwise identity-consistent and assert the published refusal locus; retain a separate path-disagreement case. | qsl-package/src/emit/extent_agreement.rs:851 |

## Verdict

FAIL pending FND-001. No Cargo or final gate was run by this reviewer. Existing logs establish four lowering passes, one real evaluator pass, two retained refusal-control passes and Clippy success; native coverage is intentionally ignored for IR-1345. Shared canonical rational construction is reused, and no interval is fabricated as a carrier.

No applicable AssuranceProfile was found. Runtime model metadata is unavailable and is recorded as unknown. The author-provided focused logs were inspected; no heavy gate was rerun.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: BQ-06 and BQ-05 update both endpoints and their structural identities; the recursive replacement reaches every nested identity/reference/self type, and the graph is re-sorted. The unreduced case asserts the literal-value locus, the reversed case the node locus; a third case retains deliberate path disagreement. |

## Round 1 verdict

CONDITIONAL source PASS: the finding is fixed by inspection, with no new finding. Focused fix logs show Clippy success, four lowering tests passing and the future IR test compiling but remaining ignored for IR-1345. Native runtime admission and the bounded-result refusal API remain external holds; full acceptance and merge readiness are not claimed. No Cargo or final gate was run by this reviewer.

