---
id: SR-9023
title: "Evidence review of QSL-902 temporal replay"
type: SpecReview
analysis: evidence
scope: "spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-072-implement-typed-replay-result.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md, spec/test-cases/TC-847-an-undefined-letter-fails-a-supplied-trace.md"
review_set: all
---

## Summary

The authored verification methods for the observed-request round trip, facade dispatch, temporal-result and clause-identity criteria are `Test`. TC-847 covers the observed request-arm refusals, source and position mismatches, false/true/pending outcomes, typed insufficient-data/incomplete outcomes, same-position locus/cause mismatches, and their O-16 categories. The catalog advisor did not complete, so its deterministic recommendations could not be confirmed.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | `quoin advise --repo . --inconclusive-only` produced no output for over three minutes and was interrupted with exit 130; no catalog recommendation is claimed for the new observed-replay obligations. The authored `Test` methods have concrete TC-847 procedures, but should be compared with advisor output once the command completes on a supported tool version. | FR-071-AC-8, FR-072-AC-6, FR-098-AC-28, FR-331-AC-7, TC-847 |
| FND-002 | low | Resolved in the final second-fix-round verification: TC-847 now asserts the missing-document digest, model payload, function target, `ReplaySource::Input` and envelope/payload position mismatch refusals before document admission, plus the clause-node mismatch. | FR-071-AC-8, FR-331-AC-7, TC-847 |
| FND-003 | low | Resolved in the final second-fix-round verification: TC-847 explicitly asserts that `Completed(false)` has O-16 category `violation` and `Completed(true)` has category `success`; its category list also maps false and undefined to violation. | FR-072-AC-6, TC-847 |

## Round 2

Confirmed in TC-847 step 5: the observed request rejects `ReplaySource::Input`, a missing document digest, model payload, function target and envelope/payload position mismatch before admission; clause-node mismatch also refuses before admission. Undefined same-position locus-only and cause-only mismatches settle `ReplayParity`. False, true and pending preserve typed values; insufficient-data preserves all typed fields; incomplete preserves its typed charge point; matching non-empty fairness produces the typed missing-fairness refusal before formula evaluation. Explicit O-16 categories are stated for false/undefined, true, pending, insufficient-data, incomplete and unsupported outcomes. FND-002 and FND-003 are resolved. The evidence advisor remains unavailable as recorded in FND-001; no advisor rerun was performed.
