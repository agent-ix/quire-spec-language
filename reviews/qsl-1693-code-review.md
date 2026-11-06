---
id: SR-20261105101058
title: "code-review review of QSL-1693 Rational bounds"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; QSL-1693; PR #2430; FR-505/TC-1300 and FR-517-AC-12/TC-1312; intake.rs, model_intake.rs, checked_handoff.rs, R model and library"
review_set: subset
---

## Summary

The feature parses canonical signed-64 Rational bound strings without widening, accepts safe JSON integers, and tests exact type retention in the emitted handoff. The stale parser comment identified in review was corrected.

## Verdict

**PASS** — No open findings remain. FND-001 was fixed by updating the parser comment to match the safe-number and signed-string contract.

## Scope

- FR-505-AC-1 and FR-505-AC-3: bound representation and typed refusals.
- FR-517-AC-12: producer output retains the authored `Ratio` and `Exact` types and bounds.
- `qsl-semantics/src/model/intake.rs`: bound parsing and numeric/string validation.
- `qsl-semantics/tests/it/model_intake.rs`: exact values, malformed strings, unsafe numbers, and refusal order.
- `tests/it/checked_handoff.rs`: emitted models and independently read typed Rational literal values.
- R model and library fixtures: authored bounds and source reachability.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `bound_operand` doc comment says the integer branch does not authorize numeric wire operands, but the caller accepts safe JSON integers through 2^53 under FR-505 and QSpec FR-154. Update the comment to describe the current contract. | qsl-semantics/src/model/intake.rs:4139-4141 |

## Coverage

The checked parser uses a canonical decimal round-trip and an `i64` conversion for strings; the caller enforces the safe-number magnitude. It validates structure and applicability before missing-key checks, validates keys before operand shapes in keyword order, and checks the domain before the optional unit. Tests exercise those paths and assert the emitted bounded domain for each handoff package. No implementation defect was found. The comment was corrected after the review finding.

## Evidence and limits

Independent source review only. No build, test, or gate was run by this reviewer; the final scoped gate remains with the lead after the lock refresh. No Cargo files or implementation files were modified by this reviewer.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The parser comment now states the accepted safe JSON integer and canonical signed-64 string forms. |

After excerpt: `/// The caller admits safe JSON integers through ±2^53 or canonical signed-64 decimal strings under QSpec FR-154 / TC-981.`


## Fix review — round 1

Reviewed frozen head `a22cf0c22966f4c1b56aafb4f85adbb3edfb5347` in the ticket's existing worktree. The refreshed IR lock admits the signed-64 string bounds. The first final-gate attempt failed because imported predicates were not called by the three root sources: no Rational literal nodes reached those packages. This fix calls both typed predicates with all four required literals inside each source unit's existing temporal `holds` expression. The model and both declared domains remain unchanged.

The revised oracle independently admits the emitted package bytes, reads each literal with `literal_read()`, checks its type identity against the node's semantic type, requires `BoundedDomainForm::RationalRange` over `ScalarTypeForm::Rational`, then checks the four ordered named `CheckedTerm::Binding` members and exact integer values/types. All four required values must appear in each of the three packages. Removing a root call loses a required map entry; widening or changing any bound fails its exact comparison. There is no raw-JSON-only substitute, unsupported-type skip, new mock boundary, or expansion of kernel projection behavior.

**PASS** — no new or remaining finding in the reviewed feature/fix scope. The prior parser-comment finding remains fixed. Existing specification and EARS conclusions remain unchanged: this fix modifies fixtures and the acceptance oracle, not normative requirements.

Evidence: `/tmp/qsl1693-public-typed-bounds.log` records the exact focused acceptance test passing (one test, all three package kinds) at this head. The original single final gate remains a recorded failure, and was not rerun by this reviewer. No build or test was run by this reviewer; the lead owns the remaining confined Clippy checks. No new worktree, clone, source edit, commit, or push was made.

No applicable AssuranceProfile document was found under this repository's `spec/` tree. Plan completion: not assessed. This is a scoped continuation review, not a whole-repository coverage claim.

## Disposition round 1

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The parser comment still describes the accepted safe JSON integer and canonical signed-64 string representations. No regression was introduced by the fixture/oracle fix. |

After excerpt: `/// The caller admits safe JSON integers through ±2^53 or canonical signed-64`

The original Linear review marker had `clean: true` while the retained code-review artifact records this already-fixed comment finding. This disposition preserves the artifact's finding verbatim and makes its fixed outcome explicit; no historical finding or comment was rewritten.
