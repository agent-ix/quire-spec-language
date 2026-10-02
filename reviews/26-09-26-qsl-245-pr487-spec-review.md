---
id: SR-724
title: "QSL-245 spec review of PR 487 (catalog 1-draft.8 adoption)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@913375e2255351626fdcb6f318b130938d351df9; spec/functional/FR-001-read-exact-source.md; spec/functional/FR-010-report-native-outcomes.md; spec/functional/FR-018-construct-native-runtime-inputs.md; spec/functional/FR-026-run-standalone-native-workflow.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/spec.md; spec/tests.md; spec/test-cases/TC-424, TC-425, TC-428, TC-430, TC-431, TC-500"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---

## Summary

Ticket: QSL-245 (PR agent-ix/quire-spec-language#487; also QSL-160, QSL-148).
The diff is spec-only. It was measured against QSpec commit 84ed5298
(`proposals/quire-v1/definitions/native-diagnostics.md` revision
`1-draft.8`, FR-272, FR-044, FR-140, FR-145 and `value-accounting.md`) and
against QSL code at the reviewed sha.

Matches confirmed:

- The ten FR-096 key-table rows match the catalog's codes, causes and payload
  keys exactly (`expected` for each, plus `flags` for `ieee_not_exact` and
  `actual` for `ieee_nan_payload_not_representable`). The value spellings
  match: `Int[lo, hi]`, `Decimal[lo, hi; smin, smax]`,
  `Rational[lo, hi; dmin, dmax]`, `Text[min, max; profile]`,
  `binary32`/`binary64`, and flag order
  `invalid,divide_by_zero,overflow,underflow,inexact`.
- The blank-label and empty-path precedence in FR-001, FR-010, FR-018,
  FR-026 and TC-424/425/430/431 matches QSpec FR-272-AC-12. Labels are
  checked first, in the order authority, identity, revision_namespace,
  revision. Blank means empty or only Unicode `White_Space`.
- The `sum` rule matches QSpec FR-145 steps 1 to 4, FR-044-AC-10,
  FR-140-AC-5 and the value-accounting collection sum schedule.
- The "not built" status claims match the code:
  `qsl-eval/src/value/expression/evaluate.rs:1488-1502` and `:1575-1585`,
  `qsl-foundation/src/source.rs:249`, and
  `quire-exact/src/outcome.rs:185-219`.
- `CheckMode::Kernel` is the only path that skips `Definedness`
  (`qsl-semantics/src/check/mod.rs:1247`).
- The new ids collide with nothing on origin/main or in open PRs #485 and
  #486. FR-100 and FR-106 are not edited.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The status says AC-8 is backed for `ForeignReference`, but the rewritten AC-8 also requires `Refusal::cause()` to return the table's cause. `Refusal::cause()` returns `None` for `ForeignReference` (quire-exact/src/outcome.rs:204-219). So AC-8 is not backed for `ForeignReference`. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:440; spec/test-cases/TC-428-a-refusal-record-carries-code-category-locus-and-fields.md:61; spec/tests.md:214; spec/spec.md:510 |
| FND-002 | low | FR-001-OQ-1 concludes that the three edit host causes "need a catalog code before QSL claims revision 1-draft.8". They already have one: the retained host code `invalid_source_map`, whose broad meaning is "Correspondence, source binding or queried range is invalid". The catalog leaves that code's causes open, and all three are raised at `Phase::SourceMap`, like `EditRanges`. The OQ can be settled in QSL. | spec/functional/FR-001-read-exact-source.md:202-210 |

## Verdict

Approve with changes. Fix FND-001 in this PR by stating that AC-8 is backed
for `CardinalityOutOfBound` only, and that `ForeignReference`'s record is
built but its `cause()` is planned. FND-002 is a recommendation for the owner
to rule on.

## Dispositions

Verified against `git diff 913375e2..e0d028af` on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e0d028af: FR-096 Status, TC-428, `tests.md` and `spec.md` now say AC-8 is backed for `CardinalityOutOfBound` only, and that `ForeignReference`'s `cause()` returns `None`. |
| FND-002 | fixed | e0d028af: FR-001-OQ-1 is replaced by the ruling. The three host causes now refuse with `invalid_source_map`, keep their host cause and name no region. New FR-001-AC-12 is backed by TC-424 step 7 and marked planned. It matches the catalog's retained `invalid_source_map` meaning and the three call sites. |
