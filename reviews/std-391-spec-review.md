---
id: SR-16212
title: "spec-review of the QSL accounting setting names (quire-spec-language PR 1339)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@55f6e010; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md; spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md; spec/test-cases/TC-921-accounting-budgets-are-settable-by-counter-name.md; spec/test-cases/TC-1166-every-exact-product-build-result-settles.md; spec/test-cases/TC-770-library-outcomes-serialize-to-one-outcome-document-with-the-undefined-label-on-non-proof-outcomes-only.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: references
---

# Spec review of the QSL accounting setting names (quire-spec-language PR 1339)

## Summary

Ticket: STD-391 (spec gap for QSL-911), the QSL side of QSpec #390 (STD-323).
One spec-review pass over `git diff origin/main...55f6e010`. Oracles: QSpec
#390 at 15f862d6 (FR-461 Behavior 6, FR-461-AC-5 and -AC-8, the provider
vectors), QSpec `value-accounting.md`, and QSL main FR-098, FR-100 and FR-255.

Gates: `make check-spec-validation`, run with `QSPEC_DIR` at QSpec origin/main
merged with #389 and #390, exits 0. It reports no warning on any file this PR
touches. QSL conformance `cargo test -p qsl-package --lib checked_v2` at this
head, with the same `QSPEC_DIR`, passes 46 with 0 failed. origin/main is an
ancestor of the head.

Examined, clean:

- Consistent with QSpec #390. The ten `ScalarLimits` counters take
  `accounting.<counter>`, `admission.*` and `model.*` keep their stage
  names, and a bare counter is an unknown setting. FR-255's accounting
  table, Behavior 9, FR-255-AC-7 to AC-11, FR-203-AC-7, TC-1166 step 2,
  FR-286-AC-5, TC-770 and TC-921 all use `accounting.work_units`. The
  `limit` stays the counter, as in QSpec's positive vector.
- Name uniqueness. No setting-table or engine-table row starts with
  `accounting.`, and FR-255-AC-12's pairwise-distinct check still holds.
- No new ids. No follow-up, deferral or narrowing is left.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [major] Two requirement statements still give the bare counter as the setting. FR-098's replay conversion says "FR-255 names each by its counter name, which is its setting, and a `stage_limits` entry of that name replaces the request's accounting-limits value". Under this PR the setting is `accounting.<counter>` and a bare `stage_limits` name is unknown, so FR-098 now contradicts FR-255 Behavior 10 and FR-255-AC-10. FR-255 Behavior 8 also still reads "Given an operand `<counter>=<value>` whose name is in the accounting table", while the table's names, and AC-8, are `accounting.<counter>`. Fix: make FR-098 say "by its accounting setting `accounting.<counter>`, and a `stage_limits` entry of that name ...", and make Behavior 8's operand `accounting.<counter>=<value>`. | spec/functional/FR-098-execute-a-replay-request.md:262-265; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:336-337 | wrong-requirement |

## Verdict

Not mergeable yet. Medium findings block. The renaming matches QSpec #390,
and the gates and QSL conformance pass. FND-001 leaves two statements that
contradict the new rule, and the fix is two short edits. TC-921's title
("settable by counter name") could also say "by accounting setting name".
That is optional. Merge it after QSpec #390.
