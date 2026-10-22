---
id: SR-13911
title: 'Spec integrity analysis of PR #1188: sampled-witness replay takes the checked subject (STD-250)'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@7eea6669d; git diff origin/main...7eea6669d (PR #1188, branch std-qsl570-250): FR-193, FR-185 "Package emission", TC-628, and statements elsewhere under spec/ (reviews/ excluded) about witness replay and recompiling; ticket STD-250'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-193
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-024
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-128
  type: reviews
---

## Summary

Ticket: STD-250. PR: quire-spec-language#1188, head 7eea6669d. The check covers ids, relationships, traceability, and statements elsewhere in spec/ (reviews/ excluded) that contradict the new text. The PR adds no ids and touches only FR-193-AC-5, so no id can collide with main or with another open PR. The new `depends_on` edges to FR-125 and FR-185 resolve. TC-628's scope and steps name the new AC. A grep for `recompil` over spec/ found ADR-024 SV-7/SV-8, ADR-018 CX-3 and FR-128. ADR-028 WS-5, CE-2, FR-201 and FR-202 also recompile a probabilistic package for the exact engine. Those are outside FR-193 and are listed here only for the lead, not as findings of this PR.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-193 relationships | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:5-24 | examined |
| FR-193-AC-5 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:138 | examined |
| FR-193-AC-3 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:135 | context_only |
| FR-185 Package emission | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:209-217 | examined |
| TC-628 | spec/test-cases/TC-628-sampled-witnesses-replay.md | examined |
| ADR-024 SV-7, SV-8 | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:178-179 | examined |
| ADR-018 CX-3 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:342 | context_only |
| FR-128 Description | spec/functional/FR-128-replay-a-model-counterexample.md:33-44 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Statements elsewhere still contradict the new text and carry no amendment. ADR-024 SV-7 says a sampled witness's counterexample "replays through ADR-018 CX-3 as one", and SV-8 says a sampled witness "replays by ADR-018 CX-3". CX-3 "recompiles the package (FR-098)". FR-128's Description says `replay_model_trace` "recompiles the package". FR-193 now says this replay recompiles nothing and makes no `package_id` comparison. Neither ADR-024 nor FR-128 records the exception for a package S4 does not emit. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:178-179; spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:342; spec/functional/FR-128-replay-a-model-counterexample.md:39 |
| FND-002 | low | FR-193-AC-5 says "the converted counterexample of AC-3 settles `refuted` over it", in the singular and over `Service`. AC-3 converts two witnesses: one over `Service` and one over `Health` for the fault claim. `Health` also declares probabilistic claims, so S4 also refuses it, but neither AC-5 nor TC-628 step 4 names it. It is unclear whether AC-5 covers the `Health` conversion. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:135,138; spec/test-cases/TC-628-sampled-witnesses-replay.md:24 |

## Verdict

Ids, relationships and the TC trace are sound. Two statements that the new text overrides still say that replay recompiles: ADR-024 SV-7/SV-8 (through ADR-018 CX-3) and FR-128's Description. They need an amendment note (FND-001). AC-5 should say whether it covers the `Health` conversion of AC-3 (FND-002).

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@153ba1fee` (fix commit `2f4a4c525`, then a merge of origin/main). I checked each outcome against the spec at that head. A grep of spec/ (reviews/ excluded) for `replay_sampled_witness`, sampled witnesses and recompiling finds no other statement that contradicts the new text. Open PR #1186 edits other rows of ADR-024 (RX-1, SV-2) and adds FR-185-AC-9; neither overlaps this PR's text or ids.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 2f4a4c525 | ADR-024 SV-7 now reads "It replays as one by ADR-018 CX-3's steps applied to the checked subject, with no recompile (SV-8)", and SV-8 "A sampled witness replays by ADR-018 CX-3's steps, applied to the checked subject the sampler ran over, since S4 does not emit a package that holds a random parameter, a workload, a reward or a probabilistic claim (FR-185) and there is nothing to recompile". ADR-018 CX-3: "It recompiles the package (FR-098), except for a sampled-witness counterexample, which takes the checked subject of a package that S4 does not emit (ADR-024 SV-7, SV-8; FR-193)". FR-128 Description: "FR-193's `replay_sampled_witness` applies these same re-execution, fairness and evaluation steps to that subject, recompiles nothing and compares no `package_id`." The CX-3 and SV-8 wording has gaps of its own (new findings FND-003, FND-004). |
| FND-002 | fixed 2f4a4c525 | FR-193-AC-5 now adds "and so does the converted counterexample of the `Health` fault claim of AC-3", and TC-628 step 4 reads "Refuse `Service` and `Health` at S4, then run steps 2 and 3's `Service` replays and step 3's `Health` replay over their checked subjects", with expected step 4 "the `refuted` outcome for the `Health` fault claim". AC-5's wording about `Health`'s refusal is a new finding (FND-005). |

## New findings (disposition pass 1)

These were found at `153ba1fee`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | ADR-018 CX-3 still opens "E9 replay of a model counterexample goes through the layer-6 facade entry `qsl_replay::replay_model_trace`", and the new exception covers only the recompile. Read alone, CX-3 says `replay_model_trace` serves a sampled-witness counterexample too, while FR-128 and FR-193 say `replay_sampled_witness` does. The amended sentence also runs on ("..., which takes the checked subject of a package that S4 does not emit (...), and re-admits ... and re-executes"), so the re-admission reads as part of the exception. The exception should name the entry and be its own sentence. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:342 |
| FND-004 | low | In ADR-024 SV-8 the parenthetical "(re-execution through `ModelSystem`, post-state digests, SM-1 evaluation)" now follows "and there is nothing to recompile", so it reads as a description of the recompile and not of CX-3's steps, which it lists. It belongs after "ADR-018 CX-3's steps". | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:179 |
| FND-005 | low | FR-193-AC-5 asserts only that "`Service` is refused at S4 (FR-185-AC-7)", then says the `Health` counterexample settles `refuted` "although `emit_checked` refuses the same package". `Health` is a different package, so "the same package" does not name its refusal, and AC-5 does not require `Health` to be refused, although TC-628 step 4 refuses both. AC-5 should state that `Health` is refused at S4 too. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:149; spec/test-cases/TC-628-sampled-witnesses-replay.md:24 |
