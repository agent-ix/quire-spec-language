---
id: SR-17190
title: "QSL-926 PR #1524 base spec review of the on each S3 check and S4 emission"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/functional/FR-338-check-an-en-1-closure-certificate.md, spec/functional/FR-339-check-an-en-1-component-certificate.md, spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md, spec/test-cases/TC-1265-an-on-each-clause-emits-its-trigger-and-guard.md, spec/test-cases/TC-520-model-behaviours-read-as-temporal-traces.md, spec/test-cases/TC-521-explicit-state-model-check.md, spec/test-cases/TC-525-an-en-1-closure-certificate-is-checked.md, spec/test-cases/TC-1201-smt-monitor-items-guarded-and-multi-instance.md, spec/test-cases/TC-1215-a-guarded-liveness-clause-s-component-certificate-is-checked.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1265
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524, head a8b073ce6 (b1394d084 plus the TC-1264 to TC-1265 renumber), diff `origin/main...HEAD`, merge-base 0d1a9e580. Spec-only. This is the base review. It checks consistency with merged FRs and whether QSL-1037 can be coded from the text against the real code.

Code read: qsl-semantics/src/check/temporal_clause.rs (`is_emitted`, `not_emitted`, `Activation::Each { guarded }`, `parameters`, `classify_domains`), temporal_clause/condition.rs, check/mod.rs (package check, `condition`, `named_populations`), lowering/temporal.rs and lowering/hybrid.rs (the hybrid trigger at level 1), qsl-eval/src/model_subject.rs (`ClauseLetters::activation` returns `Result<AtomReading, Incomplete>`, `AtomStop`), simulation/model/letters.rs (`ModelLetters::activation` is always `True`, empty `activation_reads`), qsl-replay model_check/replay.rs, qsl-analyze model_check/liveness.rs, and qsl-package checked_v2/clause_root.rs.

Probe: in a private copy, S3 was run on equivalent holds conditions over the ConfigVersion fixture. `c.versionNumber = c.versionNumber + 1` refuses `Unproved(Range{required [0,1000], proved [1,1001]})`. `on each (q: Config::Absent)` refuses `ParameterType`/`MissingQualifiedName`. `on each (q: Integer)` and `on each (c: ...)` get past S3 to `NotEmitted`, so S3 puts no constraint on `T` or on the binder's name today.

Review set: base plus integrity, ears-conformance, dependency, evidence, failure-domain and gap-analysis. criterion-strength was skipped because no Jev client is installed. AC strength was judged by reading, under integrity (SR-17191).

Examined units: FR-337 (statement, Inputs, "`on each` activation at S3", "`on each` activation at S4", "Clauses S4 does not emit"), FR-337-AC-6 and FR-337-AC-7, FR-125 "The `on each` activation guard", FR-125-AC-18, FR-126 (the certificate paragraph), FR-126-AC-16, FR-338-AC-10, FR-339-AC-5, FR-362 item 8, ADR-033, TC-1265, TC-520 step 17, TC-521 step 9, TC-525 step 9, TC-1201 and TC-1215, all examined. Read for context only: FR-338 "Activation" keys (`each`, `each-when`, `each-buchi`, `infinite`), FR-383-AC-2, FR-447 (activation, ActivationVariable, FR-447-AC-9), FR-381 (scope table, FR-381-AC-4, FR-381-AC-10), FR-104-AC-12, FR-125-AC-6, FR-125-AC-9, FR-125-AC-12, FR-218-AC-16, TC-1056 row 23a, FR-281's settlement table, and QSpec FR-370 member 3.

