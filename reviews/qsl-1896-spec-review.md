---
id: SR-2026111321481803
title: "spec review of PR2701 FR-571 bounded-static game amendment"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md (the PR hunk and the text it must agree with), ADR-036 PA-16 to PA-19 and the slice 4 row, TC-1411; PR2701 76d8a5a1ce34b00a49bcdce11ffe1decf5e92d2f"
review_set: subset
---

## Summary

Ticket: QSL-1896 (ADR-036 slice 4). Base spec review, covering integrity and testability, of the one changed spec file and the text it must agree with. The PR adds three Behavior bullets to FR-571 that state the `bounded-static` profile fact: every step is an environment move, and an owner node has one branch. The fact is correct (SR-2026111321481801 Summary). The PR does not carry it into the criteria and the ADR, so FR-571-AC-6 is untestable and ADR-036 PA-16 to PA-19 still describe free owner moves. Exact replacement wording follows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-571-AC-6 cannot fail in V1. In `bounded-static` no owner choice exists to avoid a history or to be cut, so both `inconclusive` cases are vacuous and TC-1411 step 6 cannot be carried out. Replace the row with: "FR-571-AC-6 \| In the `bounded-static` profile no arena step is an owner's branch selection: a `choice` decision reads only its owner's received records, protocol captures and constants (FR-218), and `qsl-eval` enters the one case whose guard holds. So no history is avoidable by an earlier owner choice and no owner choice is cut, PA-18's two `inconclusive` cases do not arise in V1, and an `EnvironmentDefeat` has exactly one entry in `defeats_per_branch` (PA-16, PA-18). \| Test (TC-1411)". Replace TC-1411 step 6 with "Decide the step 2 protocol that one admitted loss defeats and read `defeats_per_branch`", with expected result "Step 6: one entry; no owner choice exists to avoid a history or to be cut, so FR-571-AC-6's cases do not arise." Tag `an_admitted_loss_is_an_environment_defeat` with FR-571-AC-6. | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:123 |
| FND-002 | medium | The new bullet contradicts text the PR leaves unchanged. ADR-036 PA-16 still names "each choice owner's branch selection" as a move and says slice 4 adds owner-move classification to `qsl-eval`. PA-17, PA-18, PA-19 and the slice 4 row ("AND/OR evaluation with environment and owner moves") assume free owner choices. FR-571's own Outputs ("one committed branch per owner class"), observation bullet ("its own sends, receives and choices"), evaluation bullet, PA-18 bullet and AC-2 ("a defeating play per owner branch") do the same. Amend them as listed in the Verdict. | spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:144-147, 214; spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:57-58, 67-71, 99-108, 119 |
| FND-003 | low | The new bullet says a decision reads "what its owner received, the captures and the input (FR-218)". FR-218 refuses a guard's read of the `over` input; the third source is constants. The bullet also omits the initial state as a source of the branch, through captures. Replace with "reads only what its owner received, the protocol captures and constants (FR-218), and `qsl-eval` steps the one case whose guard holds, so the branch is fixed by earlier environment moves or the initial state". | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:73-79 |
| FND-004 | medium | The bullet "The declared shared facts are common to every history of every role and so never split a class" makes premise group (c) inert without defining what its strings name, and gives (d) the same treatment. Two implementers would read a non-empty set differently. State instead: "A non-empty role-knowledge or environment set is refused `invalid_runtime_input`/`invalid-value` (or answered `unsupported`) until the strings these groups hold are specified; declared-empty decides and absent is `inconclusive` (PA-8)." | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:87-91 |

## Scope and evidence

