---
id: SR-20261109172912000037
title: "integrity review of native bounded refutation"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; QSL-1826; FR-202, FR-201, ADR-028, ADR-029"
review_set: all
---

## Summary

Frozen PR #2620 at 1bb6cf42c1942b7381d7a1c172a62edc6deb2f5a, compared with 927ec42e352b5c3700c88fde46c9de50972dc414. Ticket: QSL-1826. Specification readiness only; this review does not claim implementation, qualification or shared wire completion. No applicable local AssuranceProfile exists: matches for that type were prose in prior reviews, not profile documents.

## Verdict

**PASS** for this specification slice. No introduced defects found. Implementation and wire readiness are not established.

## Analysis

The added bounded subsection specializes the preceding unbounded first-hit/terminal-leaf rules: horizon rejection may occur at a live state. The acyclicity refusal is explicitly restricted to the unbounded slice. FR-201 now separates certificate acceptance from FR-202 native witness replay; ADR-028/029 agree. No certificate acceptance, bare engine verdict, package mismatch or exhausted replay can establish the reproduced token. Atomic obligations have observable typed outcomes. Trace chain: US-023 → FR-202 → Test AC-12..16; US-023 → StR-001. The wire remains a separately blocked obligation, with no fallback pretending transport exists.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

The scope below records every criterion and statement examined; excerpts longer than 600 characters are verbatim initial excerpts. Full criteria were checked against parsed source.

```yaml
scope:
- id: FR-202-AC-1
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: '§15.4''s `Deliver`: a one-path set (`send_b` lost, `send_b` lost) with
    scheduler `send_b` at both live states replays `refuted` with path probability
    `1/25 > 3/100`. §15.2 at `2 ms`: the one-path set (`request`, `attempt` with `d
    = 3 ms`, `outcome = Ok`) replays with probability `343/5000 > 1/20`.'
- id: FR-202-AC-2
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: §15.4's path with its second step changed to `send_a` refuses `invalid_runtime_input`/`invalid-value`
    (not the scheduler's choice); with `lost` drawn as a value outside `{true, false}`
    it refuses the same; the path listed twice settles `inconclusive`, `ReplayParity`
    (not prefix-free).
- id: FR-202-AC-3
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: '§15.3''s per-window claim is refuted with subsystem evidence: a dyadic
    `Lower` certificate for `Pr(not E)` with value above `1/100`, which replay accepts
    through FR-201''s checker. FR-200-AC-2''s `Coin2` witness replays `refuted` and
    passes the fairness check; the same witness with the scheduler taking `wait` at
    the live state forever fails the fairness check and settles `inconclusive`, `ReplayParity`.
    With no authored fairness, FR-196''s capped x2 policy chooses IdleDelay3/4 and
    IdleObserve1/4, gives elapsed value5>3 and replays the threshold refutation; the
    same choice distribution after each ob'
- id: FR-202-AC-4
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: With `max_witness_paths` 0, §15.4's refutation carries subsystem evidence
    instead of a path set and still replays `refuted`. Replaying one envelope twice
    gives equal outcomes.
- id: FR-202-AC-7
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Given a `ReplayRequestWire` whose measured encoded size is `B`, when the
    replay facade receives caller-configured `ReplayLimits` with `input_bytes = B
    - 1`, it returns `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded
    { bound: B - 1, actual: B }))` before byte-provision lookup or recompile.'
- id: FR-202-AC-6
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Over the two-point model (`M` 1 or 3, probability 1/2 each), the path to
    `M = 1` (`V = 1/2`, `N = 0`) replays `refuted` for `quantile 0.5 of (M) >= 3`
    (`M` is a placeholder for the claim's parenthesised measure) (equality refutes),
    and settles `inconclusive`, `ReplayParity` for `quantile 0.9 of (M) >= 3`, which
    holds. Over its variant that activates with probability 1/2 (`M` 1 or 3 with probability
    1/4 each), the path set of the `M = 1` path and the not-activated path (`V = 1/4`,
    `N = 1/2`) replays `refuted` for `quantile 0.5 of (M) >= 3`, and the `M = 1` path
    alone settles `inconclusive`, `Re
