---
id: SR-12079
title: "Code review of quire-spec-language PR #936: source-grammar vectors for FR-362"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@580c698af87f6ccd3e49aa4136ac56616fc36116; PR #936: qsl-forms/tests/it/main.rs, qsl-forms/tests/it/source_grammar_vectors.rs"
review_set: subset
---
# Code review of quire-spec-language PR #936

## Summary

Ticket: QSL-783 (slice S1). Code review with the rust-review lane folded in.
Tests only: one `mod` line and one new test module. No production code changes.

`a_profile_line_names_only_its_identity` has three hand-written cases. Each one
asserts typed fields: `is_admissible()` false, an empty
`selections().profiles`, a first diagnostic with `Code::InvalidSyntax` whose
byte start equals `text.find(first_extra)`, and
`build_unit` returning `FormsCause::RecoveringCst` with catalog code
`InvalidSyntax`. The empty-profiles assertion can fail. `extract_selections`
runs whether or not the parse recovered (qsl-cst/src/parser.rs:180), and it
records a profile only when the token slice matches the schema exactly.

`a_relation_or_hyper_clause_refuses_the_unit_with_no_dispatch_entry` builds a
baseline of 2 forms, asserts that each extended unit is parser-admissible, and
asserts the exact `NoDispatchEntry { spelling }`, `UnsupportedConstruct`, and
`span == Some(span_of(text, clause))`. These match the `build_forms` refusal at
`node.span()` (qsl-forms/src/dispatch.rs:327-333). Both tests would fail if the
grammar admitted `version`/`digest` on a profile line, if S2 built forms from
a recovered CST, or if `relation`/`hyper` gained a dispatch entry.

The module follows the layout of its neighbours (`temporal_clause_forms.rs`):
SPDX header, `//!` module doc, `ix_trace_rs::trace`, real `qsl_cst::parse`
input, typed assertions, no message text.

## Verdict

Fix requested: one medium trace finding. The trace attributes have no AC id,
as designed. The doc comments, though, name `FR-362-AC-2` and `FR-362-AC-7`,
and the computed matrix binds those ACs from them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The `#[trace("TC-1026", "FR-362")]` attributes were left without an AC id so that the grammar half does not claim FR-362-AC-2/AC-7. But `quire matrix` at 580c698af reports `FR-362-AC-2 tagged qsl-forms/tests/it/source_grammar_vectors.rs:52` and `FR-362-AC-7 tagged ...:91`, which it takes from the `FR-362-AC-2` / `FR-362-AC-7` tokens in the doc comments. The other TC-1026 ACs (AC-1, AC-3, AC-4, AC-8) stay `untagged`. The matrix therefore shows both ACs as covered while their producer halves are missing, which is the over-claim the bare tag was meant to prevent. Fix: drop the AC ids from the two doc comments (for example "grammar half of TC-1026 step 2"), so the matrix shows AC-2/AC-7 untagged until S2a tags the producer tests. | qsl-forms/tests/it/source_grammar_vectors.rs:46; qsl-forms/tests/it/source_grammar_vectors.rs:85 |
