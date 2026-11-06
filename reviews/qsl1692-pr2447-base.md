---
id: SR-20261105101068
title: "QSL-1692 PR2447 specification checklist review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-093 Sum row, FR-517-AC-16/19/20 and dependencies, TC-1312 rows 16/19/20"
review_set: subset
---

## Summary

Ticket QSL-1692: reviewed the specification changes for numeric-family sum typing, lowering and evaluation. The changed rows remove the former quantity refusal and retain family-specific FR-322 identity, result type and operation mode.

## Verdict

PASS for the specification checklist. Implementation readiness is recorded separately by the code review and its dispositions.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined Scope

- FR-093 Sum row (examined): numeric-family identities integer, rational, decimal, float32, float64 and quantity, result member type and family-specific mode.
- FR-517-AC-16 (examined): model Amount comparison is written and admitted; obsolete sum refusal removed.
- FR-517-AC-19 (examined): model alias source names both check, retaining original alias member-bound authority.
- FR-517-AC-20 (examined): sum checks numeric families with a QSpec FR-145/FR-322 identity, including unit-bearing quantity, and Recipe R totals magnitudes in U.
- TC-1312 rows 16, 19 and 20 (examined): matching acceptance vectors and removal of delivered refusal status.
- FR-517 dependencies and Healthy description (examined): delivered unit/alias/sum behavior and remaining external scope.
- FR-145/FR-322 (context_only): authoritative per-family arithmetic, mode, law and bounded-result obligations are retained by reference rather than copied or weakened.

## Coverage

Changed documents validate with Quire: three of three documents grammar-clean, exit zero; existing module diagnostics are unchanged. No IDs, relationship edges, structural artifacts, dependency pins, hashes or duplicate owning arithmetic contracts are introduced. Rows are concrete and independently testable. No new or changed requirement statement calls for a separate EARS lens; the changes are acceptance-table/status and lowering-table edits. No AssuranceProfile is declared in this repository; no enforced assurance run applies. Plan completion: not assessed.

## Scope Excerpts

```yaml
scope:
- id: FR-517-AC-16
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: Over **R**'s model with `Amount` and `Total` as aliases of `ix://quire/unit/U`,
    a source whose `pre` compares the `Amount` parameter with `1 U` is written and
    admitted.
- id: FR-517-AC-19
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'A model alias is a source type name: over **R**''s model, `sum<M::Total>`
    and `capture targetFull: M::Total` resolve to the alias''s unit-bearing type,
    and neither refuses `UnresolvedTypeName`; both check. The alias''s original `min`/`max`
    bounds remain selected-model member constraints and enforce the same endpoint
    values as before, without widening them or imposing them on other aliases of `U`.'
- id: FR-517-AC-20
  path: spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: '`sum<N>` checks a summand of any numeric family QSpec FR-145 and FR-322
    give a `quire.op.collection.sum` identity for, including unit-bearing `Quantity<u>`,
    and lowers to that family''s identity; over **R**, the `Healthy` invariant''s
    receipt sum over `Quantity<U>` checks and evaluates to the sum of the receipts''
    magnitudes in `U`.'
- id: FR-093
  path: spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md
  role: examined
  excerpt: '| `Query{visit: Sum}` (`sum<N>`) | `collection` | `quire.op.collection.sum.<family>`
    selected from the summand''s numeric family (`integer`, `rational`, `decimal`,
    `float32`, `float64` or `quantity`) | member `type_argument` of `T(node)`; operation
    mode and result type follow that family''s FR-322 definition | `[ref(c), binding{name:
    x, value: ref(summand)}]` |'
- id: TC-1312
  path: spec/test-cases/TC-1312-write-a-checked-package-handoff-directory-from-source-in-process.md
  role: examined
  excerpt: 'Scope: FR-517-AC-1 to FR-517-AC-23.'
```
