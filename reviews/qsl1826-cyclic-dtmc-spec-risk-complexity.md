---
id: SR-20261109172912000099
title: risk-complexity review of cyclic DTMC eventual proof specification
type: SpecReview
analysis: risk-complexity
scope: agent-ix/quire-spec-language; QSL-1826; FR-201, FR-282, ADR-028
review_set: all
---

## Summary

Frozen PR #2638 at b11d8683e2bba273a8885aa7cbccc85b0ac6f768; base 33f40b811e1f2e1f5c7dc809855eadd155d7e3e3. Ticket: QSL-1826. Specification readiness only. No applicable type: AssuranceProfile document found. Runtime, API coverage, raw-key working-memory admission, FR-413 wire and upstream merge readiness are not established.

## Verdict

**PASS** — No introduced specification defects found. Implementation qualification remains unassessed.

## Analysis

FR-201 technical risk high (new cyclic graph/equation soundness and hard O(V+E) graph work/storage bound), volatility medium (external key-budget ownership). Mitigate with self-loop/leak, target-free BSCC and multistate SCC oracles; adversarial equation/rank/key/objective mutations; per-pass exhaustion/cancellation. FR-282 technical risk medium and volatility medium (fresh-process/shared source contracts), mitigated by real bounded-source process checks and preserved typed refusal. Failure-domain review finds no introduced specification defect; prerequisites stay open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Excerpts are verbatim prefixes up to 600 characters; full parsed acceptance criteria were examined.

```yaml
scope:
- id: FR-201-AC-1
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: '§15.4 at threshold 0.95: the `Exact` certificate with values `24/25`,
    `4/5`, 1 at delivered states and 0 at the two-loss state is accepted with side
    `AtLeast`. The same certificate with `4/5` replaced by `9/10` is rejected, naming
    the first state in canonical order where the minimum over actions differs from
    the certified value; the item settles `inconclusive`, `CertificateRejected`.'
- id: FR-201-AC-2
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: §15.3's `LongRun` certificate (component value `1800/1801`, reward `[up]
    − 1800/1801`, gain 0, bias 0 at `up` and `−2000/1801` at `down`) is accepted;
    with the bias at `down` changed to `−2001/1801` it is rejected. §15.1's dyadic
    `Lower` certificate at 64 bits, ranked by remaining horizon, is accepted with
    side `AtLeast`.
- id: FR-201-AC-3
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: A `Lower` certificate for §15.4 with its ranking omitted is rejected; one
    whose identity names a different claim is refused before any re-enumeration; §15.6's
    `FairTerminates` `Exact` certificate is accepted only after the checker recomputes
    that no fair end component exists, and the same certificate with its fairness
    set removed is rejected. With no authored fairness, FR-196's geometric timed policy
    is checked for derived TS-5 progress and its exact values5/4/3; rank3/2/1 has
    a positive-probability goal edge to rank0. Missing observation progress, an Observe-as-Delay
    policy, a finite maximum3
- id: FR-201-AC-4
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: A proof whose certificate the checker accepts settles `proved`; with a
    checker `max_states` of 2 the check stops and the item settles `Incomplete(LimitReached{…})`
    naming `max_states`. The same applies while rederiving R/waiting components or
    checking randomized observation evidence, with the actual consumed value/setting
    retained.
- id: FR-201-AC-5
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'An `Opt` model whose initial state has the actions `go`, which activates
    and then sets `M` to 1 with probability `1/4` and to 3 with `3/4`, and `skip`,
    which never activates. For `quantile 0.5 of (M) >= 3` (`M` is a placeholder for
    the claim''s parenthesised measure) over every scheduler, the `Upper` certificate
    with `y = 0` at the initial state and `−1/4` after `go` is accepted with side
    `AtLeast`: only `skip` is tight, and no activated state is reachable through it.
    The same claim over a variant whose `go` sets `M` to 1 and 3 with `1/2` each,
    with `y = 0` at both states, is rejected, naming t'
- id: FR-201-AC-7
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'Given a `ReplayRequestWire` whose measured encoded size is `B`, when the
    checker receives caller-configured `ReplayLimits` with `input_bytes = B - 1`,
    it returns `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded
    { bound: B - 1, actual: B }))` before byte-provision lookup or recompile.'
- id: FR-201-AC-6
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'Rejection rules and loci: AC-1''s certificate with `4/5` replaced by `9/10`
    is rejected `InequalityFails` at the `ProbabilisticProductState` of that first
    state; AC-2''s `LongRun` certificate with the changed bias, `InequalityFails`
    at the `down` state; AC-3''s `Lower` certificate with its ranking omitted, `Malformed`
    at `CertificateMember("ranking")` before any re-enumeration; AC-2''s `Lower` certificate
    with one named non-target state''s rank removed, `RankMissing` at it, and with
    that rank raised to the rank of its only positive successor, `RankBroken` at it;
    AC-1''s accepted certificate offered '
- id: FR-201-AC-8
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: 'Over each of those ACs'' own model, the checked-subject form gives the
    verdict, rule and locus that AC states: AC-1''s certificate over §15.4''s `Link`,
    including the `4/5` replaced by `9/10` rejection; AC-2''s `LongRun` certificate
    over §15.3''s model and its dyadic `Lower` certificate over §15.1''s; AC-3''s
    `FairTerminates` certificates over §15.6''s `Coin` model, the fairness-removed
    one rejected; and AC-5''s tight-action cases over its `Opt` model.'
- id: FR-201-AC-9
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: For an admitted acyclic eventual claim, writing and reading the closed
    profile preserves identity, exact rational values, complete keys, targets and
    required ranks byte for byte. Duplicate/out-of-order rows, unreduced 2/4, denominator
    0, missing ranking, extra fields, padded base64url and a model-only key are rejected.
- id: FR-201-AC-10
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: A valid document offered for another identity refuses before enumeration;
    replacing its targets or optimization while retaining valid Bellman rows refuses
    objective mismatch. A same-model key with a different binding is rejected rather
    than merged. Altering a value fails equality; removing or breaking positive-value
    rank progress fails ranking even when equalities hold.
- id: FR-201-AC-11
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass
    the profile checker at an attained threshold; a threshold strictly beyond the
    certified initial value fails. Unbounded cyclic, timed, reward, fair, multi-initial
    and multi-binding cases, bounded forms outside the admitted subset and non-exact
    kinds return explicit unsupported with no accepted certificate. Pre-cancelled
    input and exhausted byte/state/edge/arithmetic budgets stop without a truth value.
- id: FR-201-AC-12
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: ADR-028 §15.4's bounded Deliver at horizon 2 gives accepted exact minimum
    AtLeast at 24/25, maximum AtMost at 99/100 and Even workload at 391/400; after
    one loss the minimum is 4/5 and workload 17/20. Each threshold beyond the certified
    side fails. Existing unbounded profile documents retain their verdict and canonical
    bytes.
- id: FR-201-AC-13
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: A model that stays undelivered with probability 1/2 and delivers with probability
    1/2 per step has bounded horizon-2 probability 3/4 despite its raw self-cycle.
    The initial and one-loss visits share the model key but have distinct full formula-bearing
    keys and ranks 2 and 1. Horizon 0 accepts iff P holds at position 0. A terminal
    undelivered state closes to rejection by false extension without enumerating stutter
    positions.
- id: FR-201-AC-14
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: For AC-13's valid certificate, replacing a running monitor with a local
    StateId, null, another remaining horizon, or another binding cannot certify the
    claim. Changing rank 2 to 3 while retaining valid values fails RankBroken at that
    full key. An exact document fabricated from a dyadic interval is never emitted;
    unsupported lower/upper documents and precision stops produce no accepted exact
    certificate.
- id: FR-201-AC-15
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: For until, an initial A-and-not-B state has choices a and b reaching B
    with probability 1/4 and 3/4 respectively, otherwise a neither-A-nor-B state that
    later reaches B. Exact minimum AtLeast 1/4 and maximum AtMost 3/4 are accepted;
    an equal-weight resolved workload accepts both sides at 1/2. The failure state
    has value 0 and its later B successor is not expanded. Thresholds 1/3 for minimum
    AtLeast, 2/3 for maximum AtMost, and 3/5 AtLeast or 2/5 AtMost for that workload
    fail ThresholdMissed. A reached B-and-not-A state has value 1; initial B accepts
    at 1, initial neither-A-nor-B at 0, and term
- id: FR-201-AC-16
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: For always, an initial nonterminal P state has choices a and b reaching
    terminal P with probability 1/4 and 3/4 respectively, otherwise not-P followed
    by terminal P. Direct-event minimum AtLeast 1/4, maximum AtMost 3/4 and equal-weight
    resolved workload both sides at 1/2 are accepted. A prefix not-P has value 0 despite
    later P; initial terminal P has value 1 and initial not-P value 0. Treating the
    initial P as target, using 3/4 as minimum via an unswapped complement, or changing
    either boundary value fails checking. The beyond-side thresholds of AC-15 fail
    here too.
- id: FR-201-AC-17
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: Canonical state-predicate documents for AC-15 and AC-16 round-trip byte
    for byte. Replacing until B by a different checked predicate that changes the
    derived target set, or always P by one that changes failures, prevents acceptance
    of the original document under that claim even with internally consistent old
    rows. Changed event/optimization/targets/failures refuses objective mismatch;
    a foreign S3 binding or altered complete key is not an alias. Altered value, missing
    coverage or broken positive-value rank fails its existing check. A reachability/v1
    document cannot encode these events by relab
- id: FR-201-AC-18
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: AC-15 and AC-16 obey pre-cancellation and byte/state/edge/arithmetic limits
    throughout independent admission, original/stopped graph construction and checking,
    with typed stop/cancellation and no accepted truth value. Cycles in the original
    graph even after an early success, nonempty fairness, clocks, rewards, multiple
    initials/bindings, nested temporal forms, unresolved workload and non-exact documents
    remain explicitly unsupported. No state-predicate proof document is treated as
    a refutation witness.
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
    edge entering B gives value 4, including the entry cost. Swapping costs 2 and
    6 between those equally probable draws preserves the value 4; the same certificate
    accepts after independent recomputation when every equation, target, identity
    and rank remains valid. Changing the costs to 2 and 8 falsifies the certified
    initial value 4 and rejects; substituting a fixed unit-step cost also rejects
    because its value 1 fails that equation. Initial B has value 0 even with an enabled
    positive-cost operation. A reached B absorbs a la
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
- id: FR-201-AC-23
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A DTMC s with probabilities 1/3 each to s, target g and non-target absorbing
    f has exact eventual probability 1/2; values (s,g,f)=(1/2,1,0), ranks (1,0,0)
    accept minimum AtLeast and maximum AtMost at 1/2. Target g absorbs despite its
    original outgoing cycle. Initial target accepts at 1 and non-target terminal at
    0 under infinite stutter. Thresholds 3/5 AtLeast and 2/5 AtMost reject ThresholdMissed.
    A target-free two-state closed cycle has values/ranks 0 and accepts at 0; assigning
    its states 1/2 despite valid harmonic equalities rejects.
- id: FR-201-AC-24
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: In a transient SCC, a goes to b with probability 1, b to a with 1/2, target
    g with 1/4 and closed failure f with 1/4. Values a=b=1/2, g=1, f=0 and ranks a=2,b=1,g=f=0
    accept; both SCC members having condensation rank 1 rejects RankBroken. A fully
    authored workload at s choosing equally between deterministic return-to-s and
    a distribution g:1/3,f:2/3 resolves to s:1/2,g:1/6,f:1/3 and accepts exact value
    1/3, rank 1 on either workload bound at equality; 2/5 AtLeast and 1/4 AtMost reject.
- id: FR-201-AC-25
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-23/24 documents round-trip canonically; wrong values, distances, missing/extra
    coverage, foreign binding, altered complete model key, model-only key, changed
    derived targets/optimization or mismatched identity prevent acceptance. Changed
    source probabilities that falsify equations reject; equivalent source changes
    may accept. A live state with stay and exit scheduler choices refuses as unsupported
    even if a producer supplies a policy or equal values, including when that state
    lies beyond an early target. Unresolved workload, fairness, clocks, rewards, other
    formulas, multi-post and multiple
- id: FR-201-AC-26
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: AC-23/24 independently check with no solver dependency. Source/document/key
    byte, state/edge, graph work/storage and scalar/rational budgets are exercised
    at exhaustion, including SCC, reverse reachability and distance construction;
    actual typed limit and consumption are retained. Pre-cancellation and cancellation
    during each pass or exact sum stop without acceptance or truth values; no fabricated
    upstream key-budget API or cyclic refutation is claimed.
- id: FR-282-AC-1
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: 'The EN-5 certificate from FR-281-AC-1, written to bytes by `analyze` and
    passed with the byte provision and the subject binding to the EN-5 checker in
    a fresh process that links only qualified-core crates, is accepted: the process
    re-checks the provision''s sources into a `CheckedPackage`, admits the `ModelSystem`
    and runs the checked-subject form.'
- id: FR-282-AC-2
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: The same certificate with one probability bound changed is rejected with
    a cause naming the failing obligation.
- id: FR-282-AC-3
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: A zone certificate from an ADR-026 zone-search run over a dense-time model
    is accepted by the zone checker, and the same certificate with one zone removed
    is rejected.
- id: FR-282-AC-5
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: The EN-1 closure certificate of an invariant proved over a finite model
    is accepted by the EN-1 closure checker; the same certificate with one reachable
    product state removed is rejected with a cause naming the successor it no longer
    contains.
- id: FR-282-AC-6
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: The EN-1 component certificate of `always eventually q` under `fair weak
    inc`, proved over a finite model, is accepted by the SCC and ranking checker;
    the same certificate with two components swapped out of topological order is rejected
    with a cause naming the edge that runs backward.
- id: FR-282-AC-7
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: The simulation-relation certificate of a refinement that holds is accepted
    by the simulation-relation checker; the same certificate with one pair removed
    from the relation is rejected with a cause naming the concrete step left without
    a matching abstract step.
- id: FR-282-AC-8
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: The product-closure certificate of a two-copy hyperproperty proved over
    a finite model is accepted by the product-closure checker; the same certificate
    with one product-state key removed is rejected with a cause naming the successor
    it no longer contains.
- id: FR-282-AC-9
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: '**RETIRED (staleness check withdrawn).** The certificate `package_id`
    check is deleted.'
- id: FR-282-AC-10
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: '**RETIRED (staleness check withdrawn).** The certificate `package_id`
    check is deleted.'
- id: FR-282-AC-11
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: An exact acyclic eventual certificate from a random/workload source package
    that S4 refuses is written, then accepted in a fresh executable linking only qualified-core
    crates after bounded source admission and S1 to S3 checking. No emitted-package
    bytes or fabricated replay request are supplied. The same executable rejects changed
    values, refuses mismatched identity/objective, and preserves typed source/hash
    failures.
- id: FR-282-AC-12
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: With a source provision or certificate document one byte above its caller
    ceiling, the fresh checker stops/refuses before reading beyond that ceiling; pre-cancellation
    and cancellation during graph expansion stop without acceptance or a truth value.
    Exhausted state, edge and arithmetic budgets retain the actual named setting and
    consumption.
- id: FR-282-AC-13
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: A real FR-197 Exact bounded event-position certificate for FR-201-AC-13
    is written by analyze and accepted from its canonical document bytes in a fresh
    qualified-core-only process using the source provision and actual subject/claim
    binding. The process independently reconstructs formula-monitor keys and terminal
    closure through qsl-eval, without qsl-analyze or provider linkage. Changing a
    running monitor horizon, binding or bounded rank prevents acceptance.
- id: FR-282-AC-14
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: Analyze writes the exact until and always proof documents of FR-201-AC-15
    and AC-16; a fresh executable linking only qualified-core crates accepts their
    canonical bytes after independent source S1 to S3 admission and actual subject/claim
    binding, including random/workload packages that S4 cannot emit. It independently
    derives the original and stopped graphs, predicate labels and terminal-stutter
    boundary semantics. Changed formula with changed extensional labels, event, optimization,
    failure/target set, binding or complete key prevents acceptance; a threshold beyond
    the certified side rejects.
- id: FR-282-AC-15
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
  excerpt: For AC-14, source/document byte ceilings, state/edge/arithmetic limits,
    pre-cancellation and cancellation during independent graph expansion preserve
    typed nonacceptance with the actual named limit, consumption or cancellation.
    No analyzer/provider linkage, emitted-package bytes, fabricated replay request,
    solver labels, local state ids or new witness wire are used.
- id: FR-282-AC-16
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: context_only
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
  role: context_only
  excerpt: For AC-16, source/document byte ceilings, state/edge/rational/key-representation
    budgets, pre-cancellation and cancellation during independent reward evaluation/graph
    expansion preserve the actual typed failure, named limit/consumption or cancellation
    without acceptance or truth values. Both older probability document profiles retain
    their canonical bytes and verdicts.
- id: FR-282-AC-18
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: Analyze writes real exact cyclic DTMC eventual documents for FR-201-AC-23/24;
    a fresh qualified-core-only executable accepts their canonical bytes from the
    bounded source provision and actual S3 subject/claim binding, including random/workload
    packages S4 refuses. It independently reconstructs original/stopped graphs, predicate/terminal
    boundaries, support SCCs, Prob0, distances and harmonic checks without analyzer/provider
    linkage, reused solver graph, emitted-package bytes or fabricated replay request.
    Wrong values/ranks/full keys/binding, changed objective/identity or source probabilities
    f
- id: FR-282-AC-19
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: AC-18 preserves typed source/hash admission failures and actual named byte/state/edge/work/storage/scalar/rational
    limits and consumption, pre-cancellation and cancellation during independent SCC/distance/equation
    passes without acceptance or truth values. Existing profiles preserve canonical
    bytes and verdicts. Raw-model-key owner budget admission and shared FR-413 witness
    wire remain open prerequisites/gaps, not newly supplied APIs or completed coverage.
- id: FR-282-cyclic-facade
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: "For FR-201's closed reachability, cyclic DTMC eventual, acyclic state-predicate\
    \ and acyclic expected-reward profiles, the fresh-process facade SHALL\n  admit\
    \ the bounded source `ByteProvision`, re-check S1 to S3 into a\n  `CheckedPackage`,\
    \ admit the `ModelSystem` and selected claim binding,\n  read the certificate\
    \ document through that profile's strict reader and\n  invoke the independent\
    \ core checker with the caller's limits and\n  `&Cancel`. It SHALL extract/reuse\
    \ the source completeness and provision\n  identity admission in `qsl-replay/src/request.rs`,\
    \ together with existing\n  content-hash valid"
- id: FR-201-cyclic-clause-0
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: 'The codec SHALL admit `"quire.probability-objective/cyclic-dtmc-eventual/v1"`

    only for untimed infinite-trace unbounded `eventually holds(P)` over a checked

    state predicate. This is a separate closed profile: it SHALL NOT change the

    admission, canonical bytes or meaning of any existing profile. Its objective

    has exactly `profile`, `targets`, `optimization`, `fairness`; the existing

    reachability profile''s exact document body, strict canonical decoding,

    rational representation, full-key ordering/coverage and identity rules apply.

    No policy, component partition, formula AST or additional body fie'
- id: FR-201-cyclic-clause-1
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Admission SHALL independently enumerate the finite original reachable\
    \ model\n  graph, permitting real cycles, with one initial state, at most one\
    \ actual\n  S3 binding, no clocks, rewards or authored fairness, and FR-196's\
    \ current\n  single-post Markov restrictions. Over every scheduler, every original\
    \ live\n  state SHALL have exactly one semantic enabled distribution; merely finding\n\
    \  a good policy, equal certified action values or equal action distributions\n\
    \  SHALL NOT admit residual choices. A workload SHALL be fully resolved by its\n\
    \  authored semantics to one distribution per live state, with n"
- id: FR-201-cyclic-clause-2
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The checker SHALL independently read P at position 0 and each reached\n\
    \  original state using the actual checked occurrence, infinite-trace profile\n\
    \  and binding, then construct the stopped reachable graph by not expanding\n\
    \  targets. Targets are exactly its reached P states and have value 1.\n  Non-target\
    \ model terminals have value 0: infinite terminal stutter cannot\n  make P true\
    \ later. For graph analysis only, these terminal boundaries and\n  targets are\
    \ absorbing; no synthetic model/product key is introduced."
- id: FR-201-cyclic-clause-3
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "The checker SHALL compute positive-support SCCs and closed bottom SCCs\
    \ on\n  that stopped graph using its own bounded graph algorithms. All states\
    \ in\n  non-target closed bottom SCCs SHALL have value 0, including non-target\n\
    \  terminals. It SHALL independently derive Prob0 as states with no\n  positive-support\
    \ path to a target and require value 0 there too. It SHALL\n  check exact harmonic\
    \ equality `x(s) = sum(p(s,t) * x(t))` at every other\n  non-target live state,\
    \ with normalized positive semantic edge masses and\n  all values in [0,1]. These\
    \ boundaries remove every recurrent non-target\n  degree of"
- id: FR-201-cyclic-clause-4
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Ranking SHALL cover the same stopped graph as values. Targets and Prob0\n\
    \  states SHALL have rank 0; every other state SHALL have the independently\n\
    \  derived shortest positive-support edge distance to any target. This\n  satisfies\
    \ CE-3: each positive non-target state has a positive-valued\n  lower-rank successor.\
    \ It does not require every edge to decrease: a\n  self-loop may retain its rank.\
    \ Acyclic edge-decreasing heights and SCC\n  condensation heights SHALL NOT substitute\
    \ for these distances. Wrong\n  distances reject `RankBroken` at the first differing\
    \ complete key;\n  missing/extra coverage uses"
- id: FR-201-cyclic-clause-5
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Full keys SHALL use FR-196's authoritative model-key and real binding\n\
    \  owners through `ProbabilityProductKey::for_model_state`, with actually\n  absent\
    \ monitor, accumulator and pending components null. Model-only keys,\n  local\
    \ ids, invented transport or substituted bindings are not aliases.\n  Identity\
    \ mismatch refuses before enumeration; independently changed target\n  or optimization\
    \ refuses objective mismatch. Wrong values reject\n  `InequalityFails`; an unmet\
    \ exact threshold rejects `ThresholdMissed`.\n  Equivalent source changes may\
    \ still accept if all independently derived\n  contracts and eq"
- id: FR-201-cyclic-clause-6
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "Original/stopped exploration, SCC/reverse reachability, distance queues,\n\
    \  canonical-key materialization, parsing and exact sums SHALL obey caller\n \
    \ byte, state, edge, storage, work, scalar/rational and cancellation controls.\n\
    \  Graph passes SHALL use O(V+E) graph work/storage, with representation bytes\n\
    \  and exact arithmetic charged separately before allocation/operation;\n  rank\
    \ overflow stops under the actual named limit. Pre-cancellation and\n  cancellation\
    \ during any pass preserve typed nonacceptance without truth\n  values. The raw-model-key\
    \ owner budget prerequisite remains open; this\n  pro"
- id: ADR-028-cyclic-profile
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'FR-201''s `quire.probability-objective/cyclic-dtmc-eventual/v1` is a separate

    closed exact proof profile for untimed infinite-trace `eventually holds(P)`.

    The finite original graph may cycle; it has one initial, at most one actual

    binding, no fairness/rewards/clocks and only current FR-196 single-post Markov

    successors. Every original live state must have one semantic distribution,

    or an authored workload must fully resolve its choices with no residual

    scheduler/post-state choice. General cyclic MDP certification remains outside

    this profile even when a candidate policy or equal action values a'
- id: FR-196
  path: spec/functional/FR-196-build-the-probabilistic-product.md
  role: context_only
  excerpt: 'QSL''s `exact_probabilistic` module in layer A, crate `qsl-analyze`

    (ADR-028 SP-2), SHALL build, for a probabilistic

    claim, the finite product on which EN-5 computes (ADR-028 §2, SCH-2): the

    MDP over FR-187''s actions and draws, or under a workload the DTMC over

    FR-187''s step probabilities, in product with FR-126''s deterministic monitor

    for a bounded event and with an accumulator for a bounded measure. It SHALL

    reuse `model_check`''s product `TransitionSystem`, monitor translations, edge

    retention and SCC decomposition, and SHALL retain every product edge with

    its action, draw and exact probabili'
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
- id: FR-199
  path: spec/functional/FR-199-decide-long-run-fractions-by-bottom-components.md
  role: context_only
  excerpt: 'EN-5 SHALL decide a `long-run fraction` claim, unweighted or `weighted
    by

    R`, exactly (ADR-028 XF-6, LR-1 to LR-5): under a workload by the

    stationary distribution of each bottom strongly connected component (BSCC),

    over every scheduler by each maximal end component''s (MEC''s) optimal

    fraction through Dinkelbach''s parametric method over exact policy

    iteration, and in both cases by combining the component values with §7''s

    reachability. EN-5 SHALL settle the result as a proof or a refutation that

    carries no coverage.'
- id: ADR-028-CE-3
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: context_only
  excerpt: '**Conditions.** With `F` the one-step operator of the objective (the workload''s
    average, the minimum or maximum over actions, or the policy''s action) and values
    fixed at decided, target and `Prob0` states: **Upper**: `F(y) <= y` at every state,
    so the least fixed point, which is the value, is at most `y` (Knaster–Tarski).
    **Lower**: `x <= F(x)` at every state of the support, and every non-target state
    with `x > 0` has, under the workload, under the policy, or for a minimum under
    every action, a successor with positive probability, `x > 0` and a smaller rank;
    then no end component inside the su'
```
