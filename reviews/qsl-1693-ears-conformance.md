---
id: SR-20261105101061
title: "ears-conformance review of QSL-1693 Rational bounds"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; QSL-1693; FR-505/TC-1300 and FR-517-AC-12/TC-1312"
review_set: subset
---

## Summary

The changed requirement text identifies the package intake reader and the R library, names the numeric boundary, and states observable refusal or output behavior. No feature-specific EARS ambiguity was found.

## Verdict

**PASS** — No EARS finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

FR-505's canonical-number rule identifies the package check, the threshold, the refusal code, and the document pointer. FR-517-AC-12 names R's model and library and the emitted typed Rational literals. The statements are concrete and testable; no wording changes are required.

## Evidence and limits

Manual EARS review of the changed feature statements. No build, test, or final gate was run by this reviewer.


## Fix review — round 1

Reviewed frozen head `a22cf0c22966f4c1b56aafb4f85adbb3edfb5347` in the ticket's existing worktree. The refreshed IR lock admits the signed-64 string bounds. The first final-gate attempt failed because imported predicates were not called by the three root sources: no Rational literal nodes reached those packages. This fix calls both typed predicates with all four required literals inside each source unit's existing temporal `holds` expression. The model and both declared domains remain unchanged.

The revised oracle independently admits the emitted package bytes, reads each literal with `literal_read()`, checks its type identity against the node's semantic type, requires `BoundedDomainForm::RationalRange` over `ScalarTypeForm::Rational`, then checks the four ordered named `CheckedTerm::Binding` members and exact integer values/types. All four required values must appear in each of the three packages. Removing a root call loses a required map entry; widening or changing any bound fails its exact comparison. There is no raw-JSON-only substitute, unsupported-type skip, new mock boundary, or expansion of kernel projection behavior.

**PASS** — no new or remaining finding in the reviewed feature/fix scope. The prior parser-comment finding remains fixed. Existing specification and EARS conclusions remain unchanged: this fix modifies fixtures and the acceptance oracle, not normative requirements.

Evidence: `/tmp/qsl1693-public-typed-bounds.log` records the exact focused acceptance test passing (one test, all three package kinds) at this head. The original single final gate remains a recorded failure, and was not rerun by this reviewer. No build or test was run by this reviewer; the lead owns the remaining confined Clippy checks. No new worktree, clone, source edit, commit, or push was made.

No applicable AssuranceProfile document was found under this repository's `spec/` tree. Plan completion: not assessed. This is a scoped continuation review, not a whole-repository coverage claim.
