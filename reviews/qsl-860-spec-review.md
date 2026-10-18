---
id: SR-12110
title: "spec-review of quire-spec-language PR 947 (reader-valid protocol check fixtures, non-record attempt types, object-binder reads)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@b824b7a7a1211a7049f59e50b6af6b58d2d5b2ab; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md (object-binder bullet, attempt-type bullet, AC-9, AC-13, AC-14), spec/test-cases/TC-1055-s2-builds-the-capture-relationship-requirement-related-and-compensation-forms.md (row 8), spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md (fixture P, rows 15, 19-22), spec/test-cases/TC-1057-s3-checks-protocol-captures-activation-and-the-names-each-stage-reads.md (fixture R, rows 23-27), spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md (rows 2, 9-11, 14, 15, 15a, 15b)"
review_set: subset
---

## Summary

Ticket: QSL-860. One spec-review pass over `git diff origin/main...HEAD` (5
spec files, +86/-42). Integrity, test-intent and cross-file consistency checks
are folded into this one artifact. QSpec was read read-only and is cited by id
only.

- Catalog pairs. A non-record attempt type refusing
  `invalid_model_binding`/`malformed-declaration` matches QSpec FR-466 (the
  compensation References step requires `record_type` to be a
  `record_value_type` or `event_type`, with the wrong-form refusal of
  "Per-operator checks" step 1) and the FR-272 catalog. An object-binder field
  read refusing `ill_typed`/`operator-ineligible` at the read matches QSpec
  FR-464 "Slots that bar model state" and the FR-381 `over` rule.
- Reader validity. Fixture R's base protocol (record `Receipt`, `Attempt`,
  `Flag`, `Tick`, event `Pulse`, object `Node` read only in captures) and
  fixture P (record `Plain`, channel carrying a record, consistent with
  FR-218-AC-24) are reader-valid. `peer: Reference<Peer>` as a record field is
  specified: FR-056 says a record field whose typeRef names an object type
  keeps its object-reference type. The `related by` ends may read `over` and
  model state (QSpec FR-464). Every recomputed TC-1057/TC-1058 base read
  resolves against the new field sets, apart from FND-001.
- Rows recomputed: TC-1057 rows 23 to 27, TC-1058 rows 2, 14, 15, 15a and 15b,
  TC-1056 row 21. Row 26 ordering: `attempts` comes before `retry` in clause
  order, so builder-state order and source order agree (ADR-012 §4.2).
- No requirement is lost. The old TC-1057/1058/1056 row intents survive. Old
  AC-9's mismatch check survives in a new direction (FND-002).
- Ids FR-381-AC-13 and FR-381-AC-14 are free on main and on every remote branch.
- `quire validate` on the touched files: clean. `make check-spec-validation`:
  EXIT=0. `tools/check-index-completeness.sh`: pass. The diff is in present
  tense and has no history or ticket ids.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-1057 row 23's expected result is wrong. Row 23 changes only the `activate first` header and guard to `(trigger: M::Node) when { trigger.n = 1 }`. The base activation body still holds `capture activated: Boolean = trigger.ready;`, and `Node{total, tally, n}` has no `ready` field. So the row gives a second refusal at `trigger.ready` (an unresolved field), not only the `operator-ineligible` at `trigger.n`. Either the row should replace the body too, or its expected result should list the second refusal. | spec/test-cases/TC-1057-s3-checks-protocol-captures-activation-and-the-names-each-stage-reads.md:48,99 |
