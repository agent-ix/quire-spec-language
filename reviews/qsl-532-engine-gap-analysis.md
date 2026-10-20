---
id: SR-13375
title: "Gap analysis of quire-spec-language PR #1073 against FR-130, FR-131, FR-133-AC-1 and the FR-127/FR-128 UnfairLasso amendments"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0f8a69083; spec/functional/FR-130 (AC-1..AC-5, Behavior), FR-131 (AC-1..AC-3, Behavior), FR-133 (AC-1, suppression bullet), FR-127 (V-6, AC-3), FR-128 (UnfairLasso), FR-126 (AC-13), FR-153 (AC-4, context); spec/test-cases/TC-531, TC-532, TC-534, TC-1085, TC-522, TC-523; spec/decisions/ADR-019 (FS-2..FS-8, SV-4); qsl-analyze/src/model_check/liveness.rs, qsl-replay/src/model_check/{replay,settle}.rs, qsl-analyze/tests/it/{strong_fairness,strong_hint,quotient}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-130
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-531
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-532
    type: reviews
---
# SR-13375: Gap analysis of quire-spec-language PR #1073

## Summary

Ticket: QSL-532 (E10a). PR #1073 at 0f8a69083. Planless; plan completion
not assessed. Tools: `quoin matrix --json` (quoin 0.28.3), exit 0.

Matrix state at 0f8a69083 for the criteria this PR touches:

| AC | Static status | Tagged by | Assessment |
| --- | --- | --- | --- |
| FR-130-AC-1 | tagged | `strong_each_holds_where_weak_each_and_strong_whole_are_refuted` | met. The `Holds` half is asserted as `ComponentCertificateNotBuilt` over 5 product states, FR-126's certificate-less proof form |
| FR-130-AC-2 | tagged | `a_refined_remainder_passes_and_gives_the_lasso` | met |
| FR-130-AC-3 | tagged | `cancellation_mixed_kinds_and_determinism` | met, with the test-intent gaps in SR-13374 FND-004 |
| FR-130-AC-4 | untagged | none | not built (certificate) |
| FR-130-AC-5 | tagged | `two_strong_constraints_failing_on_one_component_refine_together` | not met. The tag is inflation (SR-13374 FND-001) |
| FR-131-AC-1 | tagged | `a_weakly_fair_lasso_is_refused_under_strong_each` | met |
| FR-131-AC-2 | tagged | `a_loop_that_enables_nothing_starved_and_a_stutter_loop_are_fair` | met |
| FR-131-AC-3 | tagged | `an_unfair_en1_lasso_settles_replay_parity` | met |
| FR-133-AC-1 | tagged | `the_strong_whole_mutex_refutation_carries_the_same_hint` | partly met. The weak half's test is untagged, and the settle clause is unasserted (SR-13374 FND-007) |
| FR-126-AC-13 | untagged | none | this PR only edits its text. The `GuardedActivation` and certificate halves are outside it |
| FR-127-AC-3 | untagged | none | the weak unfair lasso clause is now built by the new `settle.rs` arm, but no test asserts it (FND-003) |

The engine and replay behaviour behind FR-130's filter, its counterexample
bullets and FR-131's Behavior was checked independently. Two brute-force
oracles agree with the engine: 60,000 random graphs at the kernel and
greedy-walk level, and 1,032 engine runs over 2-process and 3-process mutex
worlds. Every refutation replays. Details and log paths are in SR-13374.

No code without an owning requirement was found. Every added line serves
FR-130, FR-131 or FR-127 V-6. No stub or tautological assertion was found,
apart from the AC-5 tag.

Unbuilt criteria and who owns them:

