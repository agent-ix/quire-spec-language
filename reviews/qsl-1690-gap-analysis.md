---
id: SR-20261105101037
title: "Gap analysis of QSL-1690 model-named unit emission"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1690; qsl-semantics/src/check/lowering/model.rs, qsl-semantics/src/check/mod.rs, tests/it/checked_handoff.rs, FR-094, FR-517, TC-1312"
review_set: subset
---

## Summary

Reviewed frozen PR #2423, including the changed Rust, owning requirements and acceptance evidence. No defects found in the assigned change.

## Verdict

**PASS** for the reviewed QSL-1690 change. The active AC18 integration test exercises the real writer and strict IR read-back for both a root unit and a derived unit, verifies the dimension and target dependencies, source ownership and exclusion of an unrelated unit. The old missing-unit refusal is removed; the alias-name and quantity-sum refusal assertions remain. No new stub, forged identity or mocked semantic boundary was found. Plan completion: not assessed.

## Scope

- FR-094 (examined), spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md: Units named by a selected model's type declarations SHALL be roots of the
checked graph even when no checked expression names them. This includes a
unit named by an alias, a collection, option or map element, a field, tuple
position, variant payload, operation signature or Rational scalar. The root
uses the admitted unit identity and the ordinary unit-node walk below, so its
dimension, base dimensions and target-unit closure retain their existing
identity, owner, scale, offset and declaration occurrences. Unused source
units that no selected model type names do not become roots by this rule.
- FR-517-AC-18 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: A measurement unit that the selected model's types name is part of the emitted package even when no expression of the compiled source reaches it: the package holds its `dimension` and `unit` nodes, and Contract IR's reader admits a package whose model `typeRef` names it. Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, a source that reads no unit-typed member is written and admitted.
- FR-517-AC-16 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, a source whose `pre` compares the `Amount` parameter with `1 U` is written and admitted. Two refusals remain the current behavior of the spine, each of which holds until the spine delivers the capability of the named criterion and ends when it does: `sum<M::Total>` refuses `UnresolvedTypeName` for `M::Total` (until FR-517-AC-19); and `sum<Cash>` over the receipts, with `type Cash = Quantity<U>;`, refuses `IllTyped(TypeMismatch)` at its state clause (until FR-517-AC-20). The change that delivers one of those criteri
- TC-1312 (examined), spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md: | 18 | 18 | delivered model-unit emission: over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, write a source that reads no unit-typed member | the package holds the `dimension` and `unit` nodes; the reader admits it |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

Review was read-only over code. Author logs inspected: focused Clippy, formatting, changed-spec validation, AC18 (1 passed) and remaining AC16 refusals (1 passed). The reviewer ran no Cargo builds, tests or final gate, as required by the dispatch. The code lead owns the one final scoped gate. There are no applicable active AssuranceProfile documents in this scope. No optional semantic-review campaign or full-repository readiness claim is made.

Plan completion: not assessed

The repository-wide `quire matrix --scope . --format json` scan was attempted but had not produced output after more than two minutes. Direct trace-tag and test-body inspection confirms active FR-517-AC-18 and retained FR-517-AC-16 bindings. Computed-matrix evidence is unavailable for this review; this is a scoped manual traceability assessment, not a repository-wide matrix PASS. Unrelated repository-wide debt is not reassessed as this ticket's completion.
