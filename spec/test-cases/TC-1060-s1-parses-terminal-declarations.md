---
id: TC-1060
title: "S1 parses terminal when and terminal any as terminal declarations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: verifies
---
# TC-1060: S1 parses terminal when and terminal any as terminal declarations

## Description

Verify that the S1 grammar parses `terminal <name> using <alias> on <model> when
<block> ;` and `terminal <name> using <alias> on <model> any ;` as one
`TerminalDeclaration` node each, and refuses a malformed declaration at the
first token the grammar does not admit. Scope: FR-124-AC-4.

Catches a `terminal` word read as an identifier, a declaration that admits
neither `when` nor `any`, a `when` without a block, and a diagnostic located at
the wrong token.

## Test Procedure

Parse one unit per row at S1 under profile alias `Complete`.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `terminal Done using Complete on M when { true };` then `terminal Halt using Complete on M any;` | admissible; two `TerminalDeclaration` nodes whose text is each declaration; the `when` declaration holds one `Block` node and the `any` declaration holds none |
| 2 | `terminal Done using Complete on M;` | `invalid_syntax` at `;` |
| 3 | `terminal Done using Complete on M when true;` | `invalid_syntax` at `true` |
| 4 | `terminal Done using Complete when { true };` | `invalid_syntax` at `when` |
| 5 | `terminal using Complete on M any;` | `invalid_syntax` at `using` |

## Expected Results

Both declaration forms parse to a `TerminalDeclaration` node. Each malformed
declaration is refused at the token the grammar does not admit.
