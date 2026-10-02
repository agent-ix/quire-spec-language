---
id: SR-1105
title: "QSL-371 spec review of PR #579: ADR-028, FR-195 to FR-204, US-023 and TC-630 to TC-639, TC-641, TC-642"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@dccdbdc908d6a45c5b7822168bdcd7375cb452cf; diff origin/spec/371-stat (2e69e94f)...dccdbdc9; spec/decisions/ADR-028-exact-probabilistic-engine.md; ADR-013, ADR-014, ADR-018, ADR-024 amendment notes; spec/usecase/US-023-prove-a-probabilistic-property-exactly-over-every-scheduler.md; spec/functional/FR-195..FR-204; spec/test-cases/TC-630..TC-639, TC-641, TC-642; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-028
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: reviews
---

## Summary

Ticket: QSL-371. PR quire-spec-language#579, head dccdbdc9, stacked on
#572 (2e69e94f, the merge-base). This file holds the base spec-review
checklist, plus the EARS, integrity and scope-boundary sub-analyses folded
in (one SR per PR, as the dispatching brief asks). The range SR-1106 to
SR-1109 was not needed.

**Gates, run here at dccdbdc9 on an archive of `spec/` and `tools/`.**
`quire validate --scope .` over the 30 changed spec files exited 0.
`tools/check-index-completeness.sh` exited 0.

**Owner rulings recorded on QSL-371 (read as data) and checked.**
- Confidence parameters apply to statistical evidence only; a claim without them is exact-only (SP-4, RU-2, FR-195).
- Fair schedulers follow Baier-Kwiatkowska (§3a, FS-1 to FS-9, FR-200).
- The scheduler picks after the draw (SCH-2, RU-4, FR-196 intermediate states).
- Weighted long-run fractions are kept (LR-2, RU-5).
- A workload that gives the delays makes the model a DTMC, and leftover delay nondeterminism is resolved by min/max (TA-1, TA-1a, RU-6, FR-204-AC-5).
- Expected reward and expected time are exact-only (XF-5, XF-7, RU-1).
- An undefined value refutes (XV-8, FR-196-AC-5, TC-641).
- The checker is in the qualified core (FR-201, US-023). ADR-028 itself does not say so (FND-004).

**Soundness checked and found correct.**
- §15.1: denominator `10^7000`, 23,254 bits.
- §15.2: `24233/25000`, `4491/5000`, and the `343/5000` path.
- §15.3: `π = (1800/1801, 1/1801)`, the gain–bias pair satisfying both equations, and 240,024 product states.
- §15.4: min `24/25`, max `99/100`, `Even` `391/400`, the `1/25` witness.
- §15.5: `99/100`, `9999/10000`, `20/9 ms`, `10/9 ms`.
- §15.6: fair end components.
- FR-187-AC-2 (`9/20`, `1/20`, `2/5`, `1/10`), FR-197-AC-4's mean, FR-198-AC-2 (`11/10`, `6/5`, `+∞`), FR-198-AC-3 (`1/2`), FR-199-AC-2 (`1000/1001`, `1800/1801`, `1400/1401`), FR-200-AC-2 and AC-3 (`k = 1` gives `1400/1401 < 0.9995`).
- FH-2's width bound `2 (h + 1) · 2^-p`.

