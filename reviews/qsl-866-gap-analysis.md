---
id: SR-12152
title: "Gap analysis of quire-spec-language PR #988: refinement-row grammar against QSpec FR-375 and QSL FR-135 to FR-137, FR-310"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0109679485fbbaf5f9c62569fb195755008dff3a; PR #988: qsl-cst refinement-row grammar and tests, qsl-forms production lists and NoDispatchEntry test; against QSpec FR-375 (incl. AC-9), FR-432, and QSL ADR-020 section 1, FR-135, FR-136, FR-137, FR-310"
review_set: subset
---
# Gap analysis of quire-spec-language PR #988

## Summary

Ticket: QSL-866. Planless. Plan completion: not assessed. Read-only; the
criterion-to-test check was done by hand over the PR's tests and `#[trace]`
tags (the PR adds none). QSpec read at its current main, cited by id.

- QSpec FR-375 Surface syntax: every production is implemented as written
  (see SR-12151). The history-literal closed set (signed integer, `true`,
  `false`, `none`, string, exact number, qualified name) is parsed and each
  spelling is tested; collection, float and expression spellings refuse
  `invalid_syntax`.
- QSpec FR-375-AC-1 to AC-9: all are S3 checks (completeness, conflicts,
  typing, node emission, identity, claim kinds). None is implemented, none
  is tagged. AC-9's syntax clause (collection/record/float refuse
  `invalid_syntax`/`unexpected-token`) is exercised by a parse test but is
  one clause of an AC whose other clauses need S3, so not tagging it is
  correct.
- S2 behaviour: `refinement` has no dispatch entry, so a unit holding any row
  kind refuses `unsupported_construct` (`NoDispatchEntry`) at the
  declaration's span. Tested. Nothing compiles silently.
- Scope call: QSL-866's text asks for parsing "in qsl-cst and qsl-forms, with
  S3 refusals as FR-375 states". The `RefinementForm` (QSL FR-310) and the
  S3 row checks (QSL FR-135 to FR-137) are the stated deliverables of
  QSL-543 ("E13a: Refinement: S2 form, S3 header and rows", In Progress).
  Leaving them out of this PR is acceptable: the refusal is explicit and
  located, there is no silent wrong result, and building them here would
  duplicate QSL-543. Not a blocking gap (FND-002 records the ticket text).
- No new production code without an owning requirement: every new
  production is in QSpec FR-375's EBNF.

## Verdict

PASS. No blocking gap. FND-001 is a spec conflict between QSL's own FR-137
and QSpec FR-375 that needs a ruling before QSL-543 lands its fairness
checks; it does not change this PR's correctness against the grammar
authority ADR-020 section 1 names (QSpec owns the spelling, QS-1).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `fair strong` conflict. QSL FR-137 Behavior ("a row written `fair strong` as `Strong`") and FR-137-AC-4 (TC-542: `ensure fair strong each Spec::Counter::inc`, `assume fair strong Impl::Counter::retryA`) and ADR-020 section 1's sketch (`fair (weak \| strong)`) admit strong fairness rows; QSpec FR-375's `fairness-row` admits only `weak`. The PR follows QSpec, as ADR-020 QS-1 directs, and its test pins `assume fair strong ...` refused at `strong`, so FR-137-AC-4 cannot pass without changing this grammar and test. Needs a ruling: add `strong` to QSpec FR-375, or drop it from QSL FR-137 and the ADR-020 sketch. | qsl-cst/src/grammar.rs:1703-1712; qsl-cst/tests/it/protocol_memory_refinement.rs:453 |
| FND-002 | low | QSL-866's text asks for qsl-forms work and S3 refusals as FR-375 states them; the PR delivers the S1 grammar and the S2 `NoDispatchEntry` refusal only. The rest is owned by QSL-543 (FR-310 form, FR-135 to FR-137 checks). Narrow QSL-866's text, or record on QSL-543 that it now owns the population, object, history and fairness rows' form and checks. | qsl-forms/tests/it/source_grammar_vectors.rs:141-169 |
| FND-003 | low | QSL FR-310's `RefinementRowForm` declares `History.initial: ExpressionForm` and `Internal`/`Visible` rows. QSpec FR-375 makes the initial value a closed `history-literal` (now its own `HistoryLiteral` production), and QSpec FR-432 puts the abstract internal steps in the refinement profile's list, not in `internal`/`visible` rows. QSL-543 inherits a form spec that disagrees with the grammar this PR lands. | spec/functional/FR-310-build-the-s2-form-of-a-refinement-declaration.md |
