---
id: SR-049
title: spec-review/ears-conformance review of exact acyclic until/always
type: SpecReview
analysis: ears-conformance
scope: agent-ix/quire-spec-language; QSL-1826; FR-201, FR-282, ADR-028 and FR-202-AC-14
review_set: subset
---

## Summary

Ticket: QSL-1826. Independent read-only review of PR 2622 at 683a61318ab99b7e2bc93c5687efad31276a0c1e.

The engine reports all four changed documents grammar-clean with zero grammar findings. Semantic review finds named actors and concrete responses for codec membership, admission, graph rebuilding, labeling, Bellman equality, rank checks and mismatch refusal. Conditional temporal and unsupported cases are observable. Multiple test assertions in an AC describe related adversarial fixture outcomes and do not create competing interpretations.

## Verdict

**PASS** — No concrete introduced defect found. Scoped specification is mergeable.

## Analysis

The worked until/always values are min(1/4,3/4)=1/4, max=3/4, and equal-weight average (1/4+3/4)/2=1/2. Thus 1/4<1/3, 3/4>2/3, 1/2<3/5 and 1/2>2/5 give the requested ThresholdMissed sides. For always, eventual-not-P probabilities are 3/4 and 1/4, so 1-max=1/4 and 1-min=3/4; failing to exchange extrema yields the specifically forbidden minimum 3/4. Infinite terminal P has event probability 1; until terminal A-and-not-B and any prefix failure have 0. FR-202 horizon-zero event probabilities remain 1 and 0: the accepting empty path has mass 1>1/2, and the rejecting empty path has mass 1>1-1/2. The source checker rejects threshold endpoints via QualitativeThreshold; the corrected interior threshold 1/2 is admitted.

No applicable AssuranceProfile exists in the reviewed spec tree (no type: AssuranceProfile or review_policy entry), so no enforced review or human acceptance gate is applicable. No cargo, make ci, Kani, CI or shared-repository operations were performed.

## Scope

```yaml
scope:
- id: FR-201-AC-1
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-2
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-3
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-4
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-5
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-6
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-1197) |
- id: FR-201-AC-7
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-8
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test (TC-636) |
- id: FR-201-AC-9
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-10
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-11
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-12
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-13
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-14
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-15
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-16
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-17
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-201-AC-18
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Test |
- id: FR-282-AC-14
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Test |
- id: FR-282-AC-15
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Test |
- id: FR-202-AC-14
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Test |
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For infinite-trace untimed `holds(A) until holds(B)` and `always holds(P)`

    over checked state predicates, the document codec SHALL admit a separate

    `"quire.probability-objective/state-predicate/v1"` profile. This does not

    reinterpret `reachability/v1`. The new objective SHALL contain exactly

    `profile`, `event`, `targets`, `failures`, `optimization` and `fairness`.

    `event` SHALL be `"until"` or `"always"`; optimization and empty fairness

    SHALL retain the closed reachability profile''s meanings. All other document

    members, canonical byte rules, exact rational representation, complete-key

    transpor'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Admission SHALL require the existing unbounded acyclic model graph apart\n\
    \  from implicit terminal stutter, one initial state, at most one actual S3\n\
    \  claim binding, no clocks, rewards or authored fairness, and FR-196's\n  existing\
    \ Markov successor restrictions. Only exact minimum AtLeast,\n  maximum AtMost\
    \ and single-resolved-distribution workload proofs SHALL be\n  admitted. Cyclic,\
    \ nested temporal, bounded, event-predicate and other\n  profile/optimization/kind\
    \ cases SHALL remain explicitly unsupported.\n- The checker SHALL bind the actual\
    \ checked occurrence, infinite-trace\n  profile, formula a"
- id: FR-282
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "- For FR-201's closed reachability and acyclic state-predicate profiles,\
    \ the fresh-process facade SHALL\n  admit the bounded source `ByteProvision`,\
    \ re-check S1 to S3 into a\n  `CheckedPackage`, admit the `ModelSystem` and selected\
    \ claim binding,\n  read the certificate document through that profile's strict\
    \ reader and\n  invoke the independent core checker with the caller's limits and\n\
    \  `&Cancel`. It SHALL extract/reuse the source completeness and provision\n \
    \ identity admission in `qsl-replay/src/request.rs`, together with existing\n\
    \  content-hash validation wherever required by the provision's so"
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201 additionally defines the narrow closed

    `quire.probability-objective/state-predicate/v1` objective for untimed,

    infinite-trace acyclic state-predicate until and always proofs. Reusing

    reachability/v1 unchanged would lose until''s absorbing failure boundary or

    incorrectly make a currently true P an always success. The separate objective

    therefore carries independently derived `event`, `targets` and `failures`,

    alongside the existing optimization and empty fairness. Until accepts B

    before considering A and rejects neither-A-nor-B permanently; always rejects

    not-P permanently and accepts ter'
- id: FR-126
  path: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md
  role: context_only
  excerpt: '- Under infinite-trace, a terminal model state SHALL contribute one terminal'
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: context_only
  excerpt: '- `holds(A) until holds(B)` SHALL fix 0 at states where neither holds
    and'
- id: qualitative
  path: qsl-semantics/src/check/probabilistic.rs
  role: context_only
  excerpt: "            Magnitude::NonPositive | Magnitude::AtLeastOne => {\n    \
    \            causes.push(ProbabilisticCause::QualitativeThreshold {\n        \
    \            claim: claim.clone(),\n                    span: value.span,\n  \
    \              });\n                false\n            }\n        };\n    // The\
    \ probability, level or fraction the confidence p"
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
