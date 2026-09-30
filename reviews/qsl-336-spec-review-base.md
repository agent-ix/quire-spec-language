---
id: SR-824
title: "QSL-336 base spec review of PR 539 (FR-122, TC-517)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@15d200d648a0fc06ec3d5a6c4270debfb40ee913; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
## Summary

Ticket: QSL-336. PR: quire-spec-language#539 at 15d200d6.

Examined FR-122 (statement, Inputs, Outputs, Behavior, AC-1 to AC-6) and
TC-517 against FR-070, FR-072, FR-098, FR-106, FR-107, FR-109, FR-116 (main
and PR #538's AC-6 edit), ADR-013 O-25 and OQ-H, and ADR-017 PF-4/G-2.

What is right: the refusal order (FR-098 recompile, stale `package_id`,
name lookup, envelope identity check, then FR-106 admission) matches FR-116
and PR #538's wording ("before it resolves the payload's ..."); no
contradiction with #538. The identity carriers follow ADR-017 G-2: the
clause node and `claim` occurrence come only from the envelope and are
checked against the recompile, and `selected_function` is unread, as PF-4
states for frames. Settlement matches FR-072 and FR-098 (`Verdicts` for a
clause that holds, `NoValue` for refused, incomplete, undefined or a family
result, internal failures as FR-100/FR-109 name them). FR-106 check numbers
(1, 2, 6, 7, 11) and codes cited are correct. Every fixture exists:
FR-108's ConfigVersion unit and cases (changed-version, unchanged-version,
forbidden-parent-change, healthy-parent, violating-parent, `sameIdentity`),
and TC-466 step 3's `probe` unit in
qsl-replay/src/spine/clause/tests.rs:2802-2861; FR-107-AC-3 confirms
`reaches(a, a, parent)` over `a -> b -> c` is false and `(a, c)` true, as
AC-4 needs. FR-122 and TC-517 collide with no ID on main or in open PRs
#538/#523. No ticket id, pin or SHA is added. The spec.md and tests.md rows
are in order; TC-517 covers every AC.

## Verdict

Sound and consistent with its neighbours. One medium gap: the Behavior does
not state which request limits drive evaluation and admission, although
AC-2 depends on it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Behavior never says which limit charges the FR-107 evaluation meter or bounds FR-106 admission. Inputs says only "FR-098's ... limits", but FR-098's request carries S1-S4 stage limits and separate accounting limits, and FR-109's run takes `work_units` and `ObservationLimits`. AC-2 requires the zero "request's `quire.value.accounting/v1` evaluation budget" to give `NoValue`: an implementer who feeds S3's `work_units` (as FR-109's `work_units` naming invites) refuses at recompile instead, and one who uses FR-109's default meter completes `false`. State that the evaluation meter is built from the request's accounting limits and admission runs under default `ObservationLimits` (FR-116's implementation does both). | spec/functional/FR-122-replay-a-state-clause-counterexample.md:68-73, 110-112, 137 |
| FND-002 | low | "the executor does not compare the envelope's witness transcript with those documents" states what is not done; the preceding sentence already states the agreement basis. Per state-what-is, drop the negative clause (or fold it into FND-003 of SR-827 by stating what the executor reads from the source arm). | spec/functional/FR-122-replay-a-state-clause-counterexample.md:116-118 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf00f646 |
| FND-002 | fixed | cf00f646 |
