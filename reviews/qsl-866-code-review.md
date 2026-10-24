---
id: SR-12151
title: "Code review of quire-spec-language PR #988: population, object, history and fairness refinement rows"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0109679485fbbaf5f9c62569fb195755008dff3a; PR #988 diff against merge-base 8711b7802: qsl-cst/src/{cst,grammar}.rs; qsl-cst/tests/it/{complete_grammar,protocol_memory_refinement}.rs; qsl-forms/src/value.rs; qsl-forms/tests/it/source_grammar_vectors.rs"
review_set: subset
---
# Code review of quire-spec-language PR #988

## Summary

Ticket: QSL-866. One code-review pass with the rust-review lane folded in,
over `git diff origin/main...HEAD` (6 files, +300/-2). Read-only: nothing was
built or run, at the dispatcher's instruction. The coder reports `cargo
check`, `clippy -D warnings`, `fmt --check` and the qsl-cst, qsl-forms and
qsl-semantics tests green; those claims were not re-run here.

Checked and clean:

- Grammar matches QSpec FR-375's EBNF production for production:
  `refinement-row`, `population-row`, `object-row`, `field-row`,
  `history-row` (with its `on` updates), `history-literal` and
  `fairness-row`. `TypeReference` stands for `type-ref`; `SignedInteger`,
  `ExactNumber` and `QualifiedName` are the existing shared productions.
- No new keyword. The leading words `population`, `object`, `history`,
  `assume` and `ensure` are `literal(...)` terminals, which
  `complete_reserved_words` does not collect, so they stay identifiers.
  Every `x(...)` word the PR uses (`from`, `on`, `fair`, `weak`, `each`,
  `whole`, `none`, `true`, `false`) was already an `x` terminal on main, so
  the reserved set does not change. The `terminal` contextual literal from
  #961 is untouched, and `#![recursion_limit = "256"]` in `qsl-cst/src/lib.rs`
  is unchanged; the seven new `productions!` entries compile under it per the
  coder's gate.
- `HistoryLiteral` is an ordered choice with `true`/`false`/`none` before
  `QualifiedName` and `ExactNumber` before it too, so `rational(...)` and
  `decimal(...)` are not read as a bare name. `float64` is reserved, so a
  float spelling refuses at its first token.
- The refinement body uses `star` with `commit_on_progress: true`, so a row
  that matches its leading word and then fails refuses at the offending
  token instead of backtracking to a different row.
- `qsl-forms/src/value.rs`: the seven productions join both exhaustive
  `match` arms that return `Err(unexpected(child))`. No production is built
  into a value silently; `refinement` still has no S2 dispatch entry.
- Test oracles: `refused_at` asserts `CompleteCode::InvalidSyntax` and the
  exact offending token; the parse tests compare each production's exact
  source text, so a missing or merged node fails. The forms test asserts
  `NoDispatchEntry { spelling: "refinement" }`, its catalog code
  `unsupported_construct` and the declaration's exact span.
- Trace tags: none of the new tests carries `#[trace]`, and each qsl-cst test
  says "Verifies the parse only and claims no criterion." FR-375's ACs need
  S3 admission, typing or node emission; a parse-only test cannot fail on
  any of them, so tagging none is honest.
- No panic surface, unsafe, integer conversion or new dependency in the
  production code.

## Verdict

PASS. The grammar is a faithful transcription of QSpec FR-375 and nothing
compiles silently. Three low findings, recorded, not blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `a_history_row_refuses_a_non_literal_initial_value_at_its_first_token` pins `set[1]` refused at `[` and `1 + 2` at `+`, the second token of each. The grammar is right (`set` is a bare-ident qualified name and `1` a signed int, both valid history-literal prefixes), but the test name and doc say "first token". FR-375's "refuses at its first token" is only literally true for spellings whose first token cannot start a history-literal. Rename the test to "at the first token that cannot continue a literal", or split the two cases out. | qsl-cst/tests/it/protocol_memory_refinement.rs:434-440 |
| FND-002 | low | The PR's claim "no new keyword" has no test. `contextual_complete_words_remain_eligible_identifiers` lists the existing contextual words; `population`, `object`, `history`, `assume` and `ensure` are not added, so a later change of one of them from `literal` to `x` would go unnoticed. Add the five words to that list. | qsl-cst/tests/it/complete_grammar.rs:543-555 |
| FND-003 | low | The module doc of `protocol_memory_refinement.rs` still describes only the refinement's `step` rows, and the non-literal refusal test's doc names a record spelling but no case writes one (and no case covers a malformed `on` update). | qsl-cst/tests/it/protocol_memory_refinement.rs:2-7; qsl-cst/tests/it/protocol_memory_refinement.rs:429-443 |
