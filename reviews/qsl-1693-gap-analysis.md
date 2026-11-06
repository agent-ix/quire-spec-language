---
id: SR-20261105101059
title: "gap-analysis review of QSL-1693 Rational bounds"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1693 feature slice; FR-505/TC-1300 and FR-517-AC-12/TC-1312; intake.rs, model_intake.rs, checked_handoff.rs, R model and library"
review_set: subset
---

## Summary

Within the QSL-1693 feature, changed requirements have executable tests and the producer path is checked by independently admitting emitted package bytes and reading each Rational literal's semantic type.

## Verdict

**PASS** — No feature-scope coverage gap found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

The model-intake tests cover safe integer and signed-64 string bounds, malformed forms, early unsafe-number refusal, and refusal ordering. The checked-handoff test exercises all three package kinds, confirms unchanged authored model bounds, and reads the literal types through the reader-admitted public CheckedTerm graph. The feature's producer, reader, and test paths are tied to FR-505 and FR-517. This is a feature-scoped review, not a repository-wide matrix or reverse-gap claim.

## Evidence and limits

Manual feature-scope requirement/test/source review. No build, test, or gate was run by this reviewer; the lead owns the one final scoped gate after the lock refresh.


## Fix review — round 1

Reviewed frozen head `a22cf0c22966f4c1b56aafb4f85adbb3edfb5347` in the ticket's existing worktree. The refreshed IR lock admits the signed-64 string bounds. The first final-gate attempt failed because imported predicates were not called by the three root sources: no Rational literal nodes reached those packages. This fix calls both typed predicates with all four required literals inside each source unit's existing temporal `holds` expression. The model and both declared domains remain unchanged.

The revised oracle independently admits the emitted package bytes, reads each literal with `literal_read()`, checks its type identity against the node's semantic type, requires `BoundedDomainForm::RationalRange` over `ScalarTypeForm::Rational`, then checks the four ordered named `CheckedTerm::Binding` members and exact integer values/types. All four required values must appear in each of the three packages. Removing a root call loses a required map entry; widening or changing any bound fails its exact comparison. There is no raw-JSON-only substitute, unsupported-type skip, new mock boundary, or expansion of kernel projection behavior.

**PASS** — no new or remaining finding in the reviewed feature/fix scope. The prior parser-comment finding remains fixed. Existing specification and EARS conclusions remain unchanged: this fix modifies fixtures and the acceptance oracle, not normative requirements.

Evidence: `/tmp/qsl1693-public-typed-bounds.log` records the exact focused acceptance test passing (one test, all three package kinds) at this head. The original single final gate remains a recorded failure, and was not rerun by this reviewer. No build or test was run by this reviewer; the lead owns the remaining confined Clippy checks. No new worktree, clone, source edit, commit, or push was made.

No applicable AssuranceProfile document was found under this repository's `spec/` tree. Plan completion: not assessed. This is a scoped continuation review, not a whole-repository coverage claim.

Computed matrix attempt: `timeout 45s quire matrix --scope /home/peter/dev/worktrees/QSL-1693-unitless-rationals --format json` ended with exit 124 and no report. This is unavailable computed coverage, not a clean matrix. Manual scoped tag/source reconciliation confirms the active `#[trace("TC-1312", "FR-517-AC-12")]` acceptance test and existing FR-505-AC-1/AC-3 intake tests; no global matrix or reverse-gap completeness claim is made. Optional semantic audit and plan-completeness bookkeeping were not requested.
