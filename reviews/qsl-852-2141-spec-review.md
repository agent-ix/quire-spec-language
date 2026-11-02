---
id: SR-202611022032
title: "Spec review of PR #603: protocol_locations is application-only (FR-322-AC-310, QSL-852)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-specification@87ce9e02; git diff origin/main...HEAD (PR #603): FR-322 protocol_locations paragraph, FR-322-AC-310, TC-233, vector PL-R3-leaf-row, tests/checked_package_v2.rs; ticket QSL-852"
review_set: subset
date: '2026-11-02'
relationships:
- target: ix://agent-ix/quire-specification/FR-322
  type: reviews
---

## Summary

Ticket: QSL-852. The PR adds the application-only rule to FR-322's protocol_locations paragraph, FR-322-AC-310 (TC-233), vector PL-R3-leaf-row and a test arm. The rule sits in the right clause: FR-340 is the frame body and does not mention protocol_locations. No id collisions with main or open PRs (AC-310 free, PL-R3 free). `cargo test --test checked_package_v2` passes (162/162). `quire validate` output for FR-322 and TC-233 is identical on main and the PR (exit 3 both, same diagnostics). Mutation check: changing the vector path from literal to application, or its expected_code, fails the test arm, so the arm is not vacuous against vector edits; but it exercises no reader.

## Verdict

PASS with findings: no high findings; mergeable. FND-001 and FND-002 are worth fixing forward.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-322 protocol_locations paragraph | spec/objects/interfaces/FR-322-checked-package-artifact.md:1099-1131 | examined |
| FR-322-AC-310 | spec/objects/interfaces/FR-322-checked-package-artifact.md:4108 | examined |
| FR-322-AC-291 | spec/objects/interfaces/FR-322-checked-package-artifact.md:4081 | context_only |
| PL-R3-leaf-row | proposals/checked-package-v2/checked-term-data-vectors.json:2520 | examined |
| TC-233 | spec/test-cases/TC-233-published-checked-package-and-scalar-contract-vectors.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-310 is backed only by a vector self-consistency check; no reader is run, so no implementation can fail it. | tests/checked_package_v2.rs:20586 |
| FND-002 | medium | PL-R3-leaf-row defines no node graph; the leaf is only a path-segment name, so the refusal is indistinguishable from AC-291's absent-node refusal. | proposals/checked-package-v2/checked-term-data-vectors.json:2520 |
| FND-003 | low | AC-310 is compound, overlaps AC-291, and exercises only literal, not Local or reference. | spec/objects/interfaces/FR-322-checked-package-artifact.md:4108 |

## Open decision

Code and locus invalid_package/invalid-value at the row's application member match EQ-R1 and PC-R2; FR-322-AC-291 records none for the non-application case. Not a defect; the planner should confirm the choice.
