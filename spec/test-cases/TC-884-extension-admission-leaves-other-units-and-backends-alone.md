---
id: TC-884
title: "An extension changes no other unit's identity, backends change no admission, and a reader without it refuses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-354
    type: verifies
---
# TC-884: An extension changes no other unit's identity, backends change no admission, and a reader without it refuses

## Description

Verify the independence of admission from unselected extensions and registered backends, and QSL's I2 reader over held and withheld extension definitions (QSpec FR-469 reader join 2).

Scope: FR-354-AC-5.

## Test Procedure

1. Compile a unit that selects no extension over a catalog with and without TC-882's extension.
2. Compile TC-882 step 1's unit with two registered backends, then with none.
3. Read TC-882 step 1's package with QSL's I2 reader, `evidence` holding the `retention` definition inserted under its `DefinitionRef`.
4. Read the same bytes with `evidence` withholding the definition.

Tag the tests `#[trace("TC-884", "FR-354-AC-5")]`.

## Expected Results

- Step 1: equal checked package identities.
- Step 2: equal checked package identities.
- Step 3: the package is admitted.
- Step 4: `unknown_required_capability` with no cause, never `unknown_required_feature`, naming the `Audit` node's `node_id`, at `Locus::Artifact` with the package's artifact identity and the pointer `/semantic_graph/nodes/{i}/body/definition`; the bytes and `package_id` are those of step 3.