- id: FR-202-AC-8
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: FR-204-AC-10's `Detour` maximum witness (`FreeDelays` rows given `[1]`
    to `B` delay 2 and given `[3]` to `B` delay 1, path set `A` at 1 ms and `B` at
    1 ms followed by `G` at 2 ms) replays `refuted` with `3/4 > 7/10`, the first step's
    merged probability `1/2` recomputed from row `[1]` and the `B` step's `1/2` from
    row `[3]`. The same witness with its rows swapped in order, with the row given
    `[3]` removed, with `B` delay 4 (outside `[1, 3]`), or as a `Choice` naming `WorkloadResolve{B,
    1}` at that state refuses `invalid_runtime_input`/`invalid-value`.
- id: FR-202-AC-5
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'FR-196-AC-5''s `Undefined` evidence (`drop` with `b = 0`, `UndefinedEvaluation{where:
    1, cause: absent-key}`) replays `refuted`; the same evidence with `where` set
    to 0 settles `inconclusive`, `ReplayParity`.'
- id: FR-202-AC-9
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A checked-subject acyclic reachability fixture replays a target-first-hit
    path set of mass `2/3` against `AtMost 1/2` and a terminal-nonreach path set of
    mass `2/3` against `AtLeast 1/2`; each produces a token permitting only its matching
    EN-5 refuted record, with the accepted evidence retained. Repeating replay gives
    equal semantic outcomes. Neither replay needs an emitted package or analyzer graph
    types.
- id: FR-202-AC-10
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-9, altering the claim occurrence, package or binding refuses; altering
    an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping
    paths, inconsistent scheduler choices, an earlier target on a first-hit path,
    and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant
    threshold yields replay parity.
- id: FR-202-AC-11
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion
    of each configured expansion/work/storage limit returns incomplete without a token
    or a refuted record. Unbounded cyclic, subsystem, randomized-policy, fairness,
    multiple-initial/instance and timed inputs report unsupported without a token.
    A caller cannot construct a reproduced token or settle EN-5 refuted from a bare
    engine result.
- id: FR-202-AC-12
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A cyclic undelivered model has action slow delivering with probability
    1/2 and otherwise returning undelivered, and fast delivering with probability
    3/4 and otherwise returning undelivered. For origin eventually[0,2] holds(delivered),
    slow at both full choice keys refutes minimum AtLeast 4/5 with the two-failure
    path of mass 1/4 greater than 1/5; fast at both keys refutes maximum AtMost 9/10
    with first-hit paths of masses 3/4 and 3/16, sum 15/16. Equal-weight resolved
    workload has per-step delivery 5/8: all two-failure action/draw cylinders sum
    to 9/64 and refute AtLeast 7/8; all first-hit cyl'
- id: FR-202-AC-13
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-12 initial and one-failure states share model content but carry different
    complete formula keys with remaining contexts 2 and 1. Changing a supplied key
    to a model-only key, null/local-index monitor, another remaining horizon or binding
    prevents a token. Rebinding to a changed source package, horizon, threshold, claim
    occurrence, actual S3 over-parameter node or selected instance prevents reuse
    of the original token/evidence as an accepted refutation of that item. A checked
    source with two possible object values is restricted to one admitted over binding;
    replay uses that object's predicate
- id: FR-202-AC-14
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: At horizon 0, initially true P gives an empty accepting path of mass 1
    refuting AtMost 0, while initially false P gives an empty rejecting path refuting
    AtLeast 1; opposite-verdict empty paths yield parity. At horizon 2, a terminal
    initially false P closes by false extension with mass 1 and no fabricated stutter
    step, refuting AtLeast 1/2. A live undecided prefix after one failure does not
    refute; after two failures horizon rejection does, even though another model action
    is enabled. An initial true P path extended by an operation cannot reproduce.
- id: FR-202-AC-15
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-12 a missing represented path cannot be counted, equal-threshold
    represented mass yields parity, and a changed action/draw/post-state, duplicated
    draw-cylinder, overlapping path or inconsistent choice at one full key cannot
    produce a token. First-hit masses under slow are only 3/4 and cannot refute AtMost
    4/5. A policy may choose different enabled actions at the same model content with
    different remaining horizons; this is a consistent full-product policy.
- id: FR-202-AC-16
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-12 replay with pre-cancellation, cancellation at its final acceptance
    poll, or each configured expansion/work/storage/arithmetic budget exhausted returns
    its typed incomplete cause with no truth, token or accepted evidence in a refuted
    record. A refused predicate preserves its typed refusal; FR-197 Bounds and PrecisionBudget,
    unsupported bounded until/always/reward/timed/fair inputs, subsystem evidence
    and nonreplayed exact results produce no bounded reproduced token. The native
    entry needs neither S4 emission nor QSpec FR-413 document transport.
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'The same native entry SHALL additionally admit origin-activated untimed

    `eventually[0,h] holds(P)`, with finite nonnegative event-position horizon h,

    one initial state and at most one admitted binding, for minimum `AtLeast`,

    maximum `AtMost`, and a bound workload with either comparison. Admission SHALL

    retain FR-201''s bounded subset''s supported Markov successor restrictions and

    single resolved workload distribution; unsupported residual post-draw choices

    SHALL NOT be silently resolved by selecting a successor. Other temporal forms,

    timing, rewards, fairness and randomized policies remain outsi'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Replay SHALL independently bind the actual checked claim occurrence, threshold,

    formula, profile and scheduler/workload from the `CheckedPackage` and admitted

    `ModelSystem`. For an `over` claim the monitor context SHALL use the actual S3

    parameter node and selected instance key, as FR-201''s bounded checker does;

    an alias, fabricated parameter node or engine-supplied predicate SHALL NOT

    replace it. The existing item/package and binding checks and reproduced-token

    settlement contract apply unchanged.'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'The core SHALL use `qsl-eval`''s qualified bounded-monitor initialization,
    step,

    canonical key and terminal false-extension operations. It SHALL read position
    0

    once, advance one position per actual resolved model step, include position h,

    and stop a path at its first decided verdict. Complete initial and post-product

    keys SHALL carry FR-196''s actual formula-monitor context, parsed automaton key

    and remaining horizon/debts; decided keys SHALL use the counter-free absorbing

    verdict. Local monitor indices, model-only keys, a null monitor and unbounded

    reachability keys SHALL NOT substitute for th'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'At an undecided model leaf, replay SHALL independently establish that
    there is

    no enabled continuation and apply qualified terminal false-extension closure.

    This closure contributes mass factor 1 and no model step, draw or elapsed time;

    the final supplied model-step key remains the actual post-observation key, and

    the closed decision is derived by replay. It SHALL NOT require fabricated

    stutter steps or treat a live non-target prefix as a rejecting leaf. Horizon

    exhaustion may decide rejection at a live model state. Horizon 0 is decided by

    the initial position alone, permitting an empty path w'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Every represented path SHALL have positive independently recomputed exact

    probability, rederived semantic steps/draws and keys, and pairwise disjoint

    prefix-free execution cylinders. Duplicate paths and paths that continue past

    a decided verdict SHALL fail replay agreement. Under every scheduler, the

    deterministic witness policy SHALL agree at every represented full product key;

    it need cover only the evidence''s choice states. Remaining finite-horizon

    choices can be extended arbitrarily to enabled choices, so no assertion of

    optimality is required. Under a workload the policy SHALL be empty, a'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Let R be the sum of represented rejecting-path probabilities, and A the
    sum of

    represented accepting-path probabilities, under that consistent policy or

    workload. For `AtLeast theta`, every submitted path SHALL decide rejection and

    replay SHALL reproduce a refutation only if `R > 1 - theta`: the actual event

    probability under that scheduler/workload is at most `1 - R < theta`, hence

    the minimum also fails. For `AtMost theta`, every submitted path SHALL decide

    acceptance and replay SHALL reproduce only if `A > theta`: the event probability

    under that scheduler/workload is at least A, hence the '
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'This increment SHALL emit bounded path evidence only from FR-197 `Exact`

    results. `Bounds`, `PrecisionBudget`, refused predicates, unsupported shapes,

    unreplayed engine results and certificate acceptance alone SHALL NOT become a

    bounded refutation or reproduced token. Expansion/work/storage/arithmetic

    exhaustion and cancellation, including the final acceptance poll, SHALL preserve

    their original non-truth outcomes with no token or accepted probabilistic

    evidence in a refuted record. The global QSpec FR-413 witness wire still lacks

    the closed bound, scheduler-choice and semantic product-action/'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'In addition to the wire facade below, layer 6 SHALL expose a checked-subject

    path-set replay entry for untimed acyclic unbounded reachability claims and the

    bounded event-position subset below, with no fairness,

    one initial state, at most one bound instance, and a deterministic memoryless

    witness scheduler (or the bound workload). It SHALL take the `CheckedPackage`,

    its admitted `ModelSystem`, the claim occurrence and binding, typed path-set

    evidence, caller-configured expansion/work/storage limits and `&Cancel`.

    This entry decodes no wire and recompiles nothing, as the checked-subject

    certifi'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Typed evidence SHALL retain actual semantic action identities, resolved

    `ModelTransition` content and draws, initial and post-state content, and complete

    canonical product keys. The entry SHALL reuse the qualified semantic types

    (`ScheduledIdentity` and `ModelTransition`) without importing `qsl-analyze`

    product/action types or treating local state/action numbers as identities.

    The core SHALL independently re-enumerate enabled transitions, evaluate target

    predicates, check the scheduler, and recompute exact path mass and prefix freedom

    under the Replay rules below. An asserted transition probab'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'For an `AtMost` bound `Pr(E) <= theta`, accepted paths SHALL end at their
    first

    target hit and their mass SHALL exceed `theta`. For an `AtLeast` bound

    `Pr(E) >= theta`, accepted paths SHALL avoid the target and end at leaves where

    the core independently confirms no enabled continuation; their mass SHALL

    exceed `1 - theta`. An initial target is a first hit with an empty path; a live

    non-target prefix is not a terminal nonreach leaf. These are sufficient

    refutations under the bound workload or a consistent deterministic witness

    policy; an engine''s assertion that a policy is optimal is not replay'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'The entry SHALL produce an opaque reproduced-witness token only after
    all

    checks and a final cancellation poll. The token SHALL bind the package, claim

    occurrence, subject binding and accepted evidence. Layer-6 EN-5 terminal

    construction SHALL require the matching token to settle `refuted` and retain

    the probabilistic counterexample evidence in the analyze record. A caller

    SHALL NOT construct the token or turn a bare engine verdict into `refuted`.

    Replay disagreement/refusal SHALL retain OP-2''s `replay_parity`/`replay_refused`

    settlement; exhaustion and cancellation SHALL remain `incomplete` w'
- id: FR-202
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'For the unbounded subset this entry SHALL report unsupported for cyclic
    subjects.

    Both subsets SHALL report unsupported for subsystem evidence,

    randomized schedulers, fairness, multiple initial states/instances, timed or

    other claim forms outside this slice. It SHALL NOT project them onto this

    slice. This additional entry does not change the wire facade obligation, the

    broader replay rules, or any public JSON key or witness codec.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "- The checker SHALL independently construct the model/monitor product\
    \ using\n  `qsl-eval`'s qualified property-automaton translation, initialization\
    \ and\n  transition semantics. It SHALL read position 0 during initialization\
    \ and\n  one subsequent position per observation step, including the inclusive\n\
    \  position h. At a terminal model state it SHALL apply the existing FR-126\n\
    \  profile false-extension closure, rather than invent model progress or\n  infer\
    \ acceptance from current predicate truth.\n- `qsl-eval` SHALL own a canonical\
    \ monitor-state key operation alongside its\n  existing automaton key own"
- id: ADR-028
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s closed reachability document profile owns the QSL specialization

    inside QSpec FR-412''s existing open `objective` object. Its versioned

    `profile` selects extensional complete target keys, independently checked

    against the claim, the existing minimum/maximum/workload optimization and

    an explicitly empty fairness array. Extensional targets avoid inventing a

    second semantic expression language. The discriminator versions this closed

    member contract without adding a top-level certificate field. FR-196''s

    complete binding-bearing key basis remains authoritative despite QSpec''s

    older five-com'
- id: ADR-029
  path: spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
  role: examined
  excerpt: "- **Input.** The checked package; the claim items, each named by its\n\
    \  occurrence key and carrying the settlement CG's `negotiate_*` gave it,\n  which\
    \ `analyze` routes and never makes itself (FR-281); the subject binding (the `ModelSystem`\
    \ and the initial\n  population, FR-120); each engine's request settings and budgets\
    \ (for\n  example ADR-018's model-check limits); the accounting limits; `&Cancel`.\n\
    - **Output.** `AnalyzeOutcome`: one terminal record per requested item, with\n\
    \  its O-16 category, its typed cause, its basis, its counterexample or\n  certificate\
    \ where it has one, and its accounting."
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: "- Under a workload, the product SHALL be a DTMC whose edges are FR-187's\n\
    \  `weighted_steps`; a `NotMarkov` from FR-187 SHALL stop the build with\n  `Unsupported(NotMarkov{…})`.\n\
    - Under every scheduler, the product SHALL be an MDP whose actions at a\n  state\
    \ are FR-187's `actions`. A draw with one post-state SHALL be an edge\n  with\
    \ the draw's probability, summed over draws that reach the same\n  post-state.\
    \ A draw with several post-states SHALL lead, with its\n  probability, to an intermediate\
    \ state keyed by (state, action, drawn\n  vector), whose actions are its post-states,\
    \ each with probability 1"
- id: FR-338
  path: spec/functional/FR-338-check-an-en-1-closure-certificate.md
  role: context_only
  excerpt: '`AutomatonStateKey` is the property-automaton translation''s canonical

    encoding of an automaton state (`qsl-eval`, ADR-018 LA-2), the same for

    every order in which the states are materialized.'
- id: US-023
  path: spec/usecase/US-023-prove-a-probabilistic-property-exactly-over-every-scheduler.md
  role: context_only
  excerpt: '**As a** verification operator with a probabilistic QSL model

    **I want** a probabilistic claim proved or refuted exactly, under a stated

    workload, over every scheduler, or over every fair scheduler, including

    claims sampling cannot settle (rare events, thresholds near the true value,

    long-run availability, expected times, deadlines of timed models)

    **So that** a proof carries a certificate a small checker verifies in exact

    rationals, a refutation carries an adversary and paths I can replay, and no

    floating-point number decides a verdict.'
- id: StR-001
  path: spec/stakeholder/StR-001-native-assessment-trust.md
  role: context_only
  excerpt: Verification operators require that native finite-state assessments shall
    retain the meaning and identity of the selected authored obligation.
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'The bounded subset is a capability extension of the same

    `quire.probability-objective/reachability/v1` document contract: its members,

    extensional target meaning, optimization, fairness and complete-key transport

    do not change. Existing unbounded documents retain their meaning and admission.

    No second formula AST, opaque monitor id or public horizon field is introduced.'
- id: FR-201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: "- The checker SHALL independently construct the model/monitor product\
    \ using\n  `qsl-eval`'s qualified property-automaton translation, initialization\
    \ and\n  transition semantics. It SHALL read position 0 during initialization\
    \ and\n  one subsequent position per observation step, including the inclusive\n\
    \  position h. At a terminal model state it SHALL apply the existing FR-126\n\
    \  profile false-extension closure, rather than invent model progress or\n  infer\
    \ acceptance from current predicate truth.\n- `qsl-eval` SHALL own a canonical\
    \ monitor-state key operation alongside its\n  existing automaton key own"
```
