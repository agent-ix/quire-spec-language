---
id: SR-103
title: "Native package integrity review"
type: SpecReview
analysis: integrity
scope: "US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and native package wire/API/schema"
review_set: all
evaluated_revision: "41da6e5eb86bb727fbad5370fd23b38d330bcfea"
review_date: "2026-09-09"
---

## Summary

Checked completeness, consistency, hidden assumptions and atomic obligations.
The three FRs separate static identity, artifact construction and verified
reconstruction, with one common representation contract and explicit limits.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Corrected canonical control-escape spelling from ambiguous double backslash at 9a80805 to the exact single JSON escape at the evaluated revision. | FR-021-AC-1; docs/native-linked-packages.md |
| FND-002 | low | Draft accounting overclaim repaired: only actually exposed upstream usage is retained; package passes record their own measured work and absent passes explicitly. | NFR-007; FR-020-AC-9 |
| FND-003 | low | Source-bound static identity, exact artifact identity and projection metadata now have consistent inclusion/exclusion rules across requirements, schema, cases and matrix. | FR-019-AC-9; FR-021 |

## Completeness matrix

| Story | Functional requirement | Stakeholder path | Verification |
| --- | --- | --- | --- |
| US-002 | FR-021 native static derivation | US-002 → StR-001 | TC-082/088/090/091 |
| US-002 | FR-019 complete checked artifact | US-002 → StR-001 | TC-078–082/088/089 |
| US-002 | FR-020 exact read/rebind | US-002 → StR-001 | TC-081–089/091 |

NFR-007 constrains all three FRs and all five metrics map to TC-088. IT-007
covers real integration and complete observations; no missing code is called
qualified. FR-019 emits, FR-021 derives identity, FR-020 verifies/reconstructs.
Their acceptance criteria decompose inventories and refusal variations, rather
than introducing unrelated user behaviors.

## Hidden-assumption probes

The production API calls no external CLI, authenticated/paginated API, registry
lookup or scaffolder. There is no unspecified retry/concurrency policy.
Dependency selection uses explicitly supplied immutable inventories; native
linking refuses conflicts rather than first-wins. Missing source/model meaning
has no fallback. Existing IR BoundPackage is retained for later actual lowering,
and the source-bound proof view explicitly does not impersonate it.

The two adopted definition digests are independently named. A changed IR or
definition selection is unknown_profile. Required fields and nulls are closed;
unknown selector strings reach selection instead of being prematurely rejected
as Deserialize enum failures. Canonical members use the specified typed order,
including exact u64 encoding. Canonical self-reference is excluded before
hashing. Semantic pins include all selected native model content, even unused
declarations. Projection metadata is verified despite canonical exclusion.

The schema is structural data and deliberately cannot prove duplicate JSON
member rejection, lexical integer spelling or source/constructor coherence.
TC-083 explicitly separates those reader assertions from planned Rust schema
qualification. Generic numeric recognition is not canonicalization.

## Verdict and provenance

PASS for planning and implementing this producer/reader slice. Agent A applied
the installed QUOIN 0.22.5 skills serially under the owner's existing all-review
selection; no required AssuranceProfile applies. This is the author's recorded
review, not independent B/C acceptance. All package cases remain planned.
Changes to the reviewed requirements or interface reopen specify/spec-review.
Full LC02/FS05 interchange, LC04 backend qualification and LC05 Quire integration
remain required. No Cargo build, additional agent or hosted CI dispatch ran.
