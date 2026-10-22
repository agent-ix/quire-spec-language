---
id: SR-13913
title: 'Gap analysis of PR #1188: sampled-witness replay takes the checked subject (STD-250), changed ACs against TC rows'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@7eea6669d; git diff origin/main...7eea6669d (PR #1188, branch std-qsl570-250): FR-193-AC-5 (new), TC-628; ticket STD-250'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-193
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-628
  type: reviews
---

## Summary

Ticket: STD-250. PR: quire-spec-language#1188, head 7eea6669d. Method: gap analysis of the new AC against its TC row, done by hand from the spec and the trace tags. The PR is spec-only, so there was no matrix run and no cargo tests. Plan completion: not assessed. Tooling: quoin 0.28.3, spec-artifacts-process@b82e42e.

| AC | Verification | TC row |
| --- | --- | --- |
| FR-193-AC-5 (new) | Test (TC-628) | TC-628 scope names it, step 4 runs it (refuse `Service` at S4, then repeat the `Service` replays of steps 2 and 3), and expected step 4 gives the FR-185-AC-7 refusal and the outcomes of steps 2 and 3. Every clause of the AC has a step. |

The FR-185 sentence that was edited is a statement with no AC of its own, and FR-193-AC-5 verifies it. No test is tagged FR-193-AC-5 yet. The PR body lists that test and the AC-3 settlement as remaining code work, so this is expected and not a finding. The existing TC-628 tests (qsl-analyze/tests/it/statistical_witness_replay.rs) already replay `Service` witnesses over the in-process checked package with no emission.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-193-AC-5 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:138 | examined |
| TC-628 scope, step 4, expected step 4 | spec/test-cases/TC-628-sampled-witnesses-replay.md:15,24,33 | examined |
| TC-628 tests tagged FR-193-AC-2 and AC-3 | qsl-analyze/tests/it/statistical_witness_replay.rs:111,200,280,375 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The new AC has a TC row that covers all of its clauses. The AC wording issues are recorded in SR-13911 FND-002 and SR-13912 FND-002.
