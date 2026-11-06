---
id: SR-20261105101043
title: "EARS conformance review of QSL-1691 model alias names"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; QSL-1691; qsl-semantics/src/check/assemble.rs, qsl-semantics/tests/it/unit_typed_members.rs, tests/it/checked_handoff.rs, FR-091, FR-517, TC-1312"
review_set: subset
---

## Summary

Reviewed the QSL-1691 change and owning acceptance evidence. No defects found in the scoped change.

## Verdict

**PASS** for the reviewed QSL-1691 change. The targeted engine check reports 3/3 changed documents grammar-clean, with zero grammar findings. The admitted model-alias mapping and retained member-bound clauses name their actor and response concretely; the existing declarative acceptance rows remain verifiable and preserve the explicit pending quantity-sum boundary.

## Scope

- FR-091 (examined), spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md: - a qualified name `M::T` that names an admitted alias type of the model
  selected as `M` SHALL resolve through that alias's admitted target by the
  same model value-type mapping used for model members. Alias chains retain
  their existing cycle refusal and nominal unit identity. The alias's
  `min`/`max` constraints SHALL remain selected-model member bounds
  (FR-230), not be dropped or applied to every value of the underlying unit;
- FR-517-AC-16 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`, a source whose `pre` compares the `Amount` parameter with `1 U` is written and admitted. Until FR-517-AC-20 is delivered, `sum<Cash>` over the receipts, with `type Cash = Quantity<U>;`, refuses `IllTyped(TypeMismatch)` at its state clause. The change that delivers that criterion deletes this refusal from this criterion and from TC-1312 row 16 in the same change.
- FR-517-AC-19 (examined), spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md: A model alias is a source type name: over **R**'s model, `sum<M::Total>` and `capture targetFull: M::Total` resolve to the alias's unit-bearing type, and neither refuses `UnresolvedTypeName`. The capture checks; until AC-20 is delivered, the sum retains its quantity-sum `IllTyped(TypeMismatch)` boundary after name resolution. The alias's original `min`/`max` bounds remain selected-model member constraints and enforce the same endpoint values as before, without widening them or imposing them on other aliases of `U`.
- TC-1312 (examined), spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md: | 19 | 19 | `sum<M::Total>` over the receipts and `capture targetFull: M::Total`; read the selected model's `Amount` and `Total` member bounds and test their endpoints and one value outside each | both names resolve without `UnresolvedTypeName`; the capture checks; the sum still refuses its quantity-family type mismatch until row 20; alias-bound identities and values are retained, endpoints satisfy and outside values refuse, and Amount's bounds do not replace Total's |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Review is limited to frozen PR #2424 against its QSL-1690 stacked prerequisite, not the combined main diff. Source-only review: no Cargo command, build, test or final gate was run by this reviewer. Author logs record focused Clippy and five passing alias controls; the lead owns the final scoped gate. No applicable active AssuranceProfile is declared in this scope. Optional full semantic campaign was not requested.