These are consistent with merged text: the guard-less vs `when (true)` keys (`each`/`each-when`) match FR-338 as merged in #1516/#1519 and FR-383-AC-2's protocol analogue; the level-1 trigger matches FR-447 and lowering/hybrid.rs; undefined-reading handling matches FR-104, FR-381 and FR-125-AC-6/AC-12; and QSpec FR-370's `trigger`/`guard` members, which do not fix the level. Ids: TC-1265 is unused on main and in every open PR (#1523 holds TC-1264). `make check-spec-validation` passes with no warning on a changed file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Contradiction with merged FR-218-AC-16 and TC-1056 row 23a. These require a clause "with an `on each` activation and a capture" to check, with S4 minting its node key. FR-337 "Clauses S4 does not emit" and the new FR-337-AC-7 state that such a clause still refuses `NotEmitted` with no package. QSL-926's own title and description also cover captures ("on-each activation or a capture"), so the PR narrows the ticket without naming a follow-up. Fix one of two ways: amend FR-218-AC-16 and row 23a to an `on each` clause with no capture and name the capture-emission ticket, or keep the AC and file that ticket and cite it in FR-337 "Clauses S4 does not emit". | FR-337:220-229, FR-337-AC-7, FR-218-AC-16, TC-1056 row 23a |
| FND-002 | medium | Q1. The trigger binder `q` has no value, the guard may not read it, and "S3 adds no rule relating `T` to `U`". This reverses the merged `on each` forms. FR-447's hybrid guard reads `q` (`ActivationVariable`), and `on each (q: Braking)` over `(h: Braking2)` refuses `ill_typed`/`type-mismatch` (FR-447-AC-9). FR-381's protocol guard reads only its binder. With `T` free, `on each (q: Integer)` checks. Because `parameters(form)` feeds the `on each` type into `classify_domains`, an unbounded `T` also joins the claim's `parameter_domains`, so a binder with no value changes the claim's extent. Ruling: require `T` = `U` (refuse `ill_typed`/`type-mismatch` at `T`, as FR-447-AC-9) and bind `q` as an alias of the instance's object that the guard may read. A read of `q` then lowers to a reference to the level-1 trigger node, as FR-447 lowers `ActivationVariable`. FR-337-AC-7's `q.value = 1` case becomes a check case, and an unrelated `T` becomes the refusal. | FR-337:140-160, FR-337-AC-7, FR-447-AC-9, qsl-semantics/src/check/temporal_clause.rs:820-826,921-926 |
| FND-003 | low | FR-337's S4 bullets give the trigger `value`/`parameter` node's name, level and type, but not its occurrence. The `over` node records `TemporalSite::Over`, and the hybrid trigger records the claim's location, so two implementations could record different occurrences and give different package bytes. State the occurrence: for example the binder name's span with `OccurrenceRole` as for the `over` parameter. | FR-337:197-202, qsl-semantics/src/check/lowering/temporal.rs:49-61 |

## Verdict

Changes requested. FND-001 is a contradiction with merged FR-218-AC-16. Fix it, and settle FND-002 (Q1/Q4, see SR-17195 FND-001), before QSL-1037 is coded. FND-003 can be fixed with them.

Implementability: apart from these findings, QSL-1037 can be coded from the text. S3 types the guard in the same `Typer`/`Definedness` pass `check_conditions` uses, under a new `TemporalSite` for the guard, and keeps it beside `conditions` so `named_populations` covers it. `is_emitted` drops the `Origin` test. lowering/temporal.rs mirrors lowering/hybrid.rs:339-354 for `kind`/`trigger`/`guard`. `ModelLetters::activation` and `activation_reads` evaluate the kept guard as `atom`/`reads` do.

The FR-125 bullet "stop the check exactly as a `holds` atom's evaluation that gives no reading" needs `ClauseLetters::activation`'s error type widened from `Incomplete` to `AtomStop` (qsl-eval/src/model_subject.rs:178-183), with qsl-replay's `?` conversion following. That is a code change the text implies, not a spec defect.

EN-1 liveness still returns `ModelCheckRefusal::GuardedActivation` for a guarded TP-4 clause (qsl-analyze/src/model_check/liveness.rs:176,283, also on main). QSL-1037's comment calls that refusal already gone. It is not, and FR-126-AC-16's TP-4 halves need it removed.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-337's new "Notes for the implementation (QSL-1037)" names `ModelLetters::activation` at `qsl-eval/src/model_subject.rs` lines 178-183. That location is the `ClauseLetters::activation` trait method. `ModelLetters`' implementation is at `qsl-eval/src/simulation/model/letters.rs:271-278`. Name the trait method, which is what widens to `AtomStop`, or both. | FR-337:264-267 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-218-AC-16 and TC-1056 row 23a keep the capture case as the goal and mark it satisfiable once S4 emits captures. FR-337 "Clauses S4 does not emit" states the interim refusal and names QSL-1081, which exists in Backlog. FR-337-AC-7 cites QSL-1081. |
| FND-002 | fixed | FR-337 S3 now refuses `T` != `U` with `ill_typed`/`type-mismatch` at `T` and binds `q` at slot 1 as an alias of type `U` that the guard alone may read. A read of `q` lowers to the level-1 trigger node (FR-447 `ActivationVariable`). FR-337-AC-7 and TC-1265 step 3 have `q.value = 1` checking and `on each (q: Integer)` refusing. FR-125's seam binds slot 1 to the same reference. |
| FND-003 | fixed | FR-337 S4 now records the trigger node's `expression` occurrence and its type node's `type` occurrence at the binder's name, as for `over`. FR-337-AC-6 and TC-1265 step 1 check it. |
