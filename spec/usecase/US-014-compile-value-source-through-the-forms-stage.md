---
id: US-014
title: "Compile Value-family source through the S2 forms stage into checked declarations"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-014: Compile Value-family source through the S2 forms stage into checked declarations

## Story

**As a** QSL caller who compiles complete-V1 source (the CLI, a library
caller, or the layer-6 `replay` facade recompiling a proved package)
**I want** source that declares `Value`-family types and functions to reach
the checker through S1, the S2 forms stage and one assembler
**So that** the declarations the checker admits come from my source text
through one path, and a malformed or unsupported declaration is refused with
a typed cause and a source span instead of a partial package.

## Context

ADR-011 §2.1 fixes the path from source to check as E1 (S0 → S1), E2
(S1 → S2) and E3 (S2 → S3). The S2 `forms` core exists (FR-067), but no
family is wired onto its dispatch table, so every source is refused at S2.
The checker takes a `PackageDeclarations` value that only tests build by
hand. The M-6a lane (ADR-011 §7.3) replaces native `compile` with spine
`compile`, which takes complete-V1 source, and adds spine `run`, which calls a
named checked function (ADR-011 §5). Native-run/1 clause execution stays
until M-6c. The spine starts at source.

## Acceptance Examples (Illustrative)

### US-014-EX-1: A function declared in source is checked and callable

- **Given** source that declares an alias `Digit = Int[0, 9]` and two
  functions, one calling the other.
- **When** the caller runs S1, S2, the assembler and the checker.
- **Then** the checker admits both functions, and calling the second returns
  the value the source defines.

### US-014-EX-2: A declaration from another family is refused, not skipped

- **Given** source that holds a `Value` function and an `invariant`.
- **When** it reaches S2.
- **Then** S2 refuses the whole unit and names the `invariant` token and its
  span. The function is not handed to the checker on its own.

### US-014-EX-3: Every unresolved type is reported at once

- **Given** two functions whose parameter types name undeclared types.
- **When** the assembler runs.
- **Then** one refusal lists both names, each with its span.

## Priority and Risk (Informative)

Priority: High. The M-6a spine `compile` and `run`, and `replay`'s S1
to S4 recompile, need source to reach the checker. With no S2 production and
no assembler, the checker is reachable only from hand-built test values.

## Traceability (Informative)

- [FR-091](../functional/FR-091-produce-value-forms-and-assemble-package-declarations.md)
