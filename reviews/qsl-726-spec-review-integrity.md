---
id: SR-7821
title: "Integrity review — QSL-726 dispatch input contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@44c01d7a546fa237cbb5a0038c69ad0ce0f37cc3; changed FR-102, FR-103, FR-302, FR-302-AC-7 and relevant QSL test-index rows"
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

The changed FRs establish complementary S2 form, S3 assembly, and dispatch-linking responsibilities. Integrity review found an unsupported duplicate-source refusal and an AC with no concrete evidence binding; the FR-103 discard rule itself is consistent with the new unit-source path.

## Verdict

**FAIL** — FR-302's duplicate-body rule has no upstream basis, and FR-302-AC-7 is not linked to a test.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The duplicate-body refusal is not entailed by FR-151 or the ticket, so it adds a new rejection condition without authority. | FR-302:65-68; QSpec FR-151:122-147; QSL-726 | wrong-requirement |
| FND-002 | medium | AC-7 has no named test or test-index row, leaving its joined-source behavior without evidence. | FR-302-AC-7; spec/tests.md:266-282 | correct-requirement-no-evidence |

## Consistency checks

QSL-726 AC-1's distinction is met: FR-102 gives state clauses and operation bodies separate form variants; FR-103 says only domain-package pre/post texts are discarded and separately retains unit form trees/provenance. Existing FR-104 resolves unit pre/post operations through the effective model view. AC-1 through AC-6 retain their existing TC associations; AC-7 does not.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The join keys body expressions by DeclarationKey but leaves duplicate unit bodies resolving to that key without a defined selection or refusal; the resulting dispatch candidate body is ambiguous. | FR-302:56-67; FR-103:49-52 |

## Dispositions

Round 1, reviewed at 42e78b9d51f926db514368b4d2042091f06d5a99.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 42e78b9; after_excerpt: "A body declaration with no matching operation SHALL refuse under FR-104's missing-operation resolution rule at S3 without a checked package or partial dispatch table." |
| FND-002 | fixed | 42e78b9; after_excerpt: "Given `operation-body` declarations for `A.size` and `B.size`, retained expressions and unit pre/post clauses join to admitted operations by resolved `DeclarationKey` even when body labels and declaration order differ; `has_body` is true when an operation has a resolved body and false when it has none. A called operation with no body is not a candidate and its failed family refuses under FR-151 with no checked package or partial table." |
| FND-003 | still-open | FR-103 sets `has_body` true when a retained form resolves, while FR-302 keys the body by DeclarationKey and supplies “the body”; neither defines selection or refusal when multiple unit bodies resolve to the same key. |

Round 2, reviewed at a2ce1566aac862e715e976252e6735a4619b3e4f.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | a2ce1566; duplicate multiplicity/refusal is authorized by QSL-727 and covered by TC-986/987. |

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7821 round=2 reviewed=a2ce1566aac862e715e976252e6735a4619b3e4f model=gpt-6 run=911ea91d-376b-446f-bcea-45e412c4c9bf -->

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | a2ce1566aac862e715e976252e6735a4619b3e4f; after_excerpt: "Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table." |

+++ [reviewer data]

```yaml
reviewed_findings:
  - fnd: FND-003
    artifact_id: FR-302
    path: spec/functional/FR-302-link-dispatch-during-unit-compile.md
    lines: "69-74"
    excerpt: |-
      Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table.
    finding: "The join keys body expressions by DeclarationKey but previously left duplicate unit bodies resolving to that key without a defined selection or refusal."
dispositions:
  - fnd: FND-003
    outcome: fixed
    fix_sha: a2ce1566aac862e715e976252e6735a4619b3e4f
    after_excerpt: |-
      Exactly one executable unit `operation-body` SHALL resolve to each `DeclarationKey`. If a second body form resolves to a key already targeted by another body form, S3 SHALL refuse the compile as `conflicting_declaration`/`duplicate-declaration`, naming both body declarations, and SHALL produce no checked package or partial dispatch table.
    reason: "The rule is approved in canonical QSL-727; FR-302 states it deterministically and named TC-986/987 exercise the normal and collision cases."
```

+++
