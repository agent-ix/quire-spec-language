---
id: SR-1803001
title: "spec-review of FR-283 request-defect causes (QSL-1803)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; branch spec/qsl-1803-fr283-causes, commit b0bc9c692, diff origin/main...HEAD; FR-283, TC-765; checked against PR 2493 head f27a96252"
review_set: base
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-765
    type: references
---

# Spec review of FR-283 request-defect causes (QSL-1803)

## Summary

One commit (b0bc9c692) changes FR-283 Inputs (`temporal_selections`), Outputs
(`MonitorRecord::atom()`, the `ClauseRefused` request-defect list), Behavior
(four authentication-refusal bullets, the atom-naming bullet), AC-11 and AC-15,
and TC-765 steps 9 and 10. The review covered the base checklist only and ran no
analysis lenses. The spec text was compared with PR 2493 (`qsl-analyze/src/monitor.rs`
`authentication`, `emission_failure`, `named_atom`;
`qsl-replay/src/spine/clause/package_authentication.rs` `authenticate_emission`;
`qsl-replay/src/spine/clause/temporal.rs` `authenticate_binding`/`authenticate_trace`;
the tests in `qsl-analyze/tests/it/monitor_inputs.rs`), with QSL FR-522, and with
QSpec FR-272. No cargo was run.

