---
id: SR-20261110110514930008
title: scope-boundary review of cyclic MDP eventual proof specification
type: SpecReview
analysis: scope-boundary
scope: agent-ix/quire-spec-language; QSL-1826 PR2659; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/test-cases/TC-1404-check-cyclic-mdp-eventual-certificates.md
review_set: subset
---

## Summary

Reviewed frozen dbc659c271646d4fc190aa6b001919975382811f against base 0e8f101a972c8358d744fa74d47fb958b488e33c. Specification only; ticket QSL-1826. The delegated subset selects this method; no implementation qualification follows.

## Verdict

PASS — Native finite untimed every-scheduler eventual proofs are separated from workload, fairness, rewards, timing, pending choices, other temporal forms and refutations. Original subject/Markov/action admission remains authoritative; objective targets and evidence rows cover only initial first-target stopped reachability. Existing profiles and producer preference are preserved. No runtime, shared-wire, full matrix or complete all-expansion AC30 qualification is claimed.

## Assurance Context

No active AssuranceProfile applies in the local spec tree (anchored frontmatter type search found none). Direct delegated subset review; no enforced workflow acceptance or exception is claimed.

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
  excerpt: 'For checked untimed infinite-trace `probability op c [eventually holds(P)]`

    claims over every scheduler, the codec SHALL additionally admit

    `"quire.probability-objective/cyclic-mdp-eventual/v1"`. This separate proof

    profile SHALL retain exactly the reachability objective''s `profile`,

    `targets`, `optimization` and `fairness` members and the existing exact

    `identity`, `objective`, `kind`, `values` and `ranking` document body.

    `kind` SHALL be `"exact"`; `fairness` SHALL be empty. No policy, action

    ordinal, action-key transport, qualitative-set field or formula AST is added.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Admission SHALL require a finite untimed Markov subject, one initial\n\
    \  state, at most one actual S3 binding and the existing single-post-per-draw\n\
    \  restrictions. The checker SHALL reconstruct EVERY enabled semantic action\n\
    \  distribution from the checked subject rather than select a producer\n  policy\
    \ or merge different actions into one workload distribution. Multiple\n  choices\
    \ and real cycles are admitted. Source `Every` AtLeast SHALL select\n  `\"minimum\"\
    `; `Every` AtMost SHALL select `\"maximum\"`. This profile adds\n  no minimum\
    \ AtMost or maximum AtLeast source mode. Workload, fairness,\n  rewar"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Preserve full original-subject identity, action/draw and Markov admission,\n\
    \  including beyond an early P, under the actual caller controls. The checker\n\
    \  SHALL independently evaluate the checked P occurrence and binding while\n \
    \ traversing the initial graph BEFORE first P, stopping outgoing expansion\n \
    \ at reached P. Objective targets, values and ranks SHALL cover precisely\n  this\
    \ stopped graph. A state reachable only after P contributes no row;\n  query-P\
    \ evaluation solely after P is not required. Earlier subject\n  admission refusals\
    \ remain authoritative. Full binding-bearing model keys\n  SHAL"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Every value SHALL be an exact rational in [0,1]. A target SHALL have\n\
    \  value 1 and a non-target terminal value 0. At each other state s and\n  independently\
    \ rederived action a, compute\n  `Q_a(v,s) = sum_t p(s,a,t) * v(t)` with exact\
    \ nonnegative probabilities\n  whose complete action mass is 1. For minimum, check\n\
    \  `v(s) = min_a Q_a(v,s)`; for maximum, check\n  `v(s) = max_a Q_a(v,s)`. A tight\
    \ action means exact equality\n  `Q_a(v,s) = v(s)`, not closeness under a tolerance.\
    \ All source actions\n  SHALL enter these comparisons, including nonselected and\
    \ tied actions."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Let Z be the offered zero-valued non-target states. Every target and\n\
    \  every state in Z SHALL have rank 0; every positive-valued non-target\n  state\
    \ SHALL have positive natural-number rank. Lower-rank successors\n  MAY have value\
    \ 0: the progress boundary is targets UNION Z, not targets\n  alone and not positive-valued\
    \ successors only. A probability-zero edge\n  SHALL NOT establish progress or\
    \ positive-support closure."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- For minimum, EVERY enabled action at EVERY positive-valued non-target\n\
    \  state SHALL have SOME positive-probability successor of strictly smaller\n\
    \  rank. Every live state in Z SHALL admit SOME enabled action whose entire\n\
    \  positive-probability support lies in Z. This zero-support action also\n  follows\
    \ from exact min Bellman equality and nonnegative values, but the\n  checker SHALL\
    \ establish that condition rather than trust an asserted\n  `Prob0` label. A false\
    \ terminal needs no enabled action; its implicit\n  stutter remains in Z. No universal\
    \ progress obligation applies to Z."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- For maximum, EVERY positive-valued non-target state SHALL admit SOME\n\
    \  TIGHT maximizing action with SOME positive-probability lower-rank\n  successor.\
    \ Progress under a nontight action SHALL NOT satisfy this rule.\n  Exact max Bellman\
    \ equality at a live state in Z forces ALL enabled\n  actions to remain in Z;\
    \ the checker SHALL retain that consequence.\n  These ranks certify progress and\
    \ need not equal producer BFS distances\n  or SCC heights. Other edges may retain\
    \ or increase rank."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The checker SHALL verify these conditions without solving equations,\n\
    \  importing producer values/classifications, policy iteration or a policy\n \
    \ witness. In a finite MDP, minimum progress makes target-or-Z absorption\n  almost\
    \ sure under every policy; choose tight minimizing actions outside\n  Z and zero-support\
    \ actions in Z to realize v. Maximum progress supplies\n  a proper tight policy\
    \ outside Z, realizing v, while Bellman inequalities\n  bound every other policy\
    \ from above, including improper policies. Thus\n  both values are exact extrema,\
    \ not arbitrary Bellman fixed points.\n  The required f"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Independently check the admitted initial threshold inclusively. Existing\n\
    \  S3 probability endpoint admission remains unchanged: source thresholds\n  0\
    \ and 1 are refused, although transported values 0 and 1 are valid.\n  Wrong Bellman\
    \ values or zero-support conditions SHALL reject\n  `InequalityFails` at the first\
    \ failing complete key; wrong boundary or\n  progress ranks SHALL reject `RankBroken`\
    \ there. A valid exact vector\n  missing its initial threshold SHALL reject `ThresholdMissed`.\
    \ Identity\n  conflict refuses before enumeration; changed targets, optimization,\n\
    \  fairness or profile refuses ob"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Original/stopped expansion, all action/draw reconstruction, internal\n\
    \  raw/current/candidate/post/full keys, exact action sums, zero support,\n  progress\
    \ passes and retained rows SHALL preserve the real caller's byte,\n  state/edge,\
    \ representation/arithmetic and cumulative work/storage limits\n  and actual cancellation\
    \ cause. Undefined P or source evaluation SHALL\n  retain typed nonacceptance\
    \ rather than false, zero or a missing action.\n  Zero direct `over` instances\
    \ retain `UnsupportedBinding` and unsupported\n  disposal without invented evidence.\
    \ Existing profiles retain their\n  bytes, admissi"
