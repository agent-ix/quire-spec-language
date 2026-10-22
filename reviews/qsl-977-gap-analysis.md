---
id: SR-12334
title: "Gap analysis of QSL-977 plugin result readers and plugin replay context (PR #1222)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language PR #1222 head c0c037a7e; FR-290-AC-5..14, TC-774, ADR-029 PL-7 against git diff origin/main...HEAD; QSpec FR-331 at qspec main 429c59dd"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: reviews
---
# SR-12334: Gap analysis of QSL-977 plugin result readers and plugin replay context (PR #1222)

## Summary

Plan completion: not assessed. Sources: Linear QSL-977, including the planner's correctness-fix comment; FR-290 (merged in #1217 and amended in this PR); and QSpec FR-331 "Certification", "Result entries" and FR-331-AC-23.

Criterion coverage. Each of FR-290-AC-5 to AC-14 has one `#[trace("TC-774", ..)]` test in `qsl-replay/src/proof_result/plugin_tests.rs`, and all ten pass.

- AC-5, AC-7, AC-8, AC-10 and AC-13 are fully asserted.
- AC-6 and AC-9 are asserted, except two core-only spellings (SR-12333 FND-004).
- AC-11's detail clause is asserted only partly (SR-12333 FND-005).
- AC-12 is asserted end to end: the built request goes through `crate::replay`, settles the input arm with category violation, then `refuted` at `x = 0` and `replay-parity` at `x = 5`.
- AC-14 lacks its state-clause `operation-contract` case (FND-001).

Ticket scope:

- The typed readers are present.
- The `ReplayRequestWire` context builder is present.
- The function-contract-only correctness fix is present, and it is correct (measured, SR-12333). Its regression test is weaker than the one the planner asked for (FND-003; SR-12333 FND-002).
- STD-264 / #1221 (`occurrence_key` on function requests) is still open. `request` already takes the `OccurrenceKey`, and the PR body records the follow-up.

Stubs and coverage inflation: none found. `MappingUndetermined` checks its payload but does not keep it, and `row` in a `reduced` basis is checked but not kept. Both are as the spec states.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-14's case "a state clause's `operation-contract` record: `NotReplayable`" has no test. The PR body says it needs a model unit, but one already exists: `config_version_unit_and_packages()` in `qsl-replay/src/spine/clause/tests.rs`, which runs through parse/select/check/package with its domain packages. I used it to measure: the frame's `(frame, generated, 0)` `operation-contract` requirement and each `state_clause` `(node, claim, 0)` requirement all give `NotReplayable`. So the behaviour is correct but unbacked. Fix: make the fixture `pub(crate)` (or move it to a shared test helper) and add the case. | plugin_tests.rs:736-800; qsl-replay/src/spine/clause/tests.rs:1782,3746 |
| FND-002 | medium | No test covers the `dependencies` path, which uses the resolved closure and `CheckedUnit::library_source`. Every TC-774 fixture has no libraries, so `library_source` returning `None` or a direct-only selection would pass. Measured with a probe: unit A imports B, and B imports C. The request's `dependencies` is `[test/b, test/c]`, and `replay` of it settles `refuted` at `x = 0` and `replay-parity` at `x = 7`. The existing replay test `transitive_only_library_source_reaches_replay` (execute/tests.rs) confirms that the recompile needs the transitive C. Fix: add this end-to-end case (about 20 lines, using the `SuppliedLibrary` pattern from that test). | plugin_replay.rs:107-116, 201-219; spine/lifecycle.rs:432-439 |
| FND-003 | medium | See SR-12333 FND-002. The planner's named regression (a value-validity item inside a Boolean function) is not the case the test asserts. | plugin_tests.rs:633-706 |
| FND-004 | low | The ordered-refusal criteria (FR-290 Behavior: readers in step order, `request` in its order) are only partly backed. See SR-12333 FND-001 for the M1 and M4 survivors. | plugin_tests.rs |

Verdict: no major gaps. The mediums are test additions only, so fix them in this PR.
