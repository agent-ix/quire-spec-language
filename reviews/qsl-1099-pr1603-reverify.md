---
id: SR-60035
title: "Re-verification of QSL PR #1603 after the SR-60029 fix cycle (QSL-1099)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3b37dfc7e0e8bfb8ace02b1432adf3eea509a78e (qsl-replay certificate closure and settle, qsl-analyze component_certificate and tso_fairness tests; FR-339-AC-8, TC-526 step 7)"
review_set: subset
date: '2026-10-26'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-526
    type: reviews
---
## Summary

Ticket: QSL-1099. Verdict: PASS. Focused re-verification, no second make ci (string-edge red on main from #1595 is inherited).

FND-001 (SR-60029) fixed. Re-ran my forged-closure probes: the weak-each example (letters never hold; closure grown from 36 InitialMissing and SuccessorMissing rejections of a components-free probe) and the Spin protocol claim (8 states). check_closure and check_protocol_closure return LivenessNeedsComponents; settle_model_check and settle_protocol_check over Holds{Closure} settle Failed; the same closure with real components is NotPartition. No other route: the only ProofCertificate constructors are the closure producers (check.rs, deadlock explore.rs, neither a TP-4 item) and the liveness Component producer; settle_model_check and settle_protocol_check are the only consumers and both dispatch through the guarded checkers; no wire reader builds a ProofCertificate; a TP-4 clause that counted progress lowers to safety is correctly still a closure proof.

FND-002 fixed. Mutations on 3b37dfc7e, each reverted: NotLiveness guard off is killed by components_offered_for_a_safety_clause_are_refused and the Spin test; LivenessNeedsComponents guard off is killed by a_closure_certificate_never_proves_a_liveness_claim and the Spin test; package-id compare off in check_protocol is killed by the Spin test. component_work_charges_no_budget_and_the_closure_check_stops_first still tests its name: the closure-alone baseline is now a certificate with no components (check_closure refuses TP-4), which stops at the same meter limit, value and setting and polls the same count before any component rule, and the full certificate is accepted under the exact budgets and polls beyond that point.

Gates: qsl-analyze it (component_certificate, tso_fairness, model_settle, liveness): 60 passed; qsl-replay lib 878 passed; clippy -D warnings clean on qsl-replay and qsl-analyze.

Lows: FR-126-AC-18 tests now tag TC-521 only (honest); is_live duplicate removed (scc.rs uses qsl-eval's); settle docs updated.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | One mutation survives: weakening the NotLiveness guard from all-instances-liveness to any-instance-liveness. All instances of one clause share a translation kind, so the mixed case looks unreachable; informational. | qsl-replay/src/certificate/closure.rs | correct-requirement-no-evidence |
| FND-002 | low | FR-339-AC-6 still words the meter-stop equality as FR-338's closure check of the same closure, which can no longer be called for a TP-4 item; reword to the components-free certificate the test uses. | spec/functional/FR-339-check-an-en-1-component-certificate.md | wrong-requirement |
| FND-003 | low | Counted/interval protocol TP-4 (translate versus translate_counted) fails closed, never open: the checker recomputes the product graph and partition under translate_counted, so a certificate built from the uncounted automaton is rejected (SuccessorMissing or key ShapeMismatch) or the producer faults; an unsound accept would need the checker's own graph to pass every witness rule. Result is failed or inconclusive for a class that was unsupported before. Deserves a follow-up ticket (completeness, not soundness). | qsl-analyze/src/model_check/liveness.rs, qsl-replay/src/model_check/item.rs | correct-requirement-no-evidence |

## Verdict

PASS. No blocking or medium finding remains.