What agrees with the code: the four defect classes and their codes and causes;
`member` `temporal_selections` and `packages`; `stage` `closure` and `strict-read`;
`mismatched_dimension`, `asserted`, and `expected` only when the row holds a value;
no `member` field on the binding-mismatch record; the missing-selection owner rule
(the declaration's clause if it is selected, else the first selected clause); first
selected clause for selection and package defects; emission and authentication
before position admission; a `timestamp_unit` selection mismatch that emits and is
refused as `TimestampUnit` with `asserted` `ms` and `expected` `us`; the forged
`Profile`, `ClockParameters` and `TimestampUnit` cases with an unprovisioned
position; the three package defects all at `strict-read`; and the
InsufficientData atom text `count over past[0, 9] min_count 2 >= 1`. The cause tags
`missing-selection` (`missing_declaration`) and `invalid-value` (`invalid_runtime_input`)
belong to QSpec FR-272's closed sets. QSpec is cited by ID only. No hash, digest or
pin content was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Precedence conflict. Behavior still says several refusing clauses fail "with the refusal or fault of the first of them in request order, after the trace is read and every position admitted" and that FR-330 stage-`select` gives `UnknownClause`. The code emits and authenticates every selected clause before admission, before `UnknownClause`, and before the subject checks. So `[Absent, Window]` over a `Timestamped` trace with a forged binding or defective `packages` returns `ClauseRefused`, not `UnknownClause`, and the package defects name `Absent`, which is no clause at all. Fix: state that emission and FR-522 authentication refusals precede `UnknownClause`, the subject checks and admission, and define what a request-level defect names when the first selected name resolves to no clause (or make the code check names first). | FR-283 Behavior "If several selected clauses refuse or fault"; monitor.rs `monitor`, `authentication` (`first = request.clauses.first()`) |
| FND-002 | medium | Atom naming for `holds` atoms disagrees with the code. Behavior names an atom by "the source text of the atom's condition", and AC-15 and TC-765 step 10 say the atom of `eventually[0,5] holds(p)` is named `p`. The code names the whole `holds` atom: the test asserts `MissingValuation.atom` and `record.atom()` equal `holds(c.value > 0)`. By the same rule, AC-15's clause gives `holds(p)`. Fix: say "the source text of the atom" (the `holds(...)` application or the aggregate comparison), and change AC-15 and TC-765 step 10 to `holds(p)`. The AC-15 fixture also differs from the tested unit (`Window`, `holds(c.value > 0)`), so either align the AC with the test fixture or the test with the AC. | FR-283 Outputs `MonitorRecord::atom()`, Behavior atom bullet, AC-15; TC-765 step 10; monitor_inputs.rs `a_timestamped_trace_is_authenticated_and_reads_indeterminate` |
| FND-003 | medium | The payload field names fall outside QSpec FR-272's canonical payload contract. FR-272 says fields "use these exact ordered names when applicable" (`stage`, `expected`, `actual`, `path`, ...). The new records use `member`, which is not canonical. They reuse `stage`, which FR-272 defines as a limits-type stage label, for the reader step `closure`/`strict-read`. FR-522's `asserted` is used where FR-272's name is `actual`. FR-272's `missing_declaration`/`missing-selection` row requires "the requested typed dependency/name, owning scope and referring locus", but the record carries only `member` `temporal_selections` and names neither the declaration nor its span, which the emitter holds (`source_ref`, `span`). Fix: carry the missing declaration's identity and span (FR-272 `subject`/`locus` or `path`). Either map `member` to FR-272 `path` and the step to a non-`stage` name, or record in FR-283 that these QSL field names are the local convention (`member` is used in TC-1058) and that QSpec FR-272's list needs them added. | FR-283 Outputs ClauseRefused list; QSpec FR-272 "Canonical cause payload contract", `missing_declaration` row; package_authentication.rs `MissingSelection { clause }` drops `source_ref`/`span` |
| FND-004 | medium | No criterion distinguishes the naming rules. Every AC-15 request selects only `Window`, so "the declaration's clause when it is selected, otherwise the first selected clause" and "naming the first selected clause" cannot fail: an implementation that always names the first selected clause passes. For example, `[Window, Window2]` with only Window2's selection missing must name `Window2`, and `[Window2, Window]` with a defective `packages` must name `Window2`. Fix: add these two-clause cases to AC-15 (or a new AC) and TC-765 step 10. | FR-283-AC-15; TC-765 step 10; monitor.rs `emission_failure` owner filter |
| FND-005 | medium | The `stage` `closure` refusal has no criterion. Outputs and Behavior specify it, but every package defect in AC-15 is `strict-read`. The test comment states that the fixture's package has no `dependency_selections`, so closure "has no input here". Fix: add an AC and TC step over a unit with a library dependency whose dependency document is missing or another document, expecting `member` `packages`, `stage` `closure`. | FR-283 Outputs packages bullet, Behavior read-back bullet, AC-15; monitor_inputs.rs `domain_packages_the_strict_read_refuses_are_request_refusals` |
| FND-006 | low | The binding-mismatch text says the binding "differs ... in one dimension" (Outputs) or "in any FR-522 dimension" (Behavior). When several dimensions differ, the code reports only the first, in FR-522's order `Profile`, `Clock`, `ClockParameters`, then the member dimensions. When several selected clauses fail, it names the first failing clause in request order. Fix: say "the first differing dimension in FR-522's order" and "naming the first such clause in request order". | FR-283 Outputs binding bullet, Behavior binding bullet; FR-522 Authentication; temporal.rs `authenticate_binding`; monitor.rs `authentication` loop |
| FND-007 | low | Two authentication outcomes are unstated. First, a `Timestamped` trace with no `binding` is refused `invalid_runtime_input`/`invalid-value` with no fields (`authenticate_binding(None)`), which is neither a mismatch record nor listed. Second, an emission refusal other than the four listed (including an emitter limit stop under the default `SpineLimits`, since `MonitorLimits` has no emit/import limits) becomes `StageFailure::Fault`, and cancellation during emission becomes `Cancelled`. Fix: add the absent-binding case to the binding bullet. State that emission runs under the published default emit and import limits and that any other emitter refusal is an internal failure. | FR-283 Outputs ClauseRefused list, internal-failure bullet, Inputs `limits`; package_authentication.rs `_ => Fault`; temporal.rs `authenticate_binding` |
| FND-008 | low | The Outputs selection-defect bullet reads poorly: two colon-introduced lists in one sentence ("... and an invalid clock value: a selection repeated ..., a configuration of another profile than its declaration's: `invalid_runtime_input`..."), and the examples omit the invalid clock value. Fix: one list, "a repeated selection, a surplus selection at a span no declaration holds, a configuration of another profile, or an invalid clock value", then the code. | FR-283 Outputs, `temporal_selections` bullet |
| FND-009 | low | AC-15 now bundles about 15 independent claims, including the indeterminate verdict, the source refusal, pending, the clock mismatch, five selection defects, three package defects and three forged dimensions. This breaks the single-claim rule and makes a failure hard to localize. Fix: move the request-defect half (from "Over the same `Timestamped` trace (binding unit `ms`)") into a new FR-283-AC-16 or AC-17, mirrored in TC-765 step 10, and retag the four defect tests. | FR-283-AC-15; TC-765 step 10 |
| FND-010 | low | The `impl MonitorRecord` block in Outputs lists only `atom()`, but the public surface in PR 2493 also has `report()`, `counterexample()` and `item(index)`, and the ACs rely on them (`record.report().disposition`, `record.counterexample()`). Fix: list them, or say the block is partial. | FR-283 Outputs; monitor.rs `impl MonitorRecord` |

## Code-side note for PR 2493

The doc comment on `emission_failure` ("FR-283 names only a selected clause's missing
selection ...") is stale once this spec lands; update it in PR 2493.

## Verdict

Changes requested. There are no high findings. The cause vocabulary is QSpec FR-272's
closed variants, and the codes, causes, fields, owner rules and pre-admission ordering
match PR 2493. FND-001 and FND-002 are real spec–code disagreements. FND-003 is a
payload-contract gap against QSpec FR-272 that needs a decision: rename the fields, or
record the local names. FND-004 and FND-005 are untested branches of the new text.

## Dispositions

| ID | Disposition | Note |
| --- | --- | --- |
| FND-001 | fixed | Outputs states emission precedes the stage-`select` check and that "first selected clause" is the first element of `clauses` as supplied; the several-clauses Behavior bullet orders authentication before admission. Naming a non-clause is a code-side behavior reported to the code lane. |
| FND-002 | fixed | Atom named by the source text of the whole atom; AC-15 and TC-765 step 10 read `holds(p)`. |
| FND-003 | fixed (recorded) | FR-283 Outputs records `member`, `stage` and `asserted` as QSL's own fields and that the `missing-selection` record carries no declaration identity or span; reported as a code/vocabulary deviation. |
| FND-004 | fixed | New FR-283-AC-16 and TC-765 step 11: two-clause naming cases. Tests to be written in the code lane. |
| FND-005 | fixed | FR-283-AC-16 and TC-765 step 11: `stage` `closure` over a library dependency. Test to be written in the code lane. |
| FND-006 | fixed | First differing dimension in FR-522's order; first failing clause in request order. |
| FND-007 | fixed | Absent binding, cancellation during emission and other emitter refusals (default limits) stated. |
| FND-008 | fixed | Selection-defect bullet rewritten as one list with FR-337's `InvalidTemporalClockValue`. |
| FND-009 | fixed in part | Untested new claims go to AC-16; the existing defect claims stay in AC-15, where PR 2493's tests trace them (no code change in this PR). |
| FND-010 | fixed | The `impl MonitorRecord` block is marked partial. |
