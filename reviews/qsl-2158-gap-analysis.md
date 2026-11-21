---
id: SR-202611210022442313144108509
title: "QSL-2158 PR 3285 FR-1043 finite-horizon measure profile gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/filament-spec-language@986e56a8e157cc8cf694ff4daed7cd2dcdc144e0 (merge-base f3aa681d7ed753a2cdd998bc24842ef38dda3c86); criteria FR-1043-AC-1..AC-11, FR-202-AC-27..AC-30, FR-203-AC-10..AC-11, FR-1042-AC-12..AC-14 (added in this PR); computed quire matrix; tests fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs, tools/fsl-probability-check/tests/it/certificate_kinds.rs, fsl-eval/src/probabilistic.rs tests; production diff of PR 3285"
review_set: subset
relationships:
  - {target: "ix://agent-ix/filament-spec-language/FR-1043", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-1042", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-202", type: references}
  - {target: "ix://agent-ix/filament-spec-language/FR-203", type: references}
---

## Summary

Ticket: QSL-2158, PR 3285. This is a planless gap analysis scoped to the
criteria this PR implements or adds. It does not cover the global backlog.
The matrix was computed with `quire matrix --scope . --format json`
(quire 0.52.0, engine 0.62.2) at frozen head `986e56a8e1`.

The test audit was required by the dispatch brief, so a semantic review was
performed. It judged assertion strength, adverse cases, independent oracles
and real physical replay. The code-review artifact
SR-202611210022442313144108508 records the test-strength findings that belong
with code. This artifact records traceability and AC-oracle gaps.

Matrix result: FR-1043-AC-1 to AC-5 and AC-7 to AC-11, FR-202-AC-27 to AC-30,
and FR-203-AC-10 and AC-11 are tagged by real, non-ignored tests.
FR-1043-AC-6 is untagged. FR-1042-AC-12 to AC-14, which this PR adds, are
untagged. AC-12 is inside this PR's narrow upper-minimum and lower-maximum
scope.

## Verdict

**FAIL**. The verdict rule makes any untagged in-scope criterion a FAIL
(FR-1043-AC-6 and FR-1042-AC-12). There are no high findings. Every finding
is fixable within this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-1043-AC-6 is untagged in the computed matrix. `a_finite_horizon_quantile_certificate_is_accepted` asserts it (shifted `11/135`, separate-process acceptance, level 0.5 refuted) but carries only `TC-2065`/`FR-201-AC-51`. | tools/fsl-probability-check/tests/it/certificate_kinds.rs:120 |
| FND-002 | medium | FR-1042-AC-12, added by this PR and implemented by the narrow fraction-base `bounds.rs` path, has no tagged test. Its scenario is not exercised: an initial terminal state, a false predicate, window 2, an all-zero `upper` minimum and `lower` maximum with rederived ranks, accepted with no policy rows. | spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md:238 |
| FND-003 | medium | FR-1043-AC-8's stated every-scheduler extrema (minimum `11/48`, maximum `5/12`) and the checker-selected policies (`slow` for the upper on the minimum, `fast` for the lower on the maximum) are never asserted. The tagged tests check only `Refuted`, accept hand-edited bound values `1/4`/`2/5`, and check `fast` only in the FR-202-AC-30 slack fixture. | fsl-analyze/tests/it/exact_probabilistic_item/measure_profile.rs:1076 |
| FND-004 | low | FR-1042-AC-13 and AC-14, added by this PR, have no tagged test, while TC-3151's amended scope claims "FR-1042-AC-7 to FR-1042-AC-14". Under the brief they belong to the general FR-1042 PR, but no `Remaining work` record marks them as not delivered here. | spec/test-cases/TC-3151-certify-value-bounds.md:21 |

## Coverage

- Plan completion: not assessed
- Matrix (scoped): 20 of 24 examined criteria are tagged. The untagged ones
  are FR-1043-AC-6, FR-1042-AC-12, FR-1042-AC-13 and FR-1042-AC-14.
  FR-1042-AC-1 to AC-11 are also untagged, but they are general FR-1042
  backlog outside this PR and are not counted. No criterion is tagged only by
  an ignored test.
- Reverse gap: the new production code has owning requirements: FR-1043,
  FR-196 (key and control ownership), FR-201, FR-202, FR-203 and FR-1042. The
  speculative `ProbabilityCertificatePart::Post`/`Reset` variants are recorded
  in the code review (FND-018).
- Stubs and inflation: no source stubs. The `#[ignore]` stubs that remain in
  `certificate_kinds.rs` (FR-1041, general FR-1042) predate this PR and are
  keyed to QSL-2158. Two `#[ignore]` markers were removed for delivered
  behavior (P95, TC-2065 quantile), and no test was deleted.
