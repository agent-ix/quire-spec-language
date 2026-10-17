---
id: SR-12045
title: "QSL-516 PR #924 gap analysis: refinement classification against FR-341 and FR-342"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@be1f10ec80fc270c6dd0745787415fc0089ceb1e; qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine/clause/tests/refinement.rs, spec/functional/FR-341-classify-a-spine-compile-result-for-a-refinement-gate.md, spec/functional/FR-342-classify-a-clause-run-disposition-for-a-refinement-gate.md"
review_set: subset
---

## Summary

Ticket: QSL-516. PR quire-spec-language#924. A planless gap analysis of the 12 new tests (TC-861, TC-862) against FR-341-AC-1 to AC-6 and FR-342-AC-1 to AC-5. Plan completion: not assessed. Read-only, so no `quoin matrix` run was made. The test-to-AC bindings were read from the `#[trace]` tags. Untested items the author already recorded (the protocol `compensate` unit for FR-341-AC-5, `StalePackage` for FR-342-AC-4, Forms limit data, the admission Incomplete limit) are not reported here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-341-AC-4 names a forms-stage limit of one and a type limit of one. `a_reached_limit_is_incomplete_naming_the_limit` sets `s1.nodes` and `s3.nodes` instead, so the type-limit path (`AssemblyCause::TypeLimit` to `ReachedLimit::of`, an `environment.*` setting) has no test, although the test carries the AC-4 tag. | qsl-replay/src/spine/clause/tests/refinement.rs:210-246 |
| FND-002 | medium | FR-342-AC-2's dangling-parent case (`refused` at `admit`) and FR-342-AC-3's incomplete-population case (`incomplete` at `admit`) are not tested, so the `Admit(Refused)` and `Admit(Incomplete)` arms have no test. Both fixtures already exist (`run_clause_refuses_a_dangling_parent_reference`, `run_clause_reports_incomplete_for_an_incomplete_population`). The AC-2 and AC-3 tests carry the tags anyway. | qsl-replay/src/spine/clause/tests/refinement.rs:373-432, qsl-replay/src/spine/refinement.rs:391-395 |

## Verdict

FIX, because of FND-001 and FND-002.

Covered with exact oracles (literal classes, codes and causes): FR-341-AC-1 (ConfigVersion admitted, no causes), AC-2 (Profile with exact cause, Intake, DependencyInput, Import, invariant plus typed cause in both orders), AC-3 (ill-typed unit, and the not-yet-implemented-then-typed vector with both causes in order), AC-5 (built `not-yet-implemented` check, `Omitted`, an `unsupported_projection` Emit), AC-6 (both catalog causes, exact). FR-342-AC-1 (healthy, violating, and frame violation with exact cause), AC-2 (Absent at select, Integer function at select, unknown parameter at admit), AC-3 (exhausted work at evaluate naming `work_units`/0), AC-4 (UnknownLanguage, Admit(Fault), EvaluateFault), AC-5 (missing-model compile classifies as `classify_refusal` gives it).
