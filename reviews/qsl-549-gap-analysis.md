---
id: SR-13431
title: "Gap analysis of quire-spec-language PR #1089: FR-149, FR-144-AC-7 and TC-556"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ba1d01ad278d124405c1de0a1c1e1b6df56cfb60; PR #1089: FR-149 (Description, Inputs, Outputs, Behavior, AC-1 to AC-5), FR-144-AC-7, TC-556 steps 1 to 6, against qsl-replay/src/certificate/simulation.rs, qsl-replay/src/model_check/refinement_settle.rs, qsl-analyze/src/model_check/refine.rs, qsl-analyze/tests/it/{refinement_certificate,refinement_budget}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-556
    type: reviews
---
# SR-13431: Gap analysis of quire-spec-language PR #1089

## Summary

Ticket: QSL-549. Plan completion: not assessed (planless). Each AC in scope
was traced to its tagged tests, and each test was read and run at the
reviewed head (all pass). The FR-149 Behavior bullets were traced to code.

- FR-149-AC-1 / TC-556 step 1: `the_engines_certificate_is_accepted`.
  Asserts one position per stored product state, equal list lengths, and
  `Accepted`. Covered.
- FR-149-AC-2 / step 2: `a_changed_taken_is_rejected`,
  `a_post_index_at_the_initial_position_is_rejected` and
  `a_removed_edge_is_rejected` assert the exact rule and locus.
  `a_rejected_certificate_settles_inconclusive` asserts the settlement.
  Extra mutation tests cover every field. Covered.
- FR-149-AC-3 / step 3: `a_false_relation_is_rejected_with_its_step_verdict`
  asserts `StepFails` at the `commitB` edge from `(1, 0, f, 0, t)`, with
  `AbstractStepRejected{inc, Postcondition}`. Covered.
- FR-149-AC-4 / step 4: `a_transition_limit_stops_the_check` (and
  `max_states`) and `another_refinement_is_refused_and_a_check_repeats`.
  Covered, but the stale case is by declared name only (FND-002).
- FR-149-AC-5 / step 6: `a_certificate_checked_over_the_budget_stops_not_rejects`
  asserts `Accepted` under the larger meter and `Stopped(EvaluationMeter,
  B)` under B. Covered.
- FR-144-AC-7 / step 5: the `Certified` row and the `CertificateRejected`
  row are covered. The `ensure` row (`proved`, `Uncertified`) is not
  (FND-001).
- FR-149 Behavior. Production: `refine.rs` `certificate()`. Initial
  states, Successors, Steps, Order, Limits, Purity and Settlement: in
  `simulation.rs` and `refinement_settle.rs`. Identity: partial (FND-002).
  The SR-13430 code-review findings cover the defects in these bullets'
  implementation (horizon certificate, untested rules, meter
  classification).
- No production code in the diff lacks an owning requirement.

## Verdict

Mergeable on coverage once the two gaps below have owners. Both are
deferrals the coder reported on QSL-549 (2026-10-20 comment), and both are
real unmet spec text. Each needs a recorded follow-up (a ticket or a spec
amendment) before QSL-549 closes. Neither affects soundness: the checker
recomputes every step, so a stale certificate can only be rejected, never
falsely accepted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-144-AC-7's second clause, "with its `ensure` row it settles `proved`, `Uncertified`", is not implemented or tested. FR-143's liveness half and the item-level combination of the two halves are not built. `settle_refinement_safety` on the full `CasRefinesCounter` (with `ensure`) now settles the safety half `Proved{Exhaustive, Certified}`, which is correct for that half alone, and no item record exists. The three `FR-144-AC-7` tags therefore claim an AC that is only partly covered (see SR-13430 FND-009). Record the `Uncertified` row as remaining work on a follow-up ticket for the liveness or item-combination slice. | qsl-replay/src/model_check/refinement_settle.rs:20-23; qsl-analyze/tests/it/refinement_certificate.rs:546-653 |
| FND-002 | medium | FR-149 Identity says the checker SHALL refuse `stale_dependency`/`content-mismatch` when the refinement node, either `package_id` or the obligation identity differs. The certificate carries only the refinement's declared name (`String`, not `NodeKey`), so a certificate for an edited package under the same refinement name is checked rather than refused. The FR-282 package binding (`Row::bound`) that the other EN-1 certificates settle through is not applied. The module doc says why: S4 emits no refinement, so no `package_id` or obligation identity exists. Spec text is unmet, but this is not a soundness gap. Defer, with a follow-up ticket tied to the refinement emit (FR-145 facade), or amend FR-149 to name the interim identity. | qsl-replay/src/certificate/simulation.rs:28-33; qsl-replay/src/certificate/simulation.rs:181-186; qsl-replay/src/certificate/simulation.rs:82-84 |

## New findings (disposition pass 2)

Round 2 is against the merged FR-149 (#1095), whose AC-5 to AC-10 are new. AC-5 (a) to (d), AC-7 and AC-8 have tagged, passing tests. AC-6 and AC-10 (cancellation) are covered by tests tagged `TC-556` alone.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | Two merged FR-149 ACs have no test. FR-149-AC-9: FR-144 refuses a certificate whose `concrete_package` or `obligation` differs, before the checker runs. It needs the package and obligation identity that FND-002 defers. FR-149-AC-10's second half: an FR-120 expansion stop that names no limit, produced by a state constraint whose family evaluation refuses, gives `Stopped(ResourceExhausted)` and settles V-7. The code path exists (`expansion_halt`) but is untested. Record both as remaining work on QSL-549. | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md; qsl-replay/src/certificate/check.rs:131-150 |
| FND-004 | low | Two tests that assert an AC exactly do not carry its tag: `a_terminal_state_lists_its_stutter_step` is FR-149-AC-6, and `a_cancelled_check_stops` is FR-149-AC-10's cancellation half. Both are tagged `TC-556` only, so the computed matrix shows AC-6 uncovered. Add `FR-149-AC-6` and `FR-149-AC-10` to their tags. | qsl-analyze/tests/it/refinement_certificate.rs:690-694; qsl-analyze/tests/it/refinement_certificate.rs:766-770 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred | Round 1 (45e3ab64a): recorded as remaining work on QSL-549. FR-144-AC-7's `Uncertified` row needs FR-143's liveness half and the item-level combination of the halves, neither built in this slice. The AC-7 tags now sit only on the `Certified` and `CertificateRejected` tests. |
| FND-002 | deferred | Round 1: recorded as remaining work on QSL-549. FR-149 Identity by `package_id` and obligation identity needs a refinement emit (FR-145 facade), which S4 does not produce. Not a soundness gap: the checker recomputes every step, so a stale certificate cannot be falsely accepted. |
| FND-003 | deferred | Round 3 (85274c320): remaining work on QSL-549. FR-149-AC-9 needs the package and obligation identity deferred as FND-002. FR-149-AC-10's expansion-stop half (`Stopped(ResourceExhausted)` from a refused state-constraint family) has a code path (`expansion_halt`) but no test. |
| FND-004 | fixed | Round 3: `a_terminal_state_lists_its_stutter_step` is tagged `("TC-556", "FR-149-AC-6")` and `a_cancelled_check_stops` is tagged `("TC-556", "FR-149-AC-10")`. The AC-10 tag covers its cancellation half only, as FND-003 records. |
