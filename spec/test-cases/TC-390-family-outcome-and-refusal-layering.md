---
id: TC-390
title: "FamilyOutcome, FamilyResult and EvalOutcome live once in quire-eval, no QSL layer below 5 names them, and the check core names no family cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-390: FamilyOutcome, FamilyResult and EvalOutcome live once in quire-eval, no QSL layer below 5 names them, and the check core names no family cause

## Description

Verify FR-090-AC-9. `FamilyOutcome`, `FamilyResult` and `EvalOutcome` are
each defined once, in the layer-E crate `quire-eval` (ADR-011 §6.1,
FR-401), and no QSL crate defines any of them. Layer 1 (`qsl-cst`), layer 2
`forms`, layer 3 `qsl-semantics` and layer 4 `qsl-package` never reach any
of the three. K (`quire-exact`) and SV (`quire-semantic-value`) cannot reach
them, because of the crate DAG. The `check` core names no family cause type: it holds the
`ProtocolClause` snapshot cause, `ModelRefusal` and the `StateModel`
undefined cause only through `CatalogCoded` and `UndefinedCoded` (ADR-013
O-16, O-17). Scope: FR-090-AC-9.

The test uses the same resolved-import and definition-scan approach as
TC-256, TC-170 and TC-176 (`xtask/src/import_graph.rs`,
`xtask/src/definition_scan.rs`). A `use` scan alone is not enough, because a
fully-qualified inline path such as `crate::family::FamilyResult` with no
`use` line would pass it.

This catches five faults: a second definition, such as a QSL copy under
`value::expression` or `family`; a `qsl-semantics` module that imports the
family outcome to report a query result, which makes the checker depend on
the evaluation leaf; F growing an evaluation-aware category map; the kernel
gaining a family variant; and a `check`-core item that names a family cause
type, which turns `FamilyResult` back into a shared cause list.

## Test Procedure

1. Scan every `.rs` file of every member of the QSL workspace (each
   `members` entry of the root `Cargo.toml`, the root crate's `src/`
   included) for an item definition named `FamilyOutcome`,
   `FamilyResult` or `EvalOutcome` (`enum`, `struct` or
   `type`). Use the `syn`-based definition scan the repository already has
   (`xtask/src/definition_scan.rs`).
2. Resolve every `use` edge and every inline path under `qsl-cst/src/`,
   `qsl-forms/src/`, `qsl-semantics/src/` and `qsl-package/src/`, and check
   whether any of them names one of the three types.
3. Resolve every `use` edge and every inline path under the `check` core, and
   check whether any of them names the `ProtocolClause` snapshot cause type,
   `ModelRefusal` or the `StateModel` undefined cause type.
4. Read every dependency table of `qsl-foundation/Cargo.toml`,
   `qsl-cst/Cargo.toml`, `qsl-forms/Cargo.toml`, `qsl-semantics/Cargo.toml`
   and `qsl-package/Cargo.toml`, and the `[dependencies]` table of
   `qsl-eval/Cargo.toml`.

Tag the test `#[trace("FR-090-AC-9", "TC-390")]`.

## Expected Results

- Step 1 finds no definition of any of the three types in any workspace
  member. (That each resolves at its `quire_eval::` path is FR-401-AC-3.)
- Step 2 finds no edge.
- Step 3 finds no edge.
- Step 4 finds that none of the five layer F to 4 manifests names
  `quire-eval` in any table. `qsl-foundation` may name `quire-exact` and
  `quire-semantic-value`, and `qsl-cst` may name `qsl-foundation` and
  `quire-exact` (ADR-011 §6.1). `qsl-package`'s `[dependencies]` name
  `qsl-semantics` and no workspace crate outside `qsl-semantics`,
  `qsl-foundation` and `quire-exact`, and `quire-contract-model` as their one
  ecosystem crate outside the workspace, plus `quire-checked` once FR-400's
  X-13 lands; its `[dev-dependencies]` may also name `qsl-forms`.
  `qsl-eval`'s `[dependencies]` are exactly `qsl-foundation`, `qsl-package`,
  `qsl-semantics`, `quire-canonical`, `quire-exact`, `serde` and `thiserror`
  (ADR-011 X-8, FR-101), plus `quire-checked` once X-13 lands and
  `quire-eval` once X-14 lands; it has no `[build-dependencies]`, and its
  `[dev-dependencies]` may also name `qsl-forms` and `qsl-cst`, and no other
  workspace crate.
