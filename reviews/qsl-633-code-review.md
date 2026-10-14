---
id: SR-7836
title: "Code and Rust review — QSL-633 PR #786 checked-IR imports"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@1b37afac2d336455acc9ced3a77fbab25914173a; QSL-633; FR-068-AC-6; qsl-semantics/src/check/{ir.rs,mod.rs,state_clause.rs}; xtask/src/import_graph.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-068, type: references }
---

## Summary

Reviewed public PR #786 at exact head `1b37afac2d336455acc9ced3a77fbab25914173a`. The three imports now use the ADR-011 §6.1 `value::definition` layer directly, and the existing real-tree TC-175 test now asserts the no-flat-aggregate rule; static review found no defect. No build was run during this reviewer pass; the exact-head gate is separately queued.

## Verdict

PASS — the changed import paths comply with FR-068-AC-6 and the architecture boundary, and the added assertion checks the intended regression.

## Examined Scope

```yaml
scope:
  - id: FR-068-AC-6
    path: spec/functional/FR-068-split-expression-checking-into-check-stage.md
    role: examined
    excerpt: "Every shipped import under qsl-semantics/src/check resolves into a permitted module; every value import names its submodule, never value's flat aggregate."
  - id: ADR-011-6.1
    path: spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md
    role: examined
    excerpt: "Layer 3 orders semantic_value before model, library, and check core; definition is a semantic_value module."
  - id: qsl-semantics/src/check/ir.rs
    path: qsl-semantics/src/check/ir.rs
    role: examined
    excerpt: "selection: Option<crate::value::definition::AdmittedIntegerDivision>"
  - id: qsl-semantics/src/check/mod.rs
    path: qsl-semantics/src/check/mod.rs
    role: examined
    excerpt: "DefinitionLock::pinned() and CatalogRole::Root use crate::value::definition."
  - id: qsl-semantics/src/check/state_clause.rs
    path: qsl-semantics/src/check/state_clause.rs
    role: examined
    excerpt: "DefinitionLock::pinned() and CatalogRole::Root use crate::value::definition."
  - id: xtask/src/import_graph.rs
    path: xtask/src/import_graph.rs
    role: examined
    excerpt: "real_check_layer_edges_have_no_violation rejects a flat value:: aggregate edge before checking every edge's layer classification."
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Notes

- Rust review covered layer correctness, visibility, test traceability, assertion strength, determinism, and panic/resource concerns in the changed hunks. The new assertion is attached to `TC-175` / `FR-068-AC-6`; the existing flat-use fixture independently checks that a flat import resolves as a violation.
- `cargo fmt --check`, clippy, and tests were not run by this reviewer; their exact-head gate remains separate and its results are not inferred here.
