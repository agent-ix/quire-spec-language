---
id: SR-1957025
title: "Base specification review — FR-050 checked successors"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; FR-050 additions, AC-1..8 verification updates and TC-150 at PR #1973 head 95ec6998923cbb58fb831a1422f99505390ad6d9; disposition round 4 reviewed at b365e72bcc6de760b36b40bcdabc35f1699c11ec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-150
    type: reviews
---

## Summary

The new checked `/2` API contract names the checked package, admitted selection, exact source locus, clock binding, and typed refusal boundaries. AC-1..8 now cite TC-150 alongside TC-138, and TC-150 names executable checked-route and writer procedures. Two coverage claims overstate the asserted negative cases.

## Verdict

**FAIL** — AC-5 producer limits and full source-locus rejection are not established by the named successor tests.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-050-AC-5 is mapped to TC-150, but its exact/one-short/retry test measures the strict reader only; the producer-side `native::admit_v2` accounting is not exercised. | FR-050-AC-5, TC-150 step 3 |
| FND-002 | medium | TC-150 asserts a reference refusal only after changing source authority. It does not independently demonstrate that changed source identity, declaration name, and span each fail the exact checked-to-admitted locus join. | FR-050-AC-6, TC-150 step 1 |

## Coverage

- IDs and frontmatter relationships validate; no new artifact ID gap or broken TC reference was found.
- Trace tags name TC-150 for FR-050-AC-1..8. The checked successor uses the actual emitted bytes, strict `/2` reader, checked package and v2 adapter; TC-138 remains referenced as the historical emitted-artifact suite.
- `quire validate --scope .` on FR-050 and TC-150 reported 2/2 documents grammar-clean and 0 grammar findings.

## Dispositions

Round 4, reviewed at head b365e72bcc6de760b36b40bcdabc35f1699c11ec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The named successor now exercises native producer limits and retry in addition to reader bounds. after_excerpt: `let producer = native::admit_v2(proofs, selected, &tables.producer, Limits::default());` |
| FND-002 | fixed | TC-150 now enumerates independent source authority, identity, declaration-name and byte-span substitutions. after_excerpt: `substitute source authority, source identity, declaration name with the same span, and byte span with the same source identity` |

## Final disposition

**PASS — clean at `b365e72bcc6de760b36b40bcdabc35f1699c11ec`.** Round 4 verified all findings in the pass-3 record as fixed and found no new findings. The original `## Verdict` and `## Findings` sections above preserve the pass-3 results; the round-4 outcomes are recorded in `## Dispositions`.
