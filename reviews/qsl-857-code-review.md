---
id: SR-12113
title: "Code review of quire-spec-language PR 949: the terminal declaration S1 production, S2 form and assembly refusal"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7e2480dc5d66f1a03473dafe3979455f774158fd; PR 949: qsl-cst/src/{cst,grammar,token}.rs, qsl-cst/tests/it/{terminal_declaration,complete_grammar,main}.rs, qsl-forms/src/{dispatch,lib,syntax,terminal_declaration,value}.rs, qsl-forms/tests/it/{terminal_declaration_forms,protocol_clause_forms,value_forms,main}.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/assemble/tests.rs, src/complete/editor.rs"
review_set: subset
---
# Code review of quire-spec-language PR 949

## Summary

Ticket: QSL-857. Code review with the rust-review lane folded in, scoped to the
Rust files of `git diff origin/main...HEAD`.

- S1. The `TerminalDeclaration` production is `terminal ident using ident on
  ident ( when Block | any ) ;`. That is QSpec's shared-grammar `terminal-decl`
  token for token (QSpec shared grammar, `terminal-decl`; QSpec FR-366). It is
  reachable from the source-declaration choice. `terminal` is added to the
  complete vocabulary, which QSpec FR-366 requires as a reserved word. No
  existing QSL source fixture uses `terminal` as an identifier. `Block` is
  `{ Expression }`, so the `when` arm cannot carry anything but one expression.
- S2. `terminal_declaration` reads the node's three direct identifier tokens in
  order (name, `using` alias, model alias). `any` is a keyword token, so it is
  not counted. It builds `When` from the sole `Block`'s `Expression` through the
  shared `expression` builder, and `Any` only when no block is present and `any`
  is present. Every other shape returns `unexpected(node)`, a typed
  `FormsRefusal`. There is no `unwrap`, index or panic on the production path, so
  a malformed CST cannot panic. The form span is the node span, `;` included.
- Assembly. `Unit::new`'s exhaustive `DeclarationForm` match records each
  terminal span. `assemble_with_cancel` pushes one
  `AssemblyCause::UnsupportedDeclarationForm { form: "terminal" }` error per span
  before any other stage. Every later stage returns `refuse(errors)` when errors
  is non-empty, so no unit holding a terminal declaration yields a package. The
  `code()` and `catalog_code()` arms map it to
  `unsupported_construct`/`declaration-form`, an existing catalog pair (FR-110,
  FR-056). It is an `AssemblyCause`, not a `CheckCause`, so the seam-probe list
  is unchanged.
- Exhaustive matches. `qsl-replay` `spine.rs` `assembly_message` and
  `spine/refinement.rs` `assembly_limit` gain the arm. The other `AssemblyCause`
  matches in `qsl-replay` (`lifecycle.rs`) and `qsl-semantics` (`check/mod.rs`,
  `protocol_clause.rs`) are `if let` or targeted `matches!`, which are correct
  for a non-limit cause. `Unit::new` is the only production consumer of
  `DeclarationForm`. The `qsl-forms` non-value production lists in `value.rs` and
  the editor's `is_symbol` gain `TerminalDeclaration`.
- Tests. `qsl-cst/tests/it/terminal_declaration.rs` checks the two node spans,
  that the `Block` sits only in the `when` form, and four malformed
  declarations, each located at the offending token as `invalid_syntax`.
  `qsl-forms/tests/it/terminal_declaration_forms.rs` asserts typed form fields
  and spans, never a rendered message. The assembler test asserts the exact
  error vector (cause and span), `Code::UnsupportedConstruct`, and the catalog
  pair.
- Rust idiom. No `#[allow]`, no ticket ids in code comments, no legacy or
  compatibility code, and nothing vendored. The `expect` calls are in test
  helpers only. `form: &'static str` follows the existing `AssemblyCause`
  style.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. The S1 production is QSpec's `terminal-decl`. The S2 form maps it field
for field, with typed tests and no panic path. Every exhaustive `AssemblyCause`
match handles the new variant, and the refusal carries the right catalog pair
at the declaration's span, so no terminal declaration compiles and gets
silently ignored. No code defect was found.
