---
id: SR-2026111305495001
title: "failure-domain review of QSL-1901 postcondition range facts (FR-577)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md; spec/functional/FR-104-check-state-clauses.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-1416-a-postcondition-s-range-is-proved-from-its-operation-s-precondition.md; PR quire-spec-language#2696"
review_set: subset
---

## Summary

Ticket: QSL-1901. Soundness and correctness lens over the fact rules FR-577
adds, checked against FR-103, FR-104, FR-120, FR-121, FR-122, FR-206, FR-302,
FR-569, `qsl-semantics/src/check/facts.rs`, `qsl-semantics/src/check/check.rs`,
`qsl-eval/src/value/expression/evaluate.rs`, and QSpec FR-146 and FR-151 (by
ID only).

The difference-bound algebra is sound: `x <= y` gives `x - y <= 0`, the strict
`x < y` gives `x - y <= -1` (integers), `>=`/`>` mirror them, a false outcome
takes the negated ordering, a repeated bound on one ordered pair keeps the
smaller `c` (a conjunction), and a join keeps a bound common to every
alternative with the larger `c` (the hull). Refining `a - b` by `u - v <= c`
(upper end `c`) and `v - u <= c` (lower end `-c`) is the right direction.
Matching a precondition read to a postcondition's `pre(e)` read agrees with
FR-104 "Observations of reads" (a precondition reads `pre`). The claim that
evaluation refuses `IntegerOutOfDomain` and does not clamp agrees with the
code: the expected-type rule wraps the arithmetic in a `Coerce` carrying the
arithmetic node's own location (`check.rs:2723-2731`), and evaluating a
`Coerce` outside its target refuses `IntegerOutOfDomain`
(`evaluate.rs:2446-2449`). The test values are right: `[0,100] - [0,100]` is
`[-100, 100]`; AC-3's cases give `[1, 100]` and `[0, 100]`; AC-4 and AC-5
refuse under the stated rules; AC-6's envelope has every value in range, so
FR-122 settles `inconclusive`/`NoValue`.

The defect is in the transfer premise. FR-577 assumes that a postcondition is
required only where its sibling unit preconditions held. Two QSL contexts break
that. FR-151 and FR-569 form a redefining member's effective precondition as a
disjunction with the redefined member's. FR-206 runs a protocol attempt over
only the clauses its `contracts` list selects. In both, a postcondition is
required where a unit precondition on its operation is false. A clause FR-577
admits can then fail evaluation at an enabled application, which is the
failure the static range check exists to prevent.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Precondition facts transfer into a postcondition even when the operation redefines another member or is redefined. QSpec FR-151 (and FR-569, "an effective precondition is the disjunction of the redefining operation's own clauses with the inherited ones") gives `eff(m) = own(m) or eff(r)`, with an absent clause counting as `true`. The effective postcondition is the conjunction of own and inherited posts. So a `post` on a redefining member, or a redefined member's `post` inherited into the redefiner's effective postcondition, is required where the unit `pre` clause is false. FR-577 does not exclude redefinition families. Its premise also cites QSpec FR-377/FR-468 rather than FR-151, which defines `eff(m)`. Transfer should be limited to an operation whose redefinition family is only itself, or should take the facts of `eff(m)` instead. | FR-577:36-38, FR-577:67-73, FR-569:215-220, FR-569-AC-5 | wrong-requirement |
| FND-002 | high | A protocol attempt evaluates the postconditions its `contracts` list selects without the omitted preconditions. FR-206 applies an attempt "over the clauses the attempt's `contracts` select", and FR-206-AC-2 says "An attempt whose `contracts` list omits a precondition clause is enabled where only that clause is false". With `attempt ... on Bank::Account::withdraw contracts [Debited]`, `Debited` is evaluated where `amount > pre(self.balance)` and refuses `IntegerOutOfDomain`, which FR-120 records as a `ContractUndetermined` finding. FR-577 neither states this consequence nor refuses an attempt that selects a fact-dependent post without the pres it depends on. | FR-577:36-38, FR-577:94-100, FR-206:70-73, FR-206-AC-2 | missing-requirement |
| FND-003 | low | When several precondition clauses are combined, the result for one key is unstated. Interval facts on the same (observation, path) from two `pre` clauses have to be intersected. An empty intersection (for example `amount <= 3` and `amount >= 5`, each satisfiable alone) makes `Q`'s walk unreachable. Only a single clause's unreachable true outcome is addressed. This is sound either way, but two implementers could differ. | FR-577:68, FR-577:83 | wrong-requirement |

## New findings (disposition pass 1)

Checked at 7b0adbe07.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The finding count for the protocol attempt is wrong. FR-577-AC-33 and TC-1416 step 14 expect "one `ModelFinding::ContractUndetermined` for `Debited`". But FR-120 evaluates the postcondition once per candidate post-state within the frame, and records a finding per undecided clause per candidate (`candidate` is the candidate's state key). `withdraw` modifies `balance: Int[0, 100]`, so from `balance = 3` there are 101 candidates. Each evaluates `pre(self.balance) - amount = -2` and refuses, which gives one finding per candidate. The Behavior bullet ("SHALL record the refusal as ... ContractUndetermined for `Q`") also does not say it is per candidate. | FR-577:131-136, FR-577-AC-33 (FR-577:235), TC-1416:61-63, TC-1416:95-96, FR-120:640-660, FR-120:608-615 |
| FND-005 | low | FR-577 now depends on FR-103's `redefines` keys on the spine (AC-26/27 use a domain-package redefinition), but FR-120:585-587 still says "On the spine every operation record's `redefines` is `None` (`qsl-semantics/src/model/intake.rs:1444`)". That contradicts FR-103-AC-10 and the current intake, which reads `redefines` at `intake.rs:2984` and `intake.rs:3456`. A reader of FR-120 would conclude that AC-26 cannot arise on the spine. The stale FR-120 sentence should be corrected alongside this change. | FR-577:87-97, FR-577-AC-26, FR-120:585-587 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 7b0adbe07 adds FR-577:87-97: transfer only when `op` "neither redefines another operation nor is redefined by an operation of the effective view", and none otherwise. It cites QSpec FR-151 `eff(m)`, and AC-26/AC-27 cover it. |
| FND-002 | fixed | 7b0adbe07 adds "Evaluation outside the precondition" (FR-577:125-136): the evaluator refuses `IntegerOutOfDomain`, and an attempt that omits the discharging pre records `ContractUndetermined` with no successor. AC-33 covers it. The finding count it asserts is wrong; see FND-004. |
| FND-003 | fixed | FR-577:98-101 now intersects interval facts per key, keeps the smaller `c` per ordered pair, and treats an empty intersection as unreachable. AC-23 covers it. |
| FND-004 | fixed | c5b1581c6: FR-577:132-137 now says the attempt evaluates `Q` once per candidate post-state of the frame (FR-120 "Post-states"), with one `ContractUndetermined` per refusing candidate and no successor. AC-33 and TC-1416 step 14 expect 101 findings from a one-account state with `balance = 3`, `limit = 0`. That matches FR-120:640-660: `modifies [Account/balance]` gives every `Int[0, 100]` value to `balance`, keeps `limit`, and has no `creates`/`deletes`, so there are 101 candidates. Each evaluates `3 - 5 = -2` and refuses. |
| FND-005 | fixed | c5b1581c6: FR-120:585-588 now says intake reads the inline `redefines` property as the redefined operation's key (FR-103-AC-10), and a family member takes the family's applicable clauses. This agrees with FR-103-AC-10, with `intake.rs:2984`, and with FR-120's own QSpec FR-151 `eff(m)` text above it. |
