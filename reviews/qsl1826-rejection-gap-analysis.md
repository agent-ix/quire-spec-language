---
id: SR-20261109172912000006
title: QSL-1826 probability rejection gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; QSL-1826; qsl-analyze/src/analyze/exact.rs, qsl-analyze/tests/it/exact_probabilistic_item.rs,
  qsl-replay/src/certificate.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-201
  type: references
---

## Summary

Ticket: QSL-1826. Scoped review of probability inequality/threshold rejection settlement; no introduced defect found.

## Verdict

CONDITIONAL for this slice. Computed matrix unavailable; manual scoped mapping is complete. Mergeable under the AGENTS owner policy separating incomplete assurance from engineering delivery. Other QSL-1826 work remains open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201-AC-6
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'Rejection rules and loci: AC-1''s certificate with `4/5` replaced by `9/10`
    is rejected `InequalityFails` at the `ProbabilisticProductState` of that first
    state; AC-2''s `LongRun` certificate with the changed bias, `InequalityFails`
    at the `down` state; AC-3''s `Lower` certificate with its ranking omitted, `Malformed`
    at `CertificateMember("ranking")` before any re-enumeration; AC-2''s `Lower` certificate
    with one named non-target state''s rank removed, `RankMissing` at it, and with
    that rank raised to the rank of its only positive successor, `RankBroken` at it;
    AC-1''s accepted certificate offered '
- id: FR-203-AC-3
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: FR-201-AC-1's rejected certificate settles `inconclusive`, `CertificateRejected`;
    FR-202-AC-2's prefix-duplicated path set settles `inconclusive`, `ReplayParity`;
    FR-199-AC-3's zero-weight variant settles `unsupported`, `ZeroWeightComponent`.
    None of these records carries a `measured` value.
- id: FR-281-AC-4
  path: spec/functional/FR-281-analyze-claims-with-in-process-engines.md
  role: examined
  excerpt: 'With the test engine''s certificate altered in one member before checking,
    the item settles `inconclusive` with cause `CertificateRejected`; with an engine
    that reports a proof no core certificate checker covers (EN-1 under partial-order
    reduction, ADR-021 RV-1), the item settles `proved`, category success, with the
    certification `uncertified` naming that engine, and the same item unreduced settles
    `certified`. This clause is observed at `analyze` over an engine set holding EN-1:
    the reduced item is an EN-1 item whose request selects partial-order reduction
    (FR-159), so EN-1 reports `HoldsReduc'
```

## Coverage

Reviewed frozen df76c1c4e39b95b18ba897cb28ac93b272c27234 against origin/main 7e0fbcdf9e94f3f9e19bc4537eb3fe94960dbe0c. Rust lane checked API, implementation, safety, wire and testing rules; repository AGENTS/CLAUDE/README/CONTRIBUTING, workspace lints and deny configuration examined. No CI, dependency or gate changes. No applicable AssuranceProfile found.

Three production behaviors inventoried: typed inequality/threshold mapping, inconclusive settlement without evidence, and full-key base64url serialization. All have FR-201/FR-203/FR-281 ownership; zero introduced unowned behaviors or source/test stubs found in this slice. Runtime reconstruction failures retain failed. Malformed remains failed and explicitly tested; it is a tracked pre-existing deviation, not fulfillment of all rejection obligations.

The tamper seam changes only solver certificate input, then runs the actual independent checker and adapter. Assertions check exact typed cause, absence of evidence, complete key including binding, rule spelling and serialized OutcomeItem. Threshold test covers settlement directly, not full adapter threshold rejection or its serialized outcome. The existing checker unit covers ThresholdMissed generation. The expected base64 spelling uses existing ModelKey encoding as an independent locus-serialization oracle; it is not an independent base64 implementation.

Inspected actual QSpec proposals/backend-provider-v1/schema.json definitions ProbabilityProductStateLocus and ProbabilityProductKey and the probability CertificateRejected branch (line 1071): the new product_key object and unpadded full-octet encoding match that shape and allowed rule pair. Backend envelope cause uses type; QSL OutcomeItem intentionally uses its existing kind projection. No claim of executing a complete QSpec schema validation over this outcome.

Validation evidence inspected: coder final adapter log 10 passed, threshold unit 1 passed, touched/all-target Clippy complete with warnings denied, formatting log empty with reported exit 0, docs complete (pre-rebase). Reviewer git diff --check passed. No concern justified duplicating these runs; aggregate gate, cargo-deny and whole-workspace qualification not run by reviewer.

Reconciliation: manual scoped tag/criterion mapping only. Installed quire matrix --help exposes repository scope but no criterion/path filter; prior whole-repository computation was stopped after seven minutes without output. Per dispatch and owner policy no duplicate broad computation was run. Computed matrix counts and evidence-store status unavailable. This is not repository-wide assurance or full AC coverage: examined excerpts identify only this slice. FR-201-AC-6 includes ranking/long-run/quantile/malformed obligations outside this slice; FR-203-AC-3 includes witness replay/zero-weight and FR-281-AC-4 includes EN-1 obligations not discharged here. No optional semantic-review campaign run.

Plan completion: not assessed

## Scoped bindings

These are correct bindings to the examined settlement/rule/locus portions only; they do not discharge the full compound criteria.

```yaml
bindings:
- test_id: a_rejected_certificate_never_settles_proved
  ac_id: FR-281-AC-4
  trace: correct
- test_id: a_rejected_certificate_never_settles_proved
  ac_id: FR-203-AC-3
  trace: correct
- test_id: a_rejected_certificate_never_settles_proved
  ac_id: FR-201-AC-6
  trace: correct
- test_id: threshold_rejection_retains_its_rule_and_complete_product_key
  ac_id: FR-201-AC-6
  trace: correct
- test_id: threshold_rejection_retains_its_rule_and_complete_product_key
  ac_id: FR-203-AC-3
  trace: correct
```
