---
id: TC-458
title: "Spine intake and assembly admit operations and frames"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: verifies
---
# TC-458: Spine intake and assembly admit operations and frames

## Description

Verify that I1 reads operation frames and that the E3 assembler declares
operations, over the ConfigVersion domain package and its mutations.

Scope: FR-103-AC-1 to FR-103-AC-5.

## Test Procedure

The fixture is `examples/config-version/model.semantic-ir.json` (FR-108): a
Semantic IR 2.0.0 document for package `example/config-version` version `1`
with constructs bound to `quire.meaning.model.object-type/v1`,
`quire.meaning.model.value-type/v1` and `quire.meaning.model.population/v1`,
declaring:

- value type `VersionNumber`, integer `0..=1000`;
- object type `ConfigVersion` with field `versionNumber: VersionNumber`
  (required, `[1, 1]`) and field `parent: ConfigVersion` (optional,
  `[1, 1]`);
- population `config_history` over `ConfigVersion`, extent `closed`;
- operation `attemptUpdate` on `ConfigVersion`, no `params`, `returns`
  `ix://quire/native/Boolean`, `frame` `{modifies:
  ["ix://example/config-version/ConfigVersion/versionNumber"], creates: [],
  deletes: []}`.

Admit it through `model::intake::unit::admit_unit` and assemble a unit that
declares `model Config = ...` over it.

1. Admit and assemble the fixture as written.
2. Replace `modifies` with `[".../ConfigVersion/missing"]`; then with
   `[".../ConfigVersion"]`; then set `creates` to
   `[".../ConfigVersion/versionNumber"]`.
3. Add parameter `note` typed `ix://quire/native/Text`; then replace it with
   parameter `delta` typed `VersionNumber`.
4. Add a record value type beside the operation.
5. Admit the fixture twice; then admit it with its operation and population
   records moved to the end of the document.

Tag the tests `#[trace("TC-458", "FR-103-AC-n")]`.

## Expected Results

- Step 1: `Config::ConfigVersion` has `versionNumber: Int[0, 1000]` and
  `parent: Option<Reference<Config::ConfigVersion>>`; operation
  `attemptUpdate` has no parameters, result `Boolean`, and effect
  `modifies == [versionNumber's key]`, `creates == []`, `deletes == []`.
- Step 2: `missing_declaration`/`missing-name` at the entry; then
  `invalid_model_binding`/`malformed-declaration` at the entry, twice. No
  declaration admitted.
- Step 3: `unknown_required_feature`/`unsupported-feature` at the `model`
  declaration; then `delta: Int[0, 1000]`.
- Step 4: `UnsupportedModelMember` naming the record value type's node.
- Step 5: equal `PackageDeclarations` and effect key lists in all three.

## Status

Planned (QSL-273).
