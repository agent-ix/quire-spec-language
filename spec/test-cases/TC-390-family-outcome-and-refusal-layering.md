---
id: TC-390
title: "FamilyOutcome, FamilyResult and EvalOutcome live once in the check core, no lower layer names them, and the check core names no family cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-390: FamilyOutcome, FamilyResult and EvalOutcome live once in the check core, no lower layer names them, and the check core names no family cause

## Description

Verify FR-090-AC-9. `FamilyOutcome`, `FamilyResult` and `EvalOutcome` are
each defined once, in the layer-3 `check` core (ADR-011
§6.1). Layer 1 (`qsl-cst`), layer 2 `forms` and the layer-3 modules below the
`check` core (`semantic_value`, `model`, `library`) never reach any of the
three. K (`quire-exact`) and F (`qsl-foundation`) cannot reach them, because
of the crate DAG. The `check` core names no family cause type: it holds the
`ProtocolClause` snapshot cause, `ModelRefusal` and the `StateModel`
undefined cause only through `CatalogCoded` and `UndefinedCoded` (ADR-013
O-16, O-17). Scope: FR-090-AC-9.

The test uses the same resolved-import and definition-scan approach as
TC-256, TC-170 and TC-176 (`xtask/src/import_graph.rs`,
`xtask/src/definition_scan.rs`). A `use` scan alone is not enough, because a
fully-qualified inline path such as `crate::family::FamilyResult` with no
`use` line would pass it.

This catches five faults: a second definition, such as a copy under
`value::expression`; a `model` or `library` module that imports the family
outcome to report a query result, which is an upward edge inside layer 3;
F growing a `check`-core-aware category map; the kernel gaining a
family variant; and a `check`-core item that names a family cause type,
which turns `FamilyResult` back into a shared cause list.

## Test Procedure

1. Scan every `.rs` file under `src/`, `quire-exact/src/`,
   `qsl-foundation/src/`, `qsl-cst/src/`, `qsl-forms/src/`,
   `qsl-semantics/src/`, `qsl-package/src/` and `qsl-eval/src/` for an item definition named `FamilyOutcome`,
   `FamilyResult` or `EvalOutcome` (`enum`, `struct` or
   `type`). Use the `syn`-based definition scan the repository already has
   (`xtask/src/definition_scan.rs`).
2. Resolve every `use` edge and every inline path under `qsl-forms/src/`,
   `qsl-semantics/src/model/`, `qsl-semantics/src/library/`, the `semantic_value` modules
   (`src/value/{definition, enumeration, unit, quantity, key, reference}`,
   ADR-011 §6.2), `qsl-semantics/src/value/model_query.rs` (layer 3 `model`) and
   `qsl-cst/src/`, and check whether any of them names one of the three
   types. `src/value/outcome.rs` (layer K) left this list under QSL-131 O2:
   its module was deleted, and every caller now imports
   `quire_exact::{Outcome, Refusal, Undefined}` directly.
3. Resolve every `use` edge and every inline path under the `check` core, and
   check whether any of them names the `ProtocolClause` snapshot cause type,
   `ModelRefusal` or the `StateModel` undefined cause type.
4. Read the `[dependencies]` tables of `quire-exact/Cargo.toml`,
   `qsl-foundation/Cargo.toml` and `qsl-cst/Cargo.toml`, and both dependency
   tables of `qsl-package/Cargo.toml` (layer 4, QSL-182) and
   `qsl-eval/Cargo.toml` (layer 5, QSL-183).

Tag the test `#[trace("FR-090-AC-9", "TC-390")]`.

## Expected Results

- Step 1 finds exactly one definition of each type, all in the `check`
  core. Today that is `qsl-semantics/src/family/`, which its own module doc names as
  ADR-012 §13.1's check core.
- Step 2 finds no edge.
- Step 3 finds no edge.
- Step 4 finds that none of the three manifests names the crate that defines
  the three types, or any crate at layer 3 or above. `qsl-foundation` may name
  `quire-exact`, and `qsl-cst` may name `qsl-foundation` and `quire-exact`
  (ADR-011 §6.1). `qsl-package`'s `[dependencies]` name `qsl-semantics` and
  no workspace crate outside `qsl-semantics`, `qsl-foundation` and
  `quire-exact`, and `quire-contract-model` as their one ecosystem crate
  outside the workspace; its `[dev-dependencies]` may also name
  `qsl-forms`. `qsl-eval`'s `[dependencies]` are exactly `qsl-attrs`,
  `qsl-foundation`, `qsl-package`, `qsl-semantics`, `quire-exact`, `serde`,
  `serde_json` and `thiserror`, it has no `[build-dependencies]`, and its
  `[dev-dependencies]` may also name `qsl-forms`, and no other workspace
  crate. The root crate names
  `qsl-eval` in none of its tables: no root-crate code calls layer 5 yet.

## Status

`✅ Passed locally`. Backed by the four tests in
`tests/it/family_outcome_layering.rs`, over `xtask::definition_scan::scan_dirs`
and `xtask::import_graph::resolved_paths`.
