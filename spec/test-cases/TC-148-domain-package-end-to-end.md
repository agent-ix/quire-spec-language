---
id: TC-148
title: "Link native source against a domain-package bundle end to end"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/IT-012, type: references }
---
# TC-148: Link native source against a domain-package bundle end to end

## Description

Verify the full model intake path over a real domain-package bundle and the
model linker's binding of native declarations to the admitted model
declarations. Scope: FR-056-AC-7, FR-036-AC-1, FR-036-AC-3.

The bundle's constructs cover object, record, enum, reference, operation,
relationship and population exports. Which module and fixture supply it is
recorded under agent-ix/quire-spec-language#131.

## Test Procedure

1. Run bundle → `quire_rs::semantic::extract_semantic` with the real module
   manifests → `agent-ix-extraction-frontend` lift → `agent-ix-semantic-ir` read →
   model declarations. Use no hand-built semantic context or model-shaped JSON.
2. Compile a native package whose state predicate reads an object field, follows
   a reference, calls an operation contract and quantifies over a population,
   and whose protocol names a relationship. Link it against the admitted domain
   package.
3. Inspect each bound model reference: domain package identity and digest,
   declaration key, export kind and source loci on both sides.
4. Substitute a same-shaped declaration from a different domain package, and
   separately a changed domain package digest.

## Expected Results

- Step 1 admits every type definition and relation with no diagnostics.
- Step 2 binds every model reference in the native package; no reference
  resolves by spelling alone.
- Step 3 retains the exact domain package identity, digest, declaration key and
  export kind for each binding.
- Step 4 refuses each dependent native declaration with its typed cause while
  unrelated declarations stay bound.
