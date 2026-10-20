---
id: SR-13572
title: 'Spec review of PR #1101: ConfidenceBeyondPrecision settles with a typed cause, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-570. PR: quire-spec-language#1101, head f810277ee, scoped to `git diff refs/remotes/origin/main...f810277ee` (one commit on main 8a5569839, which is #1098 merged; two files, +5/-4). Spec-only. FR-192 moves the `ConfidenceBeyondPrecision` refusal out of the shared `unsupported_projection`/`unsupported-requested-capability` diagnostic row into its own row, which settles `unsupported` with the typed cause `confidence-beyond-precision` and terminal value `Unsupported(cause)`. FR-192-AC-1 and TC-627 step 1 cover the new row. Checked against terra's engine (origin/qsl-570-impl dd3a654d7): `check_statistical` returns `StatisticalRefusal::ConfidenceBeyondPrecision` for a regenerative request whose `normal_quantile_upper` reports `BeyondPrecision`, before any sample is drawn (statistical.rs:666-681). The settlement map is not implemented yet, so the row is a spec-level mapping. FR-189's refusal list (line 96) still names the refusal, and FR-190 does not mention it. QSpec PR #300 (head 1d1d3e976, open) spells the cause `confidence-beyond-precision`, a payload-free cause, and adds it in four places: FR-408 Admission (line 57), FR-331's closed `unsupported` cause list, the backend-provider-v1 schema's `UnsupportedCause` enum, and FR-290's negotiation sentence. Its vectors hold a positive case (`results/2/cause/type` = `confidence-beyond-precision`) and an adverse one (`confidence-beyond-precisions`, refused). The FR-192 spelling matches. Merge order: the row cites a cause that QSpec main (2aadb7e) does not yet admit. On main, FR-331 would refuse the record with `invalid_result`/`unknown-cause`. The PR body's 'Merge after QSpec #300' is therefore the right order, and #1101 should not merge while #300 is open. Methods: spec-review/base (SR-13570), spec-review/ears (SR-13571), spec-review/integrity (SR-13572). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 17 warnings, none in FR-192 or TC-627; tools/check-index-completeness.sh passes. The diff has no status wording. quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: Consistency with the engine's refusal, the merged #1098 text and QSpec #300's cause.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-192-beyond-precision-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-diagnostic-refusal-row | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-outputs-unsupported-cause | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| FR-192-AC-1 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md | examined |
| TC-627-step-1 | spec/test-cases/TC-627-statistical-results-settle-as-measured.md | examined |
| FR-189-refusals | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| PR-1079-code | agent-ix/quire-spec-language@dd3a654d7 qsl-analyze/src/statistical.rs | context_only |
| QSpec-PR-300 | agent-ix/quire-specification@1d1d3e976 FR-331, FR-408, FR-290, proposals/backend-provider-v1/schema.json, vectors.json | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new row settles the refusal as `Unsupported(cause)` with cause `confidence-beyond-precision`, but the Outputs line that lists the variants `UnsupportedCause` gains still reads 'NotMarkov{…}, TotalAboveDrawRange{total}, EveryScheduler, MissingConfidence, ExactOnlyForm'. As written, the terminal value has no variant to carry the new cause, so an implementer following Outputs cannot build the row. Until this PR the refusal used the diagnostic form and needed no variant. Fix: add `ConfidenceBeyondPrecision` to the 'UnsupportedCause gains' line, written as `confidence-beyond-precision` on the wire. | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:73 |

## Verdict

One medium finding. What holds: the spelling `confidence-beyond-precision` matches QSpec #300's FR-408 Admission, FR-331 list, schema enum and vectors. The engine raises the refusal before any sample, as FR-408 Admission requires. The remaining four refusals keep the merged #1098 diagnostic row unchanged. Merge order: correct as the PR body states, after QSpec #300; #300 is still open, so #1101 is not mergeable yet even once FND-001 is fixed.

## Dispositions

Round 1, head 51e9aaa53: fix commits 02b07326 and 51e9aaa53 over f810277ee. The diff against main 8a5569839 holds only FR-192, TC-627 and this review's three round-0 SR files, which are unchanged; this file supersedes them. The restored reviews/comment-*.md files are byte-equal to main and show no diff. QSpec #300 is merged (44afcf4a9): QSpec main's FR-331 and FR-408 now admit `confidence-beyond-precision`, so the merge-order condition is met. Gates: make check-spec-validation exits 0 with the same 17 warnings, none in FR-192 or TC-627; tools/check-index-completeness.sh passes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
