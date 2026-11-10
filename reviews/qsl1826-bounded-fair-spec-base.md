---
id: SR-20261110115912864202
title: base review of bounded fair eventual proof specification
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language; QSL-1826 PR2662; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/test-cases/TC-1406-check-bounded-fair-eventual-certificates.md
review_set: subset
---

## Summary

Independent delegated subset review of frozen b04f632a389e2da1db7a9ba047af262fb9a193d3 against 373a74f47c064d645c26bc202c20af11c8ed4213 for QSL-1826. Specification only.

## Verdict

PASS — New AC47–50 and TC1406 are unique and testable. TC procedures distinguish authored normalization from malformed document ordering, valid objective mismatch, full-key/monitor mutations, horizon zero/interior source thresholds, typed limits and independent reconstruction. This establishes specification readiness, not executed trace coverage or a green whole-repository matrix.

## Assurance Context

No active AssuranceProfile is declared in the local spec tree (anchored frontmatter type search). No enforced workflow acceptance is claimed. The dispatched applicable subset is code-review, base, integrity, dependency, failure-domain, EARS, object and scope-boundary. Separate evidence/risk/security/application/criterion-strength lenses were not selected: no changed verification method or deployment/security boundary; finite-model and budget risks are examined in failure and scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'The codec SHALL admit the separate profile

    `"quire.probability-objective/bounded-fair-eventual/v1"` only for

    origin-activated event-position `eventually[0,h] holds(P)`, finite integer

    h >= 0, checked state predicate P, untimed finite Markov models, one subject

    initial state, one admitted claim binding and a nonempty checked fairness

    set. Only Every-scheduler AtLeast/minimum and AtMost/maximum exact proofs

    are admitted. Source probability bounds remain strictly between 0 and 1;

    this does not exclude transported state probabilities 0 or 1. Horizon zero

    is admitted. Workload, clocks, rewards, unb'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The objective SHALL contain exactly `profile`, `targets`, `optimization`\n\
    \  and `fairness`. Targets and optimization retain the bounded reachability\n\
    \  profile's extensional meaning; optimization is `\"minimum\"` or\n  `\"maximum\"\
    `. Each fairness record SHALL contain exactly `fairness_kind`\n  (`\"weak\"` or\
    \ `\"strong\"`), `granularity` (`\"whole\"` or `\"each\"`) and\n  `target`. Target\
    \ SHALL reuse QSpec FR-322-AC-293's closed model-operation\n  variant of `TemporalFairnessTarget`:\
    \ exactly `{kind: \"operation\",\n  declaration: NodeId, name: Identifier}`. The\
    \ declaration is the actual\n  declaring model type's "
- id: FR-201-AC-47
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: The bounded fair profile admits the exact Every/minimum AtLeast and maximum
    AtMost slice with nonempty checked weak/strong whole/each operation fairness;
    horizon zero and transported 0/1 values work with interior source bounds. The
    fair and unfair bounded fixture both have minimum 24/25, while fair result/objective
    retain the exact checked set. Fairness-empty documents retain existing bytes.
    Unsupported shapes and zero instances emit no new-profile proof.
- id: FR-201-AC-48
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Canonical fairness records reuse the closed shared operation target and
    independently match the resolved checked set, including inherited declaring identities
    and whole/each receiver semantics. Authored reorder/duplicate spellings normalize
    identically; missing, extra, changed-kind/granularity/model/name, duplicate or
    out-of-order records and malformed/foreign targets cannot establish the claim.
    Distinct weak/strong constraints are retained.
- id: FR-201-AC-49
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Independent bounded reconstruction checks exact targets, optimization,
    complete six-part keys, actual monitor context and remaining-horizon ranks without
    producer linkage. Wrong identity/binding/receiver, goal, monitor, horizon, values/ranks,
    coverage or canonical transport cannot be accepted, with existing precise rule/locus.
    Numeric invariance introduces no fair-MEC, policy, seventh key part or new vacuity
    outcome.
- id: FR-201-AC-50
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual Requested/Deadline cancellation and byte/state/edge/work/storage/representation/arithmetic
    exhaustion during checked fairness/source declaration intake, structural identity
    traversal/encoding, canonical writing, raw/full-key admission and independent
    graph/arithmetic/rank checking retain typed causes without accepted truth. Earlier
    source refusals and unsupported/undefined evaluation remain distinct. External
    transition/private-reader and ordinary interpreter allocation boundaries are not
    claimed qualified.
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: ' is FR-201''s

    `bounded-fair-eventual/v1`: origin-activated event-position eventual state

    predicates, finite horizon, Every minimum/AtLeast or maximum/AtMost and a

    nonempty checked operation-fairness set. FS-5 justifies reusing the ordinary

    bounded operator and remaining-horizon ranking; FS-9''s fair-end-component

    reconstruction applies to forms whose value needs it, not this finite-prefix

    proof. No witness policy or new no-fair-continuation verdict is needed.

    The certificate retains exact weak/strong whole/each constraints through

    QSpec FR-322-AC-293''s closed structural operation target. They re'
- id: TC-1406
  path: spec/test-cases/TC-1406-check-bounded-fair-eventual-certificates.md
  role: examined
  excerpt: 'Specify FR-201-AC-47 to AC-50''s closed bounded-fair exact proof checks.

    These procedures are not execution evidence. Keep checked source identity,

    fairness reconstruction and qualified monitor semantics independent of the

    producer; equal fair/unfair values do not imply equal full product keys.'
