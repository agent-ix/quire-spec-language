---
id: SR-20261104025138
title: QSL-1533 PR #2250 scoped gap analysis
type: SpecReview
analysis: gap-analysis
date: '2026-11-04'
scope: 'agent-ix/quire-spec-language; QSL-1533 PR #2250 at 4676b710d60b0469654fb14aeab3a24bca63bab8; FR-514-AC-1..5, FR-480-AC-6 and FR-091-AC-46'
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-514
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: references
---

## Summary

Ticket: QSL-1533. PR: agent-ix/quire-spec-language#2250. This is a manual, scoped acceptance-to-test and reverse-gap check because the installed Quoin CLI does not provide the documented `gap-analysis` command. The changed behavior has owning requirements in FR-514-AC-5 and FR-480-AC-6; no added code path was found without a requirement, and the amended cycle tests are tagged to those criteria and TC-1309.

The changed test asserts the two competing FR-480-AC-6 list orders, their stable code/cause results, source identity, exact declaration-name byte spans and cycle-edge payloads. The direct assembler ordering test asserts that `[1, 2]` selects the zero-scale refusal and `[2, 1]` selects the self-cycle refusal. Existing FR-514-AC-1..4 tests remain present and tagged, and are outside this PR's implementation delta.

No source/test/spec gap was found in the reviewed scope. The computed matrix could not be obtained: `quoin matrix --repo ... --json` and `quire matrix --scope ... --format tsv` timed out after 20 and 90 seconds respectively. Clippy and tests were not run because main at `810535be` fails workspace compilation on QSL-1621's `Category::Pending` and `Category::Indeterminate` exhaustiveness error. The verdict is conditional on restoring executable gates and obtaining current matrix evidence.

## Verdict

CONDITIONAL: manual scoped review found the required acceptance behaviors represented in the implementation and tagged tests, with no uncovered behavior in the changed paths. The repository matrix, runtime test evidence and Rust gates remain unavailable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed.

The documented `quoin gap-analysis` command is unavailable (`quoin` reports that the command was not found). The static matrix did not complete: Quoin's evidence matrix timed out after 20 seconds and Quire's trace matrix timed out after 90 seconds. Manual check covered FR-514-AC-5, FR-480-AC-6 and FR-091-AC-46 against `qsl-replay/src/spine/lifecycle/tests.rs` and `qsl-semantics/src/check/assemble/tests.rs`. Context-only scope included FR-514-AC-1..4 and their existing tagged type-report/inventory tests. No applicable AssuranceProfile was found. Optional semantic review was skipped; it is not automatic in planless gap analysis.
