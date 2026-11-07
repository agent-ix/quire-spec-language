---
id: SR-20261106200930
title: "spec-review of FR-283 monitor trace shapes, refusal shapes and refusal loci"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language branch spec/fr283-monitor-rulings head a0ee4dbc0 over origin/main 2ed49b651; git diff origin/main...HEAD (FR-283, one FR-330 bullet, TC-765, TC-766)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
---

# Spec review of the FR-283 monitor rulings

## Summary

This was one direct pass at head a0ee4dbc0. It covered the base checklist and the EARS-conformance,
integrity and failure-domain analyses, all done by hand. The rulings under review: QSL-1791/1794 (the
`MonitorTrace` shapes `Document`, `Typed` and `Samples`), QSL-1792 (whole-request
`UnknownClause`/`ClauseRefused`, first in request order) and QSL-1793 (`sources`, rendered loci,
`SourceNotSupplied`). The ticket text was read as data. Each claim below was checked against the spec
and the merged code in `qsl-analyze/src/monitor.rs`, `qsl-replay/src/spine/clause/temporal.rs`
(`run_package_temporal`, `run_temporal`, `authenticate_binding`, `atom_name`),
`qsl-replay/src/spine/clause/trace_subject.rs` and `qsl-package/src/emit.rs`.

Checks that passed:

