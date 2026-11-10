---
id: SR-2026111011431564766304
title: dependency review of cyclic MDP first-hit reward specification
type: SpecReview
analysis: dependency
scope: agent-ix/quire-spec-language; QSL-1826 PR2661; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
review_set: subset
---

## Summary

Independent review of frozen 8b24acebf0c37a989f031d73f38215e7f0a0f986 against base 39832145fde6ebd1c4ce4cfed22d7dccdf02f53f. Ticket QSL-1826; specification-only delegated subset review.

## Verdict

PASS — The added verifies edges resolve locally and introduce no prerequisite cycle or new shared transport dependency.

## Assurance Context

No active AssuranceProfile applies: anchored frontmatter type search found no profile in the spec tree. No enforced workflow or human acceptance is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Independent reasoning and evidence

Scoped classification: FR-187 action/draw/reward admission and FR-196 checked product keys/units are enablement; FR-198 selected reward semantics are domain prerequisite; FR-201 checker profile is the feature; TC-1405 is verification. Directed ordering: FR-187 -> FR-196 -> FR-198 -> FR-201 -> TC-1405, with direct FR-196 -> TC-1405 and FR-198 -> TC-1405 verification references. ADR-028 records placement, not an implementation prerequisite cycle. All three new TC verifies targets exist. No FR-413 witness implementation is required for the native closed profile.

## Examined scope

