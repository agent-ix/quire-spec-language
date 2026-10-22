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

Disposition pass 2 at `agent-ix/quire-spec-language@1b18f1eff` (fix commit `1b18f1eff`). A grep of spec/ (reviews/ excluded) for `replay_sampled_witness`, "sampled witness" and recompiling now also covers the timed statistical requirements. The PR adds no ids, and FR-186 resolves. Open PR #1186 merges cleanly with this head and does not edit SV-7, SV-8 or "Package emission".

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-003 | fixed 1b18f1eff | CX-3: "It recompiles the package (FR-098). A sampled-witness counterexample replays through `replay_sampled_witness` instead (FR-193), which takes the checked subject of a package that S4 does not emit and recompiles nothing (ADR-024 SV-7, SV-8). Both re-admit the subject's initial state and universes from the byte provision, and re-execute each step through `ModelSystem` with FR-101 `replay`." The exception names the entry and is a sentence of its own. The new "Both" has a gap of its own (new finding FND-007). |
| FND-004 | fixed 1b18f1eff | SV-8: "A sampled witness replays by ADR-018 CX-3's steps (re-execution through `ModelSystem`, post-state digests, SM-1 evaluation), applied to the checked subject the sampler ran over, since S4 does not emit [...] and there is nothing to recompile, plus two checks". |
| FND-005 | fixed 1b18f1eff | FR-193-AC-5: "`Service` is refused at S4 (FR-185-AC-7) and `Health` is refused at S4 too, [...] although `emit_checked` refuses the package of each". TC-628 was not updated to match (new finding FND-008). |

## New findings (disposition pass 2)

These were found at `1b18f1eff`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The timed statistical requirements still say that a sampled witness replays with a recompile. FR-254's Witnesses bullet says a timed sampled witness SHALL "replay by FR-237", and ADR-026 SS-6 says it "replays by CT-3". FR-237 is `replay_model_trace` and "SHALL apply FR-128's recompile, identity, fairness-set and admission rules unchanged", and CT-3 is `replay_model_trace` too. FR-193's `SampledWitness` covers timed steps ("a timed step adds its delay (FR-254)"), and a timed EN-4 model has a workload, so S4 refuses it. FR-193 and SV-8 say this replay recompiles nothing through `replay_sampled_witness`. FR-254 and SS-6 should say that a timed witness replays through `replay_sampled_witness` with CT-3's timed steps over the checked subject. | spec/functional/FR-254-sample-timed-runs-for-statistical-checking.md:89-91; spec/decisions/ADR-026-dense-time.md:242; spec/functional/FR-237-replay-a-timed-counterexample.md:29,58 |
| FND-007 | low | After the new "Both re-admit [...]" sentence, CX-3's later sentences read as covering both entries, among them "Agreement settles `reproduced-with-evaluated-witness` and the item `refuted`". For a sampled witness, the item's `measured` value is unchanged (SV-8, FR-193), and the derived TP-2 claim, not the item, settles `refuted`. CX-3 should limit "the item `refuted`" to `replay_model_trace`, or name the derived claim for the sampled-witness case. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:342 |
| FND-008 | low | AC-5 now names two packages but keeps the singular pronouns of the old text: "replay `Reproduced` over its checked subject" and "settles `refuted` over it". TC-628's expected step 4 still reads "the S4 refusal of FR-185-AC-7 [...] although `emit_checked` refuses the package", with one refusal and one package, while step 4 refuses both `Service` and `Health`. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:152; spec/test-cases/TC-628-sampled-witnesses-replay.md:33 |

Disposition pass 3 at `agent-ix/quire-spec-language@e7d813f45` (fix commit `e7d813f45`). A grep of spec/ (reviews/ excluded) for "replays by FR-237", "replays by CT-3", "sampled witness" and `replay_sampled_witness` covers the timed statistical text. FR-237 itself is consistent: it specifies `replay_model_trace` over a timed subject, and FR-193 and FR-254 now borrow its timed steps without its recompile. FR-204's timed witness path is EN-5's and outside this PR. The PR adds no ids. Open PRs #1186 and #1189 each merge cleanly with this head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | still-open: FR-254 still says a sampled witness replays by FR-237 in two places | The Witnesses bullet and SS-6 are fixed. The Witnesses bullet reads "replay through `replay_sampled_witness` (FR-193) over the checked subject the sampler ran over, with no recompile [...] applying FR-237's timed steps", and SS-6 reads "It replays through `replay_sampled_witness` (FR-193) over the checked subject the sampler ran over, with no recompile [...] by CT-3's timed steps". But FR-254's Description still reads "Each sample replays by FR-237 plus draw recomputation", and FR-254-AC-2 still reads "a sampled witness replays by FR-237 and its draws recompute from the seed". FR-237 is `replay_model_trace`, which "SHALL apply FR-128's recompile, identity, fairness-set and admission rules unchanged". So FR-254's AC contradicts its own Witnesses bullet. Major (blocks merge). Both sentences should say the witness replays through `replay_sampled_witness` with FR-237's timed steps. |
| FND-007 | fixed e7d813f45 | CX-3: "For `replay_model_trace`, agreement settles `reproduced-with-evaluated-witness` and the item `refuted`; disagreement settles `inconclusive`, `ReplayParity`. For `replay_sampled_witness`, agreement returns `Reproduced` and settles the derived TP-2 claim `refuted` (the item's `measured` value is unchanged, FR-193), and disagreement returns `Inconclusive(ReplayParity)`." |
| FND-008 | fixed e7d813f45 | FR-193-AC-5: "The witnesses of AC-1 replay `Reproduced` over the checked subject of `Service`, and so do the witnesses of the `Health` fault claim of AC-3 over the checked subject of `Health`, in each case although `emit_checked` refuses the package". TC-628 expected step 4: "the S4 refusals of `Service` (FR-185-AC-7) and of `Health`; [...] although `emit_checked` refuses each package". |

## New findings (disposition pass 3)

These were found at `e7d813f45`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | low | Minor. ADR-026 still says a sample replays by CT-3, which it defines as `replay_model_trace`: SS-2 ("the run is a timed behaviour that replays by CT-3") and the §11 worked example ("each sample replays by CT-3"). SS-6 now routes a timed witness through `replay_sampled_witness` with CT-3's timed steps. The two sentences should say "by CT-3's timed steps" or point at SS-6. | spec/decisions/ADR-026-dense-time.md:238,357 |
| FND-010 | low | Minor. FR-254's Witnesses bullet now cites `replay_sampled_witness` (FR-193), but FR-254 has no `depends_on` edge to FR-193. | spec/functional/FR-254-sample-timed-runs-for-statistical-checking.md:5-16,89-90 |
