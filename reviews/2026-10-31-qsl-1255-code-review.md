---
id: SR-1939001
title: "Code review of PR #1939 FR-049 replacement tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-eval/tests/it/accounting_boundaries.rs, checked_package_call.rs, protocol_slots.rs, relationship_reachability.rs, state_model_family.rs, main.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1072
    type: reviews
---
# SR-1939001: Code review of PR #1939

## Summary

Ticket: QSL-1255. PR agent-ix/quire-spec-language#1939, head `74b3eea6b`.
Tests-only change (code-review with the rust-review lane). Every `#[trace]`
tag was checked against the full text of its AC/TC: TC-136 (H1, H2, H4),
TC-137 (per-counter table, no-work, above-default, C1/C2, three 512 KiB
stack cases), TC-1072 rows 7/8 with FR-390-AC-8, FR-049-AC-7 and
NFR-009-AC-3. FR-049-AC-1 is correctly left untagged. gap-analysis was not
run: no production code changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-137 requires the independently counted fixture quantity to equal the metered usage; for integer_bits (41 vs 42), decimal_digits (3 vs 7) and value_occurrences (n vs n+1) it does not, and value_occurrences/work_units/result_units have no counted oracle at all, only marginal deltas. The TC-137 tag claims the row while the clause is unmet; the gap is disclosed only in the PR body, with no TC-137 amendment ticket. | qsl-eval/tests/it/accounting_boundaries.rs:363-383, 498-515, 584-598 |
| FND-002 | low | The test docstring says "its independently counted quantity is the metered usage", which is what the test does not assert (it asserts the count at the named charge point), contradicting the PR's own disclosure. | qsl-eval/tests/it/accounting_boundaries.rs:579-583 |
| FND-003 | low | Tautology: `Category::Undefined.exit_code() == 10` and `Category::Success.exit_code() == 0` are constant checks not derived from the evaluated outcome; the category/exit clause of FR-390-AC-8 rests only on the `FamilyResult::Undefined` variant match. | qsl-eval/tests/it/protocol_slots.rs:971-985 |
| FND-004 | low | An unused `profile p` was added to the unrelated QSpec-TC-198 fixture (only `declarations_over` needs it for `gated_package`); the alias also shares its spelling with the parameter `p`. Out-of-scope fixture change. | qsl-eval/tests/it/state_model_family.rs:866 |
| FND-005 | low | TC-136 H2 does not assert that the checked package actually holds `internal_guard`; if checking dropped the clause, H2 collapses into H1 and still passes. | qsl-eval/tests/it/checked_package_call.rs:427-471 |
| FND-006 | medium | QSL-1255 is in state Done while PR #1939 is open, and its FR-049-AC-1 item (compile-fail half) and TC-136 H3 remain uncovered with no follow-up ticket recorded. | QSL-1255 |

## Verdict

ACCEPT WITH FINDINGS. The C1/C2, no-work, above-default, 512 KiB stack,
H1/H4 and charge-before-work prefix assertions are real and field-based. The
coder's disclosures are accurate. FND-001 and FND-006 need a tracked
follow-up (TC-137 wording or oracle; FR-049-AC-1 compile-fail and H3) before
the ticket is truly done.

## Dispositions

Round 1, fix commit `628ad0e13` (head `942a5232c`).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Per-case `usage_equals_counted`; usage == counted asserted for the five counters where it holds, usage > counted for integer_bits/decimal_digits; docstring states TC-137's equality clause is not claimed for the five divergent counters. TC-137 wording amendment still untracked (see FND-006). |
| FND-002 | fixed | Docstring rewritten to the asserted behaviour. |
| FND-003 | fixed | Constant exit_code asserts removed; replaced by an outcome-derived `fields["operation"] == "ready"` assertion; doc says category follows from the variant. |
| FND-004 | fixed | Unused `profile p` removed from the QSpec-TC-198 fixture. |
| FND-005 | fixed | `package.graph().declares_function("internal_guard")` asserted before the refusal. |
| FND-006 | still-open | QSL-1255 still Done with PR open; no follow-up ticket for the FR-049-AC-1 compile-fail fixture, TC-136 H3 or the TC-137 wording amendment. Owned by the code lead, not the coder. |

Round 2, authorized reviewer handoff at report-only head `907cd2238`. The original review and round-1 rows above are unchanged.

| FND | outcome | reason |
| --- | --- | --- |
| FND-006 | fixed | The completion-state/tracking defect is fixed: live QSL-1255 is In Progress; comment f3bbb401-f26f-4833-888a-c1254f7d07ec records the remaining ProtocolSlot, H3 node-identity and TC-137 quantity work as linked children QSL-1363, QSL-1364 and QSL-1365. The missing normative behavior itself remains deferred, not fixed. |

## Disposition verification (round 2)

At head `907cd223883c3798475d07ad783daf51483f457e`, the only change since reviewed `4eea75ef7` is the report-only commit copying SR-1939002 and SR-1939003 verbatim into reviews/. Byte comparison passed, and git diff over qsl-eval is empty. No new code review or aggregate run was needed. No new finding or regression was identified.

Live tracker verification: QSL-1255 is In Progress, assigned to Peter Krenesky, with has-review retained. Comment f3bbb401-f26f-4833-888a-c1254f7d07ec, created 2026-10-31T16:59:53.155Z, explicitly retains the remaining scope and defers the three gaps to child tickets QSL-1363 (Specify), QSL-1364 (Specced), and QSL-1365 (Specify). All three child-parent links were verified. The children currently have no individual assignee; the dispatching code lead has adopted their routing and the parent remains assigned. This is tracked deferral, not a claim of completed implementation.

The original finding rows and verdict remain unchanged as historical review evidence. The disposition below is current: partial tests-only delivery is mergeable from the review standpoint, subject to the lead's remaining gates and existing baseline policy. Normative assurance completion still FAILS until the linked work is resolved. The inherited completion-state discrepancy SR-1939001 FND-006 is separately fixed by the reopened parent and recorded remaining scope; none of the production gaps is fixed by that administrative repair.
