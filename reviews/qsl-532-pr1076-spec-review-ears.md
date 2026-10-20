---
id: SR-13381
title: 'Spec review of PR #1076: FR-133 hints name only starved constraints, EARS'
type: SpecReview
analysis: ears-conformance
scope: 'agent-ix/quire-spec-language@080408b8c; git diff refs/remotes/origin/main...080408b8c (PR #1076, base main 9567ed601): spec/decisions/ADR-019-strong-fairness.md, spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md, spec/test-cases/TC-534-strong-fairness-hint.md; read against FR-131, QSpec FR-369 (quire-specification main) and qsl-replay replay.rs on main and on qsl-532-engine (#1073, open); ticket QSL-532'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-133
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-534
  type: reviews
- target: ix://agent-ix/quire-spec-language/ADR-019
  type: reviews
---

## Summary

Ticket: QSL-532. PR: quire-spec-language#1076, head 080408b8c, scoped to `git diff refs/remotes/origin/main...080408b8c` (one commit, three files, +15/-8). Spec-only: FR-133 drops 'emit no hint when the clause already holds the named constraint' and states that a hint names only a constraint the lasso starves; ADR-019 SV-6 gets the same rule; new FR-133-AC-5 and TC-534 step 6. Unreachability verified. The hint conditions (FR-133 Behavior, SV-6) are: Whole when the operation is enabled at some loop state and no loop step takes it; otherwise Each for an identity enabled at a loop state and never taken. FR-131/SF-4's strong check is: when the constraint is enabled at some loop state, some loop step takes it. On qsl-532-engine (#1073, open) unfair_constraint builds a strong Whole constraint over every identity of the operation the loop enables or takes, and one strong constraint per such identity for Each; check_loop's strong rule is taken || !enabled_somewhere. So a held strong Whole fails exactly when the Whole hint would fire, and a held strong Each fails exactly when either hint would fire (a Whole-starved operation also starves an identity). In both cases replay returns UnfairLasso before strong_hints runs (strong_hints is called only in the Reproduced arm). The one combination left, held strong Whole with an Each hint (FR-133-AC-1's second sentence: loop takes acq(2), acq(1) enabled at position 0 and never taken), suggests {Strong, acquire, Each}, which differs from the held {Strong, acquire, Whole}: a different granularity, never matched by the old 'same kind and granularity' suppression. Weak held constraints never matched either, since every hint is Strong. QSpec FR-369 (the semantic authority, ADR-019 QS-8) already says 'A reproduced lasso is fair under every constraint the clause carries, so no suggestion names one of them', so QSL now agrees with QSpec. On main the premise is spec-only: replay.rs refuses any strong constraint with ModelTraceRefusal::StrongFairness; strong replay arrives with #1073, and #1073's strong_hints still carries the now-dead `if !clause.fairness().contains(&suggested)` and the doc line 'An operation the clause already constrains that way gets none' (code, outside this diff). FR-133-AC-5 / TC-534 step 6 checked by hand against FR-131-AC-1: lasso 0 -acq(2)-> 2 -rel-> 0 under strong each acquire, acq(1) enabled at position 0 never taken, refuses UnfairLasso{Strong, acquire, Each} with no result; FR-131-AC-3 / FR-127 settles an EN-1 outcome so refused inconclusive ReplayParity (V-6), carrying no hint since it is not refuted. Correct. Stale text: grep of spec/ for suppress, 'already carries', 'already holds', 'already constrain' and every hint/would-exclude mention (ADR-018 AM-4 note, ADR-019 §6 worked example, DS-1, QS-7, QS-8, RU-4, US-017, FR-129, FR-130, FR-131, TC-531, TC-532) finds no remaining suppression wording. EARS: the edited Description statement stays event-driven ('When replay reproduces the lasso ..., the replay executor SHALL add ... for each operation ... whose strong fairness would exclude that lasso (ADR-019 SV-6), that is, for each operation the lasso starves'), one SHALL, a named subject. The removed bullet was the only SHALL about suppression; its replacement carries no SHALL (FND-001). The quire EARS lint reports nothing for FR-133. Gates on the head: make check-spec-validation exits 0 with 17 warnings, the same 17 as on main 9567ed601 and none in FR-133, TC-534 or ADR-019; tools/check-index-completeness.sh passes. quoin 0.28.3, quire 0.36.2 (engine 0.50.2).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-133-description | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-starves-bullet | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| ADR-019-SV-6 | spec/decisions/ADR-019-strong-fairness.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new Behavior bullet sits among SHALL bullets but carries no SHALL: it is a derived property with its rationale. That is acceptable as prose, but 'refused one that is not as ReplayRefusal::UnfairLasso' reads as a garble (ADR-019 SV-6 repeats 'refused one that is not as UnfairLasso'). Either mark it as a consequence ('Hence a hint names only a constraint the lasso starves: replay (FR-131) refuses, as UnfairLasso with no result, a lasso unfair under any constraint of the clause's fairness set, before any hint is computed') or make it a requirement ('The executor SHALL compute hints only for a lasso FR-131 accepts as fair under the clause's fairness set'). | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:79-83 |

## Verdict

Mergeable: one low wording finding; the edited requirement statement conforms.

## Dispositions

Round 1 checked at head a54220159 (fix commit a54220159 on top of merge ff6a60244 with main 40f8707b1); gates: make check-spec-validation exit 0, 17 warnings, same 17 as main 40f8707b1, none in FR-133, TC-534, ADR-019 or reviews/; tools/check-index-completeness.sh passes.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The bullet now has SHALLs, but (a) the garble persists: 'SHALL refuse a lasso that is not with ReplayRefusal::UnfairLasso and no result' (and SV-6 'refuses one that is not with UnfairLasso') still drop the word 'fair'; and (b) the SHALLs restate FR-131's obligation with subject 'Replay' (two SHALLs in one statement) instead of stating FR-133's own. Suggested: 'The executor SHALL compute hints only for a lasso that replay accepts as fair under every constraint of the clause's fairness set (FR-131, ADR-019 FS-8); a lasso that is not fair is refused with ReplayRefusal::UnfairLasso and no result.' and in SV-6 'refuses a lasso that is not fair with UnfairLasso'. |
