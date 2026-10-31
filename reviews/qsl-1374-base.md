---
id: SR-1957009
title: "Base checklist review of QSL-1374"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-106-admit-snapshots-and-invocations.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; spec/functional/FR-401-host-the-evaluation-interface-in-the-quire-eval-leaf.md; QSL-1374"
review_set: all
---

## Summary

Ticket: QSL-1374; PR 1959. The three changed requirements retain valid IDs and verification methods. The new FR-390-AC-23 is not executable against its cited TC-1070 E fixture because that fixture has no nested reference field or overlapping candidate populations.

## Verdict

**FAIL** — Resolve the test-fixture mismatch in FR-390-AC-23 before treating the criterion as covered.

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
| FND-001 | medium | FR-390-AC-23 relies on a TC-1070 E fixture that does not define Receipt.peer: Reference<Other>, Other, or the otherNodes/otherSpare populations, so its claimed nested-reference and overlapping-population cases cannot be run against the cited procedure. | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:606 |

## Coverage

The repository-wide `quire matrix --scope . --format tsv` did not return within 120 seconds and was interrupted. No repository-wide matrix result is claimed. The concrete criterion/fixture mismatch is recorded above.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | TC-1070 E-P states that references in target-object fields or relationship ends extend the recursive closure, but the fixture defines no such references and has no row exercising the resulting check-7/check-8 outcome. | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:246 |
| FND-003 | medium | The E-P path mutation matrix tests the valid scalar path forward.total as a non-reference, but no path outside the supplied value (for example, a nonexistent nested field) tests the separate extra-path unknown-member case. | spec/test-cases/TC-1070-evaluate-the-expressions-of-protocol-nodes-from-a-checked-package.md:232 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-390-AC-23 now names the separate E-P fixture and TC-1070 adds E-P with `Receipt.peer`, `Other`, `otherNodes`/`otherSpare`, and concrete root/record/option/sequence occurrence paths and overlap cases. after_excerpt: `TC-1070's E-P procedure carries one exact population declaration for every root and nested reference occurrence in a supplied source, including record, present-option and sequence paths.` |
| FND-002 | still-open | The E-P definitions contain only marker fields on Other and Elsewhere, and retain the existing Node-to-Node Owns relationship. There is no concrete reference field or relationship end in the fixture and no row with a recursive target or expected closure refusal, so the stated closure behavior remains unverified. |
| FND-003 | still-open | The sole added unknown-member mutation uses the existing scalar field path forward.total. The procedure has no occurrence entry on a path that does not exist in the supplied value, so the separate extra-path case remains unverified. |
