---
id: SR-11697
title: "Gap analysis of quire-spec-language PR #893: Text value types at intake and typed frame comparison"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1aa51904f8d83a9d2524d78b233677764da392b1; qsl-semantics/tests/it/native_snapshot_values.rs, qsl-semantics/tests/it/model_operations.rs, qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/frame.rs, spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-1038-snapshots-carry-text-values.md, spec/test-cases/TC-1044-text-value-types-in-the-model-document-and-typed-frame-comparison.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #893

## Summary

Ticket: QSL-806 (and QSL-798). Manual check of acceptance criteria against
tests over `git diff origin/main...HEAD`. No builds were run and `quoin matrix`
was not run. Plan completion: not assessed.

Examined: FR-056-AC-11, AC-37, AC-38 and AC-39, and FR-106-AC-33, AC-36,
AC-37, AC-38 and AC-40. These were checked against the new tests, the TC-1038
steps 1, 3 and 7 to 10, TC-1044 steps 1 to 4, and TC-897 step 3. The QSL-806
outcome text was read as data: (b) "frame comparison is typed", tests from
TC-897 and TC-1038 steps 1, 3 and 7 to 10.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-106-AC-40 has no test, though this PR implements it and its code cites it. `equals_by` and `typed_values_equal` name FR-106-AC-40 in their docs, and QSL-806's outcome (b) is typed frame comparison. TC-1044 step 4 is AC-40's verification. Its `aliases` rows are not tested: reordered is a change, whitespace-only is not. Nor are the absent-to-present `subtitle` row, the String `label` row (U+00E9 to U+0065 U+0301 is a change) or the UUID `token` row. Only the rows TC-1038 step 7 shares are covered. The new `Sequence` element-type branch of `equals_by` (`ValueType::Collection` gives `element()`) has no test, so a regression there would give a wrong frame verdict and nothing would catch it. These rows use the frame harness this PR already extends, and they belong to this ticket, not to later work. Fix: add the TC-1044 step 4 rows tagged `TC-1044`/`FR-106-AC-40`. | qsl-semantics/src/model/observation.rs:925-1000, qsl-semantics/src/model/observation/frame.rs:559-574, qsl-semantics/tests/it/native_snapshot_values.rs:1835-1880 |

## Verdict

FAIL: 1 finding (FND-001). This is in-scope coverage that the ticket's outcome
(b) requires. The lead decides whether it blocks.

Clean (each test asserts what its row states):
- FR-056-AC-37 / TC-1044 step 1: the six profiles, any order, bounds `0,0` and
  `3,3`, read at the reader. The field types `Text[1,8;nfc]`,
  `Text[0,4;binary-utf8]` and `Text[0,4;unicode-scalars]` and a String field
  are asserted on the checked graph. The parameter and result are covered by
  the `retitle` invocation test.
- FR-056-AC-38 / TC-1044 step 2: rows (b) to (k) for code and naming, and the
  native-String `textProfile` row for cause, `what` and member. Row location
  is a code-review finding (SR-11696 FND-001). Rows (a), (l), (m) and (n) are
  correctly left to the validator, confirmed against the pinned validator.
- FR-056-AC-11 / TC-897 step 3: a bare `ix://quire/native/Text` parameter
  refuses `declaration-form` at the parameter, naming `profile`.
- FR-106-AC-33: TC-1038 step 1 (value `Ready`, reordered members and
  whitespace give the same digest and admit) and step 3 (both selections,
  `PreCall` reads no result).
- FR-106-AC-36: TC-1038 step 8 (Witness exempts none of the three rows).
- FR-106-AC-37: TC-1038 step 7 (grant and no grant, T1 to T2, reordered,
  `Ready` are no change under `nfc`; `subtitle` under `unicode-scalars`
  is a change) and step 9 (exact count 15 admits, 14 refuses).
- FR-106-AC-38: TC-1038 step 10 (three escape/byte documents and the
  wrong-token variant refuse `byte-digest-mismatch`; the oversize document
  refuses `input-bytes-exceeded` naming `observation.input_bytes`).

Scope judgment on the rows the author did not do:
- TC-1044 step 4 (FR-106-AC-40): in scope. See FND-001.
- TC-1044 step 3 (FR-056-AC-39): the replayed `Input`/`Witness` argument rows
  are FR-098 replay admission, a code path this PR does not touch, and
  QSL-806's outcome does not name them. They can legitimately come later, but
  they need an owning ticket. The `Bin` binary-utf8 row and the snapshot
  surface are already exercised by TC-1038 steps 2 and 6 (QSL-798).
- TC-1044 step 2 rows (a), (l), (m) and (n): validator-only, so nothing for QSL
  to test at the reader.

Recorded (not blocking):
- low: FR-056-AC-37's `Tag` alias row ("`target` String stays `String`") is not
  re-asserted with a constraint-free `Tag` alias in the Text fixture. That
  alias behaviour predates this PR.
