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

No test implements this procedure yet. It replaces TC-164's earlier
procedure (a compiled-symbol search and an `E0004` compile-fail fixture over
a composed-checker input enum), which asserted code shape rather than
behaviour and which ADR-012 §4.3 contradicts: `Expression::Call` is a
`Value`-owned variant of the one `Expression` enum. The QSL-148 spec lane
made this change under the
[testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
(Peter, 2026-09-22). `synthesized_dispatch_candidate_is_not_callable_by_name`
(`qsl-eval/tests/it/dispatch_calls.rs`) covers one declaration-body
`missing-name` refusal through `PackageDeclarations::check`; it does not
compare the positions. Owner: QSL-148.