- The concrete vectors hold:
  - FR-283-AC-11's first clause is FR-327-AC-7's `B` alone under `always[0,2]` `on origin`. `B` is
    short of data at 0 and true at 1 and 2, so the result is `InsufficientData{where: 0, count: 1,
    min_count: 2}`.
  - The second clause of AC-11 is FR-330-AC-17's `always[0,0] B'` verbatim.
  - In FR-283-AC-12, `always[1,1] (count over past[0, 9] min_count 1 >= 1)` `on origin` reads only
    position 1. Its window there is positions 0 and 1, so it holds 2 values. That gives
    `WindowLimitReached` (`max_window_values` 1, `value` 1, per FR-194 Behavior and FR-194-AC-3) and
    `Tested` with a limit of 2 (count 2 >= 1, min_count 1 met).
  - AC-12's `ConditionRefused` vector is FR-330-AC-7 over a `Document`.
  - FR-283-AC-13's lasso, `Absent`, `ghost` and FR-501-AC-7 vectors match FR-330-AC-3 and FR-501-AC-7.
    The code's admit order (profile fit at temporal.rs:605, then the bounded-over-lasso check at :682)
    gives `invalid-value`.
  - FR-283-AC-15 matches FR-501-AC-4 (Indeterminate `where` 0 `MissingValuation{1, p}`; `W` 3 gives
    `Pending`), FR-330-AC-11 (`mismatched_dimension` `Clock`) and FR-330-AC-9 (finite-window clause
    over `Finite`, `admit` `invalid-value`).
- The `Samples` route is implementable on a `CheckedPackage`. `run_trace_subject_clause` evaluates
  over `checked.package()` alone (trace_subject.rs:142 to 231) and does not read its emission. The only
  changes it needs are the caller-supplied `max_window_values`, in place of `DEFAULT_MAX_WINDOW_VALUES`
  at :634, and the caller's sources, which the new FR-330 bullet states.
- Consistency with FR-286-AC-13 holds. It names the FR-283-AC-11 insufficient-data run by id, and
  TC-765 step 11 renders steps 9 and 10.
- TC coverage is complete:
  - Every new or changed AC has a TC row and appears on the TC scope line: AC-11 and AC-15 in TC-765
    (steps 9 and 10), AC-12 and AC-13 in TC-766 (steps 6 and 7).
  - AC-14 is TC-765 step 11.
- QSL-1639 (the decisive position against the activation position) is resolved in the current main
  text. FR-327 Behavior names the activation's own position and gives the failing atom's position to
  `decisive`. FR-330 ("The report's violation position stays the failing activation's position
  (FR-327); `decisive.position` may be later or earlier than it") agrees, and so do FR-328-AC-1 and
  FR-283's Violated bullet and AC-1/AC-10. Only ADR-011's gate table still calls it open (FND-007).

Main gap: the `Typed` `Progressed`/`Timestamped` path cannot authenticate its binding from what
`monitor` receives (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The `Typed` `Progressed`/`Timestamped` route is not implementable from FR-283's inputs. FR-283 Behavior has `monitor` authenticate the trace's `binding` by FR-522 against the `profile_selections`/`temporal_bindings` rows of "`package`'s emission (FR-515's evidence package built from the `Emission`)". Those rows hold the clock configuration of an explicit `TemporalClockSelection` given at emission. `emit_checked*` of a package whose finite-window or progressed clause has no selection refuses `MissingTemporalClockSelection` (`missing_declaration`/`missing-selection`, qsl-package/src/emit.rs:7506). The FR-515 strict read also needs `admission_evidence(models, checked, emitted, ...)` (qsl-replay/src/spine/bind.rs:60), which takes the front end's `AdmittedModels` and `CheckedUnit`, not a `CheckedPackage`. `MonitorRequest` carries no selections, no emission and no evidence. Today `run_package_temporal` passes `authentication: None` (temporal.rs:521), so this path faults. As written, FR-283-AC-15 and AC-13's `Timestamped` `TraceRefused` vector have no source for the rows. | FR-283 Inputs, Behavior; FR-283-AC-13, AC-15; FR-522 Authentication; FR-515; emit.rs:4395, 7506; temporal.rs:499-524, 578-602; clause.rs:1148-1228 |
| FND-002 | medium | The QSL-1793 rule "When a record is `Undefined`, `monitor` shall name its undefined atom by the source text" targets a record that has no atom. The `Undefined` row's members are `position`, `counterexample` and `cause` (`UndefinedEvaluation{where, cause}`). `TemporalVerdict::Undefined{at, cause}` has no atom for a kernel or family cause. In the code, atoms are named by source text only in the finite-window reasons (`atom_name`, temporal.rs:1531, falling back to `node@N` because `run_package_temporal` passes `sources: &[]`). `InsufficientData`'s `atom` is a bare formula index. So the last sentence of FR-283-AC-12 (AC-5's record names `10 / x > 0`) and TC-766 step 6 ("read the undefined atom's name") assert a member that no row defines. The `SourceNotSupplied` trigger "or an undefined atom" is undefined for the same reason. | FR-283 Outputs table (Undefined, InsufficientData, Indeterminate rows), Behavior; FR-283-AC-12, AC-15; TC-766 step 6; temporal.rs:1506, 1531-1550 |
| FND-003 | medium | `SourceNotSupplied` departs from FR-100 without saying so, and the case of a reference that matches but whose bytes do not is unstated. FR-283 renders the locus "as FR-100's `locus`", but FR-100 makes a locus naming no supplied source an internal failure: stage `spine-run`, invariant `locus-source-supplied`, exit 30. FR-283 makes it a refusal with exit 20. That is defensible because `sources` is caller input here, but FR-283 does not say it overrides FR-100. Separately, `RawSourceRef` is `{authority, identity}` only, with no content identity. A caller can therefore supply a `Source` whose reference matches but whose bytes do not contain the region's span (out of range, or not on a UTF-8 boundary). `atom_name` then falls back to `node@N`, which the new "never by a formula node index" rule forbids, and FR-283 states no outcome for that input. | FR-283 MonitorRefusal, Behavior; FR-100 (locus bullet, "Internal failure at S6a"); qsl-foundation provenance.rs:61; temporal.rs:1546-1549 |
| FND-004 | medium | The order of FR-522 binding authentication and position admission is unstated for a `Typed` trace. One Behavior bullet admits "every position of a `Document` or `Typed` trace ... before it evaluates any clause". Another authenticates a `Progressed`/`Timestamped` `binding` "before it visits any position". FR-330-AC-11 places authentication before any position is visited. FR-283 does not say which refusal a trace with both a mismatching binding and an inadmissible position reports: `ClauseRefused` or `Admission`. The `Admission { position }` index of a `Timestamped` trace is also not stated as its listing index or its FR-501 canonical-order position. | FR-283 Behavior bullets 2, 3 and 6; MonitorRefusal::Admission; FR-330-AC-11; FR-501 "Canonical order"; temporal.rs:637-654 |
| FND-005 | medium | `monitor`'s own `over` and subject-shape refusals are credited to FR-330, and some cases have no rule. The `ClauseRefused` bullet lists "an `over` present for a `Samples` run" (`invalid-value`) and "an `over` ... absent for a clause over a model subject" (`wrong-role-mapping`). Both are `monitor`'s own checks. FR-330's trace-subject route takes no `over`, and FR-330's `Temporal` selection's `over` is not optional (`OverObject`, temporal.rs:392). The only Behavior bullet that yields `ClauseRefused` is "If FR-330 refuses a selected clause at stage `admit`". Two cases are unstated: a record-trace-subject clause selected over a `Document`/`Typed` trace, and the stage or order of the `over` checks relative to FR-330's. | FR-283 Inputs (`clauses`), MonitorRefusal::ClauseRefused, Behavior; FR-283-AC-11, AC-13; FR-330 trace-subject route |
| FND-006 | low | Precedence across clauses is stated for refusals only. "Report, of the clauses that refuse the request, the first in request order" does not say what happens when one clause faults (`StageFailure::Fault`) and another refuses, or when a `SourceNotSupplied` (found after evaluation) and a `ClauseRefused` (found at admit) fall on different clauses. The code returns at the first clause in request order whose run refuses or faults (monitor.rs:221-237). | FR-283 Behavior; Outputs "internal failure" bullet; monitor.rs:221-237 |
| FND-007 | low | Text elsewhere was left stale by this change. ADR-029 OP-3 Input still names only "the bytes of a `quire.monitor-trace/1` trace document", with no `Typed`/`Samples` shapes and no `sources`. OP-3 Output still lists "the QSpec FR-351 separating witness", which contradicts FR-283's "no `witness`". FR-275's operation table says "a trace document". ADR-011's gate table (line 1318) lists QSL-1639 as open, but it is resolved (Summary). | ADR-029 OP-3; FR-275 line 47; ADR-011 line 1318 |
| FND-008 | low | Clause names have inconsistent types. `MonitorClause.name` changed from `QualifiedName` to `String`, but `MonitorRecord.clause` in Outputs is still `QualifiedName`. The merged code uses `String` for both (monitor.rs:35, 96). | FR-283 Inputs, Outputs; monitor.rs:35, 96 |
| FND-009 | low | EARS shape and atomicity. (a) The empty-loop bullet appends a declarative sentence that carries a second requirement ("A `Typed` `Lasso` with an empty loop is ... a `TraceRefused` record"). (b) The Violated bullet is limited to `Document`/`Typed`, and the `Samples` violation (`decisive`, no `counterexample`) is stated only in Outputs prose, with no Behavior bullet. (c) The "first in request order" bullet is a precedence clause with no trigger. (d) AC-15 adds a unit ("0, 2, 5 ms") that FR-501-AC-4 does not have and that no binding in the AC fixes. | FR-283 Behavior; FR-283-AC-15; FR-501-AC-4 |

Recommended fixes:

- **FND-001:** Give `MonitorRequest` the authentication source. Add
  `temporal_selections: &'a [TemporalClockSelection]`, from which `monitor` emits with
  `emit_checked_with_temporal_selections` and builds FR-515's evidence from `packages`, or take the
  caller's `&Emission` / strictly admitted package. State the refusal when emission refuses, for
  example `ClauseRefused` with `missing_declaration`/`missing-selection` for a missing selection. Add
  that input to AC-15 and to AC-13's `Timestamped` vector.
