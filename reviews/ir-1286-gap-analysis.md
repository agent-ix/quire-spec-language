---
id: SR-2177002
title: "Gap analysis - IR-1286 text literal provenance, PR 2177"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-091-AC-69; FR-091-AC-41; FR-093-AC-50; QSpec TC-186 T06b; QSpec FR-141-AC-4; qsl-eval/tests/it/native_literals.rs; qsl-forms/tests/it"
review_set: subset
---

## Summary

Ticket: IR-1286. PR 2177 head 78b1556bb. Plan completion: not assessed (no plan supplied).
`quire matrix --scope .` lists FR-091-AC-69 as tagged by four tests including the new `text_literals_keep_their_source_spelling_as_provenance` (qsl-eval native_literals.rs:216). The new eval test asserts bytes `c3 a9` and distinct `SourceLiteral` spellings for the raw and escaped literal, and was shown failing on the base. The updated forms tests (AC-41, AC-69) assert the spelling for ten source forms. No stubs or inflated tests found; the new code has an owning requirement (FR-091 String paragraph, QSpec FR-141 spelling-as-provenance).
Gaps: the paragraph's claim that the spelling takes no part in node identity or lowering has no test aimed at it; QSpec TC-186 T06b says the values "compare equal" and the test compares payload bytes only; the history-row Text literal path has no provenance test.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | QSpec TC-186 T06b requires the raw, escaped and runtime-bytes values to compare equal while keeping distinct provenance; the new test compares payload bytes and provenance but never evaluates `Text("é") = Text("é")` (or Value equality of the two), so the equal half of T06b is tagged QSpec-TC-186 without an assertion | qsl-eval/tests/it/native_literals.rs:216-229 |
| FND-002 | medium | FR-091 now says the spelling takes no part in node identity or lowering, but no test or AC fails if it does: FR-093-AC-50 shares a node only for `é` vs `é`, never for raw vs escaped spelling of one text, and no test builds two units differing only in spelling and compares node identity | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:895-898 |
| FND-003 | medium | The history-row Text literal path (HistoryLiteralForm::Text now carries a spelling, history.rs passes it to provenance) has no test reading its provenance and FR-310-AC-4 does not state it | qsl-semantics/src/check/refinement/history.rs:237-240 |
| FND-004 | low | The new test covers one pair under one profile; it has no case for equal spellings giving equal provenance, an escape other than `é` (`\/`, `\"`, surrogate pair), or a profile with normalization | qsl-eval/tests/it/native_literals.rs:216-229 |
