---
id: SR-1671002
title: "Gap analysis of quire-spec-language PR #1671: symmetry on zones (QSL-1005a)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1dba086ab; PR #1671 own diff 21aa9813d..1dba086ab against FR-430-AC-3, FR-430-AC-4, FR-430 Reductions over a dense-time subject, FR-151, FR-152, FR-159, FR-160 and TC-1170; STD-493 comment ad5c9496 (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: reviews
---
# Gap analysis of quire-spec-language PR #1671

## Summary

Ticket: QSL-1005, slice 1005a. Plan completion: not assessed.

**FR-430-AC-3 (engine side).**
- The deadlock item holds with fewer stored states and settles
  Reduced/Uncertified (`symmetry_permutes_each_timer_with_its_clock`).
- The latching variant's lifted counterexample `arm(t1), arm(t2)` replays
  with the unreduced run's delays.
- The bound claim is checked once per orbit (`t1`), and its
  counterexample replays.
- A swapped state and zone canonicalise equal.
- Partial-order reduction, a constraint and both together settle V-8,
  naming partial-order reduction first, with nothing stored.

**FR-430-AC-4.** `strong_fairness_reads_enabledness_between_positions`
covers both halves. Weak fairness gives Violated with a `tick` lasso of
loop delay 1 that replays. Strong fairness gives Holds with a certificate
carrying a CF-2 Strong reason, which the core accepts. The engine already
read strong fairness over time, so this PR adds the test.

**STD-493 items (comment ad5c9496).** I checked all three, and each is a
real spec gap:
1. AC-3's `always holds(not (t1.armed and t2.armed))` cannot be written:
   QSL has no key literal, and a clause has one `over` binding. FR-151's
   stabiliser of `t1` in `[[t1,t2]]` is trivial, so the nearest claim that
   can be written runs unreduced. Testing the deadlock item and a bound
   claim instead is the right substitute.
2. FR-235 has no reduced row, so FR-160's row is used.
3. FR-430 is silent on whether the time-lock item and liveness-product
   clauses reduce.

**Symmetry refused for time-lock and liveness.** This deferral is
acceptable. The trap search and the liveness product read the edges of the
unreduced graph. Lifting a lasso through a quotient needs the loop repeated
up to the permutation's order (FR-161), and FR-430 gives no rule for that
over time. An honest V-8 with an owner (QSL-1005, a later slice) and a spec
question (item 3) is the right disposition. The order of that refusal is
SR-1671001 FND-002.

**Negotiation half.** The "at negotiation" half of AC-3 (qsl-route, the
`reductions` member of the provider manifest, FR-159-AC-3's second half
and FR-159-AC-5) is deferred to slice 1005a2 of QSL-1005. Only the engine
refusal exists here.

## Verdict

**APPROVED with low findings.** The engine half of AC-3 and all of AC-4 are
covered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two settlement paths have no EN-6 test: `SymmetryBroken`, where the initial states are not closed under the group (V-6 inconclusive through FR-160's row), and the `Symmetry` admission refusal. A partial class (an object outside every declared class) is also untested. My local probe shows that case correct: three timers, group `[[t1,t2]]`, the same verdicts reduced and unreduced, and lifted counterexamples that replay. Add one test for each. | qsl-analyze/tests/it/zone_dense.rs |
| FND-002 | low | AC-3 is satisfied only in part, and the rest needs tracking. "At negotiation" (V-8 with no candidate run, through qsl-route and the manifest's `reductions`) and the FR-159-AC-4 identity effect are deferred to slice "1005a2". No Linear ticket by that name exists; QSL-1005 has no children. Record 1005a2's scope on QSL-1005, so the AC-3 remainder has an owner beyond the PR body. | FR-430-AC-3; FR-159-AC-3, AC-4, AC-5 |
