---
id: SR-12934
title: "Gap analysis of PR #970 round 4: FR-310, FR-137, FR-147 refinement reshape (QSL-543)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d22cb40341ae6df53c4e534d2b888124e6476b06; PR #970; spec/functional/FR-310, FR-136, FR-137, FR-138, FR-147; spec/test-cases/TC-542, TC-552, TC-889; qsl-forms/tests/it/refinement_forms.rs, qsl-semantics/tests/it/refinement_declarations.rs, qsl-cst/tests/it/protocol_memory_refinement.rs"
review_set: subset
evaluated_revision: "d22cb40341ae6df53c4e534d2b888124e6476b06"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-310
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: reviews
---
# Gap analysis of PR #970 round 4: FR-310, FR-137, FR-147 refinement reshape (QSL-543)

## Summary

Ticket: QSL-543 (QSL-874 for `fair strong`). Planless. Plan completion: not assessed. Round 4 gap analysis of the reshaped branch at d22cb4034 (scope: `git diff origin/main...HEAD`). `quoin matrix --json` (quoin 0.28.3) reports these as tagged: FR-310-AC-1..4, FR-136-AC-1..3, FR-137-AC-1 (spec-side binder only), AC-2, AC-4 and FR-147-AC-2. These are untagged: FR-136-AC-4, FR-137-AC-3, FR-138-AC-1..5, FR-147-AC-1, AC-3 and AC-4. All tagged tests ran green.

Recorded gaps. These are not defects of this PR, which does not claim them, and each is refused rather than ignored where source can spell it:

- FR-147-AC-1, AC-3, AC-4 and TC-552 steps 1, 3 and 4 (the abstract protocol side, abstract_internal, the refinement record, the liveness half). These are blocked on STD-182 (the QSpec header production) and STD-198/STD-199 (where the profile's internal list lives). S2 cannot build `Protocol` from source, and S3 refuses a hand-built one as unsupported.
- FR-136-AC-4 (coverage over a concrete protocol subject) and FR-137-AC-3 (scheduler constraints in F_C). These are blocked on STD-198, because no requirement says how a header selects a concrete protocol subject. Over a model subject, a keyword or memory step row refuses missing-name.
- FR-138-AC-1..5 (history rows). S1 and S2 build them, and S3 refuses each with RefinementCause::Unsupported. The code choice is open in STD-199 item 4 (see code review SR-12933 FND-001).
- The `seq`/`only` half of FR-310-AC-2 waits on FR-139 (population-valued forms). The grammar has neither form.
- The node-identity halves of FR-135-AC-4 and FR-137-AC-1, and the Dependency half of FR-135-AC-1, wait on S4 emission. These are carried over from earlier rounds and are correctly tagged TC-only.

## Verdict

The fairness rows that were built are fully covered. FR-137-AC-2's two duplicate pairs are each asserted with the code, the cause, the refusal region and the first row's span. The weak/strong distinctness and the ensure-set duplicate are asserted too, as is FR-137-AC-4's Strong Each inc and Strong Whole retryA. FR-310-AC-3 and AC-4 are covered at S2, and FR-375-AC-10's kindless refusal is covered at S1 and S2. There are no stubs, and every new production symbol in qsl-forms/src/refinement.rs is reached by a test.

Two medium findings: two AC tags claim more than their tests cover (FND-001), and the ticket's added FR-361-AC-4 item is unaddressed although its blocker has landed (FND-002). One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two criteria are tagged although most of each is unimplemented, so `quoin matrix` reports both as tagged and nothing would flag them when the blocking work lands. FR-147-AC-2 has five refusal variants. protocol_step_rows_refuse_over_a_model_subject covers only `-> node Spec::Twice::a1(self)` in CasRefinesCounter. The other four (`abstract Spec::Thrice`, a list naming `zz`, the list/row conflict, a list in CasRefinesCounter) wait on STD-182 and STD-198. FR-310-AC-2 is tagged on an_open_argument_builds_open and step_sources_and_node_targets_build_as_written. Neither builds RingIsQueue's `items` row with `seq` and `only`: the grammar has neither form, because FR-139 is not implemented. Neither builds `Protocol(Spec, Twice)` either (STD-182). This PR's own round-0 fix (gap-analysis SR-12815 FND-001) set the rule: tag only the TC (`TC-552`, `TC-889`) while part of an AC is unbuilt. Fix: drop FR-147-AC-2 and FR-310-AC-2 from those trace attributes and keep the TC ids. | qsl-semantics/tests/it/refinement_declarations.rs:1209-1258; qsl-forms/tests/it/refinement_forms.rs:221-304 |
| FND-002 | medium | QSL-543 has an added acceptance item: FR-361-AC-4 is checked for the refinement form, so that a refinement stays ungated by the model-profile gate. The PR description defers this because 'the gate (#964) is not on main'. That is no longer true. #964 (2350bf839) is in this branch's merge base 4978f722a, and model_gate::refusals runs in assemble.rs:2527. Every refinement test uses a `quire.value.complete/v1` header (unit_text), so the value-profile half is only exercised implicitly. No test checks a refinement under `quire.model.complete/v1` (refused only as RefinementNotEmitted) or asserts that the gate leaves it alone. The rest of the PR description is also stale: it says history rows and FR-310-AC-2 are not parsed or tagged, and both now are. Fix: add the FR-361-AC-4 refinement case (model profile: only RefinementNotEmitted; value profile: no item-5 family), and refresh the PR description. | qsl-semantics/tests/it/refinement_declarations.rs:410-415 |
| FND-003 | low | The forms fixture is not ADR-020 §8's CasRefinesCounter. It has one `assume` row where §8 has six (beginA..retryB), and `peek -> any` where §8 has `peek -> stutter`. The qsl-semantics fixture (refinement_declarations.rs:536-555) has all six. So FR-310-AC-1's 'assume rows in source order' is checked for one row only. Separately, FR-310-AC-4's `rational(1, 2)` is asserted only as `HistoryLiteralForm::Exact(_)`, so a builder that dropped or swapped the numerator and denominator would pass. Fix: reuse §8's rows and assert the order of the six assume rows. Assert the Exact form's kind and its two integers. | qsl-forms/tests/it/refinement_forms.rs:23-36, 351 |
