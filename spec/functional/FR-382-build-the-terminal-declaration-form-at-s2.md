---
id: FR-382
title: "Build the terminal declaration form at S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: depends_on
---
# FR-382: Build the terminal declaration form at S2

## Description

When S2 builds the form of a `terminal` declaration, it SHALL build one
`TerminalDeclarationForm` that holds every member the production writes, with
its span (ADR-012 §1, §3). The form records which states the model's author
marks intended, and nothing else. S2 records what was written and resolves
nothing; S3 checks it (FR-124).

## Use case

A verification operator writes `terminal Done using t on Config when {
counter.value = 3 };` for a counter that stops at 3, and `terminal Halt using t
on Config any;` for a model that halts wherever it halts. Each declaration
reaches the checker as a typed form with its name, aliases and predicate, so a
refusal about the declaration points at the text the operator wrote.

## Inputs

- The CST of one `TerminalDeclaration`, parsed by QSpec's shared-grammar
  production `terminal-decl = 'terminal', ident, 'using', ident, 'on', ident,
  ( 'when', block | 'any' ), ';'` (FR-124-AC-4). The `block` is the same
  `block` production a state clause reads.

## Outputs

In `forms::terminal_declaration` (crate `qsl-forms`):

- `TerminalDeclarationForm{name: DeclaredName, profile: UsingAlias, model:
  DeclaredName, kind: TerminalDeclarationKind, span}`.
- `TerminalDeclarationKind::{When(ExpressionForm), Any}`, where the
  `ExpressionForm` holds the block's expression and the span of each of its
  nodes.
- `DeclarationForm::Terminal`, built for a declaration whose leading token is
  `terminal`.

## Behavior

- S2 SHALL build one `TerminalDeclarationForm` per `TerminalDeclaration` node,
  in source order, with the written name, `using` alias and `on` model alias,
  each with the span of its identifier.
- S2 SHALL build `When` for `when` followed by a block and `Any` for `any`,
  and SHALL build the `When` predicate as the expression it is written, with
  no evaluation or typing.
- S2 SHALL set the form's span to the whole declaration, `;` included.
- S2 SHALL keep every `terminal` declaration it finds, so a second
  declaration for one model alias builds a second form. S3 refuses it
  (FR-124).
- S2 SHALL resolve neither alias, so a model alias that names no model gives
  no S2 refusal.
- S3 resolves both aliases and checks the declaration (FR-124). A `terminal`
  declaration on an alias that names no `model` declaration refuses
  `missing_declaration`/`missing-name` at that alias. No S4 emission writes a
  checked `terminal` declaration yet, so a package that checks with one
  refuses `unsupported_construct`/`not-yet-implemented` at the declaration's
  span until S4 writes it, and the declaration never compiles as silently
  ignored.
- A well-formed CST adds no S2 refusal. A CST that carries an error or
  recovery node SHALL refuse as `FormsCause::RecoveringCst`, with no partial
  form. Building the same unit twice gives equal forms.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-382-AC-1 | `terminal Done using t on Config when { not false };` builds one `TerminalDeclarationForm` with name `Done`, `using` alias `t`, model alias `Config`, kind `When` and a predicate whose root is `not` over the Boolean `false`; each identifier span slices to its text, the predicate's root span slices to `not false`, and the form's span slices to the whole declaration including `;`. | Test (TC-1059) |
| FR-382-AC-2 | `terminal Halt using t on Config any;` builds kind `Any` with the written name and model alias, and a span that slices to the whole declaration. | Test (TC-1059) |
| FR-382-AC-3 | A unit that writes `terminal First using t on Config any;`, a `temporal` clause, then `terminal Second using t on Config when { true };` builds three forms in that order: a `Terminal` form of kind `Any`, a `Temporal` form and a `Terminal` form of kind `When`. The two terminal forms name the same model alias and neither is merged or refused. | Test (TC-1059) |
| FR-382-AC-4 | A `terminal` declaration that omits its closing `;`, or whose `when` is not followed by a block, has a recovering CST and refuses `FormsCause::RecoveringCst` with no form. | Test (TC-1059) |
| FR-382-AC-5 | Building the same unit twice gives equal `TerminalDeclarationForm`s, for both a `When` and an `Any` declaration. | Test (TC-1059) |
| FR-382-AC-6 | A unit that holds `terminal Done using v on M when { true };`, and a unit that holds `terminal Halt using v on M any;`, each assemble, with `M` naming no `model` declaration, to one refusal with code `missing_declaration`, cause `missing-name` and the span of `M`; with `M` naming a model, each checks and the package refuses `unsupported_construct`/`not-yet-implemented` at the span of the declaration until S4 writes it. | Test (TC-1059) |

## Dependencies

- ADR-012 §1, §3 (forms); [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the `terminal` declaration and its S3 check); FR-308 (the protocol forms
  that S2 builds beside it); FR-325 (`ExpressionForm`).
- QSpec owns the grammar: the shared grammar's `terminal-decl` production.

## References

- QSpec FR-366 (deadlock freedom: the `terminal` declaration).
