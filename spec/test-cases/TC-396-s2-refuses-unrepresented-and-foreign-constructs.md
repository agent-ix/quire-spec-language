---
id: TC-396
title: "S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-396: S2 refuses unrepresented constructs, and the check stage refuses another family's construct with that family's cause

## Description

Verify two things. S2 refuses the whole unit, and never yields a partial
form, for a construct with no `Expression` variant (ADR-011 §2.3 E2). And
S2 builds the `forms`-core variant for `deref`, `pre` and `allInstances`
nested in a function body, which the check stage then refuses with the
owning family's catalogued cause.

This catches a production that refuses another family's construct at S2
with `unsupported_construct`. That would discard the `forbidden-pre-read`
distinction that FR-090 assigns to `ProtocolClause`.

Scope: FR-091-AC-7, FR-091-AC-8.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. For each body below, build an admissible unit whose one function body is
   that construct, and run S2: `"text"`; `decimal(1, 2)`; `a mod b`;
   `xs[0]`; `none`; `reaches(a, b, M::R)`.
2. Run S2 on the body `if a then b else a mod b`.
3. For each body `B` of `deref(x)`, `allInstances<M::T>(x)` and `pre(x)`,
   build an admissible unit, with no admitted domain package, whose one
   declaration is `function f using v(x: Int[0, 9]): Boolean pure { B }`.
   Do the same for
   `function g using v(x: Int[0, 9]): Boolean pure decreases(pre(x)) { true }`.
   Run S2, then the assembler, then `PackageDeclarations::check` on
   whatever the assembler returns.

Tag the test `#[trace("FR-091-AC-7", "FR-091-AC-8", "TC-396")]`.

## Expected Results

- Every step-1 case refuses with cause `UnrepresentedConstruct`, names the
  construct's CST production and exact span, and returns no parsed unit.
- Step 2's span is the `a mod b` sub-construct, not the whole body, and it
  returns no parsed unit.
- In step 3, S2 returns a parsed unit holding `Deref`, `AllInstances` or
  `Pre` where the construct is written.
- `pre(x)`, in `f`'s body and in `g`'s measure, refuses at check with
  `wrong_snapshot`/`forbidden-pre-read`.
- `deref(x)` refuses at check with `ill_typed`/`type-mismatch`.
- `allInstances<M::T>(x)` refuses in the assembler with an
  unresolved-type-name error naming `M::T`, code
  `missing_declaration`/`missing-name`.
- No step-3 refusal has code `unsupported_construct`.

## Status

Backed by `qsl-forms/tests/it/value_forms.rs` (`a_construct_no_variant_represents_refuses_the_unit`, `nested_constructs_of_other_families_are_built_where_written`) and `qsl-semantics` `check::assemble` tests (`nested_constructs_of_other_families_refuse_with_their_own_causes`).
