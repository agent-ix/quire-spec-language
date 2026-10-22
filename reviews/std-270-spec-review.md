---
id: SR-15700
title: "spec review of STD-270 temporal clause-run wires and refused holds condition (QSL PR #1240)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@67251f09b21c32b88e4cc68cddecc75549c7ebb7; spec/functional/FR-267-write-run-results-as-native-run-result-2.md, spec/functional/FR-312-read-a-clause-run-request-and-run-it.md, spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md, spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-312
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1160
    type: reviews
---
# SR-15700: spec review of STD-270 temporal clause-run wires and refused holds condition (QSL PR #1240)

## Summary

Ticket: STD-270 (unblocks QSL-902). Diff `origin/main...67251f09b` (head confirmed with `git ls-remote`). Five spec files: FR-312 gains a `temporal` selection, FR-267 gains the "Temporal disposition wire" section, cross-check rows and AC-13/AC-14, FR-327 gains "Refused condition" and AC-6, FR-330 gains a refused-condition behaviour and AC-7, and TC-1160 is new. QSL main moved to 43f4e2086 (STD-301, #1236) after the branch was cut, so main is not an ancestor of the head. `git merge-tree` shows a clean merge. `QSPEC_DIR=~/dev/quire-specification make check-spec-validation` exits 0 with no diagnostics on the touched files.

The reviewer checked the claims against QSL `origin/main` 43f4e2086 and QSpec `origin/main` 5ee0071:

- The nine kinds are complete. They match `qsl-eval` `TemporalOutcome` (Holds, Violated, Undefined, Pending, MissingFairnessPremise, Incomplete, WindowLimitReached), with Undefined split on `UndefinedCause::InsufficientData` exactly as `TemporalVerdict::category` splits it (`undefined` against `violation`). Condition-refused is the ninth kind, matching `HoldsStop::Refused`. The member names `count`, `min_count`, `limit`, `value` and `setting` match `TemporalVerdict` in `qsl-replay/src/spine/clause.rs:382-422`.
- The counterexample shape agrees with QSpec FR-364's observed arm: `initial_state` and `over_binding` are null, steps are `{document}`, the undefined envelope's `where` carries `position`, `document` and `locus`, `interval` is null for the envelope, and `trace_position` is the prefix's last position. FR-267-AC-13's vector agrees with FR-331-AC-1.
- FR-312's `over` and `trace` match the in-process `ClauseRunSelection::Temporal{name, trace, over: OverObject}` and `TraceInput::{Finite, Lasso}`. No `Model` variant exists in code, and `over` is not optional, so lead default 3 is grounded. An empty `finite` array does reach `TemporalTraceRefusal::EmptyTrace` at stage `evaluate` before `over` is resolved (`temporal.rs` checks it first), so lead default 5 is grounded.
- `integer_out_of_domain` has category `refusal` (`qsl-foundation/src/diagnostic.rs:1015`), so FR-330-AC-7's exit 20 is right.
- The QSpec precondition is real. QSpec TC-255 NR-07 refuses "an unknown disposition member", and FR-352 names `frame_witness` as the only extra disposition member. See FND-003 for what else that follow-up must cover.
- The ids TC-1160, FR-312-AC-7, FR-267-AC-13, FR-267-AC-14, FR-327-AC-6 and FR-330-AC-7 are unique. No other open PR (46 checked) adds them or touches FR-267, FR-312, FR-327 or FR-330.
- FR-400 to FR-403 move crates and keep type meaning. The kernel names this PR uses (`Outcome::Refused`, `FamilyResult::Refused`, `Refusal::CheckedInvariant`) are not renamed there, so nothing conflicts.
- No status wording and no compatibility layer.

Units examined: FR-267 "Temporal disposition wire" (statement, table, member bullets, reader bullet, round-trip bullet), the new cross-check rows, FR-267-AC-13 and AC-14; FR-312's temporal selection and reader bullet, and FR-312-AC-7; FR-327 "Refused condition" and FR-327-AC-6; FR-330's refused-condition bullet and FR-330-AC-7; TC-1160. Context only: FR-266 Behavior, FR-330 Outputs, FR-331, QSpec FR-243, FR-352, FR-364 and TC-255, and the code named above.

Lead defaults (none is an owner answer):

1. The verdict travels in `temporal`, not `witness`: sound, because FR-351 has no temporal form. Closed-scope as the basis is not settled (FND-006).
2. The full observed counterexample on violated and undefined: sound against FR-364. The report does not carry it yet (FND-001).
3. No `Model` on the wire, `over` required: grounded in code.
4. A refused condition is a typed refusal at the atom, and `CheckedInvariant` stays an internal failure: sound against FR-100. The evaluator's return for `CheckedInvariant` is unstated (FND-009).
5. No length check at the reader: grounded in code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: The writer cannot build the `counterexample` from what it is given. FR-267 writes from an FR-109 `ClauseRunReport`. FR-330 Outputs and the in-process `TemporalVerdict` hold only the position (and, for undefined, the cause). The counterexample needs each position's document digest, the clause's checked fairness set, the first interval, and, for undefined, the atom's locus. No requirement puts these in the report, so the round-trip "restore an equal `ClauseDisposition`" either loses the counterexample or rests on a disposition that no requirement defines. <br>Fix: amend FR-330 Outputs (and the violated and undefined mapping bullet) so the `Temporal` disposition carries the trace's `DocumentRef`s or the FR-364 counterexample, the checked fairness set, the interval and the undefined atom's locus. Then point FR-267 at that. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:192-210, spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:58-63 |
| FND-002 | high | major: FR-267-AC-14 cannot pass for the empty-loop refusal. A `TemporalRefusal` is written as `refusal` with "exactly its `code` and `cause`". EmptyTrace, EmptyLoop, TraceTooLong, TupleArity, Position and Window all share `invalid_runtime_input`/`invalid-value` (`TemporalTraceRefusal::catalog_code`), so a reader cannot restore an equal `ClauseDisposition::TemporalRefusal(EmptyLoop)` from those bytes. <br>Fix: write a discriminating member (the refusal variant, or FR-100 `fields`), or limit the round-trip for refusals to code, cause and category and say so in the bullet and the AC. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:211-215, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:223-224, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:384 |
| FND-003 | medium | major: The evaluate-stage members in the table depart from QSpec FR-352 and from FR-267's reader, and the stated QSpec follow-up does not cover it. FR-352 says an evaluate refusal, unsupported or incomplete result "retains FR-100's exact `outcome` vocabulary", and the reader refuses a `refusal` member at stage `evaluate` (`native_result.rs:1233-1243`). Yet `missing-fairness-premise` and `TemporalRefusal` use `refusal`, `window-limit-reached` (incomplete) and `insufficient-data` (undefined) have no `outcome`, and `pending` uses category `inconclusive`, which the reader refuses ("clause run has no inconclusive category") and which FR-352 and TC-255 do not list. <br>Fix: either use FR-100's `outcome` forms (refused for missing-fairness-premise and the trace refusals, incomplete for window-limit-reached if FR-100's limit vocabulary allows it, undefined for insufficient-data), or list each departure in the "Needs a QSpec change" precondition (FR-352's evaluate-outcome rule, the inconclusive category, TC-255 vectors). Name the Linear ticket for that QSpec change, and say that QSL-902 cannot pass QSpec conformance until it lands. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:186-190, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:211-215 |
| FND-004 | medium | major: The reader rule "a `temporal` on a disposition that is not a `Temporal` selection's evaluate disposition" cannot be decided from the document. A `/2` document does not record its selection. FR-267-AC-1's `AllBelow` over `low` document (`success`, `closed-scope`, no `witness`) with `temporal {"kind": "holds"}` added is a well-formed temporal holds document. AC-14's edit "adding `temporal` to FR-267-AC-1's `forall` document" refuses only for AC-1's decisive document, and then because of its top-level `witness`, not because of the rule under test. <br>Fix: state the reader rule as shape rules (a `temporal` only at stage `evaluate`, never with `witness`, `frame_witness` or `internal_fault`, with the row's category), and pick the AC-14 edit so it fails only on the rule it checks. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:216-222, spec/functional/FR-267-write-run-results-as-native-run-result-2.md:384 |
| FND-005 | medium | minor: The opening statement says every stage `evaluate` disposition of a `Temporal` selection carries `temporal`. The `TemporalRefusal` bullet then writes an evaluate disposition without it. Internal failures (`TemporalRefusal::Fault`, `EvaluateFault`, and a refused condition with `CheckedInvariant`) are evaluate dispositions with no row. <br>Fix: say that `temporal` is carried by an evaluate disposition whose result is an FR-327 outcome or a refused condition, and that trace refusals and internal failures carry none. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:174-179 |
| FND-006 | medium | major: The basis assignment sits where FR-266 does not see it, and it reverses tested code. FR-266 owns the basis per selection and has rows for Clause, Function and Frame only. The merged code sets `SettlementBasis::Unavailable` on every temporal report (`qsl-replay/src/spine/clause/temporal.rs:102`), and `a_clause_run_over_a_trace_settles_no_item` (`tests/temporal.rs:306-314`) asserts `unavailable` for holds and violated. Closed-scope does follow from QSpec FR-352 ("an evaluated `success` or `violation` carries ... `closed-scope`"), but FR-243 offers other labels a reviewer would expect to be weighed: `unsettled` (truth pending) for `Pending`, and the open-scope decisive labels for an FR-328 prefix verdict. <br>Fix: add a Temporal row to FR-266 that names the basis per kind and its rationale, and state that the existing test changes. Ask the owner to confirm closed-scope and `unavailable` for pending. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:183-187, spec/functional/FR-266-report-a-clause-run-s-witness-and-settlement-basis.md:52-58 |
| FND-007 | medium | minor: Two of the nine kinds have no AC. FR-267-AC-14 says "each FR-330 temporal report" but lists no `insufficient-data` or `window-limit-reached` vector. No AC covers an undefined with a family cause or an FR-328 finite-prefix violation (prefix only, empty loop). A writer that drops `count`, `min_count`, `limit` or `setting` passes every AC. <br>Fix: add FR-194 window vectors (a `min_count` shortfall, and `max_window_values` exceeded) to AC-14 and TC-1160 step 3. | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:384 |
| FND-008 | low | minor: FR-330-AC-7's refusing `holds` condition is ungrounded, but it can be grounded. FR-096-AC-8 (TC-428) shows a `Coerce` into `Int[0, 3]` refusing `integer_out_of_domain`/`outside-domain` with `expected` naming the domain. A coercion of `c.value` into `Int[0, 1]` (for example through a function parameter of that type) refuses at 2 and completes at 0. <br>Fix: name that construct in the AC, or have the TC author confirm it, before QSL-902 starts. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:115 |
| FND-009 | low | minor: FR-327 says `Refusal::CheckedInvariant` "is an internal failure, as FR-100 maps it", but not what the evaluator returns (the refused condition, which FR-330 then maps, or `TemporalRefusal::Fault`). FR-327-AC-6's third clause ("it is an internal failure") therefore has two readings at the evaluator level. <br>Fix: state the evaluator's return for `CheckedInvariant` and assert it in AC-6. | spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:130-135, spec/functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md:161 |
| FND-010 | low | minor: FR-312-AC-7 uses snapshot `s2` in its success case but introduces only `s0` and `s1`. TC-1160 lists `Fair` as a clause of TC-840's unit, but `Fair` is TC-848's (FR-330-AC-6). <br>Fix: add "and `s2` (2)" to AC-7, and cite TC-848 for `Fair`. | spec/functional/FR-312-read-a-clause-run-request-and-run-it.md:163, spec/test-cases/TC-1160-a-temporal-clause-run-travels-the-request-and-result-wires.md:28 |