- FR-571-AC-6 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): A refutation by indistinguishable histories is not claimed when an earlier owner choice could avoid either history, and a refutation by environment defeat is not claimed when the defeat depends on a choice the bound cut; each is inconclusive (PA-18). (examined)
- FR-571-AC-2 (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): ... EnvironmentDefeat with a defeating play per owner branch of the first lost owner node. (examined)
- FR-571 Behavior, profile bullet (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): Every step of the bounded-static arena shall be an environment move ... an owner node has one branch. (examined)
- FR-571 Behavior, premise bullet (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): The declared shared facts are common to every history of every role and so never split a class. (examined)
- FR-571 Behavior, open-range bullet (spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): A strategy that passes the walk under an open range depends on the cut ... a refutation found among the explored counts stands. (examined)
- ADR-036 PA-16 to PA-19 (spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md): game, evaluation, forced refutation, witness. (context_only)
- TC-1411 (spec/tests/TC-1411-realizability-returns-a-replayable-strategy-or-a-forced-refutation.md): steps 1 to 6. (context_only)
- FR-218 decision eligibility (spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md): send payloads never eligible; over read refused; captures readable. (context_only)

## Verdict

The open-range bullet is correct: a refutation among explored counts is a real admitted play, and a pass under a cut is `inconclusive`. Exact amendments for FND-002:

- ADR-036 PA-16, replace "Two kinds of move: the environment's (channel outcomes per PA-7, admitted external events) and each choice owner's branch selection." with "Two kinds of move: the environment's (channel outcomes per PA-7, admitted external events, the record a send carries and the order in which threads step) and each choice owner's branch selection. In the V1 `bounded-static` profile every move is the environment's: a `choice` decision reads only its owner's received records, protocol captures and constants (FR-218; a `send` binder is never a decision fact, and `attempt` and `event` are outside the profile, PA-21), and `qsl-eval` enters the one case whose guard holds, so each branch is fixed by earlier environment moves or the initial state."
- ADR-036 PA-16, replace "A role's observation is its own sends, receives and choices, the outcomes visible to it (PA-7) and the group (c) facts." with "A role's observation is the records it sent and received, each with its control, the protocol captures every role holds (FR-218) and the group (c) facts; its own choice outcomes and the outcomes visible to it (PA-7) are functions of these."
- ADR-036 PA-16, replace "Slice 4 adds to `qsl-eval` only what the search needs to classify a step as environment or owner move and to read its events;" with "Slice 4 adds nothing to `qsl-eval`: each step's role, control and record are read through its existing transition accessors;".
- ADR-036 PA-17, append "In `bounded-static` an owner node has exactly one branch, so its value is that branch's value and the evaluation is the environment's conjunction throughout."
- ADR-036 PA-18, append "In `bounded-static` no history is avoidable by an earlier owner choice and no owner choice is cut, so neither `inconclusive` case arises (FR-571-AC-6): (i) is claimed for two non-empty, disjoint winning-action sets whose disjointness rests on no cut action, and (ii) records one defeating play."
- ADR-036 PA-19, after "with one committed branch per owner class" insert "(none in `bounded-static`, where no owner branch is free)". The slice 4 row: replace "AND/OR evaluation with environment and owner moves" with "evaluation over environment moves (no owner move exists in `bounded-static`, PA-16)".
- FR-571 Outputs: replace "with one committed branch per owner class," with "(in `bounded-static` an owner's branch is fixed, so the table holds sends, receives and ends only),".
- FR-571 observation bullet: replace "its own sends, receives and choices, the outcomes visible to it and the declared shared facts" with "the records it sent and received, each with its control, and the protocol captures; its own choice outcomes and those visible to it are functions of these".
- FR-571 evaluation bullet: append "In `bounded-static` an owner node has one branch." PA-18 bullet: append "In `bounded-static` neither the avoidable-history case nor the cut-choice case arises; a disjointness that rests on a cut action is still non-conclusive."
- FR-571-AC-2: replace "with a defeating play per owner branch of the first lost owner node" with "with a defeating play per owner branch of the first lost owner node, one play in `bounded-static`".

With these amendments the spec matches the code, and the PR is mergeable on the spec side.
