---
id: SR-13561
title: 'Spec review of PR #1100: align FR-187 and FR-188 with the sampling code, ears'
type: SpecReview
analysis: ears-conformance
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

Ticket: QSL-920. PR: quire-spec-language#1100, head 6152faf2a, scoped to `git diff refs/remotes/origin/main...6152faf2a` (one commit, four files, +54/-22). Spec-only. It aligns FR-187 and FR-188 (and TC-622, TC-623) with terra's sampling code on PR #1099 (branch qsl-920b-impl, 9bf87265). FR-189 is not edited; the PR only refers to it. Each of the PR body's six items was checked against qsl-eval/src/simulation on 9bf87265. Items 1, 2, 4, 5 and 6 match the code. Item 3 (SampleStop::UndecidedSuccessor) does not: the variant is not in the code. Methods: spec-review/base (SR-13560), spec-review/ears (SR-13561), spec-review/integrity (SR-13562). code-review and gap-analysis were not run because no code or test file changed. Gates on the head: make check-spec-validation exits 0 with 21 warnings (17 on main). The 4 new ones are all at FR-187 lines 109-110. tools/check-index-completeness.sh passes. No status wording in the diff ('as today' was removed). quoin 0.28.3, quire 0.36.2 (engine 0.50.2). This review: EARS conformance of the changed Behavior statements.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-187-negative-reward-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-undecided-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-187-enum-bullet | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md | examined |
| FR-188-weights-bullets | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |
| FR-188-undecided-bullet | spec/functional/FR-188-draw-weighted-choices-with-the-qspec-sampler.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | One sentence holds two SHALLs with two subjects ('the expansion SHALL report ...', 'the sampler (FR-188) SHALL stop ...'), and the second repeats FR-188's own bullet. quire's grammar lint flags it: ears:unclassifiable and ears:missing-subject at lines 109 and 110. These are 4 new warnings (17 on main, 21 on the head). They are the only new warnings. Fix: 'If a contract conjunction is undecided during expansion, then ModelSystem SHALL report it as <the FND-002 result>.' Leave the sampler's stop to FR-188's bullet. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:108-110 |
| FND-002 | low | The new Behavior bullet has no SHALL. It states a fact about FR-120's encoding, says what cannot be expanded without saying what happens, and ends with advice to test authors ('a fixture declares such a field as a bounded Int or a Boolean'). That is not system behaviour. Fix: make it a Note that points to FR-120's State key section (an enumeration value refuses with EncodingRefusal). Move the fixture advice to TC-622's Fixtures line, or to the AC-1 wording, which already declares the Int and Boolean stand-ins. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:111-114 |

## Verdict

Two low findings. The new FR-188 weights statements and the FR-188 undecided statement are well-formed If/then unwanted-behaviour EARS. The FR-187 negative-reward statement keeps its When event form. The FR-187 undecided bullet adds the only new grammar warnings (FND-001).
