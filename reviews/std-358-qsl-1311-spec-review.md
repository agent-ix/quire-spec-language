---
id: SR-16341
title: "spec-review of certificate rejections use FR-331 rule and locus pairs (FR-338, FR-314) (quire-spec-language PR 1311)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@1b8cf710; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md; spec/functional/FR-201-check-a-probability-certificate.md; spec/functional/FR-245-check-a-zone-certificate-in-the-qualified-core.md; spec/functional/FR-338-check-an-en-1-closure-certificate.md; spec/test-cases/TC-1197-certificate-rejections-use-the-fr-331-rule-and-locus-pairs.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1197
    type: references
---

# Spec review of certificate rejections use FR-331 rule and locus pairs (FR-338, FR-314) (quire-spec-language PR 1311)

## Summary

Ticket: STD-358. One spec-review pass over `git diff origin/main...1b8cf710`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- Every rule/locus pair in FR-338-AC-12/13, FR-169-AC-14, FR-201-AC-6 and FR-245-AC-7 checked against QSpec FR-331 Certification table on main: listed pairs are listed, and the AC-13 unlisted ones (BadState at Query, QueryMismatch at ProofStep, ProofStepInvalid at Query, WitnessFails at State) are unlisted there.
- Locus names (Zone node/edge/component, State-graph entry, Probabilistic product state, Certificate member) match FR-331. The former open items O1/O2 are specified; no deferral language remains.
- TC-1197 exists on one other head only because #1346 (STD-379) is stacked on this PR; FR-169-AC-13 is taken by open #1327, so AC-14 is free.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

Mergeable after origin/main is merged: FR-338 conflicts with main (and with open #1354, #1356, #1377; FR-163/FR-169 with #1327). No defect in the delta. #1346 contains this PR.
