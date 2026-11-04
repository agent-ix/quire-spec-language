---
id: SR-2299001
title: "code-review and rust-review of reserved function and predicate names (quire-spec-language PR 2299)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-cst/src/parser.rs; qsl-cst/src/token.rs; qsl-cst/tests/it/reserved_declaration_names.rs; qsl-semantics/tests/it/reserved_declaration_names.rs; tests/it/parser.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: references
---

# Code review of QSL-1589 reserved declaration names (quire-spec-language PR 2299)

## Summary

Ticket: QSL-1589. Diff `origin/main...b7746f2fb`. One code-review pass with the rust-review lane.

Focused tests run on the PR head: `cargo test -p qsl-cst --test it reserved_declaration_names`
(2 pass), `cargo test -p qsl-semantics --test it reserved_declaration_names` (2 pass) and
`cargo test -p quire-spec-language --test it complete_compound_words` (1 pass). The root-crate
`#[trace("TC-113", "FR-035-AC-7")]` tag on `tests/it/parser.rs` compiles. `cargo xtask string-edge`
exits 1 with 154 occurrences, none of them in `qsl-cst/src/parser.rs` or `qsl-cst/src/token.rs`.

A throwaway probe (deleted afterwards) put each of `helper` and `cast` through S1, S2 and S3 in
other name positions under `1-draft`. A record name, field, enum name, enum member, parameter or
`let` binder spelled that way passes S1 and is refused at S2 as `UnexpectedShape`, spanning the
whole production. That refusal is misleading but not silent. The `using <alias>` position of the
same function and predicate productions this PR handles is silent: see FND-001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The PR checks only the name token (`.nth(1)`) of `FunctionDeclaration` and `Predicate`. The `using` alias of the same productions still has the bug. `function f using cast(x: Int[0, 9]): Int[0, 10] pure { x + 1 }`, with only `profile v` declared, passes S1, S2 and S3. `predicate p using rec(...)` does the same. A genuinely undeclared alias (`using nosuch`) is refused `UndeclaredAlias`. S2's `function_like` reads `[name]` from the `TokenKind::Identifier` tokens, so the `Grammar`-kind alias is silently dropped and the function loses its profile selection. Fix: refuse a reserved spelling at the alias token too, the token after `using`. A better fix is to make `Terminal::Identifier` / `TypeIdentifier` / `IdentifierAllowReserved` reject a token whose lexer kind is `Unsupported`. That refuses at S1 at the exact span in every name position, not just the declaration name. | qsl-cst/src/parser.rs:1012-1055; qsl-forms/src/value.rs:226-240 | correct-requirement-no-evidence |
| FND-002 | low | `reserved_declaration_names` is marked `#[ix_trace_rs::string_edge]`, but it contains no string comparison and no string `match`. It matches on `Production` and calls `unsupported_spelling`, so the marker silences nothing today. It would hide a future string dispatch added to this function from the scan. Drop the marker. | qsl-cst/src/parser.rs:1017 | correct-requirement-no-evidence |

## Verdict

The name-position fix is correct, minimal and reads its reserved set from the lexer, so there is
no second list. The spans, cause and message are tested exactly. Rust idioms are fine:
`filter_map` / `then`, `from_utf8(..).ok()?`, and no panics or allocations beyond diagnostics.
FND-001 is the same silent wrong-name bug, in the same two productions, at the alias position.
CONDITIONAL on FND-001.
