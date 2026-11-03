---
id: SR-2177001
title: "Code review (rust-review lane) - IR-1286 text literal provenance, PR 2177"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-cst/src/token.rs; qsl-forms/src; qsl-semantics/src/check; qsl-eval/tests/it/native_literals.rs"
review_set: subset
---

## Summary

Ticket: IR-1286. PR 2177 head 78b1556bb (merge-base 2b2f4fa3d; reviewed on a merge of origin/main fd5bde253).
Collapse reproduced before the fix and absent after: a probe of 23 spellings (raw and escaped e-acute, upper/lower hex, `\/`, `\"`, `\\`, astral raw and surrogate pair in both hex cases, `\n`, `\t`, NUL-free control escapes, composed vs decomposed, empty) under `Text[0,16; binary-utf8]`. On 2b2f4fa3d, 16 pairs of distinct spellings had equal provenance (Rust debug text, e.g. `"e\u{301}"`); on the PR head no pair mismatches: distinct spellings carry distinct provenance and equal spellings equal provenance, payload bytes unchanged. The PR's own test fails on the base at `escapedText` and passes on the head. Raw control characters in a literal are a lexical error, so multi-line raw strings cannot occur (escapes `\n` are covered).
Consumers: every `ExprNode::String` use in the workspace was read (qsl-semantics check/typing, native_literal, family `encode_expression`, hyper_clause `leaf_payload`, checked_dispatch, assemble, layer, hybrid/callee; qsl-forms value.rs Shape/ExprNode); typing, UUID, family node encoding and hyper-clause leaf hashing read decoded `text()` only. `StringLiteral` is new public API of two `publish = false` crates; nothing outside the workspace is addressable from this repo, and all workspace members build.
Gates on the merge tree: cargo check and clippy (default and --no-default-features) with -D warnings clean, arch-lint qualified-core clean, qsl-cst, qsl-forms, qsl-eval, qsl-semantics lib and `--test it --all-features` green, qsl-package 27 failures identical to clean main (same test names). No `#[allow]`, `unwrap` or pins added.

## Findings

| ID | Severity | Summary | Refs |
|----|----------|---------|------|
| FND-001 | medium | The history-row Text literal path was changed (S2 builds `StringLiteral`; `history.rs` passes the real spelling to `from_source_literal`) but nothing reads its provenance: the forms test uses only `"a"` and no check-level test shows `"é"` and `"é"` history literals carry distinct provenance | qsl-semantics/src/check/refinement/history.rs:237-240 |
| FND-002 | low | `ExprNode`, `Expression` and `HistoryLiteralForm` derive `PartialEq`, so the spelling now takes part in S2 form equality; `DataLiteral::eq` and `literals.dedup()` compare `Expression` with `==`, so two spellings of one text are distinct literals there (FR-091 now says the spelling takes no part in equality) | qsl-semantics/src/check/timed_model.rs:2644-2649 |
| FND-003 | low | `encode_quoted` turns an impossible serializer error into an empty spelling with `unwrap_or_default`, which would silently record `""` as provenance instead of failing | qsl-cst/src/token.rs:41-46 |
| FND-004 | low | `StringLiteral::new(text, spelling)` is public and unchecked, so a caller can build a pair whose spelling does not decode to the text; the invariant is documented but not enforced | qsl-forms/src/syntax.rs:533-541 |
