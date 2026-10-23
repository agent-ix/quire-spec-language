---
id: SR-16270
title: "Spec-review of the exact binomial (Clopper-Pearson) statistical method (quire-spec-language PR 1373)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@8fa4d7d6b80ca0a7956657a8c8df8797aaa96ea4; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md; spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md; spec/functional/FR-192-settle-a-statistical-result-on-the-measured-axis.md; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md"
review_set: subset
date: 2026-10-23
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-024
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-624
    type: references
---

# Spec-review of the exact binomial (Clopper-Pearson) statistical method (quire-spec-language PR 1373)

## Summary

Ticket: STD-429. Head 8fa4d7d6, with QSpec half quire-specification#420 at f40e02f5 (SR-16271).
origin/main is 2 commits ahead and is not an ancestor. The merge is clean. Gate: `make check-spec-validation`
with QSPEC_DIR at #420's head exits 0, with no warnings on the touched files.

Hand derivation (exact rationals, Python `fractions`, `math.comb`), with alpha = beta = 1/100,
m = 1, iota = 1/10, theta = 9/10:

- N = ceil(ln 100 / (2 * 0.01)) = ceil(230.26) = 231.
- G(k, 9/10) = Pr(X >= k): k=219 0.0063428 (<= 1/100, accept), k=218 0.0127723 (no).
- L(k, 9/10) = Pr(X <= k): k=196 0.0087720 (<= 1/100, reject), k=197 0.0146424 (no).
- So the method accepts at k >= 219, rejects at k <= 196 and is undecided at 197..218, which matches the
  vector. Okamoto: accept iff k/231 - 1/10 >= 9/10 (k = 231 only); reject iff k/231 + 1/10 < 9/10 (k <= 184).
  That matches too.
- The error bounds hold. G(k, p) rises in p, so Pr_p(accept) <= G(k*, theta) <= beta' for p < theta, and the
  same argument bounds rejection. Both tails can hold only when alpha' + beta' > 1, because G + L = 1 + pmf(k).
  Clopper-Pearson decides wherever Okamoto does (Hoeffding).
- Tail precision: 8(N+1) roundings at 2^(1-P) with P = 96 + ceil(log2(N+1)) give a relative error of at most
  2^-92 (< 2^-90). The statement is sound.
- At k = 219 the exact Clopper-Pearson ends (exact-rational bisection, 60 halvings) are lower 0.90351472792271070918... and upper 0.97621175445884490925...

Cross-spec: ST-11 and FR-189 agree with QSpec FR-408 on N, the tails, the strict quantile bound, the bisection,
the clause-meter charge and the `NotStatistical` refusals. ADR-024-ST-11 and FR-189-AC-16 are unique across open
PR heads. The PR merges clean with #1334 and #1332 (and with #1269, #1299, #1341, #1360 and #1362, which also
touch ADR-024/FR-189). The `NotStatistical` refusal for fraction forms is not a narrowing: those forms are not
Bernoulli, and Okamoto or regeneration still decides them.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | [soundness] FR-189-AC-16 requires that at k = 219 'the interval contains [0.9035147279, 0.9762117545], each end within 2^-64'. The exact ends are 0.90351472792271071 and 0.97621175445884491. An outward end within 2^-64 of them is about 0.9035147279227 > 0.9035147279 at the bottom and 0.9762117544588 < 0.9762117545 at the top, so the reported interval lies inside the stated pair and does not contain it. Its ends are also 2.3e-11 and 4.1e-11 from the stated decimals, not 2^-64. A correct implementation fails this AC (TC-624 step 14 also fails). Fix: state the exact ends to 20 digits (lower 0.90351472792271070918, upper 0.97621175445884490925) and require each reported end within 2^-64 of them, outward: lower <= the exact lower and upper >= the exact upper. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:266; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:35; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:54 | wrong-requirement |
| FND-002 | medium | [soundness] SV-5 reads as the complete list of NotStatistical refusals: 'SPRT or regeneration for a mean of a fraction, Okamoto or SPRT for a long-run fraction, regeneration for any other form'. It omits ST-11's Clopper-Pearson refusals for both fraction forms. RU-4 still lists '(Okamoto, SPRT, regenerative)'. FR-190, which owns the long-run refusal, still says 'the Okamoto or SPRT method for a long-run fraction'. Fix: add Clopper-Pearson to all three. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:180; spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:401; spec/functional/FR-190-measure-a-long-run-fraction-by-regeneration.md:73-75 | wrong-requirement |
| FND-003 | low | [coverage] QS-12 (a) says 'both methods' decisions', and its list names no Clopper-Pearson conformance vector (QSpec TC-355 SC-10 to SC-13). TC-624's title and description still say 'Okamoto and SPRT', though step 14 covers the third method. Fix: say 'each method' in QS-12 (a), add the binomial vector to QS-12, and name the method in TC-624's title and description. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:390; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:3; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:13 | missing-requirement |
| FND-004 | low | [other] Okamoto's half-width is ι = sqrt(ln(1/δ)/(2N)), about 1.52/sqrt(N) at δ = 1/100. The Clopper-Pearson half-width is about z * sqrt(p(1-p)/N), at most 1.16/sqrt(N). So the interval is narrower at every p̂, not only near 0 or 1. Fix: 'its interval is narrower, most of all when p̂ is near 0 or 1'. | spec/decisions/ADR-024-statistical-and-probabilistic-properties.md:141-144 | missing-requirement |

## Verdict

Not mergeable. The decisions, sample count, error bounds and tail precision are correct, and they agree with QSpec #420. The k = 219 interval clause, however, fails for a correct implementation (the same defect is in QSpec FR-408-AC-13 and TC-355 SC-11, SR-16271), and three method lists still omit Clopper-Pearson. Fix both PRs together.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-005 | low | [ambiguous] The exact upper end is 0.976211754458844909955841... The text writes it as `0.97621175445884490996…`, rounded up in the 20th digit, while the lower end `…070929…` is truncated. Where the literal serves as the threshold ('at least `0.97621175445884490996…`'), a correct reported upper end in [exact, …0996) fails it. Fix: write `0.97621175445884490995…`, truncated like the lower end, or give the digits `0.976211754458844909955841…`. | spec/functional/FR-189-decide-a-probabilistic-claim-by-statistical-model-checking.md:266; spec/test-cases/TC-624-statistical-model-checking-decides-probabilistic-claims.md:54 | wrong-requirement |

## Dispositions

Round 1, against head 1ec2395c. Gate `make check-spec-validation` with QSPEC_DIR at QSpec #420 e3bc134b: exit 0. Exact Clopper-Pearson ends at k = 219 (one-sided levels: lower = least p with G(219, p) >= beta' = 1/100, upper = greatest p with L(219, p) >= alpha' = 1/100; exact-rational bisection, 90 halvings): lower in [0.90351472792271070929237711031, 0.90351472792271070929237711113], upper in [0.97621175445884490995584141752, 0.97621175445884490995584141834]. The author's 0.90351472792271070929... and 0.97621175445884490996... are right to within the rounding of the last digit. The first-round SR's ...070918 / ...490925 were wrong: they were the lower ends of a 60-halving bracket, not the exact values. The 64-halving dyadic bracket ends that a conforming bisection reports are lower 0.90351472792271070929182724685 (exact minus 5.5e-22) and upper 0.97621175445884491000524824100 (exact plus 4.9e-20), both within 2^-64 = 5.42e-20.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
