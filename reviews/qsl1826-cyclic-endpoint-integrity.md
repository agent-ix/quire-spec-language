---
id: SR-20261109172912000103
title: integrity review of cyclic DTMC endpoint fixtures
type: SpecReview
analysis: integrity
scope: agent-ix/quire-spec-language; QSL-1826; FR-201-AC-23; S3 threshold admission
  context
review_set: subset
---

## Summary

QSL-1826 PR #2640 frozen 11171a0cda5e09e338366d4dd872347aa83ebce5 against base 951b3fbbcd6c97269a33931544fb958aa7935c25. Only FR-201-AC-23 changes. Specification fixture correction only; no runtime or wider qualification claim. No applicable AssuranceProfile document. Existing S3 admission remains authoritative.

## Verdict

**PASS** — No defects found in this fixture correction.

## Analysis

Actual before/after Quire parse sections each contain26 FR-201 criteria with exactly3 cells per row, preserving IDs including inherited AC7-before-AC6 physical order. Full acceptance_criteria extraction and independent Quire properties also yield26; the single acceptance_criteria_table scalar is not treated as a census. Exact diff changes only AC23: it distinguishes endpoint certified probabilities from interior source claim thresholds and clarifies that3/5 and2/5 failures belong to the mixed case. Other fixtures and semantic obligations are unchanged. Scoped normative validation passes with existing module-discovery warnings only.

Failure-domain, EARS, dependency, evidence, risk-complexity and scope-boundary lenses were not rerun: this single criterion substitutes admitted interior claim thresholds for impossible endpoint thresholds, preserving the existing graph, codec, identity, cancellation, ownership and proof contracts. No production changes, so no Rust/runtime gap-analysis gates. No cargo, matrix, CI or Kani run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

Full criteria were examined; excerpts are verbatim prefixes up to600 characters.

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
  role: context_only
  excerpt: For an acyclic two-step source with named integer reward R, initial choices
    a (cost 1, then cost 2 to B) and b (cost 4 directly to B) have exact minimum 3
    and maximum 4. The minimum AtLeast 3 and maximum AtMost 4 documents accept at
    equality; thresholds 4 and 3 respectively reject ThresholdMissed. An equal-weight
    resolved workload has value 7/2 and accepts either bound at 7/2. Values above
    1 are accepted; a probability-style target value 1 rejects.
- id: FR-201-AC-20
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
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
  role: context_only
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
  role: context_only
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
    original outgoing cycle. An initial target has exact value 1 and accepts minimum
    AtLeast 1/2; a non-target terminal has exact value 0 under infinite stutter and
    accepts maximum AtMost 1/2. Thresholds 3/5 AtLeast and 2/5 AtMost reject ThresholdMissed
    for the mixed case. A target-free two-state closed cycle has exact values/ranks
    0 and accepts maximum AtMost 1/2; as
- id: FR-201-AC-24
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
  excerpt: In a transient SCC, a goes to b with probability 1, b to a with 1/2, target
    g with 1/4 and closed failure f with 1/4. Values a=b=1/2, g=1, f=0 and ranks a=2,b=1,g=f=0
    accept; both SCC members having condensation rank 1 rejects RankBroken. A fully
    authored workload at s choosing equally between deterministic return-to-s and
    a distribution g:1/3,f:2/3 resolves to s:1/2,g:1/6,f:1/3 and accepts exact value
    1/3, rank 1 on either workload bound at equality; 2/5 AtLeast and 1/4 AtMost reject.
- id: FR-201-AC-25
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: context_only
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
  role: context_only
  excerpt: AC-23/24 independently check with no solver dependency. Source/document/key
    byte, state/edge, graph work/storage and scalar/rational budgets are exercised
    at exhaustion, including SCC, reverse reachability and distance construction;
    actual typed limit and consumption are retained. Pre-cancellation and cancellation
    during each pass or exact sum stop without acceptance or truth values; no fabricated
    upstream key-budget API or cyclic refutation is claimed.
- id: magnitude
  path: qsl-semantics/src/check/probabilistic.rs
  role: context_only
  excerpt: "/// Zero or negative.\n    NonPositive,\n    /// Greater than 0 and less\
    \ than 1.\n    UnitInterval,\n    /// 1 or more.\n    AtLeastOne,\n    /// A rational\
    \ with denominator 0.\n    ZeroDenominator,"
- id: claim-qualitative-admission
  path: qsl-semantics/src/check/probabilistic.rs
  role: examined
  excerpt: "|value: &ExactValueForm, causes: &mut Vec<ProbabilisticCause>| match magnitude(value)\
    \ {\n            Magnitude::UnitInterval => true,\n            Magnitude::ZeroDenominator\
    \ => {\n                causes.push(ProbabilisticCause::ZeroDenominator { span:\
    \ value.span });\n                false\n            }\n            Magnitude::NonPositive\
    \ | Magnitude::AtLeastOne => {\n                causes.push(ProbabilisticCause::QualitativeThreshold\
    \ {\n                    claim: claim.clone(),\n                    span: value.span,\n\
    \                });\n                false\n            }\n        };"
```
