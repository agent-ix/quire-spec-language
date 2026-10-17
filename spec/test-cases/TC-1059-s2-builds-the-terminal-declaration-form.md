---
id: TC-1059
title: "S2 builds the terminal declaration form with its aliases, kind and spans"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-382
    type: verifies
---
# TC-1059: S2 builds the terminal declaration form with its aliases, kind and spans

## Description

Verify that S2 builds one typed form for each `terminal` declaration a unit
writes, in source order, with its name, `using` alias, model alias, kind and
spans, resolves nothing, and refuses a recovering CST with no partial form.
Scope: FR-382-AC-1 to FR-382-AC-5.

Catches a `when` predicate dropped or built as `Any`, an `any` declaration built
with an invented predicate, a second declaration for one model alias merged or
refused at S2, an alias resolved at S2, and a partial form built beside a
recovery node.

## Test Procedure

Build the S2 forms of one unit per row with profile alias `t` and model alias
`Config`, reading the form fields and the span of each part. Assert the form
fields, never a rendered message.

| # | Input | Expected |
| --- | --- | --- |
| 1 | `terminal Done using t on Config when { not false };` | name `Done`, alias `t`, model `Config`, kind `When` over `not false`, each span slicing to its text, the form span covering the declaration with its `;` |
| 2 | `terminal Halt using t on Config any;` | kind `Any`; name `Halt`; model `Config`; span covering the declaration |
| 3 | `terminal First ... any;`, a `temporal` clause, `terminal Second ... when { true };` | three forms in source order: `Terminal` (`Any`), `Temporal`, `Terminal` (`When`); both terminal forms name `Config` |
| 4 | a `terminal` declaration missing its `;`, and one whose `when` has no block | each refuses `FormsCause::RecoveringCst`; no form |
| 5 | rows 1 and 2, each built twice | equal forms |

## Expected Results

Every `terminal` declaration reaches the checker as its own typed form with the
kind it was written with. A repeated declaration is kept as written. A
recovering CST yields no form.
