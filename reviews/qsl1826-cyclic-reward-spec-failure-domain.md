---
id: SR-20261110103121910005
title: failure-domain review of cyclic DTMC expected-reward proof specification
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; QSL-1826 PR2658; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/decisions/ADR-028-exact-probabilistic-engine.md, spec/test-cases/TC-1403-check-cyclic-dtmc-expected-reward-certificates.md
review_set: subset
---

## Summary

Reviewed frozen d7454d7a21eadadd1a553666e59b7da606c38611 against base 479b5638f6e183d9883ad91c7548341b00cb23bc. Specification only; no runtime or complete qualification is claimed.

## Verdict

**PASS** — Zero-cost nontermination remains infinity and cannot pass merely because V0 satisfies Bellman equations. Closed classes, self-loops, initial targets, positive entry cost, hidden post-target cycles, full-key aliases, malformed coverage and typed stops have explicit adverse controls. Target query evaluation and accumulated cost stop at first B; original source admission, including FR187 refusals, remains authoritative. Infinite producer results are not serialized as finite evidence, faults or refutations.

## Assurance Context

No active AssuranceProfile applies in the local spec tree (anchored type search found none). Direct delegated subset review; no enforced workflow acceptance or exception is claimed.

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
  excerpt: 'For checked `expected accumulate R until holds(B) op c` claims under a

    fully resolved workload, the codec SHALL additionally admit

    `"quire.probability-objective/dtmc-expected-reward/v1"`. This finite-only

    proof profile SHALL retain exactly the acyclic reward objective''s

    `profile`, `targets`, `reward`, `unit`, `optimization` and `fairness`

    members; `optimization` SHALL be `"workload"` and `fairness` empty. The

    document SHALL retain the existing exact `identity`, `objective`, `kind`,

    `values` and `ranking` members, with kind `"exact"`. No policy, component,

    action transport, second reward expres'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Admission SHALL require an untimed finite DTMC subject, one initial\n\
    \  state, at most one actual S3 binding, fully resolved workload and the\n  existing\
    \ one-post-state-per-draw Markov restrictions. General MDP or\n  every-scheduler,\
    \ authored fairness, timed, multi-post, interval and\n  refutation/infinity documents\
    \ remain outside this profile. Preserve full\n  original-subject validity, identity\
    \ and Markov/workload admission; this\n  profile SHALL NOT inherit the acyclic\
    \ reward profile's original-DAG or\n  all-original-terminals-are-B restrictions.\
    \ Original graph admission and\n  its caller budgets "
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The checker SHALL independently rebuild the graph reachable from the\n\
    \  admitted initial state BEFORE the first B, evaluating B on that stopped\n \
    \ traversal. A reached B SHALL absorb, with V zero and rank zero, without\n  expanding\
    \ its outgoing steps for the claim. A state reachable only after\n  B SHALL have\
    \ no value/rank row or contribution to the accumulated reward;\n  a post-B cycle\
    \ alone SHALL NOT invalidate this profile. Objective targets\n  SHALL be exactly\
    \ the B keys in this stopped graph. Query-expression\n  evaluation solely after\
    \ B is not required; earlier original-subject\n  admission re"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- `reward` and `unit` SHALL use the acyclic profile's actual checked\n\
    \  FR-196 model/reward identity, selected comparison unit, exact conversion\n\
    \  graph and null/declared/compound UnitId descriptor. Independently\n  evaluate\
    \ every relevant semantic transition and typed draw BEFORE\n  coalescing, preserving\
    \ operation/receiver/non-random-argument identity.\n  Missing R contributes zero;\
    \ admitted costs are defined, finite and\n  nonnegative integers or quantities.\
    \ At every non-B state s check\n  `V(s) = sum_e p(e) * (r_R(e) + V(to(e)))` under\
    \ the actual single\n  workload distribution. Include the cost "
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Values and natural-number ranks SHALL cover exactly the stopped graph\n\
    \  in existing canonical complete-key order. Every non-B state SHALL have\n  positive\
    \ rank and SOME independently rederived positive-probability edge\n  to a strictly\
    \ smaller rank. This applies EVEN when its value and every\n  step reward are\
    \ zero. It does not require a positive successor VALUE,\n  and it does not require\
    \ every edge to decrease: retry self-loops and\n  other cyclic edges may retain\
    \ or increase rank. These are properness\n  ranks, not exact acyclic heights.\
    \ The checker SHALL verify this condition\n  rather than tru"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- On a finite DTMC, the rank condition excludes every closed non-B class:\n\
    \  a minimum-rank member of such a class would have a positive edge leaving\n\
    \  it to a lower rank. Thus B is reached almost surely, expected hitting\n  time\
    \ and finite admitted reward are finite, and reward Bellman equality\n  has a\
    \ unique finite solution. This is the independent finiteness check;\n  the checker\
    \ SHALL solve no linear system, run no value/policy iteration\n  or import producer\
    \ graph/cost/properness classifications. FR-198 still\n  assigns positive infinity\
    \ to nonreachability, including zero-cost\n  nontermination"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Either workload bound side SHALL be checked inclusively at the initial\n\
    \  complete key under existing S3 expected-reward admission. In particular,\n\
    \  zero is a valid expected-reward threshold; the probability-threshold\n  endpoint\
    \ policy SHALL NOT be imposed on reward bounds. Wrong values\n  SHALL reject `InequalityFails`,\
    \ wrong ranks `RankBroken`, and an otherwise\n  valid document missing its bound\
    \ `ThresholdMissed`, at the first\n  applicable complete key. Conflicting identity\
    \ SHALL refuse before graph\n  enumeration; changed reward/unit/targets/optimization/profile\
    \ SHALL\n  refuse objective mism"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Original/stopped expansion, actual key construction, reward/unit and\
    \ B\n  evaluation, graph/rank passes, retained rows and exact sums SHALL retain\n\
    \  the real caller's byte/state/edge, representation/arithmetic,\n  cumulative\
    \ work/storage and cancellation controls with typed causes.\n  Zero direct `over`\
    \ instances retain `UnsupportedBinding` and unsupported\n  EN-5 disposal with\
    \ no invented evidence. Existing profiles retain their\n  canonical bytes, admission\
    \ and verdicts; the producer SHALL prefer the\n  existing acyclic reward profile\
    \ when it already applies. No completed\n  implementation, full "
