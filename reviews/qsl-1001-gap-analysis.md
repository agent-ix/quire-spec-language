---
id: SR-16621
title: 'Gap analysis of PR #1425: FR-223-AC-5 RC11 oracle and LT-12/LT-13 vectors, gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@60a259b33; git diff origin/main...60a259b33 (PR #1425, branch qsl1001-rc11-oracle): qsl-analyze/tests/support/rc11.rs, qsl-analyze/tests/it/rc11_oracle.rs, qsl-analyze/tests/it/litmus.rs, qsl-analyze/tests/it/weak_memory.rs; FR-223-AC-5, TC-668 step 5; ticket QSL-1001'
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-1001. PR: quire-spec-language#1425, head 60a259b33fd3ea4f2fea2d6b76d55b648f73259b. Method: quoin gap-analysis (planless; Plan completion: not assessed), using `quoin matrix --json` (quoin 0.28.4) on the head.

The ticket asks for one criterion, FR-223-AC-5. On main, AC-5 says that ADR-025 §15's LT-12 and LT-13 each reach their weak outcome under `ra`, the shapes PSC-7 names, and that neither outcome is reachable under `sc` or `tso`. It also says that every outcome an independent RC11 execution-graph checker allows for the two programs is reachable under `ra`. TC-668 step 5 runs both programs under `sc`, `tso`, `ra` and the checker, and its expected result is "both outcomes reachable under `ra` only; every outcome the checker allows is reachable under `ra`".

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-223-AC-5 | spec/functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md:97 | examined |
| TC-668 step 5 and its expected result | spec/test-cases/TC-668-rc11-partial-sc-order.md | examined |
| ADR-025 §15 LT-12, LT-13 rows | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:610-611 | examined |
| ADR-025 §15 PSC-7 | spec/decisions/ADR-025-weak-memory-models-for-parallel.md:602 | context_only |
| `lt12_reaches_its_weak_outcome_under_ra_only`, `lt13_reaches_its_weak_outcome_under_ra_only` | qsl-analyze/tests/it/rc11_oracle.rs:111-125 | examined |
| `the_rc11_checker_matches_the_known_litmus_outcomes` | qsl-analyze/tests/it/rc11_oracle.rs:127-177 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `the_rc11_checker_matches_the_known_litmus_outcomes` carries `#[trace("TC-668", "FR-223-AC-5")]`, but it asserts nothing AC-5 states: it never runs `sc`, `tso` or `ra`, and checks only the checker's own SB, MP and IRIW outcomes. It is calibration of the oracle that AC-5 presupposes, so the matrix lists three binders for AC-5 where two tests assert it. Drop the tag, or keep it and say in the doc comment that it validates the AC's oracle rather than the AC. | qsl-analyze/tests/it/rc11_oracle.rs:131 |

## Verdict

FR-223-AC-5 is covered. The matrix on the head lists AC-5 as tagged, with binders `lt12_reaches_its_weak_outcome_under_ra_only`, `lt13_reaches_its_weak_outcome_under_ra_only` and `the_rc11_checker_matches_the_known_litmus_outcomes` (FND-001). On main it was untagged. One low finding.

Spec against test, clause by clause:

- **The programs.** LT-12 is `u`: `X := 1` sc; `Y := 1` release ‖ `v`: `a := Y` acquire; `b := Z` sc ‖ `w`: `Z := 1` sc; `c := X` sc. LT-13 is `u`: `X := 1` sc; `a := Y` sc ‖ `v`: `Y := 1` relaxed; fence sc; `b := X` relaxed. Both match the `lt12` and `lt13` functions in order, orderings and locations. `a`, `b` and `c` are registers 1, 2 and 3, so the weak outcomes a = 1, b = 0, c = 0 and a = 0, b = 0 are `[1, 0, 0, 3]` and `[0, 0, 3, 3]`, where 3 is the unread register value.
- **"reach their weak outcome under `ra`"**: asserted.
- **"neither is reachable under `sc` or `tso`"**: asserted. `sc` runs with no selection, as the other litmus tests in `weak_memory.rs` do.
- **"every outcome that an independent RC11 execution-graph checker allows ... is reachable under `ra`"**: asserted as `rc11 ⊆ ra`. The checker is independent of `ra.rs`; see SR-16620.
- **"the shapes PSC-7 names"**: the tests assert that the checker forbids the weak outcome, and that `ra` minus the checker is exactly that outcome. The second assertion is implied by the others for these two programs (SR-16620 FND-001). By mutation, LT-12's outcome rests on the `po|≠loc ; hb ; po|≠loc` edge (PSC-7 a), and LT-13's on the fence ends of `psc_base` (PSC-7 b).
- **TC-668 step 5's expected result**: matches.

FR-223-AC-6 and AC-8 to AC-10 are untagged on the head and on main. They are outside this ticket and are not findings here.

Production code: none changed. The PR adds no code without an owning requirement.
