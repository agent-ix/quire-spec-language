---
id: SR-12883
title: "Integrity review of PR #978: FR-138 and ADR-020 against QSpec FR-375 history-literal"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@ef1b1afa0; FR-138 Inputs, S3 Behavior, fixture paragraph, AC-2, References; ADR-020 grammar sketch line 134 and AX-1; TC-543 step 2; agent-ix/quire-specification@origin/main FR-375 history-row and history-literal productions, History literals paragraph, FR-375-AC-9, TC-310 mutation 10, FR-378-AC-3, FR-149 range narrowing"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---

## Summary

Ticket: STD-170. PR quire-spec-language#978. This review checks every edited
statement against merged QSpec FR-375 (quire-specification origin/main
967ee5a, read only) for contradictions and for production text that the PR
restates when it should reference it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-138's S3 bullet restates part of FR-375's History literals rule (`none` only for `Option<T>`, the three-segment enum path) and references the rest. It leaves out `true`/`false` conforming only to `Boolean` and a member of another enum refusing `ill_typed`/`type-mismatch`. A partial restatement can drift from QSpec without anyone noticing. Either reference the rule alone or state all of it. | spec/functional/FR-138-check-and-compute-history-fields.md:72-78 |
| FND-002 | low | FR-138 Inputs still restates the history-row shape, now written with `<bounded type>` where FR-375's production has `type-ref`. In FR-375, boundedness is an S3 rule, not grammar. The shape is labelled as FR-375's production but does not match it. A plain reference, or `type-ref`, would remove the mismatch. ADR-020:134 is a sketch and is acceptable. | spec/functional/FR-138-check-and-compute-history-fields.md:40-43; spec/decisions/ADR-020-refinement-mappings-between-qsl-models.md:134 |

## Verdict

Consistent with QSpec. No contradiction. Both findings are lows.

Checked and clean:

- Literal set: FR-138 and AX-1 defer the spellings (signed-int, `true`,
  `false`, string, `exact-number`, full-path enum member, `none` for
  `Option` only) to FR-375 and contradict none of them.
- Value-set rule: "member of the declared type's value set, range included;
  a non-member refuses `ill_typed`/`type-mismatch` at the literal; not
  deferred to run time" matches FR-375's paragraph and FR-375-AC-9. FR-138-AC-2's
  `true` and `5` for `Int[0, 1]` correspond to TC-310 mutation 10's `true` and
  `5` for `Int[0, 4]`.
- Collection, record and float spellings (`invalid_syntax`/`unexpected-token`)
  and enum-path refusals (`missing_declaration`/`missing-name`; another enum's
  member `ill_typed`/`type-mismatch`) are left to FR-375 by reference. FR-138
  restates none of them wrongly.
- Fixture paragraph: QSL's static native-type conformance for update
  expressions, with out-of-range values reported at run time as
  `MappingUndetermined`, matches QSpec FR-378-AC-3 (`inconclusive`,
  `mapping-undetermined`) and FR-375's split between literals and FR-149's
  computed-expression range narrowing.
- ADR-020 AX-1 states the rule by reference, and no `<literal>` remains in
  `spec/`.
