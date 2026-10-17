---
id: SR-12080
title: "Gap analysis of quire-spec-language PR #936: FR-362-AC-2 and FR-362-AC-7 grammar halves"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@580c698af87f6ccd3e49aa4136ac56616fc36116; PR #936: FR-362-AC-2, FR-362-AC-7 (TC-1026 steps 2 and 5), ADR-033 against qsl-forms/tests/it/source_grammar_vectors.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #936

## Summary

Ticket: QSL-783, slice S1. Plan completion: not assessed (the slice plan was
read as context).

- FR-362-AC-2 / TC-1026 step 2: the grammar and `build_unit` clauses are
  covered (recovered `ParsedSource`, `invalid_syntax` at the first extra
  token, `build_unit` refuses `invalid_syntax`). The vectors cover `version`
  alone, `digest` alone, and both, which goes beyond TC-1026's single
  version+digest case. Not covered: "the producer refuses it" and "no other
  parser admits it" (FND-001).
- FR-362-AC-7 / TC-1026 step 5: the grammar half is covered. `relation` and
  `hyper` are each refused `unsupported_construct` at the clause span, and the
  baseline without them builds. "No graph and no type report" and "yields
  both" are producer clauses, planned for S2a ("Also AC-7 end to end through
  the producer").
- FR-362 item 7 (profile by identity alone) and item 8 (unmoved families
  refused at the construct) agree with the tested behaviour. ADR-033 names
  `relation`/`hyper` as the unmoved example after #935.
- No production code changed, so no unowned code was added.

## Verdict

One medium finding. It is a ticket-level ownership gap, and it is not a defect
in this PR's tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-362-AC-2 has two clauses outside S1 with no slice that owns them. The S2a plan lists only TC-1026 step 1 / AC-1 and AC-7 end to end, so "the producer refuses it with `invalid_syntax`" has no planned test. "No other parser admits it" is false today: the composed `parse_native` admits `profile X = ".." version ".." digest ".."`, which is its own spelling. That clause can only hold after S3 deletes the composed parser, and no slice or TC-1027 step tests it. Combined with SR-12079 FND-001, the matrix already shows AC-2 `tagged`, so this gap will not appear in coverage. Add the AC-2 producer vector to S2a's test list. Record the "no other parser" clause against the S3 by-absence check (QSL-830). | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:91; qsl-forms/tests/it/source_grammar_vectors.rs:46-82 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | routed to planner/ticket, no change in this PR | The AC-2 producer clause has no slice and "no other parser admits it" holds only after the composed parser is deleted; this is a ticket-level item. |
| SR-12079 FND-001 | fixed | The AC ids are removed from the doc comments in source_grammar_vectors.rs; the tests keep the bare TC-1026 / FR-362 tags. |