- id: FR-200
  path: spec/functional/FR-200-decide-every-scheduler-claims-over-fair-schedulers.md
  role: context_only
  excerpt: 'When a claim `under every scheduler` states a fairness set, EN-5 SHALL

    decide it over the schedulers under which every constraint of the set holds

    with probability 1 (ADR-028 FS-1, FS-2). It SHALL compute the maximal fair

    end components of the product by refinement (FS-3, FS-4), and reduce each

    form to a computation of FR-197, FR-198 or FR-199 on a transformed objective

    (FS-5 to FS-8). Its witness scheduler SHALL be memoryless and randomized,

    and fair (FS-9).'
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: "QSL's `exact_probabilistic` module in layer A, crate `qsl-analyze`\n(ADR-028\
    \ SP-2), SHALL build, for a probabilistic\nclaim, the finite product on which\
    \ EN-5 computes (ADR-028 \xA72, SCH-2): the\nMDP over FR-187's actions and draws,\
    \ or under a workload the DTMC over\nFR-187's step probabilities, in product with\
    \ FR-126's deterministic monitor\nfor a bounded event and with an accumulator\
    \ for a bounded measure. It SHALL\nreuse `model_check`'s product `TransitionSystem`,\
    \ monitor translations, edge\nretention and SCC decomposition, and SHALL retain\
    \ every product edge with\nits action, draw and exact probabili"
- id: FR-123
  path: spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md
  role: context_only
  excerpt: "The S3 TemporalTrace `check` (ADR-014 \xA75) SHALL check, on a temporal\
    \ clause\nwhose unit selects `quire.temporal.infinite-trace/v1`, the fairness\n\
    constraints over model operations (ADR-018 FA-1, FA-5, FA-6) and the\ninterval\
    \ operators nested in any order with unbounded ones (ADR-018 IV-1)\nthat the shared\
    \ grammar parses. It SHALL classify every admitted clause over\na model subject\
    \ into one property form (ADR-018 TP-1 to TP-4) and record\nthat form beside the\
    \ clause's requirement record."
- id: QSpec-FR-322
  path: /home/peter/dev/quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: context_only
  excerpt: '| FR-322-AC-293 | A `fairness` member carries a closed `target` union:
    model operation (`declaration` node id and `name`), protocol root, attempt node
    or compensation template (each an exact structural `node` id), parallel branch
    (`parallel` node id and `branch` label), or hybrid transition (`declaration` node
    id and `name`). `temporal-fairness-target-vectors.json` publishes one positive
    member per variant and malformed members with a missing/unknown discriminant,
    missing required identity, extra identity field and invalid node ordinal; each
    positive satisfies the V2 `OperationMember` schema a'
- id: QSpec-FR-412
  path: /home/peter/dev/quire-specification/spec/objects/interfaces/FR-412-probability-certificate.md
  role: context_only
  excerpt: '| `objective` | The target or decided states, the reward, the optimisation,
    and the claim''s fairness set when it states one. The optimisation is `minimum`
    or `maximum` over every scheduler; `workload` under a workload that leaves no
    choice; or `workload-minimum` or `workload-maximum` under a workload over a timed
    subject with free delays, the minimum or maximum over those delays'' choices.
    |'
```

## Independent reasoning and evidence

For any admitted finite action/state model, take an arbitrary scheduler up to finite h and switch after h to the uniform scheduler. Its continuation is almost-surely fair for the finite weak/strong whole/each checked set (ADR028 FS2); changing only the suffix cannot change the bounded event. Every ordinary bounded achievable value is therefore fair achievable, and fair schedulers are a subset of all schedulers, yielding equal extrema. Terminal stutter enables no constraint (FS1); it does not create a no-fair-continuation case. This argument depends on the explicit finite source restriction, not merely finite depth of the explored product. Horizon zero follows immediately.

Existing implementation grounding was inspected without execution: qsl-analyze/src/exact_probabilistic/fair.rs:125–134 calls decide_bounded_event and returns product.fairness; tests/it/exact_probabilistic.rs:711–738 asserts ordinary/fair minimum24/25 and retained checked fairness. S3 claim.rs:1915–1947 resolves the actual declaring model structural node for its PRIVATE normal hash; that byte representation is not a public target codec. QSpec checked-package-v2/schema.json defines TemporalFairnessTarget operation with exactly kind/declaration/name and additionalProperties=false. FT01 uses defining Cell + set. FR412 objective remains backend-defined. qsl-foundation/src/node_wire.rs and document/writer.rs provide shared structural NodeKey canonical encoding, so the new prose does not require an imaginary ModelOperationTarget symbol.

The actual qsl-replay bounded monitor constructor derives the clause occurrence, checked temporal profile, origin and actual S3 binding. Equal numeric fair/unfair results give no license to substitute the certificate label or assume identical complete-key bytes. Limits must be charged for actual metadata and identity traversal before growth; this is a normative implementation obligation here, not a claim existing raw-state APIs qualify every allocation.

Scoped Quire validation of the three changed documents returned exit0, 3/3 grammar-clean and zero grammar findings; existing module duplicate-archetype/edge and inline-schema warnings remain advisory. No Cargo, runtime test, full matrix or all-expansion AC30 qualification is claimed. Exact model identity is unavailable from harness metadata; markers use model=unreported. Run f92d4e38-6a23-4a3f-8672-1990b1c765a9.
