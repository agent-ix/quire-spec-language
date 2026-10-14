---
id: SR-7823
title: "Failure-domain review — QSL-726 dispatch input contract"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@44c01d7a546fa237cbb5a0038c69ad0ce0f37cc3; FR-102/FR-103/FR-302 source identity, join, refusal, and dispatch-table atomicity; QSpec FR-151"
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

The DeclarationKey join avoids confusing source-local labels, declaration order and display spellings, and the refusal path prevents partial checked output. Identity collision handling is defined, but its duplicate-body policy is not supported by the upstream dispatch contract and changes which inputs can proceed.

## Verdict

**CONDITIONAL** — the failure boundary is explicit, but the duplicate-body refusal needs an authorized semantic rule.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The identity key is explicit, but rejecting its duplicate source declarations is a newly chosen failure outcome: FR-151 does not declare source uniqueness or require this refusal. | FR-302:65-68; QSpec FR-151:122-147 |

## Failure-domain checks

Entity identity is explicit: the selected model's effective operation DeclarationKey anchors the body, signature and clauses. No extension callback or user-supplied predicate is introduced. The family walk's existing bound is unchanged. Missing bodies are assigned `has_body = false` and excluded as candidates; failed links produce no checked package or partial table. The duplicate identity case is refused, but that outcome is a new policy decision rather than a requirement inherited from FR-151.

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
| FND-002 | fixed | a2ce1566; duplicate identity collision now has an authorized deterministic refusal, with TC-987 checked against TC-986's single-body control. |

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7823 round=2 reviewed=a2ce1566aac862e715e976252e6735a4619b3e4f model=gpt-6 run=911ea91d-376b-446f-bcea-45e412c4c9bf -->

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
    reason: "QSL-727 authorizes the deterministic duplicate refusal. TC-987 verifies atomic refusal; TC-986 verifies the single-body source-join path and domain/unit distinction."
```

+++
