---
id: SR-20261109172912000070
title: "failure-domain review of finite expected-reward certificate specification"
type: SpecReview
analysis: failure-domain
scope: "QSL-1826 PR2627; FR-201 expected-reward profile and AC19-22; FR-282 profile extension and AC16-17; ADR-028 consequences; FR-195/196/198 context"
review_set: subset
---

## Summary

Reviewed frozen 88dfee5ecd760feebf204970f442083035d4d9cc against 5784ebb08306f10e400c5e050b2468978c3bb1d4. No additional actionable defect found within this method scope.

## Verdict

**PASS** — scoped direct analysis; no implementation qualification claim.

## Assurance Context

No AssuranceProfile document or review_policy was found under spec; no enforced workflow is applicable. Existing AGENTS guidance separates engineering from assurance. Full computed matrix and deterministic advisor were not repeated per dispatch because historically unavailable; coverage and evidence conclusions are manual and conditional. No active exception to the certificate semantic contract is granted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Analysis

Independent source recompilation and reward evaluation define the trust boundary. Full binding-bearing FR-196 keys, model-scoped reward identifiers and actual unit identities define equality. Original graph acyclicity precedes stopping, so cycles beyond B cannot hide. All original terminals must satisfy B, conservatively excluding zero-cost nonreachability infinity. Undefined/negative rewards, limits and cancellation retain nonacceptance. Source reward purity is already enforced by qualified semantics, not introduced as a callback.

## Validation and feasibility

Targeted normative validation succeeds (3/3 grammar-clean); actual Quire extraction contains the authored criteria. No DSL is expected in ADR-028. FR-195-AC-2 explicitly admits integer expectation thresholds 3, zero and negative rationals; qsl-semantics claim.rs uses Measured::Expectation rather than the probability (0,1) range branch, and existing semantic tests assert >=3 and exact 0.005 s to 5 ms conversion. Thus AC-19/21 threshold/source forms are feasible. Named reward expressions are qualified integer/quantity expressions; expected elapsed remains refused by this slice. Exact arithmetic manually gives min3/max4, equal workload7/2, and equal draws (2+6)/2=4 with 0.004s=4ms. New AC tests are prospective, not executed. No cargo or source edits.

## Examined units

```yaml
scope:
- id: FR-201-AC-19
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For an acyclic two-step source with named integer reward R, initial choices
    a (cost 1, then cost 2 to B) and b (cost 4 directly to B) have exact minimum 3
    and maximum 4. The minimum AtLeast 3 and maximum AtMost 4 documents accept at
    equality; thresholds 4 and 3 respectively reject ThresholdMissed. An equal-weight
    resolved workload has value 7/2 and accepts either bound at 7/2. Values above
    1 are accepted; a probability-style target value 1 rejects.
- id: FR-201-AC-20
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A draw-dependent authored reward of 2 or 6 with equal probability on an
    edge entering B gives value 4, including the entry cost. Changing the actual draw/cost
    association or using a fixed unit-step cost prevents acceptance. Initial B has
    value 0 even with an enabled positive-cost operation. A reached B absorbs a later
    positive-cost edge; a reached non-B terminal, including a zero-cost terminal,
    is explicitly unsupported under FR-198's nonreachability rule. A raw reachable
    cycle beyond B is unsupported before stopping.
- id: FR-201-AC-21
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A checked quantity reward expressed as 2 ms and 6 ms under equal draws
    gives 4 ms and accepts a source threshold 0.004 s after exact checked-unit normalization.
    Wrong reward name/model, target set, optimization, binding or full key prevents
    acceptance; changing the claimed reward/unit identity with the same numeric magnitudes
    prevents objective acceptance. Equivalent independently rederived costs under
    the actual admitted subject remain valid; source changes alone do not imply a
    staleness rejection. A dimensionally wrong source bound preserves the typing refusal.
    Noncanonical, duplicate, omitt
- id: FR-201-AC-22
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: The checker independently checks analyze's AC-19 to AC-21 exact documents
    with no qsl-analyze/provider dependency or reused solver graph/cost labels. Negative/undefined
    reward evaluation, unsupported profile/kind, exhausted graph/arithmetic/representation
    budgets, pre-cancellation and cancellation during expansion or reward evaluation
    preserve typed nonacceptance without truth values. Existing reachability/v1 and
    state-predicate/v1 canonical bytes/verdicts are unchanged.
- id: FR-282-AC-16
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Analyze emits the real expected-reward proof documents of FR-201-AC-19
    to AC-21. A fresh executable linking only qualified-core crates accepts their
    canonical bytes from the bounded source provision and actual S3 subject/claim
    binding, including random/workload source packages refused by S4. It independently
    rederives reward identity, exact unit normalization, semantic actions/draws/costs,
    original acyclicity and stopped Bellman equations, with no qsl-analyze/provider
    dependency, emitted-package bytes, fabricated replay request or solver graph.
    Source admission failures retain their typed refu
- id: FR-282-AC-17
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: For AC-16, source/document byte ceilings, state/edge/rational/key-representation
    budgets, pre-cancellation and cancellation during independent reward evaluation/graph
    expansion preserve the actual typed failure, named limit/consumption or cancellation
    without acceptance or truth values. Both older probability document profiles retain
    their canonical bytes and verdicts.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The checker SHALL resolve R under\n  the checked model and rederive every\
    \ step reward through qualified\n  transition/draw evaluation."
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: For FR-201's closed reachability, acyclic state-predicate and acyclic expected-reward
    profiles, the fresh-process facade SHALL
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'Original acyclicity and all original terminals

    being targets conservatively exclude FR-198''s positive-infinity cases,

    including zero-cost nonreachability.'
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: Build the probabilistic product
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: context_only
  excerpt: Decide unbounded reachability and expected rewards
- id: FR-195
  path: spec/functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md
  role: context_only
  excerpt: Check exact-only forms and route exact evidence
```
