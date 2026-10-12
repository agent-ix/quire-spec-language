---
id: TC-987
title: "Unit compile refuses duplicate operation bodies resolving to one key"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-987: Unit compile refuses duplicate operation bodies resolving to one key

## Description

Verify that S3 refuses two unit `operation-body` forms that resolve to the
same operation `DeclarationKey`, using the existing duplicate-declaration
diagnostic. Scope: FR-302-AC-7. TC-986 is the single-body control.

## Test Procedure

1. In a `1-draft` unit selecting profile alias `V`, select a domain package
   through model alias `M` with object type `A` and operation `A.size`.
2. Declare two operation bodies with distinct local labels but identical
   resolved targets: `body first using V on M::A::size() { 11 }` and
   `body second using V on M::A::size() { 22 }`.
3. Compile the unit and inspect the S3 refusal.

## Expected Results

Compilation refuses at S3 with
`conflicting_declaration`/`duplicate-declaration`, naming both body
declarations. It produces no checked package and no partial dispatch table.
Changing source order or the local labels does not change the duplicate
outcome. With exactly one of those bodies, TC-986's single-body control
applies: that body is associated with `A.size` by its resolved
`DeclarationKey`.
