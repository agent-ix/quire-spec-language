---
id: TC-790
title: "Each model form types through StateModel and refuses with a StateModel cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-300
    type: verifies
---
# TC-790: Each model form types through StateModel and refuses with a StateModel cause

## Description

Verify that the five model expression forms type through the `StateModel`
family's `check` hook. Scope: FR-300-AC-1, FR-300-AC-2.

Catches a form left in the `Value` typer: its refusal would carry
`FamilyKind::Value` and the `value` prefix.

## Test Procedure

1. Compile one unit per form, well-typed: a field read, `allInstances<T>(p)`
   over a population declared with maximum 3, `lookup<T>(p, r)`, `reaches`
   inside a clause, and a dispatched query call in a precondition.
2. Compile one unit per form, ill-formed: a read of an undeclared field, an
   `allInstances` over an undeclared type, a `lookup` whose key's static
   type does not conform to `T`, a `reaches` over an undeclared field, a
   dispatched call to an undeclared operation.

## Expected Results

1. Every unit compiles; the checked nodes are `Attribute`, `AllInstances`
   typed `Set<T>[0,3]`, `Lookup` typed `T`, `Reaches` and a dispatched
   call, each with the static type QSpec FR-151 and FR-153 fix.
2. Every unit refuses at S3. Each cause has family `FamilyKind::StateModel`,
   a catalog code beginning `state-model`, and the QSpec code and cause the
   expected table in the test states as literals (`missing_declaration`/
   `missing-name` for the absent members and operation, `ill_typed`/
   `type-mismatch` for the `lookup` key).
