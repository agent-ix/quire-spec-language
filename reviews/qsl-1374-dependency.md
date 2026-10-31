---
id: SR-1957012
title: "Dependency review of QSL-1374"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; QSL-1374"
review_set: all
---

## Summary

Ticket: QSL-1374; PR 1959. FR-390 already depends on FR-106, and the new provenance seed uses FR-106 checks 7 and 8. FR-401 keeps this admission work on the QSL side of the evaluator leaf boundary. No new dependency edge or cycle is introduced.

## Verdict

**PASS** — The existing FR-390 → FR-106 prerequisite and FR-401 component allocation preserve logical ordering.

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
