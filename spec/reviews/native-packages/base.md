---
id: SR-101
title: "Native package base specification review"
type: SpecReview
analysis: base
scope: "US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and native package wire/API/schema"
review_set: all
evaluated_revision: "41da6e5eb86bb727fbad5370fd23b38d330bcfea"
supplement_evaluated_revision: "2c6b9b83dc5c87c68666ebcd24c41b198f4c339b"
review_date: "2026-09-09"
---

## Summary

Reviewed US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and the
closed wire/API contract. All 27 functional criteria have planned cases; the
five limits and eight integration steps have concrete procedures. The
specification is ready for a bounded implementation plan.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Resolved missed positive-fixture precondition: header-only source cannot reach package construction; minimum positive and adverse cases now follow the adopted grammar. | TC-078; TC-087; TC-090 |
| FND-001 | low | No unresolved base requirement-quality or scoped traceability defect; planned mappings remain distinct from executed evidence. | FR-019; FR-020; FR-021; TM-005 |
| FND-002 | low | Installed classifier expects Status where the catalog requires Coverage Status; manually inspected every new row as Planned and retain the diagnostic. | TM-005 |
| FND-003 | low | Structural JSON parsed successfully and its closed records were inspected; executable Draft 2020-12/schema-counter qualification is explicitly planned, not claimed from Markdown validation. | TC-083 |

## Checklist evidence

The scoped IDs are unique and sequential: FR-019–021, NFR-007, IT-007,
TC-078–091 and TM-005. Each FR traces US-002, which traces StR-001. The story
retains two Given/When/Then examples, user value and high priority. There are
no new product options; PackageSupport and independently lowered limits are
explicit caller choices, with refusal behavior and constraints documented.
Every new relationship target and relative contract/test link was inspected
against its owning artifact. Required source/model inputs, immutable result
types, failure codes and dependency roles are concrete.

The six coverage rules have corresponding TM-005 sections: all 27 criteria;
feature/version/binding permutations; zero/exact/lowered/hard limit boundaries;
wire/native failure paths; build/read atomic stages and retries; empty
inventories, Unicode, recursive references and projection substitution.
IT-007 uses real compiler/IR/runtime APIs. A setup failure cannot pass an
adverse assertion. TC type, priority and criterion mapping are stated.

## Automated evidence

Quire 0.31.0 strict scoped validation reports 218/218 documents grammar-clean
and 60/158 property-extractable criteria. The complete outputs are in data/.
Coverage retains 205/205 existing Rust symbols bound, 196/249 backed matrix
rows, no status lies or untracked symbols, and the three historical IT-004
unmatched tags. The added 41 rows are 27 functional mappings plus 14 cases,
all unexecuted. There are 22 classifier diagnostics and six registry
diagnostics; absence of a status lie alone cannot qualify TM-005 because of
the header mismatch. No local/shared module was altered to suppress it.

## Admitted source setup correction — 2026-09-09

Re-reviewed specification 2c6b9b83dc5c87c68666ebcd24c41b198f4c339b using this
installed QUOIN lens, superseding the initial review's header-only positive
fixture assumption at 41da6e5. The original PASS and its missed precondition
remain visible at 69588ad. Escape cause: wrong-requirement.

The 14 cases still map all 27 criteria. TC-078 and TC-090 now require successful actual import/clause setup; TC-087 owns missing-import/missing-clause refusals. All rows remain planned.

Task-016 began under the original completed review gate. Its initial API-red
run is followed by a first implementation run with one passing nonempty case
and three parser setup failures. Further implementation paused for this
specification correction and all-eight re-review; no parser behavior was
relaxed. The initial stdout/stderr is retained in reviews/data/native-packages/.
No claim of executed full package, schema or canonical-vector qualification is
made. PASS to continue against the corrected admitted-source fixture contract.

## Verdict and provenance

PASS for planning and implementing this producer/reader slice. Agent A applied
the installed QUOIN 0.22.5 skills serially under the owner's existing all-review
selection; no required AssuranceProfile applies. This is the author's recorded
review, not independent B/C acceptance. All package cases remain planned.
Changes to the reviewed requirements or interface reopen specify/spec-review.
Interchange, backend qualification and integration remain required. No Cargo build, additional agent or hosted CI dispatch ran.
