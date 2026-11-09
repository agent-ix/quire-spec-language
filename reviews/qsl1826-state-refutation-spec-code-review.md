---
id: SR-20261109172912000059
title: code-review review of QSL-1826 state-predicate refutation replay
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; spec/functional/FR-202-replay-a-probabilistic-witness.md;
  spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
review_set: subset
---

## Summary

Frozen PR 2626 at 509fb879269b61197733c5f2e719fae785d4da02 against 5784ebb08306f10e400c5e050b2468978c3bb1d4. Ticket: QSL-1826. No applicable AssuranceProfile artifact was found under spec/. Review selection is the dispatched direct subset. Plan completion: not assessed. Full computed matrix and evidence advisor unavailable from prior runs; no repeated broad invocation or full assurance claim. Normative scoped validation passed, including zero grammar findings. No production tests were run for this specification-only change.

Documentation-only diff: no source, tests, dependencies, schemas or CI changed. Native semantic types and qualified evaluation retain their existing owners; no duplicated wire vocabulary, copied implementation, tracking machinery or weakened gate was added. Implementation/test alignment is not claimed for a specification extension awaiting implementation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: FR-202-AC-1
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: '§15.4''s `Deliver`: a one-path set (`send_b` lost, `send_b` lost) with
    scheduler `send_b` at both live states replays `refuted` with path probability
    `1/25 > 3/100`. §15.2 at `2 ms`: the one-path set (`request`, `attempt` with `d
    = 3 ms`, `outcome = Ok`) replays with probability `343/5000 > 1/20`.'
- id: FR-202-AC-2
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: §15.4's path with its second step changed to `send_a` refuses `invalid_runtime_input`/`invalid-value`
    (not the scheduler's choice); with `lost` drawn as a value outside `{true, false}`
    it refuses the same; the path listed twice settles `inconclusive`, `ReplayParity`
    (not prefix-free).
- id: FR-202-AC-3
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
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
  role: context_only
  excerpt: With `max_witness_paths` 0, §15.4's refutation carries subsystem evidence
    instead of a path set and still replays `refuted`. Replaying one envelope twice
    gives equal outcomes.
- id: FR-202-AC-7
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: 'Given a `ReplayRequestWire` whose measured encoded size is `B`, when the
    replay facade receives caller-configured `ReplayLimits` with `input_bytes = B
    - 1`, it returns `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded
    { bound: B - 1, actual: B }))` before byte-provision lookup or recompile.'
- id: FR-202-AC-6
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
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
  role: context_only
  excerpt: FR-204-AC-10's `Detour` maximum witness (`FreeDelays` rows given `[1]`
    to `B` delay 2 and given `[3]` to `B` delay 1, path set `A` at 1 ms and `B` at
    1 ms followed by `G` at 2 ms) replays `refuted` with `3/4 > 7/10`, the first step's
    merged probability `1/2` recomputed from row `[1]` and the `B` step's `1/2` from
    row `[3]`. The same witness with its rows swapped in order, with the row given
    `[3]` removed, with `B` delay 4 (outside `[1, 3]`), or as a `Choice` naming `WorkloadResolve{B,
    1}` at that state refuses `invalid_runtime_input`/`invalid-value`.