- **FND-002:** Retarget the rule to the records that carry an atom: the `Indeterminate` reason's atom
  (FR-501 `MissingValuation`/`ConflictingValuation`) and `InsufficientData`'s `atom`. Add `atom` to
  those rows' Members, named by the condition's source text. Replace AC-12's AC-5 sentence with AC-15's
  `atom: p`, checked with `sources` empty (giving `SourceNotSupplied`) and with `sources` present.
  Alternatively, add an `atom` member to the `Undefined` row and to `TemporalVerdict::Undefined`, and
  say so.
- **FND-003:** Add a sentence saying that for `monitor` an unmatched region reference is
  `SourceNotSupplied`, a refusal, not FR-100's `locus-source-supplied` fault, because `sources` is
  caller input. Add an If/then bullet: a matched source whose bytes do not contain the region's span
  refuses the whole request (for example `SourceNotSupplied`, or `invalid_runtime_input`/`invalid-value`
  naming `sources`), never `node@N`. Add one AC vector for it.
- **FND-004:** State that for a `Progressed`/`Timestamped` trace the FR-522 authentication of every
  selected clause comes before position admission, as FR-330-AC-11 orders it. State that
  `Admission.position` of a `Timestamped` trace is its FR-501 canonical-order position. Add a vector
  with both a mismatching binding and an inadmissible position.
- **FND-005:** Add one Behavior bullet: if a selected clause's `over` is present for a `Samples` run,
  absent for a model-subject clause, or the clause's subject does not fit the trace shape (a record
  subject over `Document`/`Typed`, a model subject over `Samples`), then `monitor` refuses the whole
  request with `ClauseRefused` and the stated code, before FR-330 admits the clause. Add the
  record-subject-over-`Document` vector to AC-13.
- **FND-006:** Say "the first clause in request order whose run refuses or faults decides the
  request's failure", matching monitor.rs. Optionally add a Fault-then-refusal vector to AC-13.
- **FND-007:** Update ADR-029 OP-3 Input to the three shapes and `sources`, and drop the FR-351 witness
  from its Output. Update FR-275's `monitor` row to "a `MonitorTrace`". Mark QSL-1639 resolved in
  ADR-011's gate table.
- **FND-008:** Make `MonitorRecord.clause` a `String`.
- **FND-009:** (a) Split the empty-loop bullet into "If a `Typed` `Lasso`'s loop is empty, then
  `monitor` shall report a `TraceRefused` record". (b) Add "When `monitor` reports `Violated` over a
  `Samples` trace, it shall attach FR-330's trace-subject `decisive` and no `counterexample`".
  (c) Phrase the precedence bullet with an If-trigger ("If more than one selected clause refuses ...").
  (d) Drop "ms" or tie it to the binding's `timestamp_unit`.

## Verdict

Not ready to merge. FND-001 must be fixed: AC-15 and part of AC-13 cannot be built from FR-283's
inputs. FND-002 to FND-005 should be fixed in the same branch. FND-006 to FND-009 are small text fixes.
