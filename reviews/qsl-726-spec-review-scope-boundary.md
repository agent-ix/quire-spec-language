---
id: SR-7822
title: "Scope-boundary review — QSL-726 dispatch input contract"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@44c01d7a546fa237cbb5a0038c69ad0ce0f37cc3; FR-102/FR-103/FR-302 responsibility allocation and interfaces; QSL-726; QSpec FR-151"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---

## Summary

The scope boundary assigns syntax-preserving form construction to S2, selected-model identity resolution and retained data to S3, and dispatch linking to the existing checker bridge. Unit-authored operation bodies/clauses and discarded domain-package clause text remain distinct. One behavior decision exceeds the stated upstream boundary: duplicate bodies are refused without a governing rule.

## Verdict

**CONDITIONAL** — responsibility allocation is coherent, subject to removing or obtaining authority for the duplicate-body refusal.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | S3 is assigned a new duplicate-body uniqueness policy without a corresponding external rule or ticket decision defining that boundary. | FR-302:65-68; QSpec FR-151:122-147 |

## Boundary assessment

S2 preserves labels, target spellings, parameters, expression trees and spans without resolving them. S3 resolves through FR-104 and the selected effective model view by DeclarationKey. Domain-package pre/post text remains an external package input that is shape-checked and discarded; unit state clauses and operation bodies are QSL source inputs retained for checking. The external FR-151 contract is assumed as the source of candidate semantics; no contradictory local contract was found apart from the additional duplicate policy.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | The join keys body expressions by DeclarationKey but leaves duplicate unit bodies resolving to that key without a defined selection or refusal; the resulting dispatch candidate body is ambiguous. | FR-302:56-67; FR-103:49-52 |

## Dispositions

Round 1, reviewed at 42e78b9d51f926db514368b4d2042091f06d5a99.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 42e78b9; after_excerpt: "A body declaration with no matching operation SHALL refuse under FR-104's missing-operation resolution rule at S3 without a checked package or partial dispatch table." |
| FND-002 | still-open | FR-103 sets `has_body` true when a retained form resolves, while FR-302 keys the body by DeclarationKey and supplies “the body”; neither defines selection or refusal when multiple unit bodies resolve to the same key. |

Round 2, reviewed at a2ce1566aac862e715e976252e6735a4619b3e4f.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | a2ce1566; QSL-727 authorizes duplicate rejection; FR-103/FR-302 preserve the domain-package/unit clause boundary. |

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7822 round=2 reviewed=a2ce1566aac862e715e976252e6735a4619b3e4f model=gpt-6 run=911ea91d-376b-446f-bcea-45e412c4c9bf -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | a2ce1566aac862e715e976252e6735a4619b3e4f; after_excerpt: "Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table." |

+++ [reviewer data]

```yaml
reviewed_findings:
  - fnd: FND-002
    artifact_id: FR-302
    path: spec/functional/FR-302-link-dispatch-during-unit-compile.md
    lines: "69-74"
    excerpt: |-
      Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table.
    finding: "The DeclarationKey join previously left duplicate unit bodies ambiguous without a defined boundary outcome."
dispositions:
  - fnd: FND-002
    outcome: fixed
    fix_sha: a2ce1566aac862e715e976252e6735a4619b3e4f
    after_excerpt: |-
      Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table.
    reason: "QSL-727 canonically authorizes the local multiplicity rule. FR-103 limits discard to domain-package pre/post text; unit OperationBodyForm and StateClauseForm remain distinct retained inputs."
```

+++