- id: FR-201-AC-39
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A live state s has actions a with probabilities (self,target,fail)=(1/2,1/5,3/10)
    and b with (1/2,2/5,1/10). With target value 1 and false-terminal fail value 0,
    the new profile accepts minimum 2/5 under AtLeast 2/5 and maximum 4/5 under AtMost
    4/5, with rank(s)=1 and boundary ranks 0. Minimum AtLeast 1/2 and maximum AtMost
    3/4 reject ThresholdMissed. Changing an otherwise matching actual action probability,
    omitting a source action from the extremum or replacing separate actions by their
    averaged distribution cannot verify the old vector. Separately let S choose A=(1/2
    fail,1/2 T) or B=(1/4 f
- id: FR-201-AC-40
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For wait-to-self versus go-to-target, the actual minimum is 0. A bogus
    minimum v(s)=1/2 satisfies Bellman equality but rejects RankBroken because wait
    supplies no lower-rank successor, even though go does. For wait-to-self versus
    attempt to target/fail with probabilities 4/5 and 1/5, the maximum is 4/5: bogus
    v(s)=9/10 is a Bellman fixed point through wait but rejects RankBroken because
    only the nontight attempt progresses; at v(s)=4/5 the tight attempt and rank 1
    accept under AtMost 19/20. Zero-valued live minimum states require a closed zero-support
    action; maximum zeros require all action s'
- id: FR-201-AC-41
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-39/40 valid documents round-trip canonical bytes through a fresh checker
    without analyzer/provider linkage. Target/non-target-terminal boundaries, stopped
    targets/coverage and complete full-key bindings are independently checked. Wrong
    objective/identity, malformed rational/JSON/key data, duplicate/missing/out-of-order
    values or ranks, extra after-target rows, model-only keys, false Bellman values,
    zero support and rank progress prevent acceptance with the specified rule/locus.
    A valid post-target cycle alone does not add certificate rows. Earlier profile
    bytes/verdicts and eligible produce
- id: FR-201-AC-42
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic
    exhaustion during original/stopped admission, raw/internal/full keys, all-action
    and draw reconstruction, min/max sums, zero-support and progress checks retain
    their real typed cause without an accepted truth. Undefined predicates/workload
    or source refusals are not coerced to zero or dropped actions. Fairness, workload
    selection, rewards, timed/general temporal, pending multi-post, multiple-initial/binding,
    non-exact or refutation requests issue no new-profile proof. Zero di
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s separate `quire.probability-objective/cyclic-mdp-eventual/v1`

    admits finite untimed every-scheduler eventual proofs using existing exact

    values/ranking transport. The checker independently reconstructs ALL

    semantic actions. Existing S3 selects minimum for AtLeast and maximum for

    AtMost; no new comparison modes or source endpoint policy are introduced.

    Neither a producer policy nor shared FR-413 action transport is required.


    Bellman equality alone admits spurious cyclic fixed points. The progress

    boundary is targets together with zero-valued states. For minimum, every

    action outside t'
- id: TC-1404
  path: spec/test-cases/TC-1404-check-cyclic-mdp-eventual-certificates.md
  role: examined
  excerpt: 'Specify FR-201-AC-39 to AC-42''s finite untimed every-scheduler proof

    controls. These are implementation obligations, not execution evidence.

    The independent checker reconstructs all actions and distinguishes

    minimum universal progress from maximum tight-action progress. Offered

    policies, action ordinals and qualitative labels are not authorities.'
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: context_only
  excerpt: 'EN-5 SHALL decide `Reach`, `ExpectedReward` and `MeanTime` forms (ADR-028
    XF-4, XF-5) by

    qualitative precomputation, maximal end-component collapse, interval

    iteration over dyadic intervals with outward rounding, and, when the

    interval still straddles the threshold, exact policy iteration with exact

    sparse linear solves (UR-1 to UR-7). Plain value iteration with a

    convergence threshold SHALL never decide a verdict.'
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
```

## Independent reasoning and evidence

The rank arguments were checked independently over finite states and finite complete action sets. For minimum, every action from a positive state has positive support to a smaller rank; finite positive-support minima give uniform positive probability of reaching target-or-zero within finitely many steps, so every policy reaches that boundary almost surely. Exact min Bellman inequalities bound every policy below, and selecting tight minimizing actions plus a zero-support action in each live zero state realizes the offered value. For maximum, choose a tight progressing action at each positive state. Its finite chain cannot have a closed class outside the boundary (take a minimum-rank state of that class), hence it realizes the offered vector. All-action max inequalities bound every other policy above, including those that wait forever. False terminals have value zero without an enabled action. The certificate therefore establishes exact extrema rather than only a threshold fact.

Fixture arithmetic: retry action a gives v=(1/2)v+1/5 and action b gives v=(1/2)v+2/5, so minimum 2/5 and maximum 4/5. Waiting supplies the bogus fixed points 1/2 and 9/10 but cannot supply the required progress. For S/T, T=2/3+S/3, minimum S=T/2 yields (2/5,4/5); maximum S=3T/4 yields (2/3,8/9). Ranks S=T=1 are valid because fail is a positive-probability rank-zero successor even though its VALUE is zero. These are manual exact checks, not executable runtime evidence.

Scoped `quire validate --scope <reviewed repo> --summary` returned exit 0, 3/3 grammar-clean and zero grammar findings (normative-validation.log). Quire parsed FR-201 successfully. Existing registry duplicate-archetype/edge and inline-schema advisories were not new document errors. TC-1404 and AC39–42 uniqueness and local declared relationship targets were checked. Whole-repository matrix, executable trace evidence and code/Rust runtime review are not assessed by this specification-only pass.

## Applicability

The pass runs code-review (spec-only), base, integrity, dependency, failure-domain, EARS, object and scope-boundary. No Rust files or Cargo manifests changed, so a Rust lane is inapplicable. No production code changed, so gap-analysis is inapplicable. Separate evidence/risk/criterion-strength/security/application lenses were not selected: this increment retains Test verification and introduces no new external interface, deployment or security boundary; finite graph failure/budget risks are addressed in the selected failure and scope lenses. No full catalog/all-analysis claim is made.

## Provenance limits

The harness does not expose an exact model identifier in model environment metadata; comment markers use `model=unreported`, rather than inventing one. Review run ef1ee0e3-3a47-4e52-b1ed-af95c8957431. No Cargo, source edits or merge actions were performed.
