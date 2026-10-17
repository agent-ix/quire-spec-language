---
id: SR-12041
title: "QSL-817 PR #920 gap analysis: the state-model seam against FR-120 and FR-106"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d459e9ddf224ef9c70858516199c52dcc6d659b8; qsl-semantics/src/model/state.rs, qsl-semantics/src/model/observation/initial.rs, qsl-semantics/tests/it/model_state.rs, spec/functional/FR-120-simulate-a-checked-package-s-state-family.md"
review_set: subset
---

## Summary

Ticket: QSL-817. PR quire-spec-language#920. This is a planless gap analysis of the seam against FR-120 ("Initial states" item 2, "Frame and observation seam") and FR-106 (checks 1, 3, 4 and 6 to 8, plus the observation builders). Plan completion: not assessed. AC-4 state-key encoding, AC-7 universe checks, AC-12, successors, invariants and effects belong to QSL-493 and are out of scope here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Seam tests carry `Trace:` tags for FR-120 ACs they do not verify. Each of those ACs is a ModelSystem or `explore_model` outcome that QSL-493 has not built. `a_changed_state_observes_its_changed_field` is tagged AC-5 (an exploration with invariant findings). `the_public_frame_decision_refuses_a_change_outside_the_frame` is tagged AC-1 (successor sets). The two admission tests are tagged AC-4 (state-key bytes). The wrong-model, dangling-reference and incomplete test is tagged AC-7, although AC-7 names none of those three causes. As a result, `quoin matrix` shows AC-1, AC-3, AC-4, AC-5 and AC-7 as covered before ModelSystem exists, which inflates coverage and hides QSL-493's gaps. | qsl-semantics/tests/it/model_state.rs:204, qsl-semantics/tests/it/model_state.rs:287, qsl-semantics/tests/it/model_state.rs:345, qsl-semantics/tests/it/model_state.rs:415, qsl-semantics/tests/it/model_state.rs:529, qsl-semantics/tests/it/model_state.rs:566 |

## Verdict

FIX, because of FND-001. Remove the AC-1, AC-4 and AC-5 tags, and the AC-7 tag on the wrong-model, dangling and incomplete test. Keep AC-7 on `an_initial_snapshot_that_is_not_a_current_initialization_refuses`, which asserts AC-7's `wrong-observation`, `wrong-anchor` and `wrong-value-kind`/`anchor` clauses exactly. AC-3's "delta is the one the frame decision returns" clause is only partly served by the seam test.

Every in-scope seam behaviour has a test, and the oracles assert exact codes, causes and values: admission into typed objects and the anchor, an absent optional field, wrong observation, wrong anchor, an `other` anchor kind, wrong model, dangling, incomplete, observation against FR-106, a changed field, a missing required field faulting, frame delta and frame refusals, and the population index. The observation-equality test does not cover relationship extents (SR-12040 FND-001).

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 985bfb519c877d26f91d6296b4093fbd24524c81: the AC-1, AC-4 and AC-5 tags and the AC-7 tag on the wrong-model, dangling and incomplete test are removed. The only tags left are AC-7 on `an_initial_snapshot_that_is_not_a_current_initialization_refuses` and AC-3 on `the_public_frame_decision_returns_the_created_and_deleted_sets`. |
