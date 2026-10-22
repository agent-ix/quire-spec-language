---
id: SR-13932
title: 'Gap analysis of PR #1185: EN-4 manifest owner, mode and evidence kind (STD-251), changed ACs against TC rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@7f855b716; git diff origin/main...7f855b716 (PR #1185, branch std-qsl570-251): FR-280-AC-3 (new), FR-192-AC-4, TC-761, TC-627, TC-767; ticket STD-251'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-280
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-192
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-761
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-627
  type: reviews
---

## Summary

Ticket: STD-251. PR: quire-spec-language#1185, head 7f855b716. Method: gap analysis of the new and changed ACs against their TC rows, done by hand from the spec and the trace tags (spec-only PR, so no matrix run and no cargo tests). Plan completion: not assessed. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

Changed ACs and their TC rows:

| AC | Verification | TC row |
| --- | --- | --- |
| FR-280-AC-3 (new, EN-4 manifest) | Test (TC-761) | TC-761 step 3 and expected result 3: the conversion, admission, the exact pair and `{statistical}`, and the `statistical`/`exact` candidate split. All clauses of the AC are covered. |
| FR-192-AC-4 (changed manifest clause) | Test (TC-627) | TC-627 step 4: "exactly (`probabilistic-satisfaction`, `bounded`) with evidence `statistical`" plus the three refusals. It matches the AC. |

No test in the tree is tagged with the new AC yet. The manifest and the evidence-kind descriptor member do not exist in code (the PR body's remaining code work), so that is expected and not a finding.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-280-AC-3 (new) | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:79 | examined |
| FR-192-AC-4 | spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md:140 | examined |
| TC-761 scope, step 3, expected result 3 | spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md:15,21,29 | examined |
| TC-627 step 4 | spec/test-cases/TC-627-statistical-results-settle-as-measured.md:33 | examined |
| TC-767 scope | spec/test-cases/TC-767-the-direction-check-keeps-frontends-analyze-the-cache-and-the-plugin.md:17 | context_only |
| arch-lint tests tagged FR-280-AC-3 | tools/arch-lint/qualified_core.rs:1079-1111 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Because the new AC shares the id FR-280-AC-3 (SR-13930 FND-001), the two arch-lint tests tagged `#[trace("TC-767", "FR-280-AC-3")]` and `#[trace("TC-767", "FR-284-AC-1", "FR-280-AC-3")]`, which check the cargo-tree direction, already count as coverage of the EN-4 manifest AC. The matrix would report that AC as covered before any TC-761 step 3 test exists. TC-761's "Scope: FR-280-AC-1 to FR-280-AC-3" also takes in the cargo-tree AC that TC-767 verifies. With the new AC renumbered, TC-761's scope should name FR-280-AC-1, FR-280-AC-2 and the new id. | spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md:15; tools/arch-lint/qualified_core.rs:1081,1111 |

## Verdict

Every changed AC has a TC row that covers all its clauses. The one gap comes from the duplicate AC id. The existing cargo-tree tests would inflate coverage of the new EN-4 AC, and TC-761's scope range takes in an AC it does not verify. Renumbering the new AC fixes both.
