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

Verify the full intake path over the architecture fixture of
agent-ix/filament-core-data#173 and the model linker's binding of native model
references to its declarations. Scope: FR-056-AC-8, FR-036-AC-9.

## Test Procedure

1. Run the fixture bundle → `quire_rs::semantic::extract_semantic` with the real
   module manifests → `agent-ix-extraction-frontend` lift → intake seam. Use no
   hand-built semantic context or model-shaped JSON.
2. Inspect each admitted Port and one admitted Connection between two ports.
3. Link a native package whose model references name a fixture object's field, a
   Port and the Connection against the admitted domain package.
4. Substitute a same-shaped declaration from a different domain package for one
   reference, and separately change the selected digest.

## Expected Results

- Step 1 admits every IR node with no refusal.
- Step 2: each Port retains its owning part, direction, interface type and typed
  multiplicity; the Connection is admitted under FR-152's connection rule.
- Step 3 binds every model reference to its declaration key and export or
  systems kind; no reference resolves by spelling alone.
- Step 4 refuses each dependent native declaration with its typed cause while
  unrelated declarations stay bound.
