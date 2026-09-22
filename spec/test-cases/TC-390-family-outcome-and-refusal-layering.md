---
id: TC-390
title: "FamilyOutcome and FamilyRefusal live once in the check core and no lower layer names them"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-390: FamilyOutcome and FamilyRefusal live once in the check core and no lower layer names them

## Description

Verify FR-090-AC-9. `FamilyOutcome` and `FamilyRefusal` are each defined
once, in the layer-3 `check` core (ADR-011 §6.1). Layer 2 `forms` and the
layer-3 modules below the `check` core (`model`, `library`) never reach
either type. K (`quire-exact`) and F (`qsl-foundation`) cannot reach them,
because of the crate DAG. Scope: FR-090-AC-9.

The test uses the same resolved-import approach as TC-256 and TC-176
(`xtask/src/import_graph.rs`). A `use` scan alone is not enough, because a
fully-qualified inline path such as `crate::family::FamilyRefusal` with no
`use` line would pass it.

This catches four faults: a second definition, such as a copy under
`value::expression`; a `model` or `library` module that imports the family
outcome to report a query result, which is an upward edge inside layer 3;
F growing a `FamilyRefusal`-aware category map; and the kernel gaining a
family variant.

## Test Procedure

1. Scan every `.rs` file under `src/`, `quire-exact/src/` and
   `qsl-foundation/src/` for an item definition named `FamilyOutcome` or
   `FamilyRefusal` (`enum`, `struct` or `type`). Use the `syn`-based
   definition scan the repository already has (`xtask/src/definition_scan.rs`).
2. Resolve every `use` edge and every inline path under `src/forms/`,
   `src/model/` and `src/library/`, and check whether any of them names
   `FamilyOutcome` or `FamilyRefusal`.
3. Parse `quire-exact/Cargo.toml` and `qsl-foundation/Cargo.toml` and read
   each `[dependencies]` table.

Tag the test `#[trace("FR-090-AC-9", "TC-390")]`.

## Expected Results

- Step 1 finds exactly one definition of each type, both in the `check`
  core. Today that is `src/family/`, which its own module doc names as
  ADR-012 §13.1's check core.
- Step 2 finds no edge.
- Step 3 finds no workspace-member or path dependency in either manifest.
