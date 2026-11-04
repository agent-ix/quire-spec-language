---
id: SR-20261104153719
title: 'Gap analysis of PR #2300: QSL-1645 xtask lib scan drift'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@6c7714f17; PR #2300 diff: FR-270-AC-1, FR-270-AC-3, FR-063-AC-7, FR-507, TC-244, TC-260; ticket QSL-1645'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1645. PR: quire-spec-language#2300. Planless gap analysis scoped to the diff. It checks that every changed behaviour has a test tagged to its criterion. Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-270-AC-3 (new addressing plants) | xtask/src/checked_input.rs:1184-1192 | examined |
| FR-270-AC-1 (contract-model package plant) | xtask/src/checked_input.rs:1060-1075 | examined |
| FR-063-AC-7 closed enums and wildcard deny | xtask/src/seam_probe.rs:1605-1648 | examined |
| FR-507 applicability matrix test | qsl-semantics/src/check/capability.rs:330-378 | examined |
| TC-244 stage-constructor table | xtask/src/typestate_scan.rs:714-797 | examined |
| TC-260 raw-wire allowance | xtask/src/typestate_scan.rs:1056-1094 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The exempt and non-exempt boundary of the FR-270 carve-out has no failing test. A plant that mixes an exempt name with a node-bearing `quire-contract-model` type, expecting one finding, is missing (same gap as the code-review and spec-review findings, recorded here as a test gap). | xtask/src/checked_input.rs:1184-1192 |

## Verdict

Mergeable. Every changed behaviour has a tagged test. The FR-507 table is pinned by the full-matrix test. The relocated denies and seam location are pinned by the FR-063-AC-7 seam_probe tests. TC-244 and TC-260 are pinned by their own scans. The carve-out is pinned by FR-270-AC-3 plants. No stubs. No test or check was deleted. The `NodeKind` row moved to the cargo-metadata dependency read, and it was not dropped.
