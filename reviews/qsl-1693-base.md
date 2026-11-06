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


## Fix review — round 1

Reviewed frozen head `a22cf0c22966f4c1b56aafb4f85adbb3edfb5347` in the ticket's existing worktree. The refreshed IR lock admits the signed-64 string bounds. The first final-gate attempt failed because imported predicates were not called by the three root sources: no Rational literal nodes reached those packages. This fix calls both typed predicates with all four required literals inside each source unit's existing temporal `holds` expression. The model and both declared domains remain unchanged.

The revised oracle independently admits the emitted package bytes, reads each literal with `literal_read()`, checks its type identity against the node's semantic type, requires `BoundedDomainForm::RationalRange` over `ScalarTypeForm::Rational`, then checks the four ordered named `CheckedTerm::Binding` members and exact integer values/types. All four required values must appear in each of the three packages. Removing a root call loses a required map entry; widening or changing any bound fails its exact comparison. There is no raw-JSON-only substitute, unsupported-type skip, new mock boundary, or expansion of kernel projection behavior.

**PASS** — no new or remaining finding in the reviewed feature/fix scope. The prior parser-comment finding remains fixed. Existing specification and EARS conclusions remain unchanged: this fix modifies fixtures and the acceptance oracle, not normative requirements.

Evidence: `/tmp/qsl1693-public-typed-bounds.log` records the exact focused acceptance test passing (one test, all three package kinds) at this head. The original single final gate remains a recorded failure, and was not rerun by this reviewer. No build or test was run by this reviewer; the lead owns the remaining confined Clippy checks. No new worktree, clone, source edit, commit, or push was made.

No applicable AssuranceProfile document was found under this repository's `spec/` tree. Plan completion: not assessed. This is a scoped continuation review, not a whole-repository coverage claim.