## Verdict

Not mergeable as is. Major: FND-001 (the report does not carry what the counterexample needs), FND-002 (AC-14's refusal round-trip cannot pass), FND-003 (the evaluate vocabulary conflicts with QSpec FR-352 and the existing reader, and the QSpec precondition understates it), FND-004 (the reader rule cannot be decided from the document) and FND-006 (the basis is not owned in FR-266, and the change reverses tested code). Minor: FND-005 and FND-007 to FND-010. Separately from these findings, the branch needs main (43f4e2086) merged in; the merge is clean.

What holds: the nine kinds are complete and match the code, the counterexample members agree with FR-364 and FR-331, FR-312's selection matches the in-process type, the ids are unique, and FR-400 to FR-403 do not conflict.

Owner and planner decisions:

- The basis per temporal kind: closed-scope for holds, violated and undefined; `unavailable` or FR-243 `unsettled` for pending (FND-006).
- Whether the temporal rows follow FR-352's evaluate `outcome` vocabulary or QSpec changes to admit `refusal` at evaluate and the inconclusive category (FND-003); a Linear ticket for the QSpec FR-352/TC-255 follow-up.
- How a `TemporalRefusal` variant is told apart on the wire (FND-002).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-011 | low | minor: FR-330-AC-7 now names the clause twice: "a `Counter` clause `always[0,1] holds(...)` under event-position false-extension, `always[0,1] holds(InUnit(c.value))` with `function InUnit ...`". It reads as two clauses, one of them a placeholder. <br>Fix: drop the `holds(...)` placeholder and keep only `always[0,1] holds(InUnit(c.value))`. | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:128 |
| FND-012 | low | minor: FR-267-AC-15's window vectors cite FR-194-AC-3, a monitor vector. Its second case stops as the monitor's `Incomplete(LimitReached{…})`, not as FR-330's `WindowLimitReached`, so it does not show which temporal clause the run selects. The `absent-key` vector names no clause or atom. <br>Fix: name the TC-629 window clause that FR-330 runs for both cases, and give the `absent-key` case a concrete atom (a stub atom as in FR-327-AC-6, or a source lookup on a missing key). | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:399 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-013 | medium | minor: FR-267-AC-15's window-limit case runs the clause "through FR-330 ... with `max_window_values` 2", but no requirement gives the FR-330 entry that input. `ClauseRunRequest` (FR-109, FR-312) carries `accounting` and no window limit. The entry evaluates with the evaluator's built-in limit, `DEFAULT_MAX_WINDOW_VALUES` = 2^20 (`qsl-eval/src/temporal/window.rs:39`), and `run_temporal` passes no limit. As written, the case can only be set up by adding a request member that no spec defines. <br>Fix: build that case as a writer/reader vector from a report holding `WindowLimitReached{limit: max_window_values, value: 2, setting: monitor.max_window_values}` (the AC checks the wire, not the run), or specify where the run reads `max_window_values` (a `ClauseRunRequest` member with an FR-312 wire name). | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:399 |

## Dispositions

Round 1, reviewed at 62f5b4811323d20c2b034d9fcbc6ad6ba60a913e.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-330 Outputs: the violation disposition carries the `TemporalCounterexample` that the entry builds (document digests, fairness set, interval, the undefined atom's locus and document). FR-267: "The writer writes it as the report holds it and derives none of its members." |
| FND-002 | fixed | New kind `trace-refused` with a `variant` table that covers every spine `TemporalTraceRefusal` variant except `Fault`. AC-14 round-trips `EmptyLoop` and `EmptyTrace` as distinct values. |
| FND-003 | fixed | Every row uses FR-100 `outcome`, with no `refusal` at evaluate. The remaining QSpec departures (temporal member, inconclusive/unsettled, closed-scope undefined, insufficient-data reason, max_window_values limit, TC-255 vectors) are listed as a precondition gating QSL-902 conformance. The QSpec ticket is still to be filed by the lead. |
| FND-004 | fixed | The reader rules are shape rules (stage, co-occurrence, the row's category, truth, basis and outcome). The AC-14 edit adds `temporal` to the FR-109-AC-3 stage `admit` document. |
| FND-005 | fixed | The opening paragraph limits `temporal` to an FR-327 outcome, a refused condition or a trace refusal. Internal failures, `CheckedInvariant` included, carry `internal_fault`. |
| FND-006 | fixed | FR-266 Temporal bullet and FR-266-AC-4: closed-scope for holds, violated and undefined; unsettled for pending; unavailable otherwise. The code impact on `temporal.rs:102` and its test is named in the PR body. |
| FND-007 | fixed | FR-267-AC-15 adds insufficient-data, window-limit-reached, a family `absent-key` undefined and the FR-328 prefix violation, plus drop-member edits. Vector grounding is in FND-012. |
| FND-008 | fixed | FR-330-AC-7 is grounded with `InUnit(x: Int[0, 1])`, a Coerce refusing `integer_out_of_domain`/`outside-domain`, `expected` `Int[0, 1]` (FR-096-AC-8, TC-428). Wording is in FND-011. |
| FND-009 | fixed | FR-327: the evaluator returns `CheckedInvariant` as a refused condition, not `TemporalRefusal::Fault`, and FR-330 reports internal-failure. AC-6 asserts this. |
| FND-010 | fixed | FR-312-AC-7 introduces `s2` (2). TC-1160 cites TC-848 for `Fair`. |

Round 2, reviewed at 7cdb0c76b5fbec7a9c46451872118ce258ca8b7d.

| FND | outcome | reason |
| --- | --- | --- |
| FND-011 | fixed | FR-330-AC-7 names one clause: "the `Counter` clause `always[0,1] holds(InUnit(c.value))`". |
| FND-012 | fixed | FR-267-AC-15 names the clause `Windowed` run through FR-330 (TC-629's count form, `past[0, 9]`, `min_count 5` gives `insufficient-data` at position 0 with count 1) and the `absent-key` atom `lookup<Counters::Counter>(cs, c) absent undefined = c` (FR-090-AC-12). The window-limit input is FND-013. |
