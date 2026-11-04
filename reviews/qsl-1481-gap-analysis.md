---
id: SR-20261104074839
title: "Gap analysis of quire-spec-language PR #2261 (QSL-1481): FR-153-AC-14 population causes"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@55ffe5444; spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/functional/FR-084-admit-closed-populations-and-resolve-lookup.md, qsl-semantics/src/model, qsl-eval/src/value, qsl-eval/tests/it"
review_set: subset
---
# Gap analysis of quire-spec-language PR #2261

## Summary

Ticket: QSL-1481. This is a planless audit of the criteria whose missing-row cause #2198 changed for QSL-1481, checked against the PR head 55ffe5444. Trace tags were found by grepping `#[trace(...)]` in the Rust tests. QSpec FR-153-AC-14 was read from quire-specification main, by ID only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-106-AC-65's missing-row case is not implemented and no test covers it. #2198 added "with no row for it `incomplete_population`/`missing-required-artifact` at that node before check 8" for QSL-1481, but no Rust test carries the FR-106-AC-65 tag. The code path, `admit_protocol_reference_seeds`, returns `incomplete-scope` for that case (code-review SR-20261104074838 FND-001). Fix: implement the split and add a test tagged FR-106-AC-65 that covers the unlisted seed, the listed-incomplete seed and the complete seed reaching check 8. | qsl-semantics/src/model/observation.rs:495-501; spec/functional/FR-106-admit-snapshots-and-invocations.md:1280 |

## Verdict

**FAIL** on FND-001. Every other criterion in scope is backed:
- FR-106-AC-67: tagged by `a_missing_row_an_open_extent_and_an_uncovered_subtype_each_have_their_own_cause`. It covers no row, a row with `complete: false`, and an unlisted required population after a listed incomplete one, where the listed one is named with `incomplete-scope`.
- FR-391-AC-5: the same test covers the missing-row and open-extent causes at the atom read. The existing named-population tests cover the decided case.
- FR-090-AC-21 and FR-084-AC-2: `tc_227_incomplete_subtype_closure_is_undefined_through_checked_evaluate` and the new test both assert the `incomplete_population`/`unclosed-subtypes` projection.
- QSpec FR-153-AC-14 (TC-198): all three causes are now tagged on the new test.

Reverse gap: none. Every changed production line belongs to FR-106 check 7, FR-391 Evaluation or FR-090. Stubs or coverage inflation: none.

## Coverage

- Criteria examined: FR-106-AC-67, FR-106-AC-65, FR-391-AC-5, FR-090-AC-21, FR-084-AC-2, QSpec FR-153-AC-14.
- Context only: FR-120-AC-7, whose simulation universe with no row is still specified as `incomplete-scope`. The PR leaves that unchanged, as its spec says.
- Matrix source: trace-tag grep at 55ffe5444. `quire matrix` was not run.
- Plan completion: not assessed
