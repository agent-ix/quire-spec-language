---
id: SR-1360
title: "Code review of quire-spec-language PR #653: the root native parser takes caller limits as given (QSL-645)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5aaaf81cd8778e1ffb83e9cf286888e736ffb538; PR #653 diff against origin/main: src/parser.rs, src/syntax.rs, src/linking/composed.rs, tests/it/deep_sources.rs, tests/it/main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: reviews
---
# Code review of quire-spec-language PR #653

## Summary

Ticket: QSL-645. Code review with the rust-review lane folded in.

The production change deletes `Limits::bounded()` (src/syntax.rs), which clamped every caller limit to the defaults (1 MiB, 100,000 tokens, 50,000 nodes), and its five call sites: `parse`, `parse_source`, `parse_native`, `parse_native_source` (src/parser.rs) and `admit_namespace` (src/linking/composed.rs). No parser logic changes; the root parser was already iterative.

- Non-recursion, checked beyond the tests. The expression parser (src/parser/expression.rs) runs on an explicit `Vec<Frame>`; temporal formulas (src/parser/composed/temporal.rs) use their own frame stack; protocol control nodes (src/parser/composed/protocol.rs `control_node`) use an explicit `Vec<Pending>`. The only re-entries are bounded one level deep (`holds(expr)` in temporal, `from`/`to` in protocol intervals, value blocks in declarations). No parser function calls itself.
- Drop/Debug/Clone of results. `ParsedUnit` and `NativeUnit` hold flat arenas (`Vec<Expr>`, `Vec<c::Expression>`, `Vec<c::Temporal>`, `Vec<c::Control>`) indexed by `ExprId`/`ControlId`. The only boxes in src/syntax are `Box<Activation>`, `Box<Protocol>` and `Box<Compensation>`, none self-referential. Derived Drop, Debug and Clone are therefore flat. The tests drop the result inside the 512 KiB thread, so Drop is exercised.
- No hidden limit left on the parser path. `Source::read_typed` (qsl-foundation/src/source.rs) checks only the given `byte_limit`; the lexer's `recognize` checks only `limits.tokens`; `lexer_limits` passes a default `work_units` that `recognize` never reads. No arithmetic on limit fields in the parser or consumers (`usize::MAX` limits are safe).
- Mutation run, through `locked-build.sh`, in a scratch worktree at the head: (a) re-adding a 100,000-token clamp in `parse_source`/`parse_native_source` fails all 10 TC-749 tests; (b) adding a depth-proportional native recursion before parsing aborts the test binary with a stack overflow. Both oracles bite.

Examined:
- src/parser.rs (examined), src/parser/expression.rs, src/parser/composed.rs, src/parser/composed/temporal.rs, src/parser/composed/protocol.rs (examined for recursion)
- src/syntax.rs, src/syntax/composed.rs (examined for recursive types)
- src/linking/composed.rs `admit_namespace`, src/linking/composed/inventory.rs:127 (examined)
- tests/it/deep_sources.rs (examined)
- qsl-cst/src/lexer.rs `recognize`, qsl-foundation/src/source.rs `read_typed` (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The TC-749 tests check only that `parse`/`parse_native_source` return `Ok`. None checks the result's shape, so a parser that accepted the input but dropped or flattened part of it would still pass. The existing unit test at src/parser.rs:400 asserts `unit.expressions.len() == depth + 1` for brackets; add an equivalent count (or a root-kind check) per shape. | tests/it/deep_sources.rs:48-75 |

## Verdict

Gates at 5aaaf81c: `cargo test --test it deep_sources` passes (10 tests, 0.72 s); `cargo clippy -p quire-spec-language --all-targets -- -D warnings` and `cargo fmt --check` are clean. Mutation results above. One low finding. The code change is correct and minimal: no recursion on any parser path, flat result trees, no remaining parser-side clamp. Mergeable on the code side once FND-001 is fixed; see SR-1364 and SR-1365 for the spec and gap findings that block.

## Dispositions

Round 1, reviewed at 9163ca383852b50150be0ab472efcbb5c6008002.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3430ab929 (bounds refined in 9ffcfbbb2): each TC-749 test now asserts one clause/declaration and an expression count of at least DEPTH (2*DEPTH-1 for the sum) |
