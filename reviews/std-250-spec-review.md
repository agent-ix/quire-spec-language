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
