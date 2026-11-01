---
id: SR-1957028
title: "Failure-domain review — FR-050 checked successors"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; FR-050 checked source selection, authenticated progress and TC-150 at PR #1973 head 95ec6998923cbb58fb831a1422f99505390ad6d9; disposition round 4 reviewed at b365e72bcc6de760b36b40bcdabc35f1699c11ec"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
---

## Summary

The checked adapter specifies strict selection and refusal behavior, and derives formula/activation from S3. Authenticated progress reuses a key that does not carry the selected checked clause identity, leaving a cross-package identity-confusion case.

## Verdict

**FAIL** — distinct checked clauses can alias in authenticated progress retention.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `AuthenticatedBinding` contains only the admitted declaration index, definition identity/revision, clock name, and parameters. If one `Ledger` is reused for two checked packages with the same index/definition/clock but different checked source locus, the retained progress key aliases them; a later watermark/closure for one clause can refuse or settle the other. Carry the checked clause or exact source-locus identity into this ledger key. | src/temporal/progress.rs:23-41, src/temporal.rs:208-223, src/temporal.rs:482-547 |

## Coverage

- Extension failure: missing/ambiguous selection and profile/clock mismatches are typed refusals before position work.
- Identity: source authority/identity, declaration name and span are specified; progress-key identity is weaker than the checked selection identity.
- Purity: checked formula and activation remain the evaluation authority; the admitted artifact supplies selection/clock evidence only.
- Topology: the checked graph traversal is bounded by admitted formula/node structures; no cycle-specific requirement was added.

## Dispositions

Round 4, reviewed at head b365e72bcc6de760b36b40bcdabc35f1699c11ec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The progress identity combines checked node/source and authenticated binding; the regression test proves independent activation capture values. after_excerpt: `pub(super) struct CheckedAuthenticatedBinding {     checked: CheckedBinding,     admitted: AuthenticatedBinding, }` |

## Final disposition

**PASS — clean at `b365e72bcc6de760b36b40bcdabc35f1699c11ec`.** Round 4 verified all findings in the pass-3 record as fixed and found no new findings. The original `## Verdict` and `## Findings` sections above preserve the pass-3 results; the round-4 outcomes are recorded in `## Dispositions`.
