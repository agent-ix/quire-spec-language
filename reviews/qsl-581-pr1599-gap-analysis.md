---
id: SR-1599002
title: "gap analysis of PR #1599 (QSL-581a, FR-239 zone safety search, FR-241 finite concretisation)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1599 git diff cb95a247a...7ac32b8d7; FR-239 AC-1 to AC-5, FR-241 AC-1 and AC-2; qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: reviews
---
# SR-1599002: gap analysis of PR #1599 (QSL-581a, FR-239 zone safety search, FR-241 finite concretisation)

## Summary

Ticket: QSL-581. Plan completion: not assessed. FR-239-AC-2, AC-3 and FR-241-AC-1 are asserted as written. Four tags name ACs their tests only partly assert, against the PR body's own "partial" notes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `an_undefined_atom_refutes_only_where_time_diverges` carries FR-239-AC-5. It checks `6 mod (2 - n)` instead of the AC's `2 / (2 - t.n) >= 1`, does not assert the cause `division-by-zero`, the time stamp 2, `x = 0` or the locus, and does not cover the time-locked variant's refuted time-lock-freedom item. | qsl-analyze/tests/it/zone_search.rs:317-345 |
| FND-002 | medium | FR-239-AC-1 requires `Holds` with a certificate. The test gets `Holds` carrying the retained graph; no certificate exists until FR-244 (581b). | qsl-analyze/tests/it/zone_search.rs:142-173 |
| FND-003 | medium | FR-239-AC-4 is tagged, but the test uses `NoLateReply`, not `Settles`. The `RequiresBound` refusal with no state explored and the byte-equal certificates and counterexamples are unasserted. | qsl-analyze/tests/it/zone_search.rs:258-289 |
| FND-004 | low | FR-241-AC-2's "replays to `reproduced-with-evaluated-witness` (FR-237)" is unasserted. Timed replay is not built yet. | qsl-analyze/tests/it/zone_search.rs:200-217 |

## Verdict

**CONDITIONAL.** No untagged claimed AC, no stub, and every new path has an owning FR. Fix the four partial tags (or split the ACs). With code-review FND-001 (high), the PR is not mergeable.

## Coverage

Plan completion: not assessed. FR-241-AC-3 to AC-5 and the time-lock-freedom item are declared out of scope (581b/581c). Semantic review: done by hand for the claimed ACs.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new doc comment says the checker refuses AC-5's literal atom `2 / (2 - t.n) >= 1` "as ambiguous". Reproduced on 1c963df6f: the refusal is `ProfileProhibitedOperator(Divide)`, because the `timed_world` fixture's selected profiles do not admit `/`. AC-5's literal form stays untested for a fixture reason, not a checker one. Select a profile that admits division in the fixture and use the AC's atom, or correct the comment. | qsl-analyze/tests/it/zone_search.rs:318-330 |

## Dispositions

Round 1, against #1599 fix commit 1c963df6f (head; 69990aada..fb3b8137f are `=` in `git range-diff cb95a247a..7ac32b8d7 3880da60a..fb3b8137f`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Spec defect, not a checker defect. FR-239-AC-5's atom `2 / (2 - t.n) >= 1` divides two Integers, and QSL types integer `/` only against an expected `Rational` type (FR-091 'integer-to-rational promotion for rational `/` with an expected rational type'; FR-100-AC-5 requires `1 / x > 0` to refuse `ill_typed`). Reproduced on c158dd83a in the Ticker fixture: the atom, and the variants `>= 1/1` and `>= rational(1, 1)`, all refuse `ill_typed`/`AmbiguousLiteral` from `arithmetic()`'s Integer/Integer Divide arm (qsl-semantics/src/check/check.rs:2832-2856). The coder's measurement is right. My round-1 `ProfileProhibitedOperator(Divide)` came from a fixture with no model clauses, where the clause was checked without the complete-value layer. Restate FR-239-AC-5 (and FR-235-AC-5 and FR-237-AC-5, which cite it) with an atom QSL types, for example integer `div`, which the test's `6 mod` already shows typed, or a rational operand with an expected domain. | spec/functional/FR-239-check-a-timed-claim-by-symbolic-zone-search.md:AC table, FR-239-AC-5 |

Round 2, against round-2 commit c158dd83a (head).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | - |
