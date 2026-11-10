---
id: SR-20261110040709820001
title: code-review review of unweighted DTMC long-run certificate specification
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; QSL-1826 PR2646; spec/functional/FR-201-check-a-probability-certificate.md,
  spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md, spec/decisions/ADR-028-exact-probabilistic-engine.md,
  spec/test-cases/TC-1401-check-unweighted-dtmc-long-run-certificates.md
review_set: subset
---

## Summary

Reviewed frozen f44c5502ec68ca4ddc7709cc807b6765921b19ac against base 5a95a823c47c6ff3919b5d32f01234634344b1dc. Specification only; no implementation qualification.

## Verdict

**PASS** — The diff contains only four normative Markdown artifacts. No implementation, Cargo/lockfile, production test, copied algorithm, pin, tracking ledger or runtime qualification change is introduced. Runtime and Rust gate methods are outside this diff. The future TC describes executable adverse and quota controls rather than substituting spec scanning for assertions.

## Assurance Context

No applicable active AssuranceProfile was found in the local spec scope. Direct delegated review; no enforced workflow acceptance is claimed. No active exception was used.

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
  excerpt: 'For a checked unweighted `long-run fraction holds(P)` claim under an

    authored workload in the untimed infinite-trace profile, the codec SHALL

    admit the separate objective profile

    `"quire.probability-objective/dtmc-long-run/v1"`. The objective SHALL have

    exactly `profile`, `predicate_states`, `optimization` and `fairness`;

    `optimization` SHALL be `"workload"` and `fairness` SHALL be the empty

    array. `predicate_states` SHALL be the complete ascending set of reachable

    complete product keys where the checked state predicate P holds. It is

    extensional evidence, not a target boundary: the checker SH'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Admission SHALL require one initial state, at most one actual S3\n \
    \ binding, no clocks or authored fairness, and FR-196's existing Markov\n  single-post\
    \ successor restrictions. The finite original graph MAY\n  cycle. At every live\
    \ state the authored workload SHALL fully resolve\n  semantic operation/receiver/non-random-argument\
    \ choices and typed draws\n  to one normalized distribution, without residual\
    \ scheduler or post-state\n  choice. A terminal SHALL use the existing infinite-trace\
    \ stutter\n  semantics: its same complete key has a probability-one self-loop\
    \ and\n  its P label is retained. No synth"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The document SHALL have exactly `identity`, `objective`, `kind`,\n \
    \ `values` and `components`, with `kind` equal to `\"long-run\"`; `ranking`\n\
    \  and `policy` SHALL be absent. Identity, exact rationals, full six-part\n  binding-bearing\
    \ model keys with null absent monitor/accumulator/pending,\n  strict canonical\
    \ JSON and unpadded base64url retain the existing owner\n  contracts. `values`\
    \ SHALL cover exactly the entire independently rebuilt\n  original reachable graph,\
    \ ordered by decoded complete key octets, with\n  values in [0,1]. Reachability\
    \ ranks or target labels SHALL NOT be used\n  to substitute f"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Each component SHALL use QSpec FR-331's existing exact\n  `{value, gain,\
    \ bias}` shape. `value` is `rho_C` in [0,1]; `gain` SHALL\n  equal `rho_C`, the\
    \ average original unweighted indicator reward in\n  FR-199's result. `bias` SHALL\
    \ have exactly the existing `{state, value}`\n  rows: every member of that component\
    \ once, sorted by decoded complete\n  key, with signed exact rational bias values.\
    \ Components SHALL be sorted\n  by their first bias key. Bias keys identify the\
    \ claimed membership;\n  no component id, producer SCC number, trusted partition\
    \ or additional\n  top-level field is introduced. Empty"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The checker SHALL independently derive every positive-support SCC and\n\
    \  closed bottom SCC (BSCC) of the original graph with its own bounded\n  graph\
    \ algorithms. The component list's bias-key sets SHALL match\n  exactly those\
    \ BSCCs, disjointly and completely; transient states SHALL\n  occur in no component.\
    \ This comparison SHALL NOT trust a producer's\n  partition, even when its per-row\
    \ equalities hold. There SHALL be at\n  least one BSCC in the complete finite\
    \ graph. In each BSCC C the checker\n  SHALL require bias zero at C's first complete\
    \ key and check, at every\n  s in C, `gain_C = rho_C` and\n "
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Every state in C SHALL have `V(s) = rho_C`. At each transient state\
    \ the\n  checker SHALL check `V(s) = sum_t p(s,t) * V(t)` using the original\n\
    \  workload distribution. These are the exact expected-component-value\n  conditions:\
    \ after all independently derived BSCCs are fixed, the\n  remaining finite states\
    \ are transient and the harmonic extension is\n  unique. No reachability positive-value\
    \ rank is required for this\n  `long-run` kind; neither a nested certificate nor\
    \ a new transport member\n  is introduced. Exact `AtLeast` and `AtMost` proofs\
    \ SHALL compare the\n  initial V against the checked thre"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Conflicting identity SHALL refuse before enumeration. A changed\n  predicate-state\
    \ set, optimization or profile for an otherwise admitted\n  claim SHALL refuse\
    \ as objective mismatch before acceptance. Structural\n  coverage/ordering/member\
    \ failures SHALL reject `Malformed` at member\n  `values` or `components` as appropriate.\
    \ A component partition mismatch\n  SHALL reject `InequalityFails` at the smallest\
    \ complete key whose\n  claimed component membership differs from its independently\
    \ rederived\n  BSCC membership, treating a transient state's membership as absent.\n\
    \  An entirely missing component t"
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- Source/document admission, exact key construction, complete graph\n\
    \  expansion, predicate/workload evaluation, SCC and membership passes,\n  bias/value\
    \ storage and exact arithmetic SHALL obey actual caller byte,\n  state/edge, work/storage\
    \ and scalar/rational limits and cancellation.\n  Exhaustion or cancellation SHALL\
    \ retain its typed stop and actual\n  control/consumption, without acceptance\
    \ or truth values. Undefined\n  reachable predicate/workload evaluation SHALL\
    \ preserve the existing\n  typed undefined/refusal path and SHALL NOT be coerced\
    \ to false, zero\n  or a missing component. Weighted for"
