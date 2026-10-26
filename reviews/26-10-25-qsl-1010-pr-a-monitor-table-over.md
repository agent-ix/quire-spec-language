---
id: SR-17228
title: "review of MonitorTable::over constructor (quire-spec-language PR 1565, QSL-1010 PR A)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@f73e94f8; qsl-eval/src/smt/monitor.rs; qsl-eval/src/smt/mod.rs; qsl-eval/tests/it/smt_query.rs; qsl-eval/tests/it/hyper_tuple.rs; qsl-eval/tests/golden/smt_monitor/"
review_set: subset
---

# Review of QSL-1010 PR A (quire-spec-language PR 1565)

## Summary

Verdict PASS (partial verification; the coordinator stopped cargo work for a reboot).

Verified:
- Goldens are genuine pre-refactor bytes: at commit 8fe681c43 (monitor.rs unchanged from main) the golden test passes against the six files.
- `of` for `on origin` routes through `over`. `translate(clause)` calls `translate_formula(formula, profile, Origin, Clause, Every, max_states, None)` and `translate_body` calls the same with the same arguments, so the automaton is identical for every input class (Selection, Counting, symmetry None, profile refusals, bounds). The tabulation body moved unchanged into `tabulate`, shared by both paths, so atom and state limits, refusals and loci are shared by construction.
- `on each` path calls `translate` then `tabulate`, unchanged. No duplicated or dead path.
- Test tags are `#[trace("TC-1264","FR-315-AC-50")]` and the tests assert what their docs claim (golden bytes; of == over with state count, initial, rejecting, numbered moves; hyper body table).
- Re-export of MonitorTable and Move is needed: the integration test in `tests/it` is outside the crate and PR B consumes `over`. Derive(Eq,PartialEq) is harmless (compiles; the equality is used by the of == over test).

Not verified (cargo stopped): head-commit test run, clippy -p qsl-eval, string-edge and arch-lint on touched files, and the reading of FR-315-AC-50 text against the AC-43 golden deferral to PR B.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No test drives a refusal (TooLarge atoms or states, PastOverFuture, timed profile) through `over`, nor an `on each` clause through `of`; equivalence rests on code identity, not on a test. Optional follow-up in PR B. | qsl-eval/tests/it/smt_query.rs | test-gap |
| FND-002 | info | The `of == over` test covers one origin clause only. | qsl-eval/tests/it/smt_query.rs | test-gap |

## Verdict

PASS. Mergeable; FND-001 and FND-002 are optional. The unverified items above remain for the coordinator's gate.