```yaml
scope:
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For checked `expected accumulate R until holds(B) op c` every-scheduler

    claims, the codec SHALL additionally admit the distinct closed objective

    `"quire.probability-objective/cyclic-mdp-expected-reward/v1"`. It SHALL

    retain exactly the reward objective''s `profile`, `targets`, `reward`,

    `unit`, `optimization` and `fairness` members, and the exact document''s

    `identity`, `objective`, `kind`, `values` and `ranking` body. Kind SHALL be

    `"exact"`, fairness empty, and optimization `"minimum"` for source Every

    AtLeast or `"maximum"` for source Every AtMost. No other bound direction,

    policy/action tran'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Admission SHALL require an untimed finite Markov subject with one\n  initial\
    \ state, at most one actual S3 binding and one post-state per\n  draw. Preserve\
    \ full original-subject identity, validity, action/draw\n  and Markov admission\
    \ before checking the first-B stopped graph.\n  Earlier FR-187 refusals, including\
    \ negative reward, retain precedence;\n  post-admission undefined/negative query\
    \ evaluation retains typed\n  nonacceptance. No missing or refused action, draw\
    \ or reward becomes\n  zero. Authored fairness, workload selection, timed/general\
    \ temporal,\n  pending multi-post, multi-initial/binding, n"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The finite body SHALL cover EVERY state reachable with positive\n  probability\
    \ under ANY actual action from the initial state before\n  first B, including\
    \ reached B states. The selected extremum SHALL be\n  finite at every such state,\
    \ not merely at the initial state. A native\n  result with any infinite covered\
    \ value SHALL retain explicit unsupported\n  mixed-infinite coverage and emit\
    \ no finite document: do not omit its\n  row, encode infinity as zero, or report\
    \ an internal fault. For example,\n  minimum initial cost 1 via a safe exit does\
    \ not admit a finite body when\n  another initial action enters"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Independently evaluate B on the stopped traversal. A reached B has\n \
    \ value and rank zero and is not expanded for the query. Targets SHALL\n  be exactly\
    \ the stopped B keys; after-B-only states have no rows or\n  query-cost contribution.\
    \ Original admission is not waived, but query\n  evaluation solely after B is\
    \ unnecessary. Post-B cycles alone do not\n  invalidate finite first-hit evidence.\
    \ Use the checked FR-196 reward\n  identity, selected comparison unit and actual\
    \ null/declared/compound\n  UnitId descriptor with existing exact conversion,\
    \ not a display alias."
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "At each non-B state s independently rebuild EVERY actual action a and\n\
    \  its typed draws, retaining operation/receiver/non-random arguments and\n  draw\
    \ reward identity BEFORE coalescing. Define\n  `c(s,a) = sum_e p(e)*r_R(e)` and\n\
    \  `Q_a(V) = c(s,a) + sum_e p(e)*V(to(e))` for its positive-probability\n  draws,\
    \ independently checking exact nonnegative probabilities and total\n  action mass\
    \ 1. Missing named R contributes zero; evaluated costs SHALL be\n  defined, finite\
    \ and nonnegative. Check `V(s) = min_a Q_a(V)` or\n  `V(s) = max_a Q_a(V)` as\
    \ selected. Never average distinct actions,\n  even when their"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Every non-B state, INCLUDING value-zero states, SHALL have positive\n\
    \  natural rank and at least one actual action. For minimum, SOME TIGHT\n  minimizing\
    \ action with `Q_a(V) = V(s)` SHALL have a positive-probability\n  successor of\
    \ strictly smaller rank. For maximum, EVERY actual action\n  SHALL have such a\
    \ successor. B alone is a boundary; a lower-rank\n  successor may have value zero.\
    \ Probability-zero edges do not progress.\n  A non-B terminal SHALL fail finite\
    \ properness: universal progress\n  SHALL NOT pass vacuously on an empty action\
    \ set (an implicit terminal\n  self-stutter likewise supplies no"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "For minimum the tight progressing actions select a proper policy:\n  a\
    \ minimum-rank state in any closed non-B class contradicts progress.\n  Its finite\
    \ Bellman solution realizes V. The all-action inequalities\n  lower-bound every\
    \ proper policy, while improper policies have FR-198\n  infinity even at zero\
    \ cost. For maximum, universal progress excludes\n  every non-B end component\
    \ under any policy; all policies are proper\n  and Bellman equality gives the\
    \ finite optimal value. The checker SHALL\n  verify these local conditions without\
    \ linear solving, iteration or\n  trusting producer policies/qualitativ"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Check admitted bounds inclusively at the initial full key; reward\n  threshold\
    \ zero is valid. Preserve strict canonical JSON/rationals,\n  complete binding-bearing\
    \ keys with monitor/accumulator/fairness/pending\n  components actually null,\
    \ sorted exact stopped coverage, early\n  identity refusal and objective mismatch\
    \ for changed reward, unit,\n  targets, optimization or profile. Wrong Bellman\
    \ values reject\n  `InequalityFails`, failed properness `RankBroken`, and missed\
    \ bounds\n  `ThresholdMissed` at the first applicable canonical full key. Missing,\n\
    \  duplicate, extra or malformed rows retain existi"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Original/stopped admission, actual raw/internal/full key construction,\n\
    \  all-action/draw, B/reward/unit evaluation, exact sums and properness\n  work/storage\
    \ SHALL retain real caller byte/state/edge, representation,\n  arithmetic, cumulative\
    \ work/storage and cancellation causes. Zero\n  direct instances retain UnsupportedBinding/unsupported\
    \ without invented\n  evidence. Existing profile bytes/defaults, producer preference\
    \ for an\n  eligible old acyclic or DTMC profile, and sealed acceptance remain\n\
    \  unchanged. External transition construction/interpreter allocations\n  remain\
    \ their existing boundary"
- id: FR-201-AC-43
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A non-B s chooses actions a or b, each with self/B probabilities 1/2 and
    1/2 and costs R=1 or R=2 on both draws respectively. The new cyclic MDP reward
    profile accepts minimum V(s)=2 under Every AtLeast 2 and maximum V(s)=4 under
    Every AtMost 4, with V(B)=0 and ranks (s,B)=(1,0). Bounds 5/2 and 7/2 respectively
    reject ThresholdMissed. Distinct action costs cannot be averaged despite identical
    successor distributions. Changing entry-B draw R changes the required value; actual
    unit conversion accepts an equivalent bound, while a foreign dimension-equivalent
    UnitId refuses objective mismatch.
- id: FR-201-AC-44
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'For WAIT self R=0 versus GO to B R=1, minimum V(s)=1 accepts with rank
    1. Bogus minimum V(s)=0 or 1/2 satisfies Bellman equality through WAIT but rejects
    RankBroken because no tight action progresses. Maximum is infinity: no finite
    native document is emitted, and bogus maximum V(s)=1 rejects RankBroken despite
    Bellman equality. A zero-cost positive-exit retry accepts V(s)=0 with positive
    rank at bound 0; rank 0 rejects. Non-B terminals and closed zero-cost classes
    have infinity and cannot pass vacuous universal progress. Minimum initial safe
    exit cost 1 versus another action to an infinite non'
- id: FR-201-AC-45
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-43/44 documents round-trip canonically through a fresh checker without
    analyzer/provider linkage. Entry-B cost is counted once, initial B has V/rank
    zero, and an otherwise admitted post-B cycle adds no rows or costs. Wrong checked
    reward/model/unit, targets, optimization/profile, identity or binding; changed
    actual draw R; false values/progress; malformed rational/JSON/full-key data; missing/duplicate/out-of-order/extra
    rows and model-only keys prevent acceptance under the precise existing rule/locus.
    Old acyclic and DTMC bytes/verdicts, producer preferences and sealed acceptance
    remain unc
- id: FR-201-AC-46
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Actual pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic
    exhaustion during original/stopped admission, actual raw/internal/full keys, all-action/draw/B/reward/unit
    reconstruction, exact sums, tightness and universal/existential progress preserve
    the typed cause without accepted truth. Earlier model/negative-reward refusal
    and deferred undefined or unsupported evaluation are not coerced to zero or omitted
    actions. Unsupported shape/fairness/workload/timed/multi-post/refutation/infinity
    forms issue no new-profile proof; zero direct i
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s distinct `quire.probability-objective/cyclic-mdp-expected-reward/v1`

    reuses exact reward values/ranks with source Every AtLeast selecting minimum

    and Every AtMost selecting maximum. Checked reward/comparison-unit identity,

    actual per-action draw costs and full original admission remain authoritative.

    The query stops at first B; only B has the boundary value/rank zero.'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'Bellman equality alone is insufficient: WAIT at zero cost versus GO to
    B

    at cost 1 has bogus minimum fixed points below 1 and a bogus finite maximum

    fixed point at 1. Minimum therefore requires a TIGHT minimizing action

    with positive-probability lower-rank progress at every non-B state;

    maximum requires progress for EVERY actual action. Both require a nonempty

    action set, positive ranks even at value zero, and no positive-value

    successor restriction. These conditions select a proper tight minimum

    policy or establish all-policy properness for maximum. Finite-state

    properness and Bellman equalit'
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'The finite body requires finite selected values at every all-action

    first-B-stopped reachable state. Initial minimum 1 via a safe exit does

    not suffice if another action enters an infinite-minimum trap. Native

    mixed-infinite coverage remains unsupported with no finite document;

    fraudulent finite rows still reject by coverage/properness. No omitted

    infinite rows, infinity-as-zero, internal-fault substitution or infinity

    wire is introduced. Existing acyclic/DTMC profiles remain preferred where

    eligible and retain their bytes. This adds no fairness, workload,

    multi-post, timed, refutation or FR-4'
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: 'Specify FR-201-AC-43 to AC-46''s finite every-scheduler reward proof

    obligations. These procedures are not execution evidence. Reconstruct

    all actual actions/draw rewards and first-B stopped coverage independently;

    do not trust producer policy, properness labels or supplied costs.'
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "1. Give non-B s two actual actions with the same self/B probabilities\n\
    \   (1/2,1/2) but named R=1 on both draws of a and R=2 on both draws of b.\n \
    \  Offer minimum 2 under Every AtLeast 2 and maximum 4 under Every AtMost\n  \
    \ 4, B value/rank zero and s rank 1. Test bounds 5/2 and 7/2 respectively.\n \
    \  Change only entry-B R, then swap equal-probability draw costs and change\n\
    \   their mean. Verify actual costs before coalescing, preserving operation,\n\
    \   receiver, arguments and typed draw identity. Test checked declared and\n \
    \  compound units, exact equivalent-bound conversion and wrong UnitId."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "2. Give s WAIT self with R=0 and GO to B with R=1. Offer minimum 1/rank\
    \ 1,\n   then minimum 0 and 1/2 with otherwise valid rows. Offer maximum 1 with\n\
    \   otherwise valid rows, and inspect the native maximum infinity outcome.\n \
    \  Separately use a proper zero-cost retry with positive B probability at\n  \
    \ threshold 0; mutate its positive rank to zero. Use non-B terminal and\n   zero-cost\
    \ closed-class controls for both extrema. Explicitly verify\n   empty actions\
    \ cannot vacuously satisfy maximum progress."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "3. Give initial s a safe R=1 exit to B and another action to a non-B\n\
    \   self-trap t with R=0. Inspect native minimum values (1, infinity, 0)\n   for\
    \ (s,t,B); require unsupported mixed-infinite coverage with no finite\n   document.\
    \ Offer forged complete finite rows with t value zero, then\n   omit t's rows.\
    \ Keep native unsupported distinct from checker rejection."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "4. Use initial B with an original positive-cost outgoing operation; then\n\
    \   use A-to-B with entry cost and an admitted positive-cost cycle reachable\n\
    \   only after B. Offer only the first-hit rows and stopped target set.\n   Mutate\
    \ after-B rows and targets separately. Round-trip valid documents\n   through\
    \ a fresh checker process without analyzer/provider linkage.\n   Apply every AC-45\
    \ objective/identity, binding, rational, JSON, full-key,\n   ordering, value and\
    \ rank mutation, asserting precise rule and locus.\n   Retain old acyclic and\
    \ DTMC canonical controls and producer preferences."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "5. Observe original/stopped admission, actual current/candidate/post/raw/\n\
    \   full-key, all-action/draw, B/reward/unit, exact sums and existential-tight\n\
    \   or universal progress phases. Cancel using the actual caller token for\n \
    \  Requested and Deadline with matched successful controls, not fixed poll\n \
    \  offsets. Exhaust actual byte/state/edge/work/storage/representation and\n \
    \  arithmetic requirements. Check earlier model/negative refusal precedence\n\
    \   and deferred undefined/unsupported evaluation without zero substitution.\n\
    \   Exercise unsupported shapes and zero direct over instances."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: 'Executable assertions SHALL carry actual criterion attributes such as

    `#[trace("TC-1405", "FR-201-AC-44")]`. This procedure writes no matrix and

    does not qualify external transition/interpreter allocations or all AC-30.'
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 1 accepts minimum 2 and maximum 4 at inclusive equality; interior\n\
    \  misses reject ThresholdMissed at the initial full key. Distinct actions\n \
    \ remain distinct despite identical successor distributions. Changed draw\n  costs\
    \ require changed Bellman values; wrong units refuse objective mismatch."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 2 accepts genuine minimum 1 and proper zero retry with positive\n\
    \  rank. False minimum fixed points and false finite maximum reject\n  RankBroken;\
    \ a nontight GO cannot justify minimum progress. Native maximum,\n  terminal and\
    \ improper closed classes retain infinity without finite\n  documents. Zero value\
    \ does not waive rank or properness checks."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 3 emits no native finite document despite finite initial minimum.\n\
    \  Forged trap rows reject RankBroken; missing rows reject Malformed at\n  values/ranking.\
    \ No infinity becomes zero or an internal fault."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 4 charges entry B once, charges nothing at initial B or after B,\n\
    \  and preserves exact stopped coverage and strict canonical transport.\n  Extra\
    \ rows reject coverage and extra targets refuse objective mismatch.\n  Existing\
    \ bytes/verdicts and sealed acceptance are unchanged."
