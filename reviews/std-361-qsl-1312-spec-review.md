---
id: SR-16342
title: "spec-review of FR-314 Outputs name SmtProofCheck; FR-127 SmtProofOutcome handoff and settle_smt_proof (quire-spec-language PR 1312)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2241124a; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md; spec/functional/FR-314-check-an-smt-proof-certificate.md; spec/test-cases/TC-1199-an-smt-proof-settles-through-the-qsl-replay-handoff.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1199
    type: references
---

# Spec review of FR-314 Outputs name SmtProofCheck; FR-127 SmtProofOutcome handoff and settle_smt_proof (quire-spec-language PR 1312)

## Summary

Ticket: STD-361. One spec-review pass over `git diff origin/main...2241124a`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- FR-314-AC-6 last vector derived by hand: `c.value * c.value` is a product of two non-literal terms, so FR-315 row 15 refuses `out-of-fragment`, the rebuild fails, and the checker rejects `QueryMismatch` at `Unrolling`; depth 3 equals the horizon, so main FR-314-AC-8's depth check does not fire first.
- FR-314-AC-6 and FR-127-AC-14 fill ids main leaves free; TC-1199 is on no other open head.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | Main already states the QueryMismatch-on-refusal rule for `too-large` ("When FR-315 refuses an expected query under the recorded limits") and the Fault bullet (STD-340, #1356). The PR adds a broader `Refused { code, locus }` QueryMismatch rule and rewrites the Fault bullet; both conflict textually with main. On merge, keep one rule per case: replace main's `too-large` bullet with the broader one rather than leaving both. | spec/functional/FR-314-check-an-smt-proof-certificate.md | wrong-requirement |

## Verdict

Mergeable after FND-001 is resolved in the merge with origin/main: FR-127 and FR-314 conflict with main and with open #1354/#1356 (FR-314) and #723 (FR-127).

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | fix verified at 1fa2658b |

Round 1 verdict at 1fa2658b: Mergeable: one broader QueryMismatch rule replaces main's `too-large` bullet, one Fault bullet remains; merge-tree against main is clean.
