---
id: SR-1957014
title: "Risk and complexity review of QSL-1374"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; QSL-1374"
review_set: all
---

## Summary

Ticket: QSL-1374; PR 1959. The provenance wrappers and exact declaration association are technically high-risk, low-volatility interface work. The new criterion provides broad path, model, completeness, dangling-reference, record-preservation, and meter scenarios; no additional mitigation gap was found in this slice.

## Verdict

**PASS** — High technical risk is mitigated by explicit adversarial cases and reuse of FR-106 checks.

## Review Policy

No applicable AssuranceProfile with an enforced review policy was found in `spec/`; this review used the direct review path.

## Examined Scope

```yaml
[
  {
    "id": "FR-106-AC-65",
    "path": "spec/functional/FR-106-admit-snapshots-and-invocations.md",
    "role": "examined",
    "excerpt": "For FR-390's reference-occurrence seeds, two same-universe, same-effective-type populations retain distinct checked declaration nodes, including when one is empty or unlisted."
  },
  {
    "id": "FR-390",
    "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
    "role": "examined",
    "excerpt": "Each occurrence pairs a path rooted at that binder, expressed with quire_semantic_value::admission::PathStep, with the exact checked population declaration NodeKey that the caller used to construct that reference."
  },
  {
    "id": "FR-390-AC-23",
    "path": "spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md",
    "role": "examined",
    "excerpt": "The TC-1070 E supply carries one exact population declaration for every root and nested reference occurrence in a supplied source, including record, present-option and sequence paths."
  },
  {
    "id": "FR-401",
    "path": "spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md",
    "role": "examined",
    "excerpt": "FR-390's ProtocolSuppliedValue and ProtocolReferenceOccurrence are QSL layer-5 admission inputs, not FR-262 evaluator-interface types. QSL SHALL validate each occurrence against the checked slot and admitted observation before starting S6a."
  },
  {
    "id": "TC-1070",
    "path": "spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md",
    "role": "context_only",
    "excerpt": "the record value types Receipt{total: Total}, Attempt{tally: Tally}, Flag{ready: Boolean} and Tick{ready: Boolean, count: Int[0, 9]};"
  }
]
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Risk Register

| Requirement | Technical risk | Volatility | Driver | Mitigation |
| --- | --- | --- | --- | --- |
| FR-106 | High | Low | Exact provenance identity must survive admission and recursive closure | AC-65 exercises empty/unlisted declarations, recursive closure, error ordering, and record preservation |
| FR-390 | High | Low | Public wrapper/path validation and model-specific population attribution | AC-23 enumerates malformed paths, overlapping populations, ghost references, and unchanged meter |
| FR-401 | Medium | Low | Cross-crate dependency boundary | New boundary statement and existing dependency criteria constrain ownership |

Top hazard: FR-390 supply validation and FR-106 closure must consume the same exact declaration identity.