- id: TC-1405
  path: spec/test-cases/TC-1405-check-cyclic-mdp-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 5 retains actual typed refusal, unsupported, limit or cancellation\n\
    \  causes with no accepted token/truth. Zero instances retain\n  UnsupportedBinding/unsupported\
    \ without invented binding or evidence.\n  No new bound direction, infinity document,\
    \ refutation or FR-413 transport\n  is implied."
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: context_only
  excerpt: 'EN-5 SHALL decide `Reach`, `ExpectedReward` and `MeanTime` forms (ADR-028
    XF-4, XF-5) by

    qualitative precomputation, maximal end-component collapse, interval

    iteration over dyadic intervals with outward rounding, and, when the

    interval still straddles the threshold, exact policy iteration with exact

    sparse linear solves (UR-1 to UR-7). Plain value iteration with a

    convergence threshold SHALL never decide a verdict.

    '
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
- id: FR-187
  path: spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md
  role: context_only
  excerpt: 'FR-120''s `ModelSystem` SHALL enumerate a random parameter over its support

    and SHALL give, at each state, the probabilistic structure of its

    successors: the **actions** (scheduled identities, ADR-024 PM-2) with each

    action''s draw distribution, and under a workload the exact step probability

    of each transition (PM-3), together with each transition''s reward values

    (PM-5). A scheduled identity and drawn vector with no post-state, or with

    several under a workload, `ModelSystem` SHALL report as `NotMarkov` (PM-3). Both

    EN-4 (FR-189) reads this structure and does not compute it itself.

    '
```

## Applicability and limits

Selected methods: code-review, base, integrity, dependency, failure-domain, EARS, object-review and scope-boundary. Rust, React and production gap-analysis do not apply to the three-file specification diff. Evidence/risk lenses are not separately selected: verification remains Test and finite-graph/resource risks are covered in failure-domain and scope-boundary. No criterion-strength, security, application or full-catalog claim; no Cargo or executable test run. Quire registry duplicate-archetype/edge advisories are existing. Exact model id is unavailable; model=unreported. Run e2f51506-8ff7-44d0-b561-eab12b3e3af5.
