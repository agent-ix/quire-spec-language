---
id: SR-12055
title: "Code review of quire-spec-language PR #926: TemporalTrace S1/S2 forms with an optional interval"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3d82d2a55d69cf2a69fb37e61e07a809fbea3597; PR #926 (git diff origin/main...HEAD): qsl-cst/src/{cst,grammar,parser,token}.rs, qsl-forms/src/{dispatch,lib,syntax,temporal_clause,value}.rs, qsl-forms/tests/it/{temporal_clause_forms,protocol_clause_forms,value_forms}.rs, qsl-semantics/src/check/assemble.rs and tests.rs, qsl-replay/src/spine.rs, tests/it/compile_command.rs, Makefile"
review_set: subset
---
# Code review of quire-spec-language PR #926

## Summary

Ticket: QSL-507. Rust lane (`rust-review`) folded into this file.

Examined, with no defect found:

- S1 grammar: `Fairness` production (`fair [weak|strong] [whole|each] QualifiedName ;`) before `Capture*` in the clause body, matching the QSpec FR-362 `temporal-clause`/`fairness` productions; `opt(Interval)` on both the unary and binary temporal operators; `Interval` bounds as the new `Terminal::Bound`, which admits only an integer token whose digits parse as `u64`. The lexer already refuses leading zeros, so the `u64` parse is exact.
- Reserved words `fair`, `weak`, `strong`, `whole` are added to the complete facet only, as QSpec reserves them for the temporal (complete) facet; `each` was already reserved.
- S2 `temporal_clause`: the formula arena is built with an explicit LIFO work stack, no recursion. The root is the first node added (every pass-through level forwards its parent), so `TemporalNodeId(0)` is correct. The left-associative `and`/`or` chain builds links root-first and queues operands so each operator's `operands` stay in source order (traced for 2 and 3 operands). `implies` is right-associative as the CST nests it. `apply` takes the operator token, at most one `Interval` child and exactly `arity` operands, refusing any other shape.
- Interval forms: `[a,b]` gives `Finite(b)`, `[a,*]` gives `Open`, inverted bounds are kept as written; the interval span is the bracketed node span; each operator keeps its whole-application span and its own token span.
- Fairness and capture forms are built in source order with their spans; an unwritten kind or granularity stays `None`.
- S3 refuses each `temporal` clause with `unsupported_construct`/`not-yet-implemented` at its declaration span instead of compiling a package that silently lacks it; TC-435's forms-stage case moved to `synthesis`, which still has no dispatch entry.
- Tests: the TC-835 tests assert values (operator, interval bounds, spans by source slice, operand structure), the `u64::MAX` boundary and the first out-of-range value at both bound positions, the S1 diagnostic span, and `build_unit` failing. Oracles are strong enough to fail on a wrong interval, a lost span, wrong operand order or a dropped fairness/capture form.
- Rust lane: no `unsafe`, no `unwrap`/`expect`/indexing panics in production paths (slice patterns and `get` only), no integer casts, no recursion in build, compare, clone or drop of the formula arena.

The deletion of `tests/it/parser_differential.rs` and its recorded baseline is intentional (a recorded baseline is not a feature) and is not reviewed as a defect. The `unexpected` arm left in `qsl-forms/src/value.rs` is intentional (the ticket's "Deletes" line is mis-targeted).

## Verdict

Pass. No major defect in the S1/S2 parser, the form construction or the tests. The one major gap found is in what the form carries, and is recorded in SR-12056.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