- Semantic review: AC-11 replays the actual `drop(0)` step and its zero
  reward. It verifies `c` is really absent, and rejects forged locus,
  position, control, sums and cause. AC-10 validates the FSpec `Result`
  schema. AC-4 uses the emitted package and alters one numerator value. The
  defined `Always` unsupported dispositions are preserved by exact regression
  tests 5 to 7 in the parent's focused list.
- Runtime: the 33 exact focused tests are pending on draco. This analysis
  does not claim them.

## Disposition pass 1

Latest reviewed source: `1bef7ca58bebe9972e3fe74189d578e060034722`; actual merge-base `529282e9dc28cc6be8b382ae634636e3b4fe4a14`. One independent Codex reviewer (GPT-6 family as supplied by harness; no more specific model version exposed), user-directed replacement of initial reviewer. Initial findings/history above are retained verbatim.

**Latest verdict: CONDITIONAL.** GA001/003/004 fixed; GA002 deferred only for architecture-allocated generic proof scope. All delivered criterion rows are tagged; runtime is compile-blocked with zero tests executed. Code review FND-019/020 separately block merge.

## Dispositions

Round 1. `fixed` records inspected source resolution; execution evidence remains separately qualified below.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Standalone TC2065 checker test now carries FR1043AC6; matrix binds the real nonignored test. |
| FND-002 | deferred | Architecture narrows this PR to mean-refutation. Its terminal false fraction Upper(Minimum)=0 is accepted with inferred TerminalStutter; Lower(Maximum)=0 reaches exactly ThresholdMissed at <=0 after local checks. Generic standalone Lower proof acceptance and full FR1042AC12 remain on QSL2158, comment d59dedf8-adff-4b9c-a5fc-3509ecb5b930. AC12 remains untagged deliberately; no invalid negative threshold/public shim or full-coverage claim. |
| FND-003 | fixed | Actual exact initial extrema11/48 and5/12 plus selected slow/fast identities are asserted; bound tests reuse those policies and independent replay parity. |
| FND-004 | fixed | Named narrow hint tests bind AC13/14; private Remaining work comment d59dedf8 explicitly retains general FR1042 AC12/13/14 obligations and ticket stays In Progress. Fixed means truthful allocation, not full generic coverage. |

### Round 1 source excerpts

FND-001 after_excerpt:

```rust
#[trace("TC-2065", "FR-201-AC-51", "FR-1043-AC-6")]
#[test]
fn a_finite_horizon_quantile_certificate_is_accepted() {
```

FND-003 after_excerpt:

```rust
    };
    assert_ne!(fast.identity.operation, slow.identity.operation);
    for (name, expected, action) in [
        ("MinNo", ratio(11, 48), &slow.identity),
        ("MaxNo", ratio(5, 12), &fast.identity),
    ] {
        let record = settle(&world, name, ModelCheckLimits::default());
        assert_eq!(record.terminal().unwrap().value(), &TerminalValue::Refuted);
        let token = record.probabilistic_counterexample.as_ref().unwrap();
        let witness = token.evidence();
        let document: Value =
```

FND-004 after_excerpt:

```rust
#[trace(
    "FR-1043-AC-8",
    "FR-1043-AC-9",
    "FR-202-AC-28",
    "FR-1042-AC-13",
    "FR-1042-AC-14"
)]
#[test]
fn mean_bound_policy_hints_constrain_search_and_absence_reconstructs_internal_choices() {
```

### Round 1 scope and verification

No cargo, local tests, clippy, deny, baseline, Kani or sweep was run. Parent reports exact1bef7ca58bebe9972e3fe74189d578e060034722 first29-profile command COMPILE101/ZERO tests: E0425 document.rs229 missing ExactProbLimits import, plus unused MeasureError at document/measure.rs7. Raw /private/tmp/qsl-3285-1bef-1.log. Commands2–14 held/not run; no42-test acceptance. Original986e33 never ran/HOLD and is superseded. Upstream monitor_root23 is reader-only evidence, not acceptance. Remote10970 admission failure is only prior post-rebase compile/probe evidence.

The actual current PR diff preserves main exports, boxed task transitions, refinement guards/time invariants, protocol replay, call_failure propagation and unavailability removals. IR20a4dfbf/core-data424ba49 remain pinned; only authorized FSpecd6d066 replaces its prior pin. No workflow lane changes; git diff --check passes. No applicable AssuranceProfile was found. No new source edits, test deletion, vendoring or speculative owner API retained. FractionValue replays the actual post-step state at h+1 with PRE undecided control/accumulator before outside-window predicate access.