- id: FR-201-AC-27
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Under Steady, the original up/down chain with up-to-up 1999/2000 and down-to-up
    9/10 accepts a long-run document with one component value and gain 1800/1801,
    bias 0 at up and -2000/1801 at down (choose the fixture's canonical key order
    up before down), and both state values 1800/1801. Both workload bound sides accept
    at equality; AtLeast 1 and AtMost 999/1000 reject ThresholdMissed. Changing the
    bias, setting transported gain to 0, or replacing long-run evidence by reachability
    values prevents acceptance. No P state absorbs its original transitions.
- id: FR-201-AC-28
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A transient self-cycling s goes to s with 1/2, P-terminal g with 1/6 and
    not-P-terminal f with 1/3. Independent terminal stutter gives BSCCs {g} and {f},
    respective component value/gain 1 and 0 and bias 0; values s=1/3,g=1,f=0 accept
    both workload bounds at 1/3. A two-state deterministic cycle alternates P and
    not-P and accepts rho=gain=1/2 with normalized biases 0,-1/2 in that key order,
    despite periodicity. Merging/splitting/omitting a BSCC, adding a transient bias
    member or changing transient V rejects; harmonic values alone cannot certify a
    wrong recurrent value.
- id: FR-201-AC-29
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-27/28 documents round-trip canonically through a fresh checker process
    with no analyzer/provider dependency. Noncanonical rational/JSON/key encodings,
    duplicate or incomplete rows/components, foreign bindings, model-only keys, wrong
    normalization or missing components prevent acceptance. Identity conflict refuses
    before enumeration; a changed derived predicate set/optimization refuses objective
    mismatch. Equivalent source changes that preserve every checked condition may
    accept. Earlier profile bytes and verdicts remain unchanged.
- id: FR-201-AC-30
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Pre-cancellation and exhaustion/cancellation during admission, full-key
    creation, workload/predicate evaluation, SCC/membership passes and exact sums
    retain typed nonacceptance without truth values under the actual caller control.
    Reachable undefined P/workload evaluation is never relabelled false/zero. Weighted
    claims emit no document of this profile; the separate existing FR-199 weighted
    zero-weight control retains Unsupported(ZeroWeightComponent). Unresolved workload,
    every-scheduler/MDP, fairness, clocks, multi-post, multiple initials/bindings
    and unsupported kind/profile remain explicitly
- id: FR-199
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: examined
  excerpt: 'For the unweighted workload result, `gain` is the original indicator

    reward''s average `rho`, as in AC-1. FR-201''s closed DTMC long-run document

    retains that gain. Its checker subtracts the component value to obtain

    ADR-028 CE-3''s centered reward and zero gain; it SHALL NOT serialize zero

    in place of the result''s gain or require a second solver result format.

    '
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s `quire.probability-objective/dtmc-long-run/v1` closes the

    unweighted untimed workload case only. It retains QSpec''s existing

    `long-run` document and `{value,gain,bias}` component transport. FR-199''s

    original indicator-reward gain is rho; subtracting the component''s rho

    gives CE-3''s zero gain for the centered reward `[P]-rho`. The checker

    therefore checks `gain=rho` and `h=[P]-rho+Ph`. The zero gain in §15.3

    describes this centered equation, not a requirement to overwrite the

    transported original-reward gain in FR-199/QSpec FR-412''s example.


    The checker independently rebuilds the comp'
- id: TC-1401
  path: spec/test-cases/TC-1401-check-unweighted-dtmc-long-run-certificates.md
  role: examined
  excerpt: "1. Compile and admit ADR-028 §15.3's availability claim under Steady.\
    \ Use\n   fixture model identities whose complete key ordering puts up before\n\
    \   down. Produce and independently check the AC-27 component document,\n   including\
    \ both inclusive threshold sides. Offer it at the two\n   beyond-side thresholds.\
    \ Alter each bias/gain/value separately and try\n   a reachability-profile document\
    \ for the same source.\n2. Compile and admit AC-28's three-state graph under a\
    \ fully resolving\n   workload, and its periodic two-state graph. Check exact\
    \ component\n   boundaries, terminal stutter, centered equation"
```

## Limits

Code/Rust runtime gates, gap-analysis implementation qualification, and Jev criterion-strength analysis were not run: no production diff or executable new profile exists, and no Jev client was selected. Normative scoped validation passed; runtime trace/matrix completeness remains future work.
