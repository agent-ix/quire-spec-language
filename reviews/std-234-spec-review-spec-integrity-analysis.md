---
id: SR-14010
title: "integrity review of STD-234 event-rate stop, FR-252 timing clauses and FR-391 population reference emission (PR #1190)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@b8b3c38bd2e3ca0d6073db552ab5fc315911ddfc; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md, spec/functional/FR-252-hand-timing-obligations-to-code-generation.md, spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md, spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/test-cases/TC-706-tick-based-monitor-plans.md, spec/test-cases/TC-707-timing-obligations-handed-to-cg.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-252
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
---
# SR-14010: integrity review of STD-234 (PR #1190)

## Summary

Ticket: STD-234. Diff reviewed: QSL `git diff origin/main...b8b3c38` (one
commit, 7 files, +49/-25), based on `48b47ab` (main before #1182, #1183 and
#1184), not stacked. The brief named head `6b0d5cf`; that object does not
exist in the clone after fetch, and the live PR head is `b8b3c38`, which is
what this review covers. There is no QSpec PR for this ticket.

Merge with current main: `git merge-tree --write-tree origin/main b8b3c38`
(main `afffa69`, which holds #1183/STD-160) is clean. #1183 added the
`terminal when` predicate as a population position and FR-391-AC-9; the
PR's new S4 bullet covers every population term, so the two do not
contradict.

Items 1 to 3 verified on main:

- Item 1: QSpec FR-322 "Population references" (line 1290 on QSpec main
  `6f45264`) defines the `relation`/`population` node keyed by the
  declaration key alone, its use as operand 0 of `all_instances`/`lookup`,
  `N` entering through `result_type`, and the three-node quantifier domain;
  FR-322-AC-83 tests it (TC-233). QSL FR-391's new Lowering bullet matches
  each of these.
- Item 2: QSpec FR-272's row and native-diagnostics.md's
  `stage_limit_exceeded` row both list `rational-bits-exceeded`, checked by
  FR-272-AC-10.
- Item 3: QSL FR-192 line 102 and AC-1 name `meter` and `max_candidates`
  with no prefix; no `model_check.meter` or `model_check.max_candidates`
  appears in either repository's spec.

Item 4: `once[0 ms, 10 ms]` at 2 events per ms gives 2 x 10 + 1 = 21 by
FR-251 Buffers ("rate times the window length, rounded up, plus one"), so
`value` 21 and 22 events overflowing are correct. Item 5: `kani_events` no
longer appears in spec/ (only in an old review file). No code implements
FR-251/FR-252 yet (no `max_event_rate`, `event_buffer`,
`TimestampContract` in any `.rs`), so there is no implemented behaviour to
compare against. No other open QSL PR touches these seven files, and the new
id FR-252-AC-3 is unique. The PR body records two lead defaults and has no
review-finding paraphrase. No docs/ file or test-parsed vector table is
touched.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-252 defines a timing clause as a comparison whose left operand is `elapsed_wall_time(h)` or `wcet(f)`, but no QSL requirement admits these forms at S1 to S3: none says how `h` (a handler) and `f` (a function) resolve, what `elapsed_wall_time(h)` and `wcet(f)` type as, or which clause kinds may hold them. Neither name appears anywhere else in spec/ or in the code, so on today's checker `elapsed_wall_time(handler)` is a call to an undeclared name and refuses at name resolution before FR-252 could dispose it. FR-252-AC-2's dispositions and FR-252-AC-3's classification therefore cannot be reached from source text. Fix: add the resolution and typing rule for the two forms (in FR-252 or the checker FR it belongs to), or state that the forms are recognised before name resolution and refuse nothing there. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:75-83; spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:93 |
| FND-002 | low | FR-251 Overflow sets `limit` to `event_buffer` and `setting` to `monitor.max_event_rate`, but FR-127 types `LimitReached{limit, value, setting}` as `limit: ModelCheckLimit` (`MaxStates`, `MaxTransitions`, `MaxAutomatonStates`, `EvaluationMeter`, `MaxCandidates`, plus FR-184's) and `setting: RequestSetting`. `event_buffer` is not a variant of that enum, and the monitor target is not a request. FR-251 names neither the type `event_buffer` belongs to nor its wire spelling, so the coder must invent both. Fix: name the type and variant (for example a monitor limit kind) in FR-251, or widen FR-127's field types. | spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:181-185; spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:98 |
| FND-003 | low | The new FR-255 prose says the monitor target's event rate is "The one exception" to "takes a name by the same rule and a row in this table", but FR-255-AC-3 and FR-255-AC-4 also exempt FR-277's supplied-value ceilings from the table, so it is not the one exception. Related: FR-251's parenthetical "(FR-255 does not list it ...)" is now inaccurate, as FR-255 names it in prose and in AC-3; it is the setting table that does not list it. Fix: "Another exception ..." in FR-255 and "FR-255's setting table does not list it" in FR-251. | spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:106-109; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md:185 |
| FND-004 | low | FR-391's Description still says QSpec FR-153 and `shared-grammar.md` state that no population binding is ambient, that QSpec FR-383-AC-13 still writes `forall o in counters`, and that "QSpec needs a follow-up that states this name-to-binding rule". QSpec FR-153 (lines 148-156) now states the rule and names FR-322's population reference, and `forall o in counters` no longer appears in QSpec spec/. The PR fixes the stale Lowering bullet of this file but leaves this paragraph, which now contradicts QSpec. Fix: replace the paragraph with a reference to QSpec FR-153's rule. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:47-54 |
| FND-005 | low | ADR-026 MN-4 still says an overflowing event settles the obligations it affects "naming the event-rate limit". FR-251 now names `event_buffer` as the limit and the rate only as the setting, so MN-4 and FR-251 disagree on which limit is named. Fix: MN-4 names the event-buffer limit and the `monitor.max_event_rate` setting, or defers to FR-251. | spec/decisions/ADR-026-dense-time.md:410 |
| FND-006 | low | FR-391's new Lowering bullet is the only QSL statement of how S4 emits the population reference. FR-093's application table, which the S4 coder follows, still gives `AllInstances` (and `Lookup`) the argument shape `[ref(p)]` over a parameter, and FR-094, which keys model-owned population nodes, has no population reference node. The S4 owners disagree with FR-391 by omission. Fix: add the population reference operand to FR-093's `AllInstances`/`Lookup` rows and the node to FR-094, or have both point to FR-391. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:222; spec/functional/FR-391-name-a-population-in-a-state-clause.md:163-170 |
| FND-007 | low | FR-391-AC-6 and TC-1076 step 8 require that "a second read of `counters` references the first's node", but the AC's fixture has only one read of `counters` (`A`) and one of `archive` (`B`). The FR-155-AC-2 precondition the AC also names is not stated to be in the same emitted package. The input that exercises the shared node is left to the tester. Fix: name the second read, for example a third precondition over `M::counters` in the same unit. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:213; spec/test-cases/TC-1076-a-state-clause-names-a-population.md:30 |
| FND-008 | low | `timestamp_parameters: Set<Name>` is one unqualified name set for the whole request, so two contracts that both have a parameter `now`, only one of which is a timestamp, cannot be told apart. The trigger "clauses relate integer parameters that `timestamp_parameters` names" also does not say whether a clause that compares one named parameter with a constant or with an unnamed parameter qualifies. Fix: key the set by contract (or by qualified parameter), and state whether one named parameter is enough. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:38-45; spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:71-74 |
| FND-009 | low | FR-252 still disposes "a timing obligation that reads ... preemption or interrupt timing" as `unsupported`, but the new timing-clause definition covers only `elapsed_wall_time` and `wcet`. No form or AC says what reads preemption or interrupt timing, so those two kinds remain unrecognisable. The ticket asked only for the elapsed-time and WCET forms. Fix: define the two remaining forms or drop them from the disposition rule. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:78-83 |

## Verdict

Items 1 to 3 are already on main, as claimed. The event-rate stop (item 4) is
consistent and its numbers check, and `kani_events` is gone. FR-391's new
emission bullet agrees with QSpec FR-322. The PR merges cleanly with
current main, including #1183. One medium finding is open: FND-001, the
timing-clause forms that no checker requirement admits. The rest are lows.
Gates at head: `make check-spec-validation` exits 0 and
`tools/check-index-completeness.sh` passes. Over the seven touched files,
`quire validate` adds two warnings at FR-252 line 79 that main does not have
(SR-14011 FND-001). quoin 0.28.3; quire 0.36.2 (engine 0.50.2), newer than
the 0.36.1 the rollout note names, with no behaviour difference observed.
QSpec gates were not run because this ticket has no QSpec PR.

## Dispositions

Disposition pass 1 at `agent-ix/quire-spec-language@655c3660668f4835ba281a90d223412bb1c0784b` (fix commit `655c3660`, on top of `9f940889`, a merge of origin/main into the branch; the head contains origin/main `b5190738`). I checked each outcome against the spec text at that head, not against the commit message. Each `fixed` row quotes the fixed text.

| FND | Outcome | Reason or after excerpt |
| --- | --- | --- |
| FND-001 | fixed 655c3660 | FR-252: "QSL SHALL recognise `elapsed_wall_time(h)` and `wcet(f)` in a timing clause before name resolution: it takes `h` and `f` as written, resolves neither against the unit's declarations, types the left operand as a time quantity, and refuses nothing at S1 to S3 for either form. These two forms are admitted only as the left operand of a timing clause." FR-252-AC-2 adds: "`elapsed_wall_time(handler)` with `handler` declared nowhere is still disposed `unsupported` and not refused at name resolution." New FND-010 and FND-011 cover the S3 owner and two edge cases. |
| FND-002 | fixed 655c3660 | FR-251: "with `limit` the monitor limit kind `EventBuffer`, written `event_buffer`, ... and `setting` the monitor setting `monitor.max_event_rate` ... `EventBuffer` and `monitor.max_event_rate` extend FR-127's `limit` and `setting` fields for a monitor run, which has no model-check request." FR-127: "A monitor run (FR-251) writes the same record with limit `event_buffer` and setting `monitor.max_event_rate`." |
| FND-003 | fixed 655c3660 | FR-255: "Another exception, besides the supplied-value ceilings of FR-277, is the monitor target's event rate". FR-251: "(FR-255's setting table does not list it, as it is an exact fraction)". |
| FND-004 | fixed 655c3660 | FR-391: "The meaning is QSL's own, and it agrees with QSpec FR-153: the name selects the population declaration and the binding is not ambient. Here the binding of `M::P` comes from the admitted observation (FR-106), and in a checked package the name is the population reference of QSpec FR-322 (\"Population references\")." QSpec FR-153 lines 148-155 on QSpec main say the same. |
| FND-005 | fixed 655c3660 | ADR-026 MN-4: "settle `Incomplete(LimitReached{limit, value, setting})` naming the event-buffer limit and the `monitor.max_event_rate` setting that raises it (FR-251), V-7." |
| FND-006 | fixed 655c3660 | FR-093 `AllInstances` row: "`[ref(p)]`, `p` a population parameter or, for a named population `M::P`, the population reference of FR-391"; `Lookup` row: "`[ref(p), ref(r)]`, `p` as for `AllInstances`". FR-094: "Its reference is a separate node keyed by the population's declaration key alone, with no `N` and no owner ... QSpec FR-322 (\"Population references\") defines that node, and FR-391 states how `check` emits it." |
| FND-007 | fixed 655c3660 | FR-391-AC-6: "a third precondition `pre C using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` in the same unit reads `counters` a second time; the precondition of FR-155-AC-2 over `counters`, in the same unit and emitted package, ... so `A` and `B` reference two nodes and `C` references the same node as `A`". TC-1076 step 8 adds `pre C`. |
| FND-008 | fixed 655c3660 | FR-252: "`timestamp_parameters: Map<ClauseId, Set<Name>>` ... names, per checked contract, the integer parameters of that contract that carry timestamps; a name is read only within the contract that keys it"; "When a clause of a contract relates two or more of the integer parameters that `timestamp_parameters` names for that contract ... including a contract whose clauses compare one named parameter only with a constant or with an unnamed parameter." FR-252-AC-3 tests the one-parameter case. |
| FND-009 | fixed 655c3660 | FR-252: "An obligation about elapsed real time or WCET is outside Kani's model, so QSL writes it as unsupported ... QSL has no authored form for a claim about preemption or interrupt timing, so it writes no obligation for one." The disposition bullet now covers timing clauses only. New FND-012 covers ADR-026 KG-4, which this change leaves behind. |

## New findings (disposition pass 1)

These were found at `655c3660`. They are new, not dispositions of the rows above.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-010 | low | FR-252 now says `elapsed_wall_time(h)` and `wcet(f)` are recognised before name resolution and not refused, but FR-065, which owns S3 call checking, still says `Application` SHALL refuse a call whose name names no function, type or operation as `missing-name`, for "every `Expression::Call` it reaches". Neither FR-065 nor FR-091 (value forms) names the two forms or the node they become, so the S3 coder reads FR-065-AC-4's `nowhere()` rule and refuses `elapsed_wall_time(handler)`. Fix: name the two forms as an exception in FR-065 (or the value form they produce in FR-091), or have FR-065 point to FR-252. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:83-87; spec/functional/FR-065-migrate-function-application-to-checked-family.md:116-117 |
| FND-011 | low | The recognition rule leaves two edges open. A unit that declares its own function `wcet` or `elapsed_wall_time` gets its call taken as a timing clause, because the rule resolves nothing; FR-252 does not say whether the user declaration or the timing form wins. And "admitted only as the left operand of a timing clause" does not say what happens to the forms anywhere else (right operand, a non-time right operand, a nested expression): refused, and with which code, or resolved as ordinary calls. Fix: state which wins over a user declaration of the same name, and the refusal (code and site) for the forms outside the left-operand position. | spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:79-87 |
| FND-012 | low | ADR-026 KG-4 still says "Elapsed real time, WCET, preemption and interrupt timing are outside Kani's model, so QSL disposes an obligation of that kind `unsupported`". FR-252 now says QSL has no authored form for preemption or interrupt timing and writes no obligation for one. KG-4 promises an `unsupported` disposition for two kinds that FR-252 says cannot occur. Fix: KG-4 limits the `unsupported` disposition to elapsed real time and WCET and says preemption and interrupt timing have no authored form, or defers to FR-252. | spec/decisions/ADR-026-dense-time.md:417; spec/functional/FR-252-hand-timing-obligations-to-code-generation.md:22-25 |

Gates at `655c3660`: `make check-spec-validation` exits 0 with no warning in a touched file, and `tools/check-index-completeness.sh` passes. No docs/ file or test-parsed vector table is touched. The fix adds no new ids. FR-252-AC-3 is still the only id the PR adds, and it is unique. No numeric vector changed: FR-251-AC-2's capacity is still 2 x 10 + 1 = 21. Open PRs #723 (FR-127) and #706 (FR-094) touch two files the fix edits. Both are stale since 2026-10-11 and add no colliding id. Greps of spec/ outside reviews/ for `kani_events`, `timestamp_parameters`, `event-rate limit`, `preemption` and `interrupt timing` find only FND-012 outside the touched files. The PR body has no review-finding paraphrase. Its "Ids touched" line does not list the files this fix commit added (FR-093, FR-094, FR-127, ADR-026, TC-720). This is a body nit, not a spec finding. The PR's committed `reviews/std-234-*.md` match the pre-disposition scratchpad copies, so they need replacing with these updated files.
