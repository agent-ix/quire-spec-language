---
id: FR-256
title: "Parse source at any nesting depth, bounded by S1's resource limits"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-027
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: traces_to
---
# FR-256: Parse source at any nesting depth, bounded by S1's resource limits

## Description

The complete-V1 lossless CST parser SHALL lex and parse source of any bracket nesting and any chain
length, bounded only by S1's resource limits: source bytes, tokens, syntax
nodes and parser work (ADR-030 D-4.1, NFR-001).

## Behavior

1. **Iterative walk.** The lexer SHALL track open delimiters on an explicit
   heap stack, and each parser SHALL run its productions over an explicit
   heap frame stack, so that no lexing or parsing step uses native recursion
   whose depth grows with the input. Each stack SHALL grow by at most a
   constant per token or syntax node already charged against `s1.tokens`,
   `s1.nodes` or `s1.work_units`.
2. **Memo table.** The complete-V1 parser's memo table SHALL hold at most one
   entry per grammar production per token position.
3. **Resource limits only.** S1 SHALL bound a unit by `s1.input_bytes`,
   `s1.tokens`, `s1.nodes` and `s1.work_units` (FR-255). When the next S1
   operation would exceed one of them, S1 SHALL stop with
   `stage_limit_exceeded` naming that limit, its configured value, the count
   reached and its setting (FR-255), located at the region S1's diagnostic
   names.
4. **Every source settles.** Each S1 parser SHALL return a parse result or
   a parse diagnostic for every source within S1's configured limits,
   whatever its bracket nesting, operator chain length or `let`,
   `if … else` or prefix chain length.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-256-AC-1 | On a thread with a 512 KiB stack, the complete-V1 parser parses each of: brackets nested 100,000 deep around a literal, a `not` chain of 100,000, a `+` chain of 100,000 terms, an `else if` chain of 100,000 and a `let … in` chain of 100,000, each under `s1.input_bytes`, `s1.tokens`, `s1.nodes` and `s1.work_units` raised to fit it. | Test (TC-722) |
| FR-256-AC-2 | At the default S1 limits, each chain of AC-1 built to the longest length the defaults admit parses, and the same chain one element longer stops with `stage_limit_exceeded` naming `s1.tokens`, `s1.nodes` or `s1.work_units`, with its configured value and the count reached, and parses once that setting is raised through FR-255's settings operation, which the driver CLI exposes as `--limit` (ADR-029 CB-1). With `s1.nodes` at 200000 and every other S1 limit at its default, brackets nested to the depth the default token limit admits parse, and one pair deeper stops with `stage_limit_exceeded` naming `s1.tokens`, bound 100000 and the count reached. | Test (TC-723) |
| FR-256-AC-3 | `qsl_cst::Limits` holds source bytes, tokens, nodes and parser work, each set through its builder method, and no S1 outcome names a nesting depth. A parse of brackets nested 10,000 deep within the default byte, token and node limits succeeds. | Test (TC-723) |

## Dependencies

- [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  D-4.1 and D-6.
- [NFR-001](../non-functional/NFR-001-bound-syntax-work.md) sets S1's
  ceilings and defaults.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md) names
  each S1 setting.

## References

- QSpec FR-460, the ecosystem depth rule (Linear STD-143, which supersedes
  STD-125).
- Linear QSL-381.
