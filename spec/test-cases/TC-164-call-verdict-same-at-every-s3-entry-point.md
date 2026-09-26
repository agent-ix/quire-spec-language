---
id: TC-164
title: "A call receives the same verdict from a declaration body and from a clause expression"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-164: A call receives the same verdict from a declaration body and from a clause expression

## Description

Verify that `Value`'s application check (`check::family::Application`)
decides every function application the S3 typer reaches, whichever entry
point reaches it. Each call below is checked as the body of a function
declaration checked through `PackageDeclarations::check`, which reaches the
body through `ValueFunctionFamily::check`, and as a precondition clause
checked through `CheckedGraph::check_clause_expression`; each refused call
is also checked as a `decreases` measure. The verdicts must agree. Scope: FR-065-AC-5.

## Test Procedure

1. Build a package that declares `f(x: Boolean): Boolean` with body `x`.
2. For each call `c` in `f(true)`, `f(true, false)`, `nowhere(true)` and
   `f(1)`, add a parameterless `Boolean`-result declaration `g` whose body
   is `c`, and check the package through `PackageDeclarations::check` with
   `CheckingLimits::default()`.
3. For each of the same calls, check the package that declares only `f`,
   then check `c` through the resulting `CheckedGraph`'s
   `check_clause_expression` with empty `parameters`, expected type
   `Boolean`, `ClauseKind::Precondition`, `CheckMode::Linked` and
   `CheckingLimits::default()`.
4. For each of `f(true, false)`, `nowhere(true)` and `f(1)`, add a
   parameterless `Boolean`-result declaration `h` with body `true` and
   `decreases` measure `c`, and check the package through
   `PackageDeclarations::check`.

## Expected Results

Verdicts are compared by refusal cause only; the refusal locations differ
between positions.

- `f(true)`: step 2 admits the package and step 3 admits the clause; both
  checked calls have type `Boolean` and call `f`.
- `f(true, false)`: step 2 refuses with exactly one refusal, and steps 2, 3
  and 4 all refuse with `ill_typed` / `type-mismatch`.
- `nowhere(true)`: step 2 refuses with exactly one refusal, and steps 2, 3
  and 4 all refuse with `missing-name` naming `nowhere`.
- `f(1)`: step 2 refuses with exactly one refusal, and steps 2, 3 and 4 all
  refuse with `ill_typed` / `type-mismatch`.

## Status

Backed by `a_call_receives_the_same_verdict_from_a_declaration_body_a_clause_and_a_measure`
(`qsl-eval/tests/it/call_verdicts.rs`, tagged `TC-164` / `FR-065-AC-5`). It
replaces TC-164's earlier procedure (a compiled-symbol search and an `E0004`
compile-fail fixture), which asserted code shape rather than behaviour and
which ADR-012 §4.3 contradicts. Owner: QSL-148.
