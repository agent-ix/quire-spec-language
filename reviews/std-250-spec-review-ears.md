---
id: SR-13912
title: 'EARS analysis of PR #1188: sampled-witness replay takes the checked subject (STD-250)'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@7eea6669d; git diff origin/main...7eea6669d (PR #1188, branch std-qsl570-250): the new and edited SHALL statements and AC of FR-193 and FR-185; ticket STD-250'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-193
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
---

## Summary

Ticket: STD-250. PR: quire-spec-language#1188, head 7eea6669d. `make check-spec-validation` exits 0 and reports no grammar warning on FR-193, FR-185 or TC-628. The new statements describe behaviour only, with no changelog, status or review wording. They were checked by hand against the EARS patterns.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-193 Description "SHALL replay a sampled witness ... over the checked subject" | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:30-35 | examined |
| FR-193 Replay "`replay_sampled_witness` SHALL recompile no package ... It SHALL re-admit" | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:90-97 | examined |
| FR-193 The qualitative claim "FR-128's re-execution, fairness and evaluation steps SHALL replay it" | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:126-128 | examined |
| FR-193-AC-5 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:138 | examined |
| FR-185 "Replay of a sampled witness (FR-193) takes that checked subject and recompiles nothing." | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:216-217 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | In "FR-128's re-execution, fairness and evaluation steps SHALL replay it as one over that same checked subject", the subject of SHALL is a set of steps in another requirement, not the system or an entry. That is the EARS missing-subject form. It should name the replay facade or the entry that replays the counterexample (see SR-13910 FND-001). | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:126-128 |
| FND-002 | low | FR-193-AC-5's closing clause, "each with no recompile and no S4 emission attempted", and TC-628's expected step 4, "with no recompile and no emission attempt", assert an absence that a test cannot observe from the outcome. The observable result is that the replays settle `Reproduced` and `refuted` although `emit_checked` refuses the same package. The clause should state that result, or say what in the replay's input shows it, for example that it takes no package reference or source. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:138; spec/test-cases/TC-628-sampled-witnesses-replay.md:33 |

## Verdict

The replay bullet and the FR-185 sentence are well-formed ubiquitous statements whose rationale is kept in a subordinate clause. One SHALL has no system subject (FND-001). AC-5 ends in an assertion that cannot be observed (FND-002). Both are wording fixes.