- id: FR-202-AC-5
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: 'FR-196-AC-5''s `Undefined` evidence (`drop` with `b = 0`, `UndefinedEvaluation{where:
    1, cause: absent-key}`) replays `refuted`; the same evidence with `where` set
    to 0 settles `inconclusive`, `ReplayParity`.'
- id: FR-202-AC-9
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: A checked-subject acyclic reachability fixture replays a target-first-hit
    path set of mass `2/3` against `AtMost 1/2` and a terminal-nonreach path set of
    mass `2/3` against `AtLeast 1/2`; each produces a token permitting only its matching
    EN-5 refuted record, with the accepted evidence retained. Repeating replay gives
    equal semantic outcomes. Neither replay needs an emitted package or analyzer graph
    types.
- id: FR-202-AC-10
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: In AC-9, altering the claim occurrence, package or binding refuses; altering
    an action, draw, post-state or full key cannot produce a token. Duplicate/prefix-overlapping
    paths, inconsistent scheduler choices, an earlier target on a first-hit path,
    and a live non-target leaf cannot reproduce a refutation. Mass equal to the relevant
    threshold yields replay parity.
- id: FR-202-AC-11
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: A pre-cancelled call, cancellation at the final acceptance poll, or exhaustion
    of each configured expansion/work/storage limit returns incomplete without a token
    or a refuted record. Unbounded cyclic, subsystem, randomized-policy, fairness,
    multiple-initial/instance and timed inputs report unsupported without a token.
    A caller cannot construct a reproduced token or settle EN-5 refuted from a bare
    engine result.
- id: FR-202-AC-12
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
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
  role: context_only
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
  role: context_only
  excerpt: At horizon 0, initially true P gives an empty accepting path of mass 1
    refuting AtMost 1/2, while initially false P gives an empty rejecting path refuting
    AtLeast 1/2; opposite-verdict empty paths yield parity. At horizon 2, a terminal
    initially false P closes by false extension with mass 1 and no fabricated stutter
    step, refuting AtLeast 1/2. A live undecided prefix after one failure does not
    refute; after two failures horizon rejection does, even though another model action
    is enabled. An initial true P path extended by an operation cannot reproduce.
- id: FR-202-AC-15
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: In AC-12 a missing represented path cannot be counted, equal-threshold
    represented mass yields parity, and a changed action/draw/post-state, duplicated
    draw-cylinder, overlapping path or inconsistent choice at one full key cannot
    produce a token. First-hit masses under slow are only 3/4 and cannot refute AtMost
    4/5. A policy may choose different enabled actions at the same model content with
    different remaining horizons; this is a consistent full-product policy.
- id: FR-202-AC-16
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: context_only
  excerpt: AC-12 replay with pre-cancellation, cancellation at its final acceptance
    poll, or each configured expansion/work/storage/arithmetic budget exhausted returns
    its typed incomplete cause with no truth, token or accepted evidence in a refuted
    record. A refused predicate preserves its typed refusal; FR-197 Bounds and PrecisionBudget,
    unsupported bounded until/always/reward/timed/fair inputs, subsystem evidence
    and nonreplayed exact results produce no bounded reproduced token. The native
    entry needs neither S4 emission nor QSpec FR-413 document transport.
- id: FR-202-AC-17
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Until fixture: initial A-and-not-B has actions a and b reaching B-and-not-A
    with probabilities 1/4 and 3/4, otherwise neither-A-nor-B followed by terminal
    B. At threshold 1/2, a''s rejecting cylinder of mass 3/4 refutes minimum AtLeast,
    and b''s accepting cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved
    workload has accepting and rejecting mass 1/2 each: its rejecting cylinders refute
    AtLeast 3/5 and accepting cylinders refute AtMost 2/5. All four produce only their
    matching reproduced token and retain semantic evidence; repeat replay gives equal
    outcomes. The failure prefix st'
- id: FR-202-AC-18
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Always fixture: initial nonterminal P has actions a and b reaching terminal
    P with probabilities 1/4 and 3/4, otherwise not-P followed by terminal P. At threshold
    1/2, a''s not-P cylinder of mass 3/4 refutes minimum AtLeast and b''s terminal-P
    cylinder of mass 3/4 refutes maximum AtMost. Equal-weight resolved workload refutes
    AtLeast 3/5 with rejecting mass 1/2 and AtMost 2/5 with accepting mass 1/2. Later
    P cannot repair the not-P prefix; the initial live P alone cannot be accepted.'
- id: FR-202-AC-19
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: With admitted threshold 1/2, empty paths of mass 1 refute AtMost for initial
    B-and-not-A until and initial terminal-P always; empty paths refute AtLeast for
    initial neither-A-nor-B until, terminal A-and-not-B until, and initial not-P always
    even with a later terminal P successor. Terminal A-and-not-B has until probability
    0 and terminal P has always probability 1 without a fabricated stutter step. Source
    threshold endpoints remain refused; probabilities 0 and 1 are admitted.
- id: FR-202-AC-20
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: In AC-17/18, changed source/package, foreign claim occurrence or S3 binding
    refuses identity mismatch; a changed formula that changes the represented verdict,
    changed threshold to equality, changed actual action/draw/post-state/full key,
    fabricated binding node, missing binding component or a formula-monitor key prevents
    a token. Formula spelling alone need not fail if the actual bound claim has identical
    semantics and identity checks pass. A wrong-verdict path, live undecided prefix,
    path extended beyond first success/failure, duplicate cylinder or prefix overlap
    yields no reproduced token; i
- id: FR-202-AC-21
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-17/18 two evidence paths demanding different actions at one complete
    choice key fail policy consistency; a policy under a resolved workload fails replay.
    Distinct supported draws reaching the same post-state contribute their separate
    exact masses once, while repeating a draw cannot increase mass. A raw reachable
    cycle beyond initial B for until or initial not-P for always is unsupported despite
    immediate absorption; acyclic counterparts remain admitted. Fairness, timed/reward/nested/bounded
    forms, multiple initials/bindings, unresolved workload, subsystem and randomized-policy
    evidence rema
- id: FR-202-AC-22
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-17/18 pre-cancellation, cancellation at final acceptance, or exhaustion
    of each caller expansion/work/storage/arithmetic limit during original admission
    or replay preserves its typed incomplete outcome with no truth, token or accepted
    evidence in a refuted record. Refused predicates preserve refusal. Bounds, PrecisionBudget,
    certificate acceptance alone and unreplayed exact engine verdicts cannot settle
    refuted. Native replay needs no emitted package or witness wire codec and imports
    no analyzer graph types.
- id: FR-202-state-predicate-paragraph-1
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'The checked-subject entry SHALL additionally admit `holds(A) until holds(B)`

    and `always holds(P)` under `INFINITE-TRACE`, using FR-201''s acyclic

    state-predicate admission: one initial state, at most one actual S3 binding,

    empty fairness, untimed subjects, existing Markov successor restrictions,

    minimum `AtLeast`, maximum `AtMost`, or a single resolved workload distribution

    with either comparison. Source bounds SHALL retain `MinAtLeast`/`MaxAtMost`

    threshold admission in the open interval (0,1); event probabilities 0 and 1

    remain valid. Nested/bounded temporal forms, event predicates, rewards,'
- id: FR-202-state-predicate-paragraph-2
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Before absorbing any outcome, the core SHALL independently enumerate the

    original reachable model graph and establish acyclicity apart from implicit

    terminal stutter. A cycle beyond an initial or intermediate decided outcome

    SHALL still refuse admission. An engine''s stopped graph, supplied labels or

    terminal flags SHALL NOT establish admission or replay agreement.'
- id: FR-202-state-predicate-paragraph-3
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Replay SHALL bind the actual checked occurrence, formula, infinite-trace

    profile, threshold, scheduler/workload and S3 parameter node/selected instance

    from the package and admitted subject. Qualified state-predicate evaluation

    SHALL classify position 0 and each actual semantic successor independently.

    For until, B decides acceptance even when A is false; neither A nor B decides

    rejection; A and not B continues until a decision or a genuine model terminal,

    where it rejects because infinite stutter never reaches B. For always, not-P

    decides rejection immediately, including before a later P stat'
- id: FR-202-state-predicate-paragraph-4
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Evidence SHALL use actual initial/post-state content, semantic scheduled

    identities, resolved model transitions and draws, and FR-196''s complete

    binding-bearing model-state keys with absent monitor, accumulator and pending

    components null, as in FR-201''s state-predicate profile. Formula-monitor keys,

    local state/action numbers, surrogate predicates, fabricated S3 nodes or

    synthetic terminal keys SHALL NOT substitute. This reuses the native evidence

    types and independent transition re-enumeration, not the reachability-only

    first-target/terminal-nonreach classifier or a trusted solver graph.'
- id: FR-202-state-predicate-paragraph-5
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Each path SHALL have positive recomputed exact mass. The core SHALL check

    pairwise prefix-free, disjoint execution/draw cylinders and deterministic

    memoryless scheduler consistency at every represented full choice key.

    Distinct supported draws reaching one state remain distinct cylinders;

    duplicates SHALL NOT multiply mass. Workload evidence SHALL contain no witness

    policy; replay SHALL rederive its exact action and draw weights. A witness

    policy need cover only represented choice states and requires no optimality

    assertion, because the admitted remaining choices can be extended consistently.'
- id: FR-202-state-predicate-paragraph-6
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'Let R sum the submitted rejecting-path masses and A sum the accepting-path

    masses. For minimum `AtLeast theta` (or workload `AtLeast`), every submitted

    path SHALL reject and replay SHALL require `R > 1 - theta`; the event probability

    under that policy/workload is at most `1 - R < theta`. For maximum `AtMost theta`

    (or workload `AtMost`), every path SHALL accept and replay SHALL require

    `A > theta`; the event probability is at least A. These are sufficient bounds,

    not an exact whole-event value. Unrepresented mass SHALL remain unknown.

    Equality, insufficient mass, opposite-verdict paths, duplic'
- id: FR-202-state-predicate-paragraph-7
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'EN-5 SHALL emit this subset''s path evidence only from exact results.
    `Bounds`

    intervals and `PrecisionBudget` SHALL NOT be relabelled exact. The existing

    opaque matching-token and retained-evidence settlement contract applies;

    certificate acceptance, a bare engine verdict or a reachability witness alone

    SHALL NOT settle these events refuted. Refused predicates and unsupported

    inputs retain their typed outcomes. Expansion/work/storage/arithmetic exhaustion

    and cancellation, including original-graph admission and the final acceptance

    poll, SHALL yield no truth, reproduced token or accepted evide'
- id: ADR-029-checked-subject-path-replay
  path: spec/decisions/ADR-029-lifecycle-cli-provider-plugin-cache-codegen-boundaries.md
  role: examined
  excerpt: "- **EN-5 checked-subject path replay (proposed amendment, QSL-1826).**\n\
    \  FR-202 adds a qualified-core entry over the checked package and admitted\n\
    \  subject for its narrow untimed acyclic unbounded reachability and bounded\n\
    \  origin eventually[0,h] holds(P) path-set slices, and FR-201's untimed acyclic\n\
    \  infinite-trace state-predicate until/always admission, alongside\n  the wire/recompile\
    \ facade. As with FR-201's checked-subject certificate\n  route, this avoids requiring\
    \ an emitted package that FR-185 refuses. The\n  core rebinds the claim and replays\
    \ actual semantic steps independently;\n  it imp"
```

## Verdict

PASS: no concrete introduced defect found. Mergeable for this specification slice under the repository owner policy separating incomplete assurance from engineering delivery. Evidence limitations remain explicit; implementation and wider ticket scope remain outstanding.
