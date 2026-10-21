---
id: SR-12293
title: "Spec review of quire-spec-language PR #1172 (QSL-898): FR-126, FR-128, FR-338, TC-521, TC-525, TC-539 edits"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@02898aab2cbbda7b837407f805deb76b6f9d9063; spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-128-replay-a-model-counterexample.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/test-cases/TC-521-explicit-state-model-check.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-539-replay-reproduces-an-undefined-evaluation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-521
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-525
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-539
    type: reviews
---
# Spec review of quire-spec-language PR #1172

## Summary

Ticket: QSL-898. Ticket and PR text were read as data. The scope is every
spec line the PR touches.

**The rewrites of existing ACs.** FR-126-AC-6, FR-128-AC-6, FR-338-AC-3 and
TC-521 step 6, TC-525 step 4 and TC-539 step 6 move from `test/tallies`
(`SumOutOfDomain`) to Counter with `pre CanInc { 6 mod (2 - self.value) >= 0 }`
(`DivisionByZero`). This is a legitimate correction, not a weakening:

- FR-120-AC-9, the AC the old text cited, already says the `test/tallies`
  package is "Not covered". The old reference pointed at a subject no AC
  defines.
- The claim that a linked package cannot reach `SumOutOfDomain` was checked.
  The evaluator raises it whenever a `sum` has an `Int` domain, so the claim
  rests on S3 refusing an unproved `sum<N>`. The `qsl-eval` tests that reach
  it check the sum alone, not in a linked unit, and S3 refuses an unproved
  range with `CheckCause::Unproved(Obligation::Range)`. The review in
  `reviews/qsl-790-pr899-spec-review.md` (FND-002) recorded the same refusal earlier. The claim is supported, though
  this review did not build a linked counter-example.
- The obligation is the same: an initial-state `pre` that is undefined ends
  the run with the empty prefix and `UndefinedEvaluation` at position 0, and
  the certificate is `BadState` with `ContractUndetermined`. Only the
  kernel-undefined cause changes. FR-338-AC-3 moves from an initial
  `BadState` to one at reached value 2; that rule is still exercised.

**The refused-contract row.** The PR does not touch it and does not claim it
is unreachable in the spec (that claim is only in a test comment and the PR
body). See SR-12292 FND-002.

**New text.** The FR-126 guarded-activation bullet, the "Unsupported liveness
clauses" narrowing to TP-4, the Outputs stutter-marker addition, AC-16, AC-17
and TC-521 steps 9/10 were checked against the code (SR-12291 probes). The
stutter marker matches QSL-530's lasso shape (`entry` = prefix length,
`terminal_stutter`, length counts the stutter step), and FR-128's
`close_loop` accepts it.

**The empty certificate claim.** Measured: a guarded `Holds` settles `failed`
today (`check_closure` finds no root, since S4 emits no `on each` clause).
Were the clause emitted, the empty certificate would be rejected, so it would
settle `inconclusive`. It never settles `proved`. The ticket's item 4 claim
holds for the guarded case too, but the spec does not say so (FND-001).

## Verdict

No major finding. The rewrites correct an AC that cited an undefined subject,
with an equivalent obligation. The new text matches the code. One medium
finding: the spec does not say that a guarded proof can never settle, and no
ticket owns that. Three low wording findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new bullet says "Such a clause's `Holds` carries an empty closure certificate, since the certified translation reads no `when` condition", but it does not say what follows. FR-127 V-1 settles `proved` only when the core checker accepts the certificate, so a guarded TP-1/2/3 proof can never settle `proved`: it is `failed` today and would be `inconclusive(CertificateRejected)` once S4 emits the clause. FR-338's state algebra has no `when` component, and QSL-878 (Done; its remaining-work comment lists no guarded case) does not own it. State the settlement in the bullet, and give the guarded certificate a tracked owner (QSL-898's remaining work or a new ticket). | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:179-188 |
| FND-002 | low | The sentence the PR edits still names `LivenessRefusal::GuardedActivation`. The code has no such variant: the refusal is `ModelCheckRefusal::GuardedActivation`. Fix the name while the line is being edited. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:364-367 |
| FND-003 | low | FR-126-AC-16 can be met only by giving the engine a checked clause directly, and its conditions (`c.value = 1`, `>= 2`, ...) are the letters' readings, not source text (the tests write `when (true)`). AC-13 says "given to the engine directly (S3/S4 refuse such a clause `NotEmitted`, FR-337)"; AC-16 does not. Add the same parenthetical, and say the condition is the reading the letter seam returns, as TC-521 step 9 does. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:449 |
| FND-004 | low | The Outputs addition sets the stutter marker "on a safety or undefined-evaluation counterexample whose last position is the terminal stutter step". An undefined evaluation cannot first occur on the stutter step: it re-reads the terminal state, whose letter was already read, and its contracts were already expanded. If it could occur, its `site.position` (stutter depth) would differ from `trace_position` (prefix length). Drop "or undefined-evaluation", or say that the case does not arise. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:404-409 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: FR-126 states a guarded Holds never settles proved (failed today, inconclusive once S4 emits the clause) and names Remaining work: QSL-898 | a0c3085b0 |
| FND-002 | fixed: names `ModelCheckRefusal::GuardedActivation` | a0c3085b0 |
| FND-003 | fixed: AC-16 carries the given-directly note and the reading-not-source note | a0c3085b0 |
| FND-004 | fixed: Outputs text and engine limit the marker to formula counterexamples | a0c3085b0 |
