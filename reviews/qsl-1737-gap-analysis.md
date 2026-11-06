---
id: SR-20261105101067
title: "QSL-1737 coverage gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1737; changed checking/lowering, evaluator/emitter tests, FR-092, FR-093, TC-413"
review_set: subset
---
## Summary

Audited the changed requirement → trace tag → assertion → production-code chains. Plan completion: not assessed. Public typed declarations run the real checker, lowering and evaluator without claiming bounded source grammar.

## Scope

- FR-092-AC-28 — examined: "An alias and equivalent reduced endpoints share its identity; changing `m` to `cm` or either endpoint gives a distinct node."
- FR-093-AC-61 — examined: "Bounded nominal quantities lower addition, conversion and less-than to their existing quantity operation identities."
- TC-413 steps 10–12 and implementation status — examined: enabled typed-API tests are distinguished from deferred native and bounded-result conversion evidence.
- TC-413 verifies FR-093 — examined: the added relationship targets the operation requirement.
- QSpec FR-322-AC-384 and FR-097-AC-6 — context_only: upstream representation and retained bare-unit exact-ID controls.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed-unit identity test uses metre versus second, changing dimension as well as unit. Add the required same-dimension metre/centimetre case so a dimension-only identity regression cannot pass. | qsl-semantics/src/check/lowering/model/tests/quantities/bounded.rs:32 |

## Verdict

FAIL pending FND-001. No unspecified production behavior or stub was found. FR-093-AC-61 is explicitly partial: bounded-result conversion lacks its shared refusal API. Ignored native coverage is not counted as passing runtime evidence. The repository-wide computed matrix produced no result during approximately five minutes and was stopped; this review uses an explicitly manual index of the six changed trace-tagged tests, not a computed coverage claim.

No applicable AssuranceProfile was found. Runtime model metadata is unavailable and is recorded as unknown. The author-provided focused logs were inspected; no heavy gate was rerun.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: the fixture admits centimetre as 1/100 metre within Length, explicitly asserts shared dimension, proves distinct bounded identities and checks that replacing only the nominal unit path operand makes the paths equal. The metre/second control remains. |

## Round 1 verdict

CONDITIONAL source PASS: the finding is fixed and the focused log records all four lowering tests passing. No new gap was found in the fix. Native runtime admission and bounded-result conversion remain explicitly incomplete; the prior manual-index and computed-matrix limitation stands. No Cargo or final gate was run by this reviewer.

