---
id: SR-13562
title: 'Spec review of PR #1100: align FR-187 and FR-188 with the sampling code, integrity'
type: SpecReview
analysis: integrity
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

Ticket: QSL-920. PR: quire-spec-language#1100, head 6152faf2a, scoped to `git diff refs/remotes/origin/main...6152faf2a` (one commit, four files, +54/-22). Spec-only. It aligns FR-187 and FR-188 (and TC-622, TC-623) with terra's sampling code on PR #1099 (branch qsl-920b-impl, 9bf87265). FR-189 is not edited; the PR only refers to it. Each of the PR body's six items was checked against qsl-eval/src/simulation on 9bf87265. Items 1, 2, 4, 5 and 6 match the code. Item 3 (SampleStop::UndecidedSuccessor) does not: the variant is not in the code. Methods: spec-review/base (SR-13560), spec-review/ears (SR-13561), spec-review/integrity (SR-13562). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 21 warnings (17 on main). The 4 new ones are all at FR-187 lines 109-110. tools/check-index-completeness.sh passes. No status wording in the diff ('as today' was removed). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: Consistency with FR-120, the merged FR-189 to FR-192 (#1098), and the TC scopes.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-187-enum-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-outputs | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-188-outputs | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-AC-7 | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-ac-table | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| TC-623-fixtures | spec/test-cases/TC-623-weighted-sampler-draws.md | examined |
| TC-623-step-5 | spec/test-cases/TC-623-weighted-sampler-draws.md | examined |
| FR-190-stop-map | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md | examined |
| FR-189-seed | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-189-undecided-bullet | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md | examined |
| FR-120-state-key | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| PR-1099-code | agent-ix/quire-spec-language@9bf87265 (PR #1099, qsl-920b-impl) qsl-eval/src/simulation/{sampled.rs,choice.rs,model/probabilistic.rs,model/frames.rs,model/frames/judge.rs,model/encode.rs} | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | 'encode Boolean and bounded Int values and no enumeration value' reads as the whole list, and it is narrower than FR-120 and the code. FR-120 also encodes Option, Reference and bounded sequence values. The encoder encodes any Value::Integer, bounded or not (encode.rs:198-261). FR-120 lists enum next to decimal, quantity, uuid, timestamp, record, tuple, union and population as the values with no encoding. Fix: 'FR-120's state key and transition identity have no encoding for an enumeration or quantity value (FR-120 State key), so ...'. That keeps the reason the Service fixture uses Int and Boolean stand-ins, and stops implying that Option and Reference fields are excluded. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:111-112 |
| FND-002 | low | This PR adds SampleStop::UndecidedSuccessor. FR-190's regenerative method lists how each sampler stop maps (NotMarkov and TotalAboveDrawRange to Unsupported, ExpansionStop to Stopped, GeneratorMismatch to a refusal), and the new variant is not on that list. So a regenerative test that meets an undecided step has no stated outcome. FR-189 covers its own tests through its 'contract conjunction evaluated undecided' bullet. Fix: add 'UndecidedSuccessor to an Undecided test with UndecidedSuccessor' to FR-190's list, or mention it in this PR's body as a follow-up. | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:50-52 |
| FND-003 | low | FR-188 now types the seed as u64 in ChoicePoint and sample_probabilistic. FR-189's StatisticalRequest still declares `pub seed: Seed`. After this PR no artifact defines `Seed`: FR-188 was its only other user. The code uses u64 throughout. Fix: change FR-189's field to `u64` in the same PR, or say where `Seed` is defined. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:55 |
| FND-004 | low | Step 5 has to leave a contract conjunction undecided during a sampled step. Neither TC-623's Fixtures line nor FR-188-AC-7 names a model that does that, while every other step uses a named fixture. FR-189-AC-5 tests the same condition with 'a Health variant with an undecided contract conjunction'. Fix: name the fixture in AC-7 and in the Fixtures line, for example the FR-189-AC-5 Health variant, with the clause it leaves undecided. | spec/test-cases/TC-623-weighted-sampler-draws.md:19, 25 |
| FND-005 | low | The new FR-188-AC-7 row sits above FR-188-AC-6, so the table is out of id order. Fix: move the AC-7 row below AC-6. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:160-161 |

## Verdict

Five low findings. None blocks merge on its own; FND-002 to FND-004 come from the new UndecidedSuccessor variant and should be fixed with base FND-001/002. What holds: FR-189 (Decision::Undecided(UndecidedSuccessor), the undecided-step bullet and AC-5) agrees with the new SampleStop variant. FR-192's Completed/Undecided row settles any UndecidedCause, so it needs no new row. FR-192's internal-failure expansion row settles the Empty/NotPositive stop failed with runtime_invariant/established-invariant-broken, the cause FR-188 names. TC-623's scope (AC-1 to AC-7) and step 5 cover the new AC, and TC-622 step 1 matches AC-1. Index completeness passes.

## New findings (disposition pass 1)

Round 1, head 8c75e31b1: fix commit 8c75e31b1 over 6152faf2a. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is now at 441223cc, which adds only review files over 9bf87265. It still has no UndecidedSuccessor result or SampleStop variant. The PR body now says item 3 is a requirement that branch does not build yet, which is accurate. QSL-920 has no comment yet recording the code follow-up for terra. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording in the FR text. The PR body's item 1 still says 'encode Boolean and bounded Int and no enumeration value', which the new FR-187 Note no longer says. That is PR-body wording only.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | `Unsteppable` has two arms, NotMarkov and UndecidedSuccessor. At a state where one application is undecided and a decided application's draw gives 0 or 2 post-states, both apply, and FR-187 does not say which `weighted_steps` returns. FR-189 settles the two differently (Unsupported against an Undecided test), so implementations could disagree on the verdict. The bullet also says 'at an application', but FR-120's ContractUndetermined is also raised for a postcondition candidate. Fix: say which arm wins (UndecidedSuccessor first, as ADR-018 V-6 keeps the state open), and say 'at an application or candidate'. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:64-65, 111-114 |

## New findings (disposition pass 2)

Round 2, head aaa7b4520: fix commit aaa7b4520 over 8c75e31b1. It commits this review's round-1 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is still at 441223cc. Scratch probe of HealthUndecided: a test added to a copy of qsl-920b-impl at 441223cc, using its sampled/world.rs harness. That branch's expansion and contract code is main's: frames.rs Conjunction::Undecided and contracts.rs are unchanged against main 01c94b48, and #1099 only adds enabled-vector capture. Under `profile v = "quire.value.complete/v1"`, FR-120's expansion at n = 2 returns no stop, no successor, and two ContractUndetermined findings, one per drawn `fault`, each with clause Guard and evaluation KernelUndefined(DivisionByZero) and no candidate. So the precondition is undecided, not refused. On #1099 today, `actions` returns Ok([]) and `weighted_steps` returns one stutter step, which is the silent drop this PR's requirement replaces. With a second operation `wobble` (empty postcondition, frame `spare`), `actions` returns wobble's one draw with 2 successors, and `weighted_steps` returns NotMarkov{post_states: 2}. The spec now requires the undecided result there instead. Under `quire.model.complete/v1`, the profile the harness uses for every TC-622/TC-623 fixture, S3 refuses the unit with unsupported_construct, ProfileProhibitedOperator(IntegerMod) (new FND-007). Ordering: weighted_steps calls actions, and actions runs the whole FR-120 expansion over every drawn vector (expand_enabled, walk.rs:187-194), before any draw's successor count is judged (probabilistic.rs:611-633). The sampler's take_step judges the drawn vector only after actions returns (sampled.rs:186-218). So 'the expansion that finds it runs before any draw is judged' is true of the code's structure, and a state-level undecided result can be returned before the NotMarkov check. FR-189 maps the two to different outcomes (Unsupported against an Undecided test) and says nothing about precedence, so the new FR-187 rule is the only place it is stated. That is consistent, not a conflict. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | In a scratch probe on qsl-920b-impl (441223cc), HealthUndecided behaves as AC-8 says only under `profile v = "quire.value.complete/v1"`. There the guard is undecided: ContractUndetermined, KernelUndefined(DivisionByZero), with no stop and no refusal. Under `quire.model.complete/v1`, which terra's sampled/world.rs harness uses for every TC-622 and TC-623 fixture, S3 refuses the unit with unsupported_construct, ProfileProhibitedOperator(IntegerMod), so AC-8 cannot be run as written. Neither AC-8 nor TC-623's Fixtures line names a profile. ADR-024 §7.1 selects only the temporal profile `ev`. Fix: state in AC-8 and TC-623's Fixtures line that HealthUndecided is checked under `quire.value.complete/v1`. Otherwise choose a guard that `quire.model.complete/v1` admits and that evaluation leaves undefined at n = 2. | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md:162 |

## Dispositions

Round 1, head 8c75e31b1: fix commit 8c75e31b1 over 6152faf2a. The fix commit also commits this review's round-0 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is now at 441223cc, which adds only review files over 9bf87265. It still has no UndecidedSuccessor result or SampleStop variant. The PR body now says item 3 is a requirement that branch does not build yet, which is accurate. QSL-920 has no comment yet recording the code follow-up for terra. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording in the FR text. The PR body's item 1 still says 'encode Boolean and bounded Int and no enumeration value', which the new FR-187 Note no longer says. That is PR-body wording only.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | still-open | AC-8 and TC-623 step 6 now point at 'FR-189-AC-5's Health variant, whose second contract conjunction is left undecided at the initial state'. But FR-189-AC-5 and TC-624 only say 'a variant with an undecided contract conjunction', and ADR-024 §7.1's Health has one contract, tick's postcondition, so 'its second contract conjunction' has nothing to refer to. TC-623's Fixtures line still does not list the variant. Fix: define the variant in AC-8, for example tick with a second postcondition clause that evaluation leaves undefined at the initial state, and name the clause. Then add the variant to TC-623's Fixtures line. |
| FND-005 | fixed | - |

Round 2, head aaa7b4520: fix commit aaa7b4520 over 8c75e31b1. It commits this review's round-1 SR files under reviews/ unchanged; this file supersedes them. Terra's #1099 branch is still at 441223cc. Scratch probe of HealthUndecided: a test added to a copy of qsl-920b-impl at 441223cc, using its sampled/world.rs harness. That branch's expansion and contract code is main's: frames.rs Conjunction::Undecided and contracts.rs are unchanged against main 01c94b48, and #1099 only adds enabled-vector capture. Under `profile v = "quire.value.complete/v1"`, FR-120's expansion at n = 2 returns no stop, no successor, and two ContractUndetermined findings, one per drawn `fault`, each with clause Guard and evaluation KernelUndefined(DivisionByZero) and no candidate. So the precondition is undecided, not refused. On #1099 today, `actions` returns Ok([]) and `weighted_steps` returns one stutter step, which is the silent drop this PR's requirement replaces. With a second operation `wobble` (empty postcondition, frame `spare`), `actions` returns wobble's one draw with 2 successors, and `weighted_steps` returns NotMarkov{post_states: 2}. The spec now requires the undecided result there instead. Under `quire.model.complete/v1`, the profile the harness uses for every TC-622/TC-623 fixture, S3 refuses the unit with unsupported_construct, ProfileProhibitedOperator(IntegerMod) (new FND-007). Ordering: weighted_steps calls actions, and actions runs the whole FR-120 expansion over every drawn vector (expand_enabled, walk.rs:187-194), before any draw's successor count is judged (probabilistic.rs:611-633). The sampler's take_step judges the drawn vector only after actions returns (sampled.rs:186-218). So 'the expansion that finds it runs before any draw is judged' is true of the code's structure, and a state-level undecided result can be returned before the NotMarkov check. FR-189 maps the two to different outcomes (Unsupported against an Undecided test) and says nothing about precedence, so the new FR-187 rule is the only place it is stated. That is consistent, not a conflict. Gates: make check-spec-validation exits 0 with 17 warnings, the same as main, none in the changed files. tools/check-index-completeness.sh passes. No status wording.

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | - |
| FND-006 | fixed | - |