- **FR-130-AC-4 (entire) and the certificate halves of AC-5.** FR-130's
  "With `Holds` ... `ComponentCertificate` ... `UnfairStrong{constraints,
  sub}`" bullet and the `Holds{Exhaustive}` it rides on are not built.
  The PR records this as comment b9959440 on QSL-532, and the PR body says
  "Remaining work: QSL-532". Two Linear tickets also name this work:
  QSL-893 (E8b remainder: `check_components` + `ComponentCertificate`) and
  QSL-896 (E10a remainder, which says "AC-4 and AC-5 also need FR-339's
  ComponentCertificate and check_components (QSL-893)"). So the AC is
  recorded, not silently dropped. But ownership is split across three
  tickets, and QSL-896's FR-130/FR-131 scope is now mostly delivered by
  this PR (FND-001).
- **FR-133 hint suppression.** Coded, but unreachable once strong replay
  is in (SR-13374 FND-005). It has no AC of its own, and no ticket owns a
  spec decision on it (FND-002).
- **FR-153-AC-4's strong agreement clause.** This is QSL-541's open gap
  (its gap analysis FND-002): the reduced verdict must equal the unreduced
  verdict. It was blocked on an unreduced strong filter, which this PR
  delivers. The `quotient.rs` header now says strong is "decided unreduced
  since FR-130, but AC-4 stays untagged" without naming who closes it
  (FND-004).

## Verdict

PASS with findings. FR-130-AC-1 to AC-3 and FR-131-AC-1 to AC-3 are met
and truthfully tagged. The FR-127 V-6 UnfairLasso row and FR-128's
`ReplayRefusal::UnfairLasso` are exercised. FR-130-AC-4 is declared
unbuilt and recorded on QSL-532. FR-130-AC-5 carries a tag it does not
earn. The lead should drop that tag (SR-13374 FND-001) and reconcile the
three tickets that claim the certificate remainder before closing QSL-532.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The certificate remainder (FR-130-AC-4, AC-5's certificate halves, and FR-130's `Holds`/`ComponentCertificate` bullet) is recorded only as free text on QSL-532 ("Remaining work: QSL-532"). Meanwhile QSL-896 (Backlog) is the named E10a remainder ticket for FR-130 AC-1..AC-5 and FR-131, and QSL-893 owns `check_components`/`ComponentCertificate`. Neither is updated by this PR. Once #1073 merges, QSL-896 claims work that is already done, and three tickets each claim the certificate. Fix (lead): move the certificate halves to one owner (QSL-893, or QSL-896 cut down to just them), and close or re-scope the other. Note it on QSL-532 before it closes. | spec/functional/FR-130-decide-strong-fairness-on-the-explicit-state-product.md (AC-4, AC-5, "With Holds" bullet) |
| FND-002 | low | FR-133's Behavior bullet "When the clause's fairness set already holds the named constraint, the executor SHALL emit no hint for that operation" cannot be reached now that replay checks strong constraints. A lasso that would yield such a hint is refused `UnfairLasso` first (SR-13374 FND-005). This is a spec question, not a code defect, and no ticket owns it. Fix (spec owner): decide whether FR-133 keeps the bullet as a stated invariant or drops it. Record the decision on the FR-133 owner's ticket. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md (Behavior, suppression bullet) |
| FND-003 | low | FR-127-AC-3's clause "replayed in an envelope for the weak `each` clause, whose fairness it fails, it settles `inconclusive`, `ReplayParity`, since EN-1 builds only fair lassos" is now built by the new `Err(ReplayRefusal::UnfairLasso { .. })` arm in `settle_model_check`. Before this PR a weak unfair lasso fell through to `Row::refused`. No test asserts the weak case. FR-131-AC-3's test covers only the strong-each refusal. Fix: add the weak `each` envelope case next to FR-131-AC-3's test, or under TC-522, and tag it `FR-127-AC-3` once its other clauses are covered. | qsl-replay/src/model_check/settle.rs:242-246 |
| FND-004 | low | FR-153-AC-4's agreement clause for the strong (AC-3) run was blocked only by the missing unreduced strong filter. This PR delivers that filter. The edited `quotient.rs` header records "AC-4 stays untagged" but names neither the remaining blocker nor an owner. Fix: name the owner (QSL-541's follow-up), or add the reduced = unreduced comparison for FR-153-AC-3's strong run now that both sides decide it. | qsl-analyze/tests/it/quotient.rs:14-18 |

## Dispositions

Round 1, head 377989edb. The matrix tags and ticket state were re-read
for this round.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The planner recorded the owner decision on Linear. QSL-893 (comment 899ff84f) is now the single owner of FR-130-AC-4, the certificate halves of AC-5, and liveness `Holds{Exhaustive}`/`ComponentCertificate`. QSL-896 (comment 68f0a352) is to be cancelled once #1073 merges and is verified |
| FND-002 | fixed | Spec PR #1076, merged 2026-10-20, removed FR-133's suppression bullet (QSL-532 comment 8f0cbac2). #1073 deleted the matching code |
| FND-003 | fixed | Added `a_weakly_unfair_en1_lasso_settles_replay_parity`, tagged TC-522. It replays a lasso that fails the weak `inc` constraint and asserts `inconclusive` / `ReplayParity`. Without the new `settle.rs` arm this would be `ReplayRefused`. The test is not tagged FR-127-AC-3, because that AC's other clauses are not covered here |
| FND-004 | fixed | The `quotient.rs` header now states that FR-153-AC-4's strong comparison (reduced against unreduced) is not made, which is why AC-4 stays untagged. The open item stays recorded under QSL-541's gap analysis (FND-002 there). Nothing in #1073's scope depends on it |
