---
id: SR-20261105101060
title: "base review of QSL-1693 Rational bounds"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-1693; PR #2430; FR-505/TC-1300 and FR-517-AC-12/TC-1312; intake.rs, model_intake.rs, checked_handoff.rs, R model and library"
review_set: subset
---

## Summary

The QSL-1693 feature matches the merged QSpec FR-154 / TC-981 Rational bound contract. The emitted `Ratio` and `Exact` types preserve their declared bounds, and their literals are checked through the package reader.

## Verdict

**PASS** — No specification defect found in the feature scope.

## Scope

- FR-505-AC-1: exact bounded Rational type admission, aliases, sharing, safe JSON integers, and canonical signed-64 strings.
- FR-505-AC-3: malformed operand refusal and early refusal of unsafe JSON integers.
- FR-517-AC-12: R emits unit-less `Ratio` and `Exact` declarations and reaches both in package values.
- TC-1300 rows 4, 9, and 26: matching bound and refusal vectors.
- TC-1312 row 12: each emitted package independently admits and reads typed Rational literals.
- QSpec FR-154 / TC-981: authoritative upstream contract read as context.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-505 uses the accepted safe-number and canonical signed-64-string representations; its unsafe numeric values are rejected before intake. The handoff criteria require declared bounds to remain unchanged and each emitted Rational value to carry its exact bounded type. No new dependency edge, pin, or independent contract was introduced by this feature.

## Evidence and limits

Specification and source review only. No build, test, or final gate was run by this reviewer; the final scoped gate remains for the lead after Draco's normal IR-main lock refresh. No claim is made about whole-repository coverage.