All budgets are caller-set with published defaults; there is no cap, pin
or compat path. Every AC has a TC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | XF-2's transform for `quantile q of M >= c` lets EN-5 prove a false claim. The transform is `X = q · [activated] − [activated] · [M < c]`, accepted when `min E(X) >= 0`, and XF-8 says equality meets the bound. `E(X) = 0` means `Pr(M < c \| activated) = q`. For a finite-support `M` that makes the `q`-quantile below `c`, so the claim is false. Example: `M` is 1 or 3 with probability 1/2 each, `q = 1/2`, `c = 3`. Then `E(X) = 0` and the claim is `proved`, but the 0.5-quantile is 1. EN-5 decides equality exactly (UR-4, Consequences), so this boundary is reachable and yields a certified false proof. Require `E(X) > 0` for the `>=` quantile form, which also makes CE-4's threshold side strict for that form, or use the corrected PF-4 reduction (SR-1100 FND-002). QSpec FR-411:72-74 has the same text. | spec/decisions/ADR-028-exact-probabilistic-engine.md:166,172; spec/functional/FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md:52-55,72 |
| FND-002 | medium | FR-204-AC-3's value does not reproduce. Under the stated semantics, each `send` follows a delay of 1 or 2 ms with probability 1/2 each after every reset (both inside the window `[1, 2]`), each send is lost with probability 1/10, and the deadline is 4 ms inclusive. An exact recursion gives `Pr(delivered by 4 ms) = 159129/160000` (≈ 0.994556), not `158509/160000`. With exclusive or shifted deadlines it gives `7749/8000` or `3194109/3200000`. The verdict (`proved` at 0.99) is unchanged, but the AC's exact value would fail. Recompute it, or state the semantics that give `158509/160000`. | spec/functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md:119 |
| FND-003 | medium | The witness scheduler for a maximum is not specified through collapsed end components. UR-2 collapses each MEC with no target. UR-7, FH-4 and FR-198 then take "the action attaining the bound at each state … ties broken by canonical transition order". At a state inside a MEC, an action that stays in the MEC attains the same value as the exit action. A canonical tie-break can pick it at every MEC state, and the induced chain never leaves the MEC, so it does not attain the maximum. A refutation of a `<= θ` bound built on that policy then fails its own replay or subsystem check (`ReplayParity`) rather than settling `refuted`. Specify how the collapsed-MDP policy expands on the product: the chosen exit action at its state, and at the other MEC states a shortest path to that state. | spec/decisions/ADR-028-exact-probabilistic-engine.md:190,197,202; spec/functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md:89-93 |
| FND-004 | low | ADR-028 does not record the ruling that the certificate checker is in the qualified core (ADR-029 RU-2). CE-7 says only "a small trusted base"; FR-201 and US-023 carry the ruling. FR-201 also depends on ADR-029 RU-2 and CB-2, which exist only on an unmerged branch (QSL-390), so that dependency is dangling at this head. Add the placement to CE-2 or CE-7, and keep the ADR-029 citation in References until ADR-029 lands. | spec/decisions/ADR-028-exact-probabilistic-engine.md:266; spec/functional/FR-201-check-a-probability-certificate.md:29,123,133 |
| FND-005 | low | §18 "The twelve spec items of the research ruling" maps research items to sections of this record, and the Status paragraph refers to it. It is a tracking table: no requirement reads it, and nothing breaks without it. Delete §18 and the Status sentence. | spec/decisions/ADR-028-exact-probabilistic-engine.md:49-50,519-534 |
| FND-006 | low | §15.3 writes the `LongRun` certificate as the reward `[up]` with gain `1800/1801`. CE-3 and FR-201 define the `LongRun` condition for the reward `R · ([P] − ρ_C)` with gain 0. The two forms are equivalent, but a certificate in §15.3's form does not meet the condition as CE-3 states it, and QS-9 (c) vectors are taken from §15.3. Write the example in CE-3's form, with gain 0 and the same biases. | spec/decisions/ADR-028-exact-probabilistic-engine.md:352-356 |
| FND-007 | low | §15.1 repeats ADR-024's "about 5,065 SPRT samples". Wald's estimate at `p ≈ 0.9999` is about 5,168 (SR-1100 FND-008). | spec/decisions/ADR-028-exact-probabilistic-engine.md:327 |
| FND-008 | low | PR-4 and FR-196 saturate the accumulator "one step / one unit above the threshold `c`". "Unit" is undefined for a quantity reward on a finer grid. Any value above `c` works as the single saturated value, since only `M <= c` and `M < c` are read. State it that way. | spec/decisions/ADR-028-exact-probabilistic-engine.md:123; spec/functional/FR-196-build-the-probabilistic-product.md:81-84 |

## Verdict

Not mergeable as it stands. FND-001 lets EN-5 issue a certified proof of a
false quantile claim. The PR also stacks on #572, which needs its own
fixes and a rebase first (SR-1100). The ruling coverage, the worked-example
arithmetic and the AC-to-TC coverage are otherwise sound.
