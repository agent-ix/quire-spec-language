---
id: SR-20261109172912000024
title: Bounded certificate spec-review/failure-domain review
type: SpecReview
analysis: failure-domain
scope: agent-ix/quire-spec-language; spec/decisions/ADR-028-exact-probabilistic-engine.md,
  spec/functional/FR-201-check-a-probability-certificate.md, spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
review_set: subset
---

## Summary

Ticket: QSL-1826. Frozen PR #2614 at cd154e77d87ccf79753a1815e7e9f48e5028464c; origin/main...HEAD only.

## Verdict

PASS — Full binding/formula/profile/origin identity prevents repeat-state merging and forged monitor reuse. Cyclic raw models terminate through bounded monitor progress; terminal false extension and horizon zero are explicit. Specification mergeable for this slice; implementation and full assurance completion are not assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Examined scope

```yaml
scope:
- id: ADR-028:L701
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: equalities both remain CE-3 obligations. The codec admits untimed acyclic
- id: ADR-028:L702
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: unbounded eventual reachability and origin-activated event-position
- id: ADR-028:L703
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: '`eventually[0,h] holds(P)`, each with one initial state and at most one'
- id: ADR-028:L704
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: 'binding. Bounded checking extends the same reachability/v1 capability:'
- id: ADR-028:L705
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: the closed member contract and extensional objective meaning are unchanged,
- id: ADR-028:L706
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: and existing unbounded documents retain their meaning. FR-196 already owns
- id: ADR-028:L707
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: the complete formula-monitor identity; FR-197 already owns bounded induction.
- id: ADR-028:L708
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: The qualified checker independently builds the bounded monitor product
    using
- id: ADR-028:L709
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: qsl-eval-owned canonical key and terminal false-extension operations, with
- id: ADR-028:L710
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: ranks matching real monitor horizon/debts rather than unbounded model height.
- id: ADR-028:L711
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: A cyclic model is admitted when its bounded undecided product terminates.
- id: ADR-028:L712
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: Dyadic lower/upper codec support and bounded FR-202 refutation replay remain
- id: ADR-028:L713
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: outstanding; no interval becomes an exact document. Unsupported forms retain
- id: ADR-028:L714
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: all broader CE obligations and are not coerced into this profile.
- id: ADR-028:L715
  path: spec/decisions/ADR-028-exact-probabilistic-engine.md
  role: examined
  excerpt: FR-282 defines the independent checked-source
- id: FR-201:L96
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For untimed unbounded `eventually` reachability and the bounded subset
    below, the
- id: FR-201:L114
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  claim binding, no clocks, reward or authored fairness. The unbounded'
- id: FR-201:L115
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  subset SHALL retain its acyclic continuation graph restriction apart'
- id: FR-201:L116
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  from implicit terminal stutter. The bounded subset SHALL admit only'
- id: FR-201:L117
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  origin-activated event-position `eventually[0,h] holds(P)` with a finite'
- id: FR-201:L118
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  nonnegative position horizon and a checked state predicate P; its'
- id: FR-201:L119
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  undecided monitor product SHALL terminate, even when the underlying
    model'
- id: FR-201:L120
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  graph is cyclic. Other temporal forms remain unsupported by this profile.'
- id: FR-201:L121
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  Both subsets retain FR-196''s Markov successor restrictions.'
- id: FR-201:L122
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  Workload admission SHALL require a single resolved'
- id: FR-201:L148
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  `ProbabilityProductKey::for_model_state` helper for the unbounded subset.'
- id: FR-201:L149
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  The bounded subset SHALL use the existing complete formula-monitor'
- id: FR-201:L150
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  representation, with null accumulator and pending components. The stale'
- id: FR-201:L159
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  satisfaction. For the unbounded subset it MAY emit checker-verified
    acyclic height ranks, with'
- id: FR-201:L174
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '#### Bounded event-position checking'
- id: FR-201:L176
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: The bounded subset is a capability extension of the same
- id: FR-201:L177
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '`quire.probability-objective/reachability/v1` document contract: its members,'
- id: FR-201:L178
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: extensional target meaning, optimization, fairness and complete-key transport
- id: FR-201:L179
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: do not change. Existing unbounded documents retain their meaning and admission.
- id: FR-201:L180
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: No second formula AST, opaque monitor id or public horizon field is introduced.
- id: FR-201:L182
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '- The checker SHALL independently construct the model/monitor product
    using'
- id: FR-201:L183
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  `qsl-eval`''s qualified property-automaton translation, initialization
    and'
- id: FR-201:L184
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  transition semantics. It SHALL read position 0 during initialization
    and'
- id: FR-201:L185
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  one subsequent position per observation step, including the inclusive'
- id: FR-201:L186
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  position h. At a terminal model state it SHALL apply the existing FR-126'
- id: FR-201:L187
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  profile false-extension closure, rather than invent model progress or'
- id: FR-201:L188
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  infer acceptance from current predicate truth.'
- id: FR-201:L189
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '- `qsl-eval` SHALL own a canonical monitor-state key operation alongside
    its'
- id: FR-201:L190
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  existing automaton key ownership and a terminal false-extension operation'
- id: FR-201:L191
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  usable by the qualified checker. Those operations SHALL preserve FR-196''s'
- id: FR-201:L192
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  checked formula/profile/origin/binding context and complete parsed FR-338'
- id: FR-201:L193
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  automaton key, including real remaining horizon/debts for running states'
- id: FR-201:L194
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  and counter-free absorbing decided verdicts. Analyzer-local `StateId`'
- id: FR-201:L195
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  values SHALL NOT become certificate identity; the checker SHALL NOT
    copy'
- id: FR-201:L196
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  analyzer monitor semantics or trust its enumerated product.'
- id: FR-201:L197
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '- Targets SHALL be exactly the independently rederived accepting decided'
- id: FR-201:L198
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  product states. Accepting states have value 1; rejecting decided states'
- id: FR-201:L199
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  have value 0; neither is expanded. Bellman equality applies at every'
- id: FR-201:L200
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  undecided state over actual model/monitor successors.'
- id: FR-201:L201
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '- Bounded ranks SHALL agree with the independently rederived remaining'
- id: FR-201:L202
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  event-position horizon/debts under QSpec FR-412''s finite-horizon rule:'
- id: FR-201:L203
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  each undecided state''s rank is its actual remaining future positions,'
- id: FR-201:L204
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  and each decided state''s rank is 0. Every undecided observation successor'
- id: FR-201:L205
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  that remains undecided consumes one position. The checker SHALL reject'
- id: FR-201:L206
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  a substituted acyclic-model height or altered remaining-horizon rank'
- id: FR-201:L207
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  even if Bellman equalities and general positive-value progress hold.'
- id: FR-201:L208
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  A wrong bounded rank is `RankBroken` at the first differing full product'
- id: FR-201:L209
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  key in canonical order; missing rank coverage remains malformed coverage.'
- id: FR-201:L210
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '- Only FR-197 `Exact` results can emit this profile''s exact document.'
- id: FR-201:L211
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  `Bounds` and `PrecisionBudget` SHALL preserve their explicit outcomes;'
- id: FR-201:L212
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  dyadic lower/upper documents remain unsupported by this closed profile'
- id: FR-201:L213
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  until their own certificate profile is defined. An interval SHALL NOT
    be'
- id: FR-201:L214
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  serialized as an exact rational certificate. This admission does not'
- id: FR-201:L215
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  establish bounded refutation/witness replay through FR-202''s unbounded'
- id: FR-201:L216
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  typed route; that work remains outstanding, as do broader certificate'
- id: FR-201:L217
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: '  forms required by this requirement.'
- id: FR-201-AC-11
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: Minimum AtLeast, maximum AtMost and resolved-workload exact proofs pass
    the profile checker at an attained threshold; a threshold strictly beyond the
    certified initial value fails. Unbounded cyclic, timed, reward, fair, multi-initial
    and multi-binding cases, bounded forms outside the admitted subset and non-exact
    kinds return explicit unsupported with no accepted certificate. Pre-cancelled
    input and exhausted byte/state/edge/arithmetic budgets stop without a truth value.
- id: FR-201-AC-12
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: "ADR-028 \xA715.4's bounded Deliver at horizon 2 gives accepted exact minimum\
    \ AtLeast at 24/25, maximum AtMost at 99/100 and Even workload at 391/400; after\
    \ one loss the minimum is 4/5 and workload 17/20. Each threshold beyond the certified\
    \ side fails. Existing unbounded profile documents retain their verdict and canonical\
    \ bytes."
- id: FR-201-AC-13
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: A model that stays undelivered with probability 1/2 and delivers with probability
    1/2 per step has bounded horizon-2 probability 3/4 despite its raw self-cycle.
    The initial and one-loss visits share the model key but have distinct full formula-bearing
    keys and ranks 2 and 1. Horizon 0 accepts iff P holds at position 0. A terminal
    undelivered state closes to rejection by false extension without enumerating stutter
    positions.
- id: FR-201-AC-14
  path: spec/functional/FR-201-check-a-probability-certificate.md
  role: examined
  excerpt: For AC-13's valid certificate, replacing a running monitor with a local
    StateId, null, another remaining horizon, or another binding cannot certify the
    claim. Changing rank 2 to 3 while retaining valid values fails RankBroken at that
    full key. An exact document fabricated from a dyadic interval is never emitted;
    unsupported lower/upper documents and precision stops produce no accepted exact
    certificate.
- id: FR-282-AC-13
  path: spec/functional/FR-282-check-analyze-certificates-in-the-qualified-core.md
  role: examined
  excerpt: A real FR-197 Exact bounded event-position certificate for FR-201-AC-13
    is written by analyze and accepted from its canonical document bytes in a fresh
    qualified-core-only process using the source provision and actual subject/claim
    binding. The process independently reconstructs formula-monitor keys and terminal
    closure through qsl-eval, without qsl-analyze or provider linkage. Changing a
    running monitor horizon, binding or bounded rank prevents acceptance.
```

## Limits and context

No active AssuranceProfile document was found under spec/. No new implementation exists in this diff; no cargo, code gap analysis or whole matrix was run. Parent reports generic table_row extraction projects only the first row and a whole matrix was stopped after 1m24s without output; neither result establishes coverage. Direct quire parse/lookup confirmed all changed AC rows. Existing module catalog duplicate-registration warnings remain. Context: FR-196 complete formula-monitor key, FR-197 Exact/Bounds semantics, FR-126 terminal closure, FR-338 canonical monitor ownership. No new external links were introduced.
