---
id: SR-2299003
title: "gap-analysis of FR-035-AC-7 reserved declaration names (quire-spec-language PR 2299)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-cst/src/parser.rs; qsl-cst/src/token.rs; qsl-cst/tests/it/reserved_declaration_names.rs; qsl-semantics/tests/it/reserved_declaration_names.rs; tests/it/parser.rs; spec/functional/FR-035-parse-composed-native-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: references
---

# Gap analysis of QSL-1589 FR-035-AC-7 (quire-spec-language PR 2299)

## Summary

Ticket: QSL-1589. Plan completion: not assessed. This run was scoped to FR-035-AC-7 and was
planless. I checked by hand how each AC-7 clause maps to the tests tagged `TC-113` / `FR-035-AC-7`.

- 1-draft refusal for a function and a predicate, with span, cause and message: covered by the
  qsl-cst `a_reserved_spelling_cannot_name_a_function_or_predicate` test and the qsl-semantics
  `..._refuses_at_s1` test.
- Near spellings parse as function names and a call checks: covered by the qsl-cst near test and
  the qsl-semantics `..._declared_and_called` test, which runs S1, S2 and S3.
- 0-draft keeps `unsupported_construct`: covered by the root `tests/it/parser.rs`
  `complete_compound_words_do_not_change_historical_tokenization` test, which compiled and passed.

All five tagged tests pass on the PR head. The production code has an owning requirement
(FR-035-AC-7), and there are no stubs or inflated tags.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No finding (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Every AC-7 clause has a test that can fail. Clean.