Plan completion: not assessed

Reconciliation: quire matrix (quire0.52.0 engine0.62.2), static only, no run evidence read. Scoped21 rows:20 tagged,1 untagged (FR1042AC12 explicitly allocated later); FR1042AC13/14 tags establish narrow hint behavior only. All owning delivered rows have real nonignored binders. Coverage command also exits0; engine diagnostics contain documentation ids outside markers, cfg/native symbol collisions and unrelated unminted headings. Compared saved initial matrix.err before classification; documentation warnings do not erase inspected real test binders, and no affected owning test binder or new dangling owning tag was found. This is no global traceability-cleanliness claim. No source/test stubs or new unowned production behavior found: control/key FR196, finite arithmetic FR197/1043, checking FR201/1042, witness FR202, settlement FR203, reward admission FR185/187. Semantic assertion audit is the authorized disposition recheck, not an optional new analysis. New code acceptance defect is owned in SR508 FND019 rather than duplicated here.

Scope entries below record criteria examined against implementation; AC12 is context_only at full generic scope.

```yaml
scope:
- id: FR-1042-AC-12
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: context_only
  excerpt: For a bounded fraction mean over an initial terminal state with a false
    fraction predicate and window 2, the base's only continuation is terminal stutter
    and its value is 0. An `upper` minimum document with every value 0 and no policy
    rows, and a `lower` maximum document with every value 0, the rederived remaining-horizon
    ranks and no policy rows, are accepted. The checker retains the existing stutter
    action at each undecided key; no fabricated operation identity or new wire form
    is required.
- id: FR-1042-AC-13
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: At an intermediate state with two enabled post-state choices `a` and `b`,
    their successors reach event success with probability `1/4` and `3/4`, respectively.
    With correct successor values and ranks, a minimum `upper` initial bound `1/2`
    with no hint is accepted selecting `a`, and a maximum `lower` bound `1/2` with
    no hint is accepted selecting `b`. Initial bounds `1/8` for the upper and `7/8`
    for the lower reject `InequalityFails` there because no action satisfies. Post-state
    selection needs no fabricated TransitionIdentity row.
- id: FR-1042-AC-14
  path: spec/functional/FR-1042-certify-value-bounds-with-lower-and-upper-certificates.md
  role: examined
  excerpt: In AC-3, minimum `upper` `s` `1/4` with hint `go` rejects `InequalityFails`
    although `wait` satisfies, and the same document without that hint accepts selecting
    `wait`. A nonexistent hint rejects `InequalityFails` even when an enabled action
    satisfies. Maximum `lower` `s` 1 with hint `wait` rejects `RankBroken`; removing
    the hint does not accept it, since no action satisfies both inequality and progress.
    When several actions satisfy every condition, an omitted hint selects the first
    in canonical full-product-action order.
- id: FR-1043-AC-1
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Retry`''s undelivered state `u` has `try`, delivering with `1/2` and
    otherwise staying; delivered is terminal. Under its workload, `quantile 0.5 of
    (steps from holds(true) until holds(delivered) within 4) <= 2` settles `proved`,
    `ExactValue{3/4}`, with a `conditional` `ratio` certificate whose numerator (payoff
    `satisfied`, `comparison` `"le"`, `threshold` 2) has values `3/4`, `1/2`, 0, 0
    at `u` with active amounts 0, 1, 2 and above the bound and ranks 4, 3, 2, 1, and
    whose denominator (payoff `activated`) is 1 there; `fsl-probability-check` accepts
    it, and rejects it `RankBroken` with the ran'
- id: FR-1043-AC-2
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With `try` split into `fast` (`1/2`) and `slow` (`1/4`), under every scheduler
    `quantile 0.4 of (…) <= 2` settles `proved` with an `exact` `shifted` document
    (`level` `2/5`) whose values are `3/80`, `−3/20`, `−2/5`, `−2/5`; `quantile 0.5
    of (…) <= 2` settles `refuted` (minimum shifted value `−1/16`, `Pr(M <= 2) = 7/16`
    under `slow`); the level-0.4 document with `3/80` replaced by `fast`'s `9/40`
    is rejected `InequalityFails` at the initial key.
- id: FR-1043-AC-3
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: 'Over AC-2''s model under every scheduler, `quantile 0.6 of (…) >= 2` (claim
    `Pr(M < 2 | act) < 3/5`) settles `proved`: the `shifted` document (`comparison`
    `"lt"`) has maximum initial value `−1/10` (through `fast`) and `−3/5` at every
    other undecided state; `quantile 0.5 of (…) >= 2` settles `refuted`, its maximum
    0 attained by `fast` with an activated state reachable.'
