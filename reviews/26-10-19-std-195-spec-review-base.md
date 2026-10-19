---
id: SR-12926
title: "Base checklist review of STD-195 FR-125 instancing (PR #1013)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@9a8a05e9d; spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md, qsl-eval/src/model_subject.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-520
    type: reviews
---
# SR-12926: Base checklist review of STD-195 FR-125 instancing (PR #1013)

## Summary

Ticket: STD-195. PR quire-spec-language#1013 at head 9a8a05e9d, scope `git diff origin/main...HEAD` (FR-125, TC-520, the `ModelLetters::bindings` doc comment). Checked: id formats, AC-to-TC coverage for the new FR-125-AC-8 to AC-10, TC-520 steps 7 to 9 and their expected results, the QSL-proposed marking against Linear STD-196, the hash/pin antipattern, and id collisions with open PRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | AC-8's `over (x: Thing)` case, where `Lamp` and `Switch` are both subtypes of `Thing`, asserts one instance per key. The Behavior rule the PR adds instances a key only for a population that covers `T`, and it defines covering by FR-104: `T` or a supertype of `T` is one of the member types. `Thing` is a proper supertype of both members, so by that definition the population does not cover `Thing` and yields no instance. AC-8 and TC-520 steps 7 and 8 (the `Thing` rows) therefore contradict the rule they test. AC-8's last sentence ("neither conform to nor are supertypes of `T`") uses a two-way relation that the Behavior statement does not. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:121-124,213; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md:37-42,66-72; spec/functional/FR-104-check-state-clauses.md:263-266 |
| FND-002 | medium | AC-9 does not say which of the fixture's member types is the canonical one (least in ascending `DeclarationKey` order). The binding's expected type, and so the AC's oracle, is left to fixture naming. TC-520 step 8 assumes `Lamp` is canonical. Name the order in the AC, for example "`Lamp`'s `DeclarationKey` orders before `Switch`'s", or state the binding type the test expects. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:214; spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md:40-42 |

## Verdict

Not mergeable while FND-001 stands: AC-8 cannot pass against the rule it tests. Clean: ids FR-125-AC-8 to AC-10 are new and are used by no open PR (no open PR touches FR-125, TC-520 or `model_subject.rs`). Each new AC maps to a TC-520 step, and TC-520's scope line lists them. AC-10's fixture (`[d, c]`, `[b, a]`) separates universe-list order from key order within a population. The QSL-proposed marking names STD-196, and STD-196 asks QSpec to rule on all four points. The diff adds no hash, digest or pin.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 01f5d081c: AC-8 now uses 'holds T' (a member type is T or a subtype). `Lamp` and `Switch` are subtypes of `Thing`, so `over (x: Thing)` yields one instance per key, which is consistent with the Behavior rule; TC-520 step 7 matches. |
| FND-002 | fixed | 01f5d081c: AC-9 and TC-520 step 7 state that `Lamp`'s `DeclarationKey` orders before `Switch`'s; the binding type no longer depends on it, because x reads the state's own reference. |
