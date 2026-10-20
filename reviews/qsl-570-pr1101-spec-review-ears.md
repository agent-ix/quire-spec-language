---
id: SR-13571
title: 'Spec review of PR #1101: ConfidenceBeyondPrecision settles with a typed cause, ears'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@f810277ee; git diff refs/remotes/origin/main...f810277ee (PR #1101, branch std-192-beyond-precision-spec): spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md, spec/test-cases/TC-627-statistical-results-settle-as-measured.md; compared with PR #1079 branch qsl-570-impl dd3a654d7, merged #1098 (main 8a5569839) and QSpec PR #300 head 1d1d3e976 FR-331, FR-408, FR-290, backend-provider-v1 schema and vectors; ticket QSL-570'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-627
  type: reviews
---

## Summary

Ticket: QSL-570. PR: quire-spec-language#1101, head f810277ee, scoped to `git diff refs/remotes/origin/main...f810277ee` (one commit on main 8a5569839, which is #1098 merged; two files, +5/-4). Spec-only. FR-192 moves the `ConfidenceBeyondPrecision` refusal out of the shared `unsupported_projection`/`unsupported-requested-capability` diagnostic row into its own row, which settles `unsupported` with the typed cause `confidence-beyond-precision` and terminal value `Unsupported(cause)`. FR-192-AC-1 and TC-627 step 1 cover the new row. Checked against terra's engine (origin/qsl-570-impl dd3a654d7): `check_statistical` returns `StatisticalRefusal::ConfidenceBeyondPrecision` for a regenerative request whose `normal_quantile_upper` reports `BeyondPrecision`, before any sample is drawn (statistical.rs:666-681). The settlement map is not implemented yet, so the row is a spec-level mapping. FR-189's refusal list (line 96) still names the refusal, and FR-190 does not mention it. QSpec PR #300 (head 1d1d3e976, open) spells the cause `confidence-beyond-precision`, a payload-free cause, and adds it in four places: FR-408 Admission (line 57), FR-331's closed `unsupported` cause list, the backend-provider-v1 schema's `UnsupportedCause` enum, and FR-290's negotiation sentence. Its vectors hold a positive case (`results/2/cause/type` = `confidence-beyond-precision`) and an adverse one (`confidence-beyond-precisions`, refused). The FR-192 spelling matches. Merge order: the row cites a cause that QSpec main (2aadb7e) does not yet admit. On main, FR-331 would refuse the record with `invalid_result`/`unknown-cause`. The PR body's 'Merge after QSpec #300' is therefore the right order, and #1101 should not merge while #300 is open. Methods: spec-review/base (SR-13570), spec-review/ears (SR-13571), spec-review/integrity (SR-13572). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-192 or TC-627; tools/check-index-completeness.sh passes. The diff has no status wording. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: EARS conformance of the changed statements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-192-beyond-precision-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-diagnostic-refusal-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-AC-1 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

No findings. The change adds no SHALL statement. Both rows sit under FR-192's existing 'The map ... SHALL be exhaustive' statement, and the AC is a direct assertion. The quire grammar lint reports nothing in FR-192 or TC-627.
