---
id: SR-730
title: "QSL-291 spec review of PR 488 (FR-100/FR-106 follow catalog 1-draft.8)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/test-cases/TC-465-admission-refuses-each-input-defect.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: reviews
---

## Summary

Ticket: QSL-291 (PR agent-ix/quire-spec-language#488). The
diff is spec-only. It was measured against QSpec
(`proposals/quire-v1/definitions/native-diagnostics.md` revision
`1-draft.8`, FR-145), against FR-096 and FR-001 as merged on origin/main by
#487, and against `quire-exact/src/outcome.rs`.

Matches confirmed:

- The FR-100 kernel-record table's twelve rows match the catalog's codes,
  causes and payload keys exactly, and match FR-096's key table on
  origin/main. `quire_exact::Refusal` has thirteen variants; the twelve
  tabled plus `CheckedInvariant`, which is the only one without a record.
- Value spellings (domain, `binary32`/`binary64`, flag order) match the
  catalog's payload value rules.
- `Undefined::SumOutOfDomain` / `sum-out-of-domain` is not named by QSpec.
  FR-145 calls it only "a located undefined outcome", and the catalog's
  "Undefined reasons" table lists only `absent-key` and
  `precondition-false`. The name is QSL's, coined in FR-096 on origin/main
  (#487), and FR-100 matches it. It fits the kernel reasons'
  kebab-case-of-variant convention and the `*OutOfDomain` family naming.
- FR-106 check 1.7 matches the catalog's `invalid_source_identity` row and
  FR-001 on origin/main: `blank-label`, field `label`, order `authority`,
  `identity`, `revision_namespace`, `revision`, blank meaning empty or only
  Unicode `White_Space`. AC-3, AC-7, TC-465 rows 8 and 42 agree with it.
- AC-9's new wording holds: thirteen constructed kernel refusals, twelve
  rendering records, five kernel undefined reasons, and the removed
  kernel-no-record row leaves no dangling reference in FR-100 or TC-452.
- The PR merges onto origin/main with no conflicts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new paragraph says a final total outside `N`'s domain is `sum-out-of-domain` "because the final total is the last running total". For an empty collection (`n = 0`) there is no running total: the result is `0`, which QSpec FR-145 step 1 says `N`'s domain "must admit". No QSL spec says what happens when it does not (no static check exists in qsl-semantics). Today the evaluator's final-total check refuses `IntegerOutOfDomain`; once that check is removed per FR-096, `sum<Int[1, 9]>` over an empty sequence would complete with an out-of-domain `0`. State the `n = 0` outcome. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:194-199 |
| FND-002 | medium | Check 1.7 says "The labels are checked first, in that order." Inside numbered check 1, "first" reads as before checks 1.1 to 1.6, which contradicts the numbered order AC-7 tests. The catalog's "first" is relative to `empty-path`. The `empty-path` sentence also has no subject here, since an admitted document has no source path. Reword to name the first blank label in that order, and drop or scope the `empty-path` sentence. | spec/functional/FR-106-admit-snapshots-and-invocations.md:174-181 |
| FND-003 | low | The paragraph locates a failing running total "at the addition". FR-096 on origin/main (and TC-500) locate it at the `sum` node; no addition node exists in the checked tree. Say "the `sum` node". | spec/functional/FR-100-run-a-named-function-through-the-spine.md:195-196 |
| FND-004 | low | The paragraph calls `sum-out-of-domain` a located outcome, but FR-100's `undefined` outcome renders only `reason`, so `spine-run-result/1` drops its location. This is pre-existing for every kernel undefined reason; recorded, not required for this PR. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:104, 194-199 |

## Verdict

Approve with changes. Fix FND-001 and FND-002 in this PR; FND-003 is a
one-word fix. FND-004 is a recommendation for the owner.

## Dispositions

Round 2, reviewed at the fix commit, rebased onto origin/main. The author's
gate log `qsl-291-ci-r2.log` ends `exit=0`.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | FR-100:202-205 makes an empty `sum` whose `N` does not admit `0` `sum-out-of-domain` at the `sum` node (owner ruling); FR-100-AC-10 and TC-452 step 5 test it |
| FND-002 | fixed | FR-106:174-179 scopes the label order "within this check" and drops the `empty-path` sentence |
| FND-003 | fixed | FR-100:194-200 locates a failing seed at the summand's node and a failing addition at the `sum` node, matching FR-096:268-270, FR-096-AC-14 and TC-500 steps 1 and 3 |
| FND-004 | accepted-no-change | Owner ruling: the `undefined` outcome keeps rendering only `reason`, as before this PR |

Round-2 note (new, low, non-blocking): FR-096 on main still says FR-100's
table "does not list it today" (FR-096:284-285) and that FR-100 has no
`sum-out-of-domain` row (FR-096:481-482); this PR makes both stale. FR-096
also states no `n = 0` outcome, which FR-100:202-205 now fixes. The FR-096
owner can correct both in the next FR-096 edit or in QSL-292.