| FND-002 | high | FR-381-AC-9 and its only verifying row disagree. AC-9 states `attempts 3 of M::Attempt` with retry binders `M::Flag`, expecting `M::Attempt` and actual `M::Flag`. TC-1058 row 14 tests the reverse: `attempts 3 of M::Flag` with binders `M::Attempt`, expecting `M::Flag` and actual `M::Attempt`. A test written to row 14 does not assert AC-9 as written. The row's direction is the workable one, because the base relation reads `earlier.tally` and `Flag` has no `tally`. So AC-9 should be restated to match row 14. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:260; spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:47 |
| FND-003 | medium | FR-381's model-state rule is narrower than the reader's. The new bullet refuses only "a field read of a binder whose type is an object type". QSpec FR-464 refuses any barred slot whose dependency closure holds a deref, navigate, reaches, reaches_field, lookup, all_instances, dispatch_call or `pre` application. With fixture P's `Plain{peer: Reference<Peer>}`, a retry or recovery binder of a record type with a reference field can be written as, for example, `earlier.peer.id`, or as an explicit `deref(...)`. That read is a field read through a reference, not a field read of an object binder, so S3 admits it and S4 emits a package the reader refuses. This is the defect class the PR fixes. The rule should cover any read that lowers to a model-state operation. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:121-129 |
| FND-004 | medium | The object-binder rule is stated only in FR-381, scoped to its four stages, while TC-1056 cites it for FR-218's slots. TC-1056 fixture P says a field read of an object binder is a model-state read "which a guard, a `visible` entry and a FIFO key bar (FR-381)". FR-381's bullet names only the activation guard, trigger guard, retry relation and recovery predicate. FR-218's choice guard, repeat guard, `check` and FIFO key bar "model state" but never define it. FR-218 also has no object-binder AC, and FR-218-AC-12/17 have no such case. An attempt, effect or event binder of an object type read by a choice guard (`attempted.n`) is therefore unspecified at S3, while the reader refuses it. FR-218 needs the sentence, or a reference to a shared definition, and an AC row in TC-1056. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:109-111; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md:36-41 |

## Verdict

FIX. The catalog pair choices are right against QSpec FR-466 and FR-272, the
object-binder refusal matches the `over` rule and QSpec FR-464, and the base
fixtures R and P are reader-valid. There are two high findings in the TCs:
TC-1057 row 23 misses a refusal (FND-001), and AC-9 is the reverse of row 14
(FND-002). There are two medium gaps. The model-state bar stops at an
object-binder field read, not at a deref through a record's reference field
(FND-003). And the rule lives only in FR-381, while TC-1056 cites it for FR-218's
slots (FND-004).

Notes (not findings): the open SP-D PR #946's TC-1070 fixture E prose says
E "differs from TC-1057's fixture R in that every binder a trigger guard,
retry relation or recovery predicate reads is a record value". That is untrue
after this PR, and #946 should drop the sentence on rebase. FR-381 lists
composite and variant attempt types as refused, but AC-13 and TC-1058 test
only an object type and a scalar type. TC-1055 rows 2, 5 and 6 still use
`M::Node`/`M::Plain` binders, which is harmless because S2 resolves no type.
TC-1065 row 9 (pre-existing) has no rule saying the send and receive binders
are not compared with a refused `carries` type, unlike FR-381's new "SHALL NOT
compare" rule for `attempts`.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | TC-1057 row 23 replaces the activation capture body too, so only the intended refusal arises; rows 24 to 27 recomputed against the base body and unchanged |
| FND-002 | fixed | FR-381-AC-9 restated in the direction of TC-1058 row 14 (attempts `M::Flag`, binders `M::Attempt`) |
| FND-003 | fixed | the FR-381 rule bars every model-state read as FR-218 defines it, citing QSpec FR-464 by id; FR-381-AC-15 and TC-1057 rows 28 to 30 cover a read through a reference and an explicit dereference |
| FND-004 | fixed | FR-218 defines model state once and bars it in choice, repeat, check, visible and FIFO key slots; FR-218-AC-26 and TC-1056 rows 43 and 44 cover it; FR-381 references the definition |
| Note: composite and variant attempt types | fixed | FR-381-AC-13 and TC-1058 row 15c |
| Note: TC-1065 row 9 | fixed | FR-218 states a refused `carries` type is not compared with send, receive or FIFO binders; TC-1065 row 9 and FR-218-AC-26 assert it |
| Note: TC-1070 fixture E prose | not changed | the open SP-D PR owns TC-1070; it drops the sentence on rebase |