- id: FR-201-AC-35
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'A DTMC retry state s has self and B-entering draws each of probability
    1/2 and R=1 on both. The cyclic reward profile accepts V(s)=2, V(B)=0 and ranks
    (s,B)=(1,0), at both workload equality bounds. AtMost 3/2 and AtLeast 5/2 reject
    ThresholdMissed. Changing only entry-B R to 3 accepts V(s)=4; offering 2 rejects
    InequalityFails. Swapping equally probable draw costs preserves 4, while changing
    their mean falsifies it. Costs 2 ms on self and 6 ms on entry give 8 ms and accept
    an equivalent 0.008 s bound through actual checked conversion; declared/compound
    descriptors retain their actual identity '
- id: FR-201-AC-36
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A zero-cost retry with positive exit probability accepts V(s)=V(B)=0 and
    ranks (1,0), including either bound at 0. Giving s rank 0 rejects RankBroken despite
    zero values. A closed non-B zero-cost self-loop or cycle rejects otherwise Bellman-valid
    finite V=0 evidence as RankBroken; the producer preserves FR-198 infinity without
    a finite document. Initial B accepts only its zero value/rank even when its original
    outgoing operation has positive cost. An admitted A-to-B subject with a positive-cost
    cycle reachable solely after B accepts first-hit evidence with rows only for A
    and B; adding after-B
- id: FR-201-AC-37
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-35/36 documents round-trip canonically through a fresh checker without
    analyzer/provider linkage. Changed actual reward name/model/unit, target set,
    optimization/profile or binding prevents acceptance; dimension-equivalent foreign
    UnitIds are not aliases. Noncanonical rational/JSON/key data, malformed or duplicate/missing/out-of-order
    rows, model-only keys, false Bellman values, and absent/wrong progress ranks retain
    precise rule/locus diagnostics. The old acyclic reward and probability/long-run
    profile bytes/verdicts remain unchanged; an eligible acyclic producer keeps its
    existing profile
