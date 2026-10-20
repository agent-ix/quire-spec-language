---
id: SR-13560
title: 'Spec review of PR #1100: align FR-187 and FR-188 with the sampling code, base'
type: SpecReview
analysis: base
scope: 'agent-ix/quire-spec-language@6152faf2a; git diff refs/remotes/origin/main...6152faf2a (PR #1100, branch std-920-align-spec): spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md, spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md, spec/test-cases/TC-622-model-transitions-carry-step-probabilities-and-rewards.md, spec/test-cases/TC-623-weighted-sampler-draws.md; compared with PR #1099 branch qsl-920b-impl 9bf87265, main 8a556983 (FR-120, FR-141, FR-142, FR-189 to FR-192 after #1098) and QSpec FR-272; ticket QSL-920'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-187
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-188
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-622
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-623
  type: reviews
---

## Summary

Ticket: QSL-920. PR: quire-spec-language#1100, head 6152faf2a, scoped to `git diff refs/remotes/origin/main...6152faf2a` (one commit, four files, +54/-22). Spec-only. It aligns FR-187 and FR-188 (and TC-622, TC-623) with terra's sampling code on PR #1099 (branch qsl-920b-impl, 9bf87265). FR-189 is not edited; the PR only refers to it. Each of the PR body's six items was checked against qsl-eval/src/simulation on 9bf87265. Items 1, 2, 4, 5 and 6 match the code. Item 3 (SampleStop::UndecidedSuccessor) does not: the variant is not in the code. Methods: spec-review/base (SR-13560), spec-review/ears (SR-13561), spec-review/integrity (SR-13562). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 21 warnings (17 on main). The 4 new ones are all at FR-187 lines 109-110. tools/check-index-completeness.sh passes. No status wording in the diff ('as today' was removed). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: Checks each changed statement and expected result against #1099's code and the merged FR-120 and FR-189 to FR-192.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-187-AC-1 | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-AC-4 | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-outputs | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-negative-reward-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-undecided-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-enum-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-188-outputs | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-weights-bullets | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-undecided-bullet | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-AC-3 | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-AC-7 | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| TC-622-step-1 | spec/test-cases/TC-622-model-transitions-carry-step-probabilities-and-rewards.md | examined |
| TC-622-expected-step-4 | spec/test-cases/TC-622-model-transitions-carry-step-probabilities-and-rewards.md | examined |
| TC-623-step-5 | spec/test-cases/TC-623-weighted-sampler-draws.md | examined |
| FR-189-undecided-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | context_only |
| FR-120-state-key | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| PR-1099-code | agent-ix/quire-spec-language@9bf87265 (PR #1099, qsl-920b-impl) qsl-eval/src/simulation/{sampled.rs,choice.rs,model/probabilistic.rs,model/frames.rs,model/frames/judge.rs,model/encode.rs} | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR body says every item was checked against the code on qsl-920b-impl, but `SampleStop::UndecidedSuccessor` is not in that code. At 9bf87265, SampleStop has GeneratorMismatch, Expansion, NotMarkov, TotalAboveDrawRange and Cancelled, and nothing else (sampled.rs:75-97). The string UndecidedSuccessor appears nowhere in qsl-eval/src/simulation outside a doc comment. `ModelSystem::actions` drops an undecided application or candidate and records `ContractUndetermined` in findings it does not return (probabilistic.rs:500-513; frames.rs:1038-1052). So on #1099 an undecided step does one of three things. With one decided candidate left it samples a normal step. With none left the draw has no successor and the sample stops NotMarkov{post_states: 0}. If every application is undecided, `actions` is empty and the sampler takes a stutter step, as at a terminal state (sampled.rs:199-208). The second clause of AC-7 and TC-623 step 5 fail against #1099. Fix: keep the requirement, since FR-189 already needs it to settle UndecidedSuccessor. State in the PR body, and in a QSL-920 comment for terra, that #1099 must add the variant and a way for expansion to report the undecided case (FND-002), and leave that clause of AC-7 untagged until then. The other way is to drop the variant here, but then FR-189's UndecidedSuccessor bullet has nothing to come from. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:108-109, 160 |
| FND-002 | medium | FR-187 says the sampler stops with UndecidedSuccessor. But FR-187's own Outputs give the sampler no way to learn that a conjunction was undecided. `actions` returns Result<Vec<Action>, ExpansionStop>, and `weighted_steps` returns Result<Result<Vec<WeightedStep>, NotMarkov>, ExpansionStop>. Neither Action, Draw nor WeightedStep carries a finding. FR-120's ContractUndetermined finding is reported by the expansion that these functions wrap, and is dropped there. The bullet also does not say what `weighted_steps` itself returns at such a state. Fix: give FR-187's functions a result for the undecided case, for example an `Undecided` arm next to NotMarkov, or the ContractUndetermined findings on Action and Draw. State what `actions` and `weighted_steps` return, and let FR-188 map that result to SampleStop::UndecidedSuccessor. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:108-110, 66-70 |
| FND-003 | medium | TC-622's Expected Results step 4 still expects `NegativeReward` naming `n = 0`. The PR removed that variant from FR-187. AC-4 now expects the refusal record `invalid_probabilistic_model`/`negative-reward` naming the operation, `refund` and `n = 0`, and the code returns that record with fields operation, reward and arguments (probabilistic.rs:466-482; tests.rs:264-291). Fix: 'Step 4: `invalid_probabilistic_model`/`negative-reward` naming the operation, `refund` and `n = 0`; equal sets.' | spec/test-cases/TC-622-model-transitions-carry-step-probabilities-and-rewards.md:33 |

## Verdict

Not mergeable. FND-001 (high) must be fixed or explicitly recorded as a code follow-up: SampleStop::UndecidedSuccessor is not in #1099, so AC-7's second clause cannot pass there. FND-002 and FND-003 (medium) should be fixed in this PR. What holds: the six AC-1 probabilities are right. With d at 90/7/3 per 100 and outcome at 98/2 per 100, and attempt the only operation enabled at phase 1, they are 441/500, 9/500, 343/5000, 7/5000, 147/5000 and 3/5000. They sum to 1, and #1099's test asserts each one. WeightsRefusal matches choice.rs:72-87 and 130-190: an empty list gives Empty, (1, 0) and (1, -1/2) give NotPositive{index: 1}, and a refusal computes no digest (weighted_choice_counted). The Empty/NotPositive to Expansion(BROKEN) mapping matches sampled.rs:143-147, and BROKEN is runtime_invariant/established-invariant-broken (model.rs:82). invalid_probabilistic_model/negative-reward is in QSpec FR-272's closed set, and the record carries operation, reward and arguments, with a random parameter ranging over its support (probabilistic.rs:415-485). Every realised type matches the code: operation String, ObjectReference, DigestRecord, u64 seed, TransitionId{Operation, Stutter}, ModelSimState. Owner decision 6 is confirmed: frames/judge.rs:178-183 and 274-286 build each parameter's domain with Domain::of over its full type, and FR-141, FR-142 and FR-187 say nothing about random parameters there. The recommendation (judge over the support) is consistent with FR-187's first Behavior bullet. No status wording.

## New findings (disposition pass 1)

Round 1, head 8c75e31b1: fix commit 8c75e31b1 over 6152faf2a. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is now at 441223cc, which adds only review files over 9bf87265. It still has no UndecidedSuccessor result or SampleStop variant. The PR body now says item 3 is a requirement that branch does not build yet, which is accurate. QSL-920 has no comment yet recording the code follow-up for terra. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording in the FR text. The PR body's item 1 still says 'encode Boolean and bounded Int and no enumeration value', which the new FR-187 Note no longer says. That is PR-body wording only.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The new FR-187 bullet gives two results: `UndecidedSuccessor{state}` from `actions`, and `Unsteppable::UndecidedSuccessor` from `weighted_steps`. No FR-187 AC or TC-622 step tests either one. FR-188-AC-8 checks `actions` through the sampler test only, and nothing checks the `weighted_steps` arm. Fix: add an FR-187 AC, or extend AC-3, with a TC-622 step. On the undecided Health variant, `actions` should return `UndecidedSuccessor{state}` and `weighted_steps` `Unsteppable::UndecidedSuccessor`, each naming the initial state's digest. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:111-114 |

## Dispositions

Round 1, head 8c75e31b1: fix commit 8c75e31b1 over 6152faf2a. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is now at 441223cc, which adds only review files over 9bf87265. It still has no UndecidedSuccessor result or SampleStop variant. The PR body now says item 3 is a requirement that branch does not build yet, which is accurate. QSL-920 has no comment yet recording the code follow-up for terra. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording in the FR text. The PR body's item 1 still says 'encode Boolean and bounded Int and no enumeration value', which the new FR-187 Note no longer says. That is PR-body wording only.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
