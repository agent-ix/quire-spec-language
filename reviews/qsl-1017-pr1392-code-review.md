---
id: SR-16320
title: "Code review of quire-spec-language PR #1392: hybrid claims and interval forms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9809b06b8cbeb1a8a02025b9a73ac02c323e1bc9; PR #1392 diff against qsl-1017-impl: qsl-cst/src/{grammar,parser}.rs; qsl-forms/src/{lib,syntax,temporal_clause}.rs; qsl-forms/tests/it/{hybrid_forms,temporal_clause_forms,time_forms}.rs; qsl-replay/src/spine.rs; qsl-semantics/src/check/{assemble,check,hybrid,mod,refusal,temporal_clause}.rs; qsl-semantics/src/check/hybrid/{callee,cause,claim,model,term,tests}.rs; qsl-semantics/src/check/temporal_clause/timed.rs; two spec files"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
---
# Code review of quire-spec-language PR #1392

## Summary

Ticket: QSL-1017, second PR in the hybrid stack. Reviewed the PR's own diff
against `qsl-1017-impl`, its callers and tests, FR-447 AC-3, AC-7, AC-9,
AC-10, AC-11 and the S3 portion of AC-15. The Rust review lane is folded into
this review. The changed spec files passed targeted `quire validate`. A
focused Cargo test was attempted through `locked-build.sh` after confirming
249 GiB free on `/`, but all build lanes were busy; it was canceled before
compilation. The full gate belongs to the code lead.

The term and formula lowering paths use explicit work stacks, checked name
lookups and typed refusals. I found no new vendoring or narrowed CI workflow.

## Verdict

**CONDITIONAL**: the hyper-expression refusal path contradicts FR-447's S3
contract; the other findings affect diagnostic location and wording. Review
the first finding against the existing FR-171 S2 rule before changing that
boundary.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-447 requires a hybrid claim whose `holds` reads a hyper expression to refuse at S3 as `unsupported_construct`/`expression-form`. The shared S2 temporal builder refuses every such `holds` as `UnrepresentedConstruct` before the new hybrid claim checker runs, so the stage and tag differ. FR-171 also requires this S2 behavior for ordinary temporal clauses; route hybrid clauses deliberately or resolve that spec conflict. | qsl-forms/src/temporal_clause.rs:483-484; qsl-semantics/src/check/hybrid/claim.rs:596-608 |
| FND-002 | low | FR-447 places an unsupported clock refusal at the clock clause span. `ClockForm` stores only the quoted string span, which `clock_of` uses; `clock "wall"` is therefore located on `"wall"`, as the test confirms, rather than the whole clause. | qsl-semantics/src/check/hybrid/claim.rs:182; qsl-forms/src/temporal_clause.rs:74-81 |
| FND-003 | low | The replay diagnostic labels every `CheckCause::Hybrid` as occurring in a `hybrid` declaration. A malformed `temporal` hybrid claim takes this arm and is reported as the wrong declaration kind. | qsl-replay/src/spine.rs:958-959 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The fix changes every temporal clause's S2 builder to `formula_form(..., true)`, so the existing `an_indexed_read_in_a_temporal_clause_is_unrepresented` test will fail: it still calls `build_unit(...).expect_err(...)`. FR-171-AC-1 does not require that S2 refusal, and FR-325 leaves meaning to S3. Update the obsolete S2 assertion to verify the preserved form and ordinary-clause S3 refusal, with an appropriate trace tag, so the test suite and new stage contract agree. | qsl-forms/src/temporal_clause.rs:99-102; qsl-forms/tests/it/hyper_clause_forms.rs:315-333 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The hybrid S3 checker now returns `HyperExpression` as `unsupported_construct`/`expression-form`, and `hybrid_claim_hyper_expression_refuses_at_s3` checks the code, tag and span. The ordinary-clause regression is FND-004. |
| FND-002 | fixed | S2 records the full clock member span; the hybrid checker uses it, and the clock refusal test expects `clock "wall"`. |
| FND-003 | fixed | Replay now says `in a hybrid model or temporal claim`, with a test for the claim case. |
| FND-004 | still-open | The existing S2 test still expects a FormsRefusal for an ordinary temporal indexed read; the changed `formula_form(..., true)` admits it. The test has not been updated to the new S2/S3 contract. |
| FND-004 | fixed | `an_indexed_read_in_a_temporal_clause_is_preserved_at_s2` now checks that the temporal form holds the indexed read, while the existing ordinary-clause S3 test checks `unsupported_construct`/`expression-form`. The S2 test is traced to FR-325, the owner of temporal form construction. |
