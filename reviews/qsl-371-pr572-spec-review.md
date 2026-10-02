---
id: SR-1100
title: "QSL-371 spec review of PR #572: ADR-024, FR-185 to FR-194, US-022 and TC-620 to TC-629, TC-640"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2e69e94f14efb0d61b5a9bf411ebd02674af70b2; PR-owned commits 404a5a88..2e69e94f; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md; ADR-013, ADR-014, ADR-016, ADR-018 amendment notes; spec/usecase/US-022-measure-a-probabilistic-property-of-a-model-or-a-live-system.md; spec/functional/FR-185..FR-194; spec/test-cases/TC-620..TC-629, TC-640; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-186
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-190
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-191
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: reviews
---

## Summary

Ticket: QSL-371. PR quire-spec-language#572, head 2e69e94f. This file holds
the base spec-review checklist, plus the EARS, integrity and scope-boundary
sub-analyses folded in (one SR per PR, as the dispatching brief asks). The
range SR-1101 to SR-1104 was not needed.

**Diff scope.** `git diff origin/spec/366-temporal-properties...HEAD` also
shows the old QSL-366 commits, because #562's branch was rebased (now at
e6a5fb56) and #572 was not rebased onto it. The review covers only the
PR's own commits, 404a5a88..2e69e94f (FND-003).

**Gates, run here at 2e69e94f on an archive of `spec/` and `tools/`.**
`quire validate --scope .` over the 29 changed spec files exited 0 with
only module-level warnings. `tools/check-index-completeness.sh` exited 0.

**Owner rulings recorded on QSL-371 (read as data) and checked.**
Long-run claims are admitted with `Asymptotic` coverage (RU-1, ST-7,
FR-190). A Rejected measurement fails the pipeline and is never `refuted`
(RU-2, SV-3, FR-192). Mean latency is dropped (RU-3, PF-5). Confidence
parameters apply to statistical evidence only, and a claim without them is
exact-only (RU-6, PF-10, FR-186, FR-192 `MissingConfidence`). A sampled
undefined rejects (SV-11, FR-189-AC-6, TC-640). All are reflected.