- id: FR-1043-AC-4
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`P95` over FR-185-AC-1''s `Service` under `Steady` at `5 ms` (FR-741-AC-2,
    FR-203-AC-1) settles `proved`, `ExactValue{24233/25000}`, with a `conditional`
    `ratio` certificate whose parts are documents of this profile (`kind` `"accumulate"`,
    `reward` the checked `duration`, `threshold` 5 in its unit) and whose denominator
    is 1 at the initial key; `fsl-probability-check` accepts it from the emitted package
    and rejects it with one numerator value altered. At `2 ms` the item settles `refuted`
    with FR-202-AC-1''s one-path set.'
- id: FR-1043-AC-5
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: Under `Retry`'s workload, `mean of fraction holds(delivered) over [0, 2]
    >= rational(2,5)` settles `proved`, `ExactValue{5/12}`, with a `fraction` document;
    at `rational(1,2)` it settles `refuted` with FSpec FR-413 subsystem evidence containing
    an independently accepted `exact` `fraction` certificate of `5/12 < 1/2`, checked
    for refutation rather than the proof threshold. The comparison event `probability
    >= 0.7 [M <= 2]` over AC-1's measure settles `proved`, `ExactValue{3/4}`, with
    an `event` document.
- id: FR-1043-AC-6
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-201-AC-51's finite-horizon quantile item over the cyclic `link` model
    under every scheduler (`Pr(M <= 3) = 13/27` for `steps from holds(true) until
    holds(m.delivered) within 4`), written at level 0.4, settles `proved` with shifted
    initial value `11/135` and a document `fsl-probability-check` accepts in a separate
    process; written at level 0.5 it settles `refuted`.
- id: FR-1043-AC-7
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-1's numerator with `window`, `threshold`, `comparison`, `reward`, `unit`,
    `level` or `payoff` changed refuses objective mismatch; a key whose accumulator
    amount or phase is altered is not an alias and fails coverage; a `shifted` value
    below `−level` is rejected `Malformed` at `values`. Pre-cancellation and exhaustion
    of each caller limit during expansion, payoff evaluation or exact sums settle
    `incomplete` with the actual limit and no truth value.
- id: FR-1043-AC-8
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: With AC-1's `try` split into `fast` (`1/2`) and `slow` (`1/4`), `mean of
    fraction holds(delivered) over [0, 2]` has minimum `11/48` and maximum `5/12`.
    Under every scheduler, `>= 1/2` is refuted by an accepted exact minimum or FR-1042
    `upper` on the minimum with initial bound `1/4` and policy `slow`; `<= 1/3` is
    refuted by an accepted exact maximum or `lower` on the maximum with initial bound
    `2/5` and policy `fast`. Both reuse subsystem evidence and are independently accepted
    by FR-202's checked-subject mean-certificate replay entry.
- id: FR-1043-AC-9
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: AC-5's exact workload mean `5/12` at `>= 5/12` and `<= 5/12`, a valid workload
    `upper` bound `1/2` at `>= 1/2`, and a valid `lower` bound `2/5` at `<= 2/5` each
    reject `ThresholdMissed` at the initial key and settle `inconclusive`, `CertificateRejected`.
    In AC-8, a valid `lower` minimum document with initial bound `1/5` at `>= 1/2`
    and a valid `upper` maximum document with bound `1/2` at `<= 1/3` each reject
    `Malformed` at `kind` with the same settlement. Identity/base mismatch retains
    its typed refusal; an invalid value rejects `InequalityFails` with `CertificateRejected`;
    a disagreeing witne
- id: FR-1043-AC-10
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: FR-203-AC-10's no-activation quantile under a resolved workload and under
    every scheduler, with and without an admitted fairness set, settles certified
    `Exhaustive` with an independently accepted conditional `vacuous` certificate,
    no `ExactValue`, shifted-value proof or numeric entry. Offering an all-zero `exact`
    `shifted` document under every scheduler, or a workload `ratio` with denominator
    0, for that no-activation claim rejects `InequalityFails` at the initial complete
    product key and settles `inconclusive`, `CertificateRejected{rule, at}`, without
    numeric acceptance or `ExactValue`. A var
- id: FR-1043-AC-11
  path: spec/functional/FR-1043-certify-finite-horizon-measure-verdicts.md
  role: examined
  excerpt: '`Coin` initially has bound object `c` with `c.v = 0`; its only operation
    `drop(b)` draws `b` uniformly from `{0,1}`, deleting `c` when `b = 0`, and declares
    reward `R = 0` for both supported draws. Under its resolved workload, `mean of
    fraction holds(c.v = 0) over [0,0] weighted by R >= 1/2` reaches undefined `division-by-zero`
    on the actual `drop(0)` step of probability `1/2`: the final tentative weighted
    sums are `(0,0)`, and the `FractionValue` location is position 1 with the actual
    post-state lacking `c`, pre-step undecided control `remaining = 1` and pre-step
    sums `(0,0)`. Native replay r'
- id: FR-202-AC-27
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: Through the checked-subject mean-certificate replay entry, FR-1043-AC-5's
    workload mean `5/12` refutes `>= 1/2` using subsystem evidence containing the
    independently accepted `exact` `fraction` certificate; the same document checks
    in its refutation role despite missing the proof threshold. An unchecked document,
    an unreplayed engine value or a path set yields no accepted mean-refutation evidence.
- id: FR-202-AC-28
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: FR-1043-AC-8's minimum `11/48` with an accepted `upper` bound `1/4` refutes
    every-scheduler `>= 1/2`; its maximum `5/12` with an accepted `lower` bound `2/5`
    refutes every-scheduler `<= 1/3`. A valid `lower` minimum document with initial
    bound `1/5` at `>= 1/2`, and a valid `upper` maximum document with initial bound
    `1/2` at `<= 1/3`, each reject `Malformed` at `kind` and settle `inconclusive`,
    `CertificateRejected`; their conditions hold but their refutation roles are wrong.
- id: FR-202-AC-29
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: AC-27's exact `5/12` at `>= 5/12` or `<= 5/12`, and a valid workload `upper`
    bound `1/2` at `>= 1/2`, each reject `ThresholdMissed` at the initial key and
    settle `inconclusive`, `CertificateRejected`. Changing identity or base payoff/optimization
    retains the typed refusal; invalid certificate values settle `inconclusive`, `CertificateRejected{InequalityFails,
    at}`; a disagreeing witness policy settles `inconclusive`, `ReplayParity`; cancellation
    or a caller limit preserves its typed incomplete result without accepted evidence.
- id: FR-202-AC-30
  path: spec/functional/FR-202-replay-a-probabilistic-witness.md
  role: examined
  excerpt: 'A mean minimum refutation with a slack `upper < θ` for which both the
    optimizer''s attaining action and a different canonical-first action satisfy every
    FR-1042 condition emits a witness matching the independently checked policy: with
    no hint it uses the canonical-first satisfying full-product action; with a representable
    valid hint it uses that hinted action. Both replay as refuted. An old optimizer
    witness disagreeing with the checked selection settles `inconclusive`, `ReplayParity`;
    implicit stutter or post-state selection uses the same rule without a fabricated
    policy identity.'
- id: FR-203-AC-10
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: 'The untimed claim `quantile 0.5 of (steps from holds(false) until holds(true)
    within 4) <= 2`, checked separately under a resolved workload and under every
    scheduler (with and without an admitted fairness set) has no activating state
    in the complete reachable product. Native checking independently accepts its `Conditional`
    `Vacuous` certificate and settlement returns `proved`, `closed-scope`, `Certified`,
    `Exhaustive`, category success, carrying that certificate with empty `values`
    and `conditional` exactly `{"mode": "vacuous"}`. The emitted result passes FR-331''s
    contract with basis `{"type":'
- id: FR-203-AC-11
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: Changing AC-10's activation predicate to `holds(true)` makes the offered
    vacuous certificate reject `InequalityFails` and settle `inconclusive`, `CertificateRejected`,
    without vacuity acceptance. Offering an otherwise well-shaped `exact`, `ratio`
    or `optimum` certificate on `exhaustive`, or generic `evidence` with or without
    its permitted vacuous certificate, refuses the FR-331 result contract. An `exact-value`
    basis missing its value, using scalar zero or denominator zero refuses; AC-1's
    ordinary exact result with a valid `Rational` value remains admitted. An engine-only
    assertion, partial ex
- id: FR-203-AC-12
  path: spec/functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md
  role: examined
  excerpt: '`Retry` has `Msg.delivered`, terminal when delivered, and the resolved
    `Steady` workload whose `fast` step delivers with probability `1/2` and whose
    `slow` weight is 0. With request initial states ordered as index 0 `{delivered:
    true}` and index 1 `{delivered: false}`, the same admitted binding `m`, and `quantile
    0.5 of (steps from holds(not m.delivered) until holds(m.delivered) within 4) <=
    2`, the independent conditional certificate check accounts for both pairs: index
    0 has unreachable activation with numerator and denominator 0, while index 1 activates
    immediately with numerator `3/4`, den'
```
