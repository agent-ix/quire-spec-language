---
id: SR-13910
title: 'Spec review of PR #1188: sampled-witness replay takes the checked subject (STD-250)'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@7eea6669d; git diff origin/main...7eea6669d (PR #1188, branch std-qsl570-250): FR-193 (Description, Inputs, Replay, The qualitative claim, AC-5, Dependencies, depends_on), FR-185 "Package emission", TC-628; ticket STD-250'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-193
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-185
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-628
  type: reviews
---

## Summary

Ticket: STD-250. PR: quire-spec-language#1188, head 7eea6669d, based on main. The PR has no QSpec half. Tooling: quoin 0.28.3, spec-artifacts-process@b82e42e.

The PR takes the ticket's second reading as a lead default: `replay_sampled_witness` replays over the checked subject the sampler ran over and recompiles nothing, because S4 refuses to emit a probabilistic package (FR-185). That matches the ticket and the code. `qsl_package::emit_checked` refuses `Service` with `ProbabilisticModelNotEmitted` (qsl-package/src/emit.rs:1246, tested by TC-620/FR-185-AC-7). `qsl_replay::statistical::replay_sampled_witness` (qsl-replay/src/statistical/witness.rs:216) already replays over an admitted `ModelSystem` with no recompile, and its TC-628 tests build `Service` in-process with no emission. QSpec FR-410 "Witness replay" and QSpec FR-364 say nothing about a recompile or a `package_id` comparison, so the QSL text agrees with the QSpec half. Gates: `make check-spec-validation` exit 0 with no warning on a touched file, `tools/check-index-completeness.sh` exit 0. The PR touches nothing under docs/ and no vector table that a test parses. It adds no ids. No other open PR touches FR-193, FR-185 or TC-628.

FR-280 consequence: FR-193's ACs run EN-4 through FR-189 `check_statistical` over a `ModelSubject`, not through FR-280's `prove` provider entry. So the `EmittedPackage` input of FR-280 makes no AC of this PR unbuildable, and the PR does not have to resolve it. PR #1185 (STD-251) and this PR each point at the other for it, so it needs one owner-tracked ticket.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-193 Description | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:29-38 | examined |
| FR-193 Inputs `replay_sampled_witness` signature | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:58-64 | examined |
| FR-193 Replay, first bullet | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:90-97 | examined |
| FR-193 The qualitative claim | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:119-128 | examined |
| FR-193-AC-5 | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:138 | examined |
| FR-193 depends_on FR-125, FR-185; Dependencies | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:16-19,142 | examined |
| FR-185 Package emission | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:209-217 | examined |
| TC-628 scope, step 4, expected step 4 | spec/test-cases/TC-628-sampled-witnesses-replay.md:15,24,33 | examined |
| FR-128 Description and Behavior | spec/functional/FR-128-replay-a-model-counterexample.md:33-140 | context_only |
| FR-125 Inputs (`ModelSubject`) | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:60 | context_only |
| FR-098 request | spec/functional/FR-098-execute-a-replay-request.md | context_only |
| FR-191 StatisticalProvenance | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:56 | context_only |
| FR-189 StatisticalRequest | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:50-65 | context_only |
| FR-280 provider entry | spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md:34,47 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | "The qualitative claim" now says FR-128's re-execution, fairness and evaluation steps replay the converted counterexample over the checked subject, with no recompile and no `package_id` comparison. FR-128 specifies only `replay_model_trace`, which takes FR-098's request and SHALL recompile and compare `package_id`, and no FR names the entry that serves this replay. The text also does not say where the clause identity, the fairness set and the `interval` that FR-128 checks against "the recompiled clause" come from, when the counterexample is for a TP-2 claim derived from the probabilistic claim and not a declared clause. A coder building FR-193-AC-3 and AC-5 has to invent the entry and choose which of FR-128's identity checks apply. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:126-128; spec/functional/FR-128-replay-a-model-counterexample.md:39,96-110 |
| FND-002 | low | The edited `request` comment reads "FR-098: provision, seed, sampler; its package reference is not read". FR-098's request carries no seed and no sampler, which FR-191's `StatisticalProvenance` (the `provenance` parameter) holds, and the `ModelSubject` already carries the snapshot provision (FR-125). So the parameter is described with members it does not have, and the one it does have is read from two places. The implementation takes no `ReplayRequest` (`WitnessReplay` holds the system, workload, provenance and cancel handle). | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:60; qsl-replay/src/statistical/witness.rs:78-88 |
| FND-003 | low | FR-193 says the checked subject is FR-125's `ModelSubject`, and FR-125 defines it as "the S4 `CheckedPackage`". FR-185 and FR-193 now say "S4 refuses a package" that holds random parameters. Read together, the subject replay takes is the output of the stage that refuses the package. The code shows the refusal is of the wire emission (`emit_checked`) and the in-process `CheckedPackage` still exists. The new text should say that S4 refuses the emission and the checked package remains. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:90-92; spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:212-217; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:60 |

## Verdict

The PR's text follows the ticket's second reading, matches the implemented witness replay, and agrees with QSpec FR-410 and FR-364. The lead default leaves the conversion replay of FR-193-AC-3 and AC-5 without a specified entry or identity rules (FND-001). The PR body lists that replay as remaining code work, but the spec should name the entry before it is built. FND-002 and FND-003 are wording fixes. The FR-280 `prove` consequence does not block this PR.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@153ba1fee` (fix commit `2f4a4c525`, then a merge of origin/main). I checked each outcome against the spec at that head. origin/main has since gained two commits (#1180, #1177) that touch none of this PR's files; `git merge-tree` merges cleanly.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed 2f4a4c525 | FR-193 now names the entry and the identity sources: "`replay_sampled_witness` SHALL replay that converted counterexample by applying FR-128's re-execution, fairness and evaluation steps to the checked subject the sampler ran over, with no recompile and no `package_id` comparison, and SHALL settle the TP-2 claim `refuted` when they agree. ... the clause identity is the derived claim's identity, that is the probabilistic claim's identity with the bound direction of this section; the fairness set is the one the checked subject declares; the `interval` is the claim event's interval. The counterexample carries the same three values, and the replay refuses `stale_dependency`/`content-mismatch` when they differ." FR-128's Description records the same exception. The entry's signature does not yet carry this replay (new finding FND-004). |
| FND-002 | still-open: the parameter is now described with a member it does not have | `request: &ReplayRequest` is gone, but the provenance comment now reads "FR-191: seed, sampler, provision, and per test the initial state and binding". FR-191 defines `StatisticalProvenance{seed, sampler, tests}` (FR-191:56, 90-92) and the code's struct has the same three fields (qsl-replay/src/statistical.rs:371-378), so it carries no provision. The provision is the `ModelSubject`'s ("snapshot provision", FR-125:60). The comment should drop "provision", and the Replay bullet's "from the provision" should say it is the subject's. |
| FND-003 | fixed 2f4a4c525 | FR-193 Description: "S4 refuses to emit a package that declares random parameters, workloads, rewards or probabilistic claims (FR-185), but its checked package remains, so the replay reads that checked package and has no emitted package to recompile." The Replay bullet says the same ("while its checked package remains"). |

## New findings (disposition pass 1)

These were found at `153ba1fee`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new bullet says `replay_sampled_witness` "SHALL replay that converted counterexample" and "SHALL settle the TP-2 claim `refuted`", but the entry's signature takes only `subject`, `provenance` and `witness: &SampledWitness`, and Outputs lists only `ReplayOutcome::Reproduced`, `ReplayOutcome::Inconclusive(ReplayParity)` and `ReplayRefusal`. Nothing says whether one call replays both the witness and its converted counterexample, how a caller asks for the TP-2 replay, or how the `refuted` settlement (FR-128's FR-072 replay result, `reproduced-with-evaluated-witness`) is returned. A coder building FR-193-AC-3 and AC-5 has to choose. The Inputs and Outputs should state the TP-2 replay's input and result. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:59-70,128-139 |
| FND-005 | low | FR-193's relationships no longer match its text. The new bullet cites "the checked probabilistic claim (FR-186)", but FR-193 has no `depends_on` edge to FR-186. FR-193 keeps `depends_on` FR-098 and the FR-098 link under Dependencies, but no statement uses FR-098 any more now that the `ReplayRequest` parameter is gone. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:12,134,154 |

Disposition pass 2 at `agent-ix/quire-spec-language@1b18f1eff` (fix commit `1b18f1eff`). `make check-spec-validation` exits 0 with no warning on a touched file, and `tools/check-index-completeness.sh` exits 0. The PR adds no ids. Open PR #1186 also edits ADR-024 and FR-185 and merges cleanly with this head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-002 | fixed 1b18f1eff | The provenance comment reads "FR-191: seed, sampler, and per test the initial state and binding", which matches FR-191's `StatisticalProvenance{seed, sampler, tests}`, and the Replay bullet re-admits the initial state "from the subject's provision". |
| FND-004 | fixed 1b18f1eff | Outputs: "`ReplayOutcome::Reproduced` when every check agrees, which includes the replay of the converted counterexample ("The qualitative claim") settling its TP-2 claim `refuted`, or `ReplayOutcome::Inconclusive(ReplayParity)`." The qualitative-claim bullet: "The same call of `replay_sampled_witness` [...] converts the witness and SHALL replay that converted counterexample [...] and SHALL settle the TP-2 claim `refuted` when they agree, which it returns as `Reproduced`, and `Inconclusive(ReplayParity)` when they disagree." The signature, Outputs and Behavior now agree. The ACs and TC-628 were not brought along (new finding FND-006). |
| FND-005 | fixed 1b18f1eff | `depends_on` FR-186 is added and FR-098 is dropped from the relationships and from Dependencies; no statement in FR-193 cites FR-098. |

## New findings (disposition pass 2)

These were found at `1b18f1eff`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | FR-193 now converts the witness and replays the counterexample inside one `replay_sampled_witness` call that returns only `Reproduced` or `Inconclusive(ReplayParity)`, and it names no conversion entry. FR-193-AC-3 and TC-628 step 3 still describe a separate step: a witness "converted to a `TemporalCounterexample` for the TP-2 claim [...] replays through FR-128 and settles that claim `refuted`", and TC-628 says "Convert a witness [...] and replay it", expecting "the TP-2 claim settles `refuted`". A test cannot observe which TP-2 claim the conversion built, or a `refuted` settlement, from a `Reproduced` return. "Through FR-128" names `replay_model_trace`, which recompiles a package that S4 refuses. And because the call builds the counterexample from the claim itself, the `stale_dependency`/`content-mismatch` refusal for mismatched identity, fairness or interval can never be reached. AC-3 and TC-628 should state the observable result of the single call, and the refusal should be removed or given an input that can differ. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:129-142,150; spec/test-cases/TC-628-sampled-witnesses-replay.md:23,32 |
| FND-007 | low | "The same call of `replay_sampled_witness`, which takes no input beyond the witness" contradicts the signature and AC-5, which say the call takes the checked subject, the provenance and the witness. The clause means the call needs no further input to replay the counterexample, and it should say that. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:129 |
| FND-008 | low | The head does not contain origin/main. Main has since gained #1185 (STD-251, which edits FR-189, FR-192, FR-195 and FR-280) and #1187 (STD-243). Neither touches this PR's files, `git merge-tree` merges cleanly, and nothing in them contradicts the new text, but the branch should be brought up to date before merge. | quire-spec-language#1188 head 1b18f1eff vs origin/main 08330ebf4 |

Disposition pass 3 at `agent-ix/quire-spec-language@e7d813f45` (fix commit `e7d813f45`, after merge `48ced337d` of origin/main). `make check-spec-validation` exits 0 with no warning on a touched file, and `tools/check-index-completeness.sh` exits 0. The PR adds no ids. Open PRs #1186 and #1189 each merge cleanly with this head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-006 | fixed e7d813f45 | FR-193-AC-3: "`replay_sampled_witness` over a witness of the `2 ms` claim returns `Reproduced`; in that one call the witness converts to a counterexample for the TP-2 claim [...] and the replay of the counterexample refutes that claim. [...] `replay_sampled_witness` returns `Reproduced`, and the counterexample it replays refutes [...]". TC-628 step 3: "Replay a `Service` witness with `replay_sampled_witness`, which converts it to a counterexample for the TP-2 claim and replays that in the one call". The identity refusal is gone: "it makes none of the checks FR-128 makes against a recompiled clause [...] and the counterexample and the claim cannot differ in them." The new negative case has a gap of its own (new finding FND-009). |
| FND-007 | fixed e7d813f45 | "The one call of `replay_sampled_witness` needs no input beyond the checked subject, the provenance and the witness to replay the counterexample". |
| FND-008 | fixed 48ced337d | The merge brought in #1185 and #1187. origin/main has moved again since (new finding FND-010). |

## New findings (disposition pass 3)

These were found at `e7d813f45`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | Major (blocks merge). FR-193-AC-3's new negative case, "The same witness with its steps changed so that the replayed positions no longer violate the bound returns `Inconclusive(ReplayParity)`", and TC-628 step 3's "a copy of that witness whose steps no longer violate the bound" do not say what in the steps changes, and FR-193's own replay rules give different results for different changes. A step whose choice changes but whose post-state digest is kept refuses `stale_dependency`/`content-mismatch` (FR-128, ADR-018 CX-3), and a step changed to a transition that is not enabled refuses `invalid_runtime_input`/`invalid-value`. Only a copy rewritten with consistent post-state digests reaches `Inconclusive(ReplayParity)`, and then the disagreement comes from the draw recomputation from the seed (and the path probability), not from the counterexample replay. Two testers would build different copies and get different outcomes. The AC should name the change, for example which fields of which step are rewritten, and its expected result. | spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:152; spec/test-cases/TC-628-sampled-witnesses-replay.md:23,32 |
| FND-010 | low | Minor. The head does not contain origin/main again. Main has gained #1193 (STD-246) and #1196 (STD-248), which edit ADR-025, FR-221 to FR-223 and TC-668. Neither touches this PR's files, and `git merge-tree` merges cleanly. | quire-spec-language#1188 head e7d813f45 vs origin/main bc0322c0e |
| FND-011 | low | Minor. The PR body no longer matches the spec. It says the clause identity, fairness set and interval "it checks are read from the checked probabilistic claim", and its remaining-work item says the code replays "with the derived-claim identity, fairness and interval checks", but FR-193 now says the call "makes none of the checks FR-128 makes against a recompiled clause". The body also does not list the FR-254, ADR-026 SS-6 and ADR-018 CX-3 edits. | quire-spec-language#1188 PR body; spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:137-144 |