- id: FR-201-AC-38
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and phase-observed Requested/Deadline or byte/state/edge/work/storage/representation/arithmetic
    exhaustion during original/stopped admission, raw/internal/full keys, draw/B/reward/unit
    evaluation, progress checking and exact sums preserve the actual typed cause with
    no accepted truth. Earlier model refusals and deferred undefined/negative evaluation
    are not coerced to zero. Unsupported kind/shape/fairness/every-scheduler, multi-post
    or refutation/infinity requests emit no finite cyclic reward proof. Direct zero
    over instances retain UnsupportedBinding/unsupported without a fab
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'For checked `ProbForm::ExpectedReward` source claims `expected accumulate
    R

    until holds(B) op c` in the untimed infinite-trace profile, the codec SHALL

    admit the separate `"quire.probability-objective/expected-reward/v1"`

    objective. It SHALL have exactly `profile`, `targets`, `reward`,

    `unit`, `optimization` and `fairness`. `reward` SHALL use FR-196''s existing

    `[model_node,identifier]` identity: the actual declaring model NodeId and

    checked reward Identifier R. `unit` SHALL use FR-196''s actual descriptor:

    null for plain numeric, `["declared",unit_key]` or

    `["compound",unit_form]` for the admit'
- id: FR-198
  path: spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md
  role: context_only
  excerpt: "- EN-5 SHALL give the value `+∞` at every state that reaches `B` with\n\
    \  probability below 1 under some scheduler, for a maximum, or under every\n \
    \ scheduler, for a minimum, decided by the graph alone; under a workload,\n  at\
    \ every state outside `Prob1`.\n- On the remaining states it SHALL collapse end\
    \ components of zero reward,\n  obtain an upper starting vector as sound value\
    \ iteration does, and run\n  interval iteration with the reward added to the operator,\
    \ then exact\n  policy iteration as for reachability.\n- A `<= c` bound SHALL\
    \ fail and a `>= c` bound SHALL hold at an initial\n  state whose val"
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: '`reward` or `weight` is `[model_node,identifier]`: the existing FR-185/

    FR-187 model-scoped reward Identifier, paired with its checked declaring

    model''s FR-322 NodeId object. This does not invent a reward node id or

    qualified-name alias. Each operation contributes that named reward, or

    zero when it declares none. The checker resolves the exact identifier

    through the checked model before accumulation. `unit` is null for a

    plain numeric reward with no unit, otherwise the admitted UnitId represented as

    `["declared",unit_key]` for a declared unit, with `unit_key` the unit

    declaration''s structural '
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s separate `quire.probability-objective/dtmc-expected-reward/v1`

    keeps the existing exact reward objective and values/ranking transport,

    while replacing the acyclic profile''s original-DAG and height restrictions

    with a checked finite-DTMC properness witness. Every non-target state,

    including a zero-value/zero-cost one, has a positive-probability edge to a

    strictly lower rank; targets have rank zero. A finite closed non-target

    class would contradict its minimum rank, so the stopped chain reaches B

    almost surely. Finite admitted costs and Bellman equality then determine

    the unique finite '
- id: TC-1403
  path: spec/test-cases/TC-1403-check-cyclic-dtmc-expected-reward-certificates.md
  role: examined
  excerpt: "1. Admit a finite untimed DTMC with initial non-B state s, self and\n\
    \   B-entering draws each of probability 1/2, and checked named R=1 on\n   both.\
    \ Offer exact V(s)=2, V(B)=0 and ranks (s,B)=(1,0). Check both\n   equality bound\
    \ sides, AtMost 3/2 and AtLeast 5/2. Separately change only\n   the entry-B reward\
    \ to 3 and verify V(s)=4 from\n   `V(s) = (1+V(s))/2 + (3+0)/2`. Swap the two\
    \ equal-probability costs,\n   then change their mean. Use actual declared costs\
    \ 2 ms on self and\n   6 ms on entry, yielding 8 ms and an equivalent 0.008 s\
    \ bound. Also use\n   a checked compound unit and a foreign same-dime"
- id: TC-1403
  path: spec/test-cases/TC-1403-check-cyclic-dtmc-expected-reward-certificates.md
  role: examined
  excerpt: "- Step 1 accepts 2 and 4 with actual reward units and inclusive bounds;\n\
    \  the interior beyond-bound controls reject ThresholdMissed at the initial\n\
    \  full key. The false 2 after entry cost changes rejects InequalityFails;\n \
    \ no unit-step or post-B charging substitute verifies.\n- Step 2 accepts proper\
    \ zero reward, but wrong/no progress rejects\n  RankBroken at the first failing\
    \ full key even when V=0. A zero-cost\n  closed non-target class keeps FR-198\
    \ infinity and produces no finite\n  proof. No probability-value-positive exemption\
    \ is used for progress.\n- Step 3 counts entry-B cost exactly once, no"
```

## Evidence and limits

The three changed normative artifacts passed scoped Quire validation: 3/3 grammar-clean, zero grammar findings. Manual exact arithmetic independently confirms the retry and unit-conversion fixtures; graph reasoning establishes rank properness and Bellman uniqueness. Context inspected at qsl-analyze/src/exact_probabilistic/unbounded.rs:557-610, qsl-replay/src/probability_certificate/reward.rs:198-286, qsl-semantics/src/check/probabilistic.rs:726-746 and 885-918. TC1403 procedures are future implementation obligations, not executed tests. Rust/runtime gap-analysis, complete matrix, evidence advisor and criterion-strength/Jev were not selected for this specification-only diff. No full FR201AC30/all-expansion or whole-repository assurance is granted.
