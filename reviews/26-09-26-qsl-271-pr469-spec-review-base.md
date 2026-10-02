---
id: SR-676
title: "QSL-271 PR 469 base checklist review of FR-100's refusal-record mapping"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ceb5d905; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; checked against spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md, spec/functional/FR-109-run-a-state-clause-through-the-spine.md, spec/tests.md, spec/spec.md, quire-exact/src/outcome.rs, qsl-foundation/src/diagnostic.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-eval/src/value/expression/causes.rs, qsl-semantics/src/model/refusal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-452
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#469, spec only, base main
9425dd82). Base checklist over the two changed files: ID formats, AC
testability, coverage, and TC-452 step 4's oracles.

Clean: IDs and relationships are well formed (`quire validate` over both files
exits 0 with no warnings). The FR-096 `depends_on` edge is correct. AC-9 and
TC-452 step 4 cover all thirteen `quire-exact` `Refusal` variants
(`quire-exact/src/outcome.rs:109-162`), matching the three kernel rows of
FR-100:130-135 (1 + 10 + 1 + 1). `CardinalityOutOfBound` is exercised on both
sides of the bound. The kernel undefined reasons, `Incomplete`, and both family
undefined reasons are unchanged and still covered. `spec/tests.md:235` needs no
change.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-452 step 4's record oracles check key names only, never values. `CardinalityOutOfBound` expects "`fields` `collection`, `bound` and `count`", and `absent-key` expects "`fields` `binding` and `key`". Step 4's inputs (lines 37-42) give no collection kind, bound or count, and no binding or key, so there is no expected value to compare. FR-100:105-106 says each key is "valued by its rendering" but not the JSON type: the record's fields are `String`s (`qsl-foundation/src/diagnostic.rs:969-975`), so `count` could be emitted as `"5"` or `5`. Failure scenario: a renderer that writes `"bound": "[0,3]"` instead of FR-096's `[0, 3]`, or writes numbers instead of strings, passes TC-452. Fix: fix concrete inputs (for example a `Set` with bound `[1, 3]` and count 0 or 4), give the exact expected `fields` object, and state that each field is a JSON string. | spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:37-42, 62-64, 70-72; spec/functional/FR-100-run-a-named-function-through-the-spine.md:102-106 |
| FND-002 | low | AC-9 names family refusals only "with and without an FR-096 key-table row", and TC-452 uses one of each (`absent-key`, `type-mismatch`). No record-bearing family cause with a different field set (`foreign-universe`, `wrong-anchor`) is rendered, and no record-less family cause whose code has a non-20 exit (see SR-679 FND-001) is constructed. Failure scenario: a renderer that special-cases `absent-key`'s keys passes. Fix: add `ModelQueryRefusal` `foreign-universe` and a record-less `AncestorSteps` (`resource_exhausted`) to step 4, with their expected exits. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:256; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md:40-42 |

## Verdict

Changes requested: FND-001 leaves AC-9's record rendering without a concrete
oracle.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@bec5791c` (fix commits `e71e60cc` and `bec5791c`, on main 9425dd82). I checked each outcome against the spec at that head and the code on main 9425dd82, not against the commit message. `quire validate` over FR-100, FR-109, TC-452, TC-468 and these reviews exits 0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed e71e60cc | TC-452 step 4 now gives literal inputs and exact `fields` objects, and FR-100's record row says each field is a JSON string. Checked against main: `kernel_refusal_record` gives `{"collection": "set", "bound": "[1, 3]", "count": "4"}` (`"0"` for below-minimum). `AbsentKey` gives `{"binding": "people", "key": "p7"}` (`render_identity`). `ForeignUniverse` gives `required` = hex(expected) = `"02"`x32 and `supplied` = hex(actual) = `"01"`x32. |
| FND-002 | fixed e71e60cc | Step 4 adds `ForeignUniverse` (record) and `AncestorSteps` (no record, `resource_exhausted`/`ancestor-steps`, exit 22 through `Code::from_code`/`Code::exit_code`), and `TypeMismatch` keeps exit 20 (`ill_typed`). |

New finding (low, non-blocking): TC-452 step 4's `locus` oracle is "a `locus` whose `span` is the region's". No literal `source_digest`, byte, line or column is given, so a renderer that computes line or column wrongly passes. Give the fixture's region literally.

Fixed in 720308d3: TC-452 step 4 compiles a literal 229-byte fixture and expects `locus` exactly `{"source_digest": "sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26", "span": {"start": {"byte": 225, "line": 3, "column": 54}, "end": {"byte": 226, "line": 3, "column": 55}}}`.
