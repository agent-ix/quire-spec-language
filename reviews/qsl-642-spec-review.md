---
id: SR-1363
title: "Spec review of quire-spec-language PR #655: i128 integer bounds and literals end to end (QSL-642, re-applied spec)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@90bc007a7d5a66e7ca1122e8cf431c4970625d15; PR #655 diff against origin/main: spec/functional/FR-033, FR-056, FR-082, FR-091, FR-092, FR-098, spec/test-cases/TC-909 to TC-913, spec/tests.md, spec/model-linking/tests.md, spec/native-lowering/tests.md, spec/reviews/qsl-642-spec-review.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Spec review of quire-spec-language PR #655

## Summary

Ticket: QSL-642. Base spec review of the re-applied QSL-642 spec (first reviewed as SR-1359 on #654, reverted by #656), checked against the code this PR lands with it. The integrity and EARS lenses are folded in. Rulings are taken as decided.

- FR-091 "Integer bounds and literals up to i128". The directly-negated definition (only layout between `-` and the literal), the single fold of 2^127, the refusal of every other out-of-range literal, and the `IntegerOutsideI128` fields (site, value, limit, span) all match the code (qsl-forms fold, `type_form.rs`, `typing.rs`). The catalog row `ill_typed`/`type-mismatch` matches `CheckCause::code`/`cause`.
- FR-092 counters and T13 to T16, L7. The counter rule matches `Counter`'s serializer, and the vectors are asserted byte for byte by the tests. The statement that no existing key changes holds: T4 and the other vectors still pass.
- FR-056 and FR-082. The decimal-string bound, the canonical-form rule and the refusal code match `bound_operand`. FR-082's exact-integer obligation matches the i128 widening.
- FR-033's two new statements describe the post-IR-662 wire. AC-6 and TC-912 are marked planned on IR-662, and FR-091's Remaining work says the IR wire still carries i64. That is consistent with the ruling.
- FR-098-AC-11 matches TC-913 and its test, including `meter_over`.

Examined:
- FR-091 "Integer bounds and literals up to i128", AC-36 to AC-38, catalog row (examined)
- FR-092 counter paragraph, T13 to T16, L7, AC-14, AC-15 (examined)
- FR-056 scalar-bound paragraph, AC-16 (examined)
- FR-082 field-domain paragraph, AC-9 (examined)
- FR-033 new statements, AC-6 (examined)
- FR-098-AC-11 (examined)
- TC-909 to TC-913 (examined)
- spec/tests.md, spec/model-linking/tests.md, spec/native-lowering/tests.md rows (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-056-AC-16 row in model-linking/tests.md reads "🚧 Planned", but spec/tests.md marks TC-911 "✅ Passed locally" and the AC's tagged test lands in this PR. The two status tables disagree. Set the row to the state TC-911 actually has (passed locally, or partial if gap-analysis SR-1362 FND-001 is still open). | spec/model-linking/tests.md:159 |
| FND-002 | low | TC-910's Description says the ranges key to "T13 to T15", but its procedure and expected results, FR-092-AC-14 and the test all include T16 (`Int[0, 18446744073709551616]`). Say "T13 to T16". | spec/test-cases/TC-910-wide-integer-ranges-key-to-their-vectors.md:13-14 |

## Verdict

The re-applied spec is consistent with the code and the rulings. There are two low bookkeeping inconsistencies in the test-case text and status tables. Mergeable once they are fixed in this PR.

## Dispositions

Round 1, reviewed at fe7a43674bff9296046a1222336d6a8606839e5b.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 73f15d363 |
| FND-002 | fixed | 73f15d363 |
