---
id: SR-2448
title: "QSL-460 gap analysis of PR #668 (FR-109, FR-108, FR-115, FR-330, FR-331; TC-468, TC-469, TC-514)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0605565cd860d8873565c349e467b656626cca7b; PR #668 diff against origin/main; quoin matrix (quoin 0.28.3) rows FR-108, FR-109, FR-115, FR-330, FR-331; spec/functional/FR-109-run-a-state-clause-through-the-spine.md; spec/functional/FR-108-run-the-configversion-spine-corpus.md; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md; spec/functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md; spec/test-cases/TC-468, TC-469, TC-514"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: reviews
---
# QSL-460 gap analysis of PR #668

## Summary

Ticket: QSL-460. Plan completion: not assessed. `quoin matrix --json` at
0605565cd binds tests to FR-108-AC-1..6, FR-109-AC-1..6 and FR-115-AC-1..6.
FR-109-AC-7 has no binder (FR-109's Status already says AC-7's record is not
implemented; not this PR). FR-330 and FR-331 have no binders and no code: there
is no `Temporal` selection in `run_clause`, so the ticket's "the temporal
clause run builds the report from the disposition, package_id and usage"
has nothing to act on yet. FR-330's Outputs already state the trimmed report,
so the later implementation inherits it.

Test-oracle strength for the trimmed report, per member:

- **disposition**: every TC-468 step still asserts stage, category, truth,
  record and exit code; nothing here changed. A wrong disposition fails.
- **package_id**: AC-6's extracted run asserts equality with the extracted
  body's own compiled id; AC-2's compile refusals now assert `None`
  (`run_clause_reports_missing_import_when_no_package_bytes_are_supplied`
  gains it); AC-2's stale case asserts both ids; TC-469 step 6 compares every
  corpus case's report with an independently re-derived `package_id`. A wrong
  `package_id` on the healthy-parent path fails TC-469 step 6. Weak spots:
  FND-001 and FND-002.
- **usage**: AC-5 asserts equality of every usage field across two runs, and
  AC-1 now asserts `evaluation_admissions > 0`. No test pins an exact usage
  value; FR-109 states none, so this is as strong as the spec.

The deleted assertions in `pre_call_invocation.rs` (provenance documents
`[invocation, pre]`) do not weaken FR-106-AC-9: its oracle is the verdict
(pre true, post false) plus the absent- and malformed-post cases, which
still fail if the post side were read.

## Verdict

FR-109's tests (TC-468) cover the trimmed report and would fail on a wrong
disposition or a missing/wrong `package_id` on the compared paths. One real
gap: FR-108-AC-5's `package_id` half has no assertion (FND-001); the PR edited
that test and removed its only identity checks. FR-115-AC-1's provenance half
is the spec conflict recorded in SR-2447 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-108-AC-5 requires the Markdown (I3) run of each case to give "the direct run's disposition and `package_id`", and TC-469 step 5 expects the same. `tc_469_step_5_markdown_run_matches_the_direct_run_via_i3` compares exit code, stage and truth only and never the `package_id`. The PR edited this test and deleted its remaining identity checks (source digest, extraction origin), so the AC's `package_id` half has no oracle. Fix: assert `extracted_report.package_id == direct_report.package_id` per case (if it differs because the body's identity enters the package, that is a spec/code conflict to surface, not a reason to skip the assertion). | tests/it/config_version_spine.rs:662-697; spec/functional/FR-108-run-the-configversion-spine-corpus.md:134 |
| FND-002 | low | FR-109-AC-1's unit test asserts only `report.package_id.is_some()`, not that it is the compiled `package_id` the AC names. TC-469 step 6 covers the healthy-parent corpus case, so this is a weak local oracle, not an unchecked AC. Fix: compare with `compose(...).emitted.package_id()` as the AC-6 extracted test does. | qsl-replay/src/spine/clause/tests.rs:753 |
