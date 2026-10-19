---
id: SR-12145
title: "QSL-557 PR #974 code review: protocol, memory and refinement-row grammar (FR-211..FR-220, FR-136)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@458459d34cc8c020cfba66243e3ab42045622d34; qsl-cst/src/cst.rs, qsl-cst/src/grammar.rs, qsl-cst/src/lib.rs, qsl-cst/src/token.rs, qsl-cst/tests/it/complete_grammar.rs, qsl-cst/tests/it/main.rs, qsl-cst/tests/it/protocol_memory_refinement.rs, qsl-forms/src/protocol_clause.rs, qsl-forms/src/syntax.rs, qsl-forms/src/value.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-semantics/src/check/protocol_clause/coverage.rs, qsl-semantics/src/check/protocol_controls.rs, qsl-semantics/src/check/protocol_reference.rs"
review_set: subset
---

## Summary

Ticket: QSL-557. PR quire-spec-language#974. Code review with the rust-review lane, scoped to `git diff origin/main...HEAD` (14 files, +537/-3, 3 commits). Read only. No build was run; the PR description reports `cargo check`, `cargo clippy --all-targets --all-features -D warnings`, `cargo fmt --check` and the qsl-cst, qsl-forms and qsl-semantics tests as the lead's receipt.

The new productions were compared with QSpec's shared grammar (`parallel`/`memory-clause`, `event-node` `attempt` ordering, `memory-order`, `fence-order`, `fence`, `control`, `protocol-step-row`, `protocol-step-source`, `protocol-step-target`) and QSpec FR-375's `refinement`, `step-row`, `step-target` and `step-argument`. They match, apart from the two known disagreements already routed to the planner (FR-220-AC-2's `fence ... relaxed`; FR-375's population, object, history and fairness rows not parsed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The parser is a PEG. Ordered choice and `opt` do not backtrack after a match. So the contextual step words shadow a model alias with the same spelling inside step rows only: `step fork::T::op -> ...` (and `cend`, `duplicate`, `lose`, `spawn`, `retire` as alias names) fails, because `opt(fork ...)` takes the alias. `-> stutter::T::op(self)` also fails, because the `stutter` alternative matches first and the row then wants `;`. Those words are identifiers everywhere else. The result is a syntax refusal, never a wrong result. QSpec reserves `fork` and `stutter` outright. | qsl-cst/src/grammar.rs:1643-1684 |
| FND-002 | low | `src/complete/editor.rs` `is_symbol` lists every top-level declaration production, but the PR did not add `Production::Refinement`. A refinement declaration therefore gets no editor symbol, unlike the other declarations. | src/complete/editor.rs:364-388 |

## Verdict

PASS. No major bug. Both findings are low and need not block.

Checked and clean:

- No new keyword. `memory`, `sc`, `tso`, `ra`, `relaxed`, `acquire`, `acq_rel`, `seq_cst`, `fence`, `refinement`, `abstract`, `concrete`, `step`, `stutter`, `node`, `fork`, `cend`, `duplicate`, `lose`, `spawn`, `retire` and `_` are all `literal(...)`. `x(...)` is used only for words already in the vocabulary (`release`, `join`, `activate`, `any`, `ordering`, `using`). `terminal` is still `literal("terminal")` in `TerminalDeclaration` and `ProtocolTerminal` (hotfix #961 holds). The existing `sequence fork { attempt terminal ... }` forms test still covers both words as identifiers.
- `->` is the only new token, and it is complete-only. Logos takes the longest match, so `->` now lexes as one token. `composed_base`/`historical` reclassify it as `Unsupported`. No base or complete construct had `-` directly followed by `>` as valid input, so no source that parsed before changes meaning.
- `#![recursion_limit = "256"]` is justified. `productions_impl!` is a tt-muncher that recurses once per production, and this PR adds nine productions. The raise applies only to `qsl-cst`.
- No silent `sc`. The `parallel` arm pushes a `MemoryClause` construct for every `memory` clause, whether `sc`, `tso` or `ra`. `event_node_anchors` pushes `AccessOrdering` for every attempt `ordering`. `fence` declares `ProtocolNodeKind::Fence`, and `covered_kind(Fence)` is `false`. `coverage::content` refuses every construct that is not a plain `Channel`, and every uncovered node kind, as `unsupported_construct`/`not-yet-implemented`. The new semantics cases (`memory tso`, `ordering seq_cst`, `fence F ordering seq_cst`) assert the code, the tag and the construct label, and each `replacen` fixture would make `only_refusal` panic if it did not apply. A `refinement` declaration has no `from_spelling` entry, so S2 refuses it with `NoDispatchEntry` (`unsupported_construct`) and it never reaches S3.
- `parallel_form` reads only the identifier tokens directly under the parallel. `memory`/`tso` sit inside the nested `MemoryClause` node, so they never become the parallel's name or a join label. The forms test asserts this.
- The `Fence` arm takes identifier index 1 (index 0 is the contextual `fence`). It returns `unexpected` rather than panicking when the token is absent.
- `value.rs` adds the new productions to both exhaustive not-a-value arms. `protocol_reference::operator` classifies `Fence` with no catch-all.

Test-oracle notes for `qsl-cst/tests/it/protocol_memory_refinement.rs`: each accepting test asserts the exact source text of every node of the production it targets, in order. Each refusing test asserts `InvalidSyntax` and the exact offending token, so a grammar that accepted too much or too little would fail it. `StepArgument` is asserted as `["1", "_", "_", "1"]`, which pins the rule that the receiver is an `Expression` and not a `StepArgument`. `StepSource` and `StepTarget` are asserted by count only. That weakness, and the trace-tag scope, are recorded in SR-12146.
