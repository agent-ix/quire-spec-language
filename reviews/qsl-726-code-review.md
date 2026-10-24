---
id: SR-7818
title: "Code review — QSL-726 dispatch input specification"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@44c01d7a546fa237cbb5a0038c69ad0ce0f37cc3; PR #760 diff against origin/main: spec/functional/FR-102-build-state-clause-forms.md, FR-103-admit-model-operations-and-frames-on-the-spine.md, FR-302-link-dispatch-during-unit-compile.md; ticket QSL-726"
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

Reviewed the three changed requirements against QSL-726, QSpec FR-151, existing FR-104 resolution rules, and the test index. The unit/domain-package provenance distinction is clear, but AC-7 lacks a concrete test binding and the duplicate-body refusal is an unsupported behavior addition.

## Verdict

**FAIL** — FR-302 adds an unapproved duplicate-body refusal, and the new source-join criterion has no named or indexed test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The duplicate-body case is not required by QSpec FR-151, which only identifies the operation-body source and says body-bearing family members are candidates, or by QSL-726. This adds an S3 refusal policy without an authorized semantic decision. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:65-68 |
| FND-002 | medium | The criterion's verification is only `Test`; it has no TC identifier, and spec/tests.md has no row binding a test to FR-302-AC-7. The identity join, body-presence behavior and missing-body refusal therefore have no declared verification evidence. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:99 |

## Review Notes

The source path and identity join align with the ticket. FR-103's discarded domain-package clause texts remain explicitly separate from unit `StateClauseForm`s and unit `OperationBodyForm`s. No production-code changes were in scope, so implementation behavior could not be checked at source level.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The join keys body expressions by DeclarationKey but leaves duplicate unit bodies resolving to that key without a defined selection or refusal; the resulting dispatch candidate body is ambiguous. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:56-67; spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md:49-52 |

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
| FND-003 | fixed | a2ce1566; canonical QSL-727 authorizes the duplicate-body outcome; TC-986/987 cover the single and duplicate cases. |

<!-- reviewer-dispositions repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-7818 round=2 reviewed=a2ce1566aac862e715e976252e6735a4619b3e4f model=gpt-6 run=911ea91d-376b-446f-bcea-45e412c4c9bf -->

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
    reason: "QSL-727's approved scope explicitly chooses this multiplicity and diagnostic rule; TC-986 is the single-body control and TC-987 covers duplicate resolution."
```

+++