**Soundness checked and found correct.** §7.1 Okamoto `N = 9,210,341`, the
exact probability `0.99990000`, and the SPRT increments and 4,601-sample
accept. §7.2's distribution (0.882, 0.8982, 0.9668, 0.96932), p95 = 3 ms,
`N = 23,026` and the 219-sample accept. §7.3's availability `1800/1801`, the
ratio variance `0.000677`, the cycle count of about 91,6xx, and the
per-window probability 0.95898, recomputed by a 10,001-step dynamic
program. FR-187-AC-1's six probabilities, FR-189-AC-2's `N = 26,492` for
`m = 2`, FR-190-AC-1's `1800/1801`, FR-194-AC-1's nearest-rank values.
FR-192's `Stopped` row (`failed`, execution `resource-incomplete`) agrees
with FR-127 V-7. There are no caps (every limit is a caller-set budget
with a published default), no pins (the sampler is selected by identity,
FR-188), no compat paths, and ticket ids appear only in References. Every
AC has a TC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The regenerative stopping rule stops on a degenerate interval. The test stops once `n >= min_cycles` and `z · S / (L̄ · √n) <= ι`. When every cycle so far has the same `Y_k / L_k`, `S = 0` and the half-width is 0. In §7.3's `Avail`, a cycle without a failure has `L = Y = 1`. The first 100 cycles have no failure with probability `(1 − 1/2000)^100 ≈ 0.95`, so `Regenerative{min_cycles: 100}` stops at 100 cycles with estimate 1 and interval `[1, 1]` on about 95% of seeds. FR-190-AC-1 (estimate within `1/5000` of `1800/1801`, about 91,624 cycles) then fails. A `long-run fraction <= 0.9995` claim, which holds, is Rejected. Use a stopping rule that cannot fire on zero sample variance, for example Chow–Robbins (`S² + 1/n` in place of `S²`), or a minimum cycle count derived from `ι`. QSpec FR-408 has the same rule. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:139; spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:54-61,77 |
| FND-002 | high | PF-4's reduction of `quantile q of M >= c` to `Pr(M < c) <= q` is wrong for the finite-support measures this record admits. With `x_q = inf {x : F(x) >= q}`, `x_q >= c` requires `F(x) < q` for every `x < c`. When `M` has finite support, `F(c−)` is attained at the largest support point below `c`, so the condition is `Pr(M < c) < q`. Counterexample: `M` is 1 or 3 with probability 1/2 each, `q = 1/2`, `c = 3`. Then `x_q = 1`, so the claim is false, but `Pr(M < 3) = 1/2 <= 1/2` accepts it. EN-4 cannot see the boundary, which lies in the indifference region, but the rule is normative, and ADR-028 XF-2 proves the false claim exactly (see SR-1105). Use `Pr(M < c) < q`, or define the `>=` form as `Pr(M >= c) > 1 − q`. QSpec FR-407 has the same text. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:121; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:116-118 |
| FND-003 | medium | #572 is not rebased onto its base. `origin/spec/366-temporal-properties` is at e6a5fb56 (the QSL-366 Wave C finish), but #572 still sits on the old 366 commits (merge-base 8337d52a). Its own commits do not apply to the current base: `git apply --check` fails on `ADR-018-temporal-properties-over-every-behaviour.md:159` and `spec/spec.md:555`. As it stands, the three-dot diff carries a stale copy of the QSL-366 work. Rebase onto the current base and resolve the two conflicts. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:159-163; spec/spec.md:580-592 |
| FND-004 | medium | #572 depends on content that exists only in #579. FR-187-AC-2 and TC-622 use "the `Link` model (ADR-028 §15.4's model text)", but ADR-028 is not in this PR, so the fixture has no definition at this head. FR-187's Description, ADR-024 RU-6 and US-022 also cite ADR-028 and EN-5. Either define `Link` in FR-187 (or ADR-024 §7) and move the ADR-028 citations into #579's amendments, or merge #572 and #579 together. | spec/functional/FR-187-give-model-transitions-step-probabilities-and-rewards.md:30,112; spec/test-cases/TC-622-model-transitions-carry-step-probabilities-and-rewards.md:19; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:383,392 |
| FND-005 | medium | The qualitative-counterexample rule is wrong for `<= θ` bounds. SV-6 defines a witness as a sample on which "the event is false". SV-7 and FR-193 say it refutes "`E` on every behaviour". FR-193:72 correctly keeps a sample where the event is true under a `<= θ` bound, but that sample refutes "not `E` on every behaviour", not "`E` on every behaviour". FR-193:110 converts it into a counterexample for the wrong TP-2 claim, and its replay would settle `ReplayParity`. State the qualitative claim per bound direction, and say what a mean-of-fraction or long-run run keeps as a witness. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:184-185; spec/functional/FR-193-keep-and-replay-sampled-witnesses.md:25-26,71-74,108-111 |
| FND-006 | medium | SPRT increment rounding is not specified so that it keeps Wald's bounds. ST-9 rounds only `N`, the thresholds and the half-widths, and ST-5 says the bounds hold "with no approximation". FR-189 asks that "each increment [be] rounded so the test never decides earlier than with the true values". A hold and a fail push the sum in opposite directions, so no single rounding direction per increment guarantees this in both directions. Specify two sums, one rounded toward each threshold, deciding Rejected only on the lower sum and Accepted only on the upper. Alternatively, cite the QSpec FR-408 method that does this. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:137,141; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:144-146 |
| FND-007 | medium | FR-186 contradicts itself on accumulate rewards. The text says "`accumulate R …` SHALL name a reward `R` of every operation of the model that can take a step; an operation without `R` contributes 0". The first clause requires every operation to declare `R`; the second admits operations that don't. §7.2's `request` and `reset` declare no `duration`. Say "a reward declared by at least one operation of the model". | spec/functional/FR-186-check-probabilistic-claim-forms-at-s3.md:103-105 |
| FND-008 | low | Two SPRT expected sample counts are off. Wald's `ln(1/β') / |drift|` gives about 5,168 at `p = 0.9999` for §7.1, not 5,065, and about 578 at `p = 0.96932` for §7.2, not 566. ADR-028 §15.1 repeats 5,065. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:248,297 |
| FND-009 | low | FR-191's use case says `NoFault` "stops at the default `max_samples` before Okamoto's nine million samples". The default is 16,777,216, which is above `N = 9,210,341`, so the default run completes. Use a set value, as FR-191-AC-1 does. | spec/functional/FR-191-bound-and-reproduce-a-statistical-run.md:31-35,41 |
| FND-010 | low | FR-188-AC-1's last clause asserts that a `quire.simulation.sampler/v1` `DefinitionRef` "runs whatever its version or digest field holds". This tests only that a version or digest check is absent, which a no-pins rule does not need a test for. Delete that clause and TC-623 step 1's matching fixture, and keep the refusal of another identity. | spec/functional/FR-188-draw-weighted-choices-with-the-revised-sampler.md:100; spec/test-cases/TC-623-weighted-sampler-draws.md |
| FND-011 | low | FR-190-AC-1's "a cycle count of the order of 91,624" is not a pass/fail criterion. With a fixed seed the count is exact: state it, or drop it. | spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:77 |
| FND-012 | low | EARS. "Two rewards of one name on an operation SHALL be refused" has no subject: state "S3 SHALL refuse …". "Non-negativity is checked … (FR-187)" is informative text in Behavior. | spec/functional/FR-185-declare-random-parameters-workloads-and-rewards.md:97-99 |

## Verdict

Not mergeable as it stands. Two high findings change verdicts (FND-001,
FND-002), and the branch must be rebased onto the current
`spec/366-temporal-properties` (FND-003). The ruling coverage, the worked
examples' main numbers and the AC-to-TC coverage are sound. FND-002 and
FND-001 also need the matching QSpec FR-407 and FR-408 fix.
