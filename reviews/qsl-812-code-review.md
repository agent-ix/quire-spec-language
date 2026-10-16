---
id: SR-11711
title: "Code review of quire-spec-language PR #902: typed record-field comparison in the frame check"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a3c4b6c28ce233112b7a9d53923a33abfff31ed3; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/frame.rs, qsl-semantics/tests/it/native_snapshot_values.rs, qsl-semantics/tests/support/native_values.rs, spec/**, reviews/**"
review_set: subset
---
# Code review of quire-spec-language PR #902

## Summary

Ticket: QSL-812. One pass of code-review with the rust-review lane over
`git diff origin/main...HEAD` (one commit, a3c4b6c28). Read-only: no build or
test run. Only major issues block; mediums and lows are recorded in the
Verdict.

Examined:
- `SnapshotValue::equals_by` and `PartialEq for SnapshotValue` (observation.rs)
- `record_keys`, `record_field_type`, `References::record_type` (document.rs)
- the record admission arm of `admit_value` and `admit_variant` (document.rs, context)
- `run` and `typed_values_equal` (frame.rs)
- the new test `the_frame_compares_a_record_fields_text_by_its_retained_sequence`,
  the changed `a_text_object_counts_as_one_value`, and the fixture change
- every `spec/**` and `reviews/**` path the commit touches
- FR-106 check 11 equality text, FR-106 value-count text, FR-106-AC-37,
  FR-106-AC-40, FR-056-AC-21

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The commit reverts PR #899 and PR #896 on main. It deletes FR-365, TC-1046, TC-1047, TC-1048, `reviews/qsl-790-pr899-spec-review.md` and `reviews/qsl-804-spec-review.md`, removes the FR-365 rows from `spec/spec.md` and FR-104, and rolls back FR-105, FR-107, FR-109, FR-120, FR-364, TC-1039, TC-468 and TC-472. `git diff b542238ff~1 HEAD -- spec reviews` is empty: the branch's spec and reviews trees equal main before #899. The ticket is a code fix and needs none of this. Fix: restore every `spec/**` and `reviews/**` path to `origin/main` so the PR touches only the five `qsl-semantics` files. | spec/spec.md:735; spec/functional/FR-104-check-state-clauses.md:181; spec/functional/FR-365-reach-an-object-over-a-relationship-end-in-a-state-clause.md |
| FND-002 | high | `a_text_object_counts_as_one_value` no longer tests what it is traced to. FR-106-AC-37 needs the Text element inside `aliases` to be the last value in walk order, and the refusal at one less to be the count that Text would have reached. The fixture now declares `Item.address` after `aliases`, so values 16 and 17 are the `Address` record and its `street`. At a bound of 16 the refusal is at `address.street`, not the `aliases` Text. The author deleted the "last value in walk order" wording instead of keeping the AC true. The count 17 itself is right (FR-106 counts the record and each contained value). Fix: keep `aliases` last in walk order for this test, for example by declaring `address` before `aliases` in the fixture, and adjust the 17/16 bounds to match. | qsl-semantics/tests/it/native_snapshot_values.rs:1942-1968; qsl-semantics/tests/support/native_values.rs:127 |
| FND-003 | high | Tag overclaim on the new test. `#[trace("FR-056-AC-21", "FR-106-AC-37")]`: FR-056-AC-21 is about record admission (one `record` export per record, effective fields, subtype conformance, `Option<Address>`, `Sequence<Address>[0, 3]`, operation parameter types), and the test asserts none of it. FR-106-AC-37 covers `title`/`subtitle`/`aliases` and says nothing about records. The behaviour tested is FR-106 check 11 ("a record the same most-specific type and equal effective fields" under each field's declared equality), which has no AC row. Fix: drop `FR-056-AC-21`, and either tag a real AC that states record-field comparison or add the AC row to FR-106 (an FR-106-AC-40 extension over TC-1044 fits) and tag it. | qsl-semantics/tests/it/native_snapshot_values.rs:1889-1893 |

## Verdict

FAIL, three blocking findings (FND-001, FND-002, FND-003).

Traced and correct:
- `equals_by` pushes each record field with
  `field_type(type_identity, name)` inside the existing work-stack loop, with
  no parallel path. `PartialEq` passes `|_, _| None`, so the untyped
  comparison is unchanged. Integer, Boolean, Reference, String, UUID and
  Timestamp still compare as written. The Text arm still needs
  `ValueType::Text` plus two Known Text natives.
- Nested records work: each nested `Record` form looks up its own fields by
  its own type identity, whatever the parent's declared type was. Option and
  Sequence fields inside a record resolve through the field's declared type
  in the existing `Present` and `Sequence` arms.
- The fallback `None` is safe. Admission resolves a record by the value's own
  identity through `References::record_type` and refuses a missing or
  unknown member, so every admitted record has a key and every field
  resolves. This uses the same map and the same `composite()` shape the
  lookup reads.
- `record_keys` is the lazy-init body moved verbatim, and `record_type`
  behaves exactly as before.
- Variants: `RawNode::Variant` holds only `type_identity`, `case` and
  `extra_member`, and `admit_variant` refuses any extra payload member. No
  payload values exist to compare, so no change is needed.
- The new test fails before the fix: the raw members of U+00E9 and
  U+0065 U+0301 differ, so the frame check gave a violation. Its second half
  asserts a real change (`Main` to `Side`).
- No new `unwrap`, `expect` or panic in non-test code.

Recorded (medium/low, not fixed):
- No test covers a nested record, or an Option or Sequence field inside a
  record, compared by its declared type.
- `frame::run` builds `record_keys` eagerly on every run, even with no record
  values. This is unmetered hashing work.
- A doc comment line in `tests/support/native_values.rs:48` runs past the
  line width.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 0e0a52dee: `git diff origin/main HEAD` (two-dot) and `origin/main...HEAD` (three-dot) both list only the five qsl-semantics files and reviews/qsl-812-code-review.md, reviews/qsl-812-gap-analysis.md. |
| FND-002 | fixed | 0e0a52dee: `Item.address` is declared before `aliases` in the fixture and in `text_fields`, so the aliases Text is value 17 and the refusal at bound 16 is that Text. The "last value in walk order" wording is restored. |
| FND-003 | fixed | 0e0a52dee: the new test carries no trace tag. Its doc names FR-106 check 11, whose record clause has no AC row (recorded, not blocking). |
