---
id: TC-147
title: "Resolve module-qualified kinds through the embedded constructs table"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
---
# TC-147: Resolve module-qualified kinds through the embedded constructs table

## Description

Verify that the compiler resolves each type definition's `{module, name}` kind
only through the IR document's embedded `constructs` table, with no module
registry, installed manifest or kind list. Scope: FR-056-AC-4.

## Test Procedure

1. Admit a lifted IR 2.0.0 document in an environment with no installed module
   manifests and no network access.
2. Remove one kind's entry from the embedded `constructs` table while its type
   definitions remain.
3. Add a second `constructs` entry for the same `{module, name}` that differs in
   module version, then separately only in manifest digest, then separately in
   one declaration member.
4. Declare the same kind `name` under two different module ids with different
   constructs and meaning ids.

## Expected Results

- Step 1 admits; resolution reads only the embedded table.
- Step 2 refuses each affected declaration with `unresolved-construct-kind`
  naming the module and kind name; other declarations stay admitted.
- Step 3 refuses the domain package with `conflicting-construct-declaration` in
  each variant.
- Step 4 admits two distinct kinds, each bound through its own construct and
  meaning id; neither inherits the other's meaning.
