---
id: SR-13731
title: "Merge-delta gap analysis of quire-spec-language PR #1137 (QSL-563): FR-214, FR-147 and FR-136 coverage after merge 3ddbcdae4"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3ddbcdae4; merge-only delta of 3ddbcdae4 over d2ca261a0 and 2f0227a5e: qsl-semantics/src/check/{mod,refinement}.rs, qsl-semantics/src/check/refinement/{protocol,step_label}.rs, qsl-semantics/src/check/refinement/protocol/tests.rs; spec/functional/FR-214, FR-147, FR-136"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-659
    type: reviews
---
# SR-13731: Merge-delta gap analysis of quire-spec-language PR #1137 (QSL-563): FR-214, FR-147 and FR-136 coverage after merge 3ddbcdae4

## Summary

Ticket: QSL-563. This is the gap analysis of the merge commit only. Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0, at 3ddbcdae4. Plan completion: not assessed.

Matrix rows for the ACs the merge touches:

| AC | Status | Honest after the merge? |
| --- | --- | --- |
| FR-214-AC-1, AC-2, AC-3, AC-5 | tagged | yes; unchanged by the merge apart from `StepLabel.step` |
| FR-214-AC-4 | tagged (protocol/tests.rs:764) | first sentence only. The second sentence (an abstract protocol's internal list) was 'not built' at d2ca261a0 (SR-13658). The merge brings in main's FR-147 internal list, so it is now built and still untested (FND-001) |
| FR-147-AC-1, AC-2, AC-4 | tagged (qsl-semantics tests/it/refinement_protocol.rs, main) | yes, for a model concrete side; no test with a concrete protocol subject |
| FR-136-AC-8 | tagged (protocol/tests.rs:712, :1399) | yes, for a model abstract side; the combined case is outside it (SR-13730 FND-003) |

Behavior the merge added and the FR text that owns it:

- FR-214 Behavior lines 61-65: `Visible{label}` for a row whose target is an abstract node, carrying 'its node'. This is `LabelStep::Node`, and no test asserts it (FND-002).
- FR-147 lines 104-110: a node right side typed over the concrete step. On a protocol row with an applied operation, this goes through main's `node_target`. On a protocol row without one, the merge refuses `ProtocolStepApplication`. That refusal matches the PR's existing refusal for an application on such a row (SR-13657 FND-005). No FR names it. It is an honest not-built refusal for a binder record that STD-249 has not specified, and no test covers it.

## Verdict

**Gaps found.** One medium (FND-001) and one low (FND-002). The test SR-13730 FND-002 proposes closes both.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-214-AC-4's second sentence, 'An abstract protocol whose internal list names `fork` as internal gives that abstract step internal, and its other abstract steps visible', is buildable since the merge and has no test. The only test tagged FR-214-AC-4 (`a_row_target_gives_its_observation`) asserts the first sentence, and its doc says the second half 'is not built', which the merge made false. Fix: a test over a concrete protocol subject and an abstract protocol with a `parallel` whose internal list names `fork A::<protocol>::<par>`, asserting `abstract_internal` holds exactly that class and a node row to another abstract node is `Visible`, tagged `#[trace("TC-659", "FR-214-AC-4")]`. Or narrow the AC on QSL-563 if the lead keeps it out of this slice. | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:81, qsl-semantics/src/check/refinement/protocol/tests.rs:758-764 |
| FND-002 | low | FR-214's 'node' half of `Visible{label}` (Behavior lines 61-65) has no test. `observation`'s `StepTarget::Node` arm can return `Either` with every test green. Fix: assert `Visible{LabelStep::Node{..}}` in the FND-001 test (FR-214-AC-4's first sentence covers observations). | qsl-semantics/src/check/refinement/step_label.rs:70-83 |
