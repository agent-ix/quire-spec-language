---
id: SR-12884
title: "Base spec review of STD-166 relation and hyper NotEmitted wording (PR #980)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; PR #980 (git diff origin/main...HEAD at d5be6676f); spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md, spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md, spec/functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md, spec/decisions/ADR-033-one-source-text-for-the-checked-graph-and-the-type-report.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-362
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1026
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-033
    type: reviews
---

## Summary

Ticket: STD-166. Base checklist review of the four edited artifacts, judged
against the behaviour of PR #971 (QSL-550, branch qsl-550-e14fe-hyper, held,
not merged) as the intended contract. Examined: FR-362 item 8, FR-362-AC-7,
TC-1026 step 5 and its expected result, FR-171 Description and Use case,
ADR-033 "Moved and unmoved families" bullet. Context only: FR-362 items 3, 9,
10 and AC-6, FR-172 Description and AC-2/AC-3, FR-173-AC-3, and the PR #971
`HyperCause` catalog codes (`qsl-semantics/src/check/hyper_clause.rs`),
`not_emitted`, `check_behaviours` and the deleted
`qsl-forms/tests/it/source_grammar_vectors.rs` step-5 test.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | FR-362-AC-7's sentence "No `relation` or `hyper` clause is refused `unsupported_construct` or `UnrepresentedConstruct`" is false under PR #971 and contradicts FR-172-AC-2, FR-172-AC-3 and FR-173-AC-3. In #971, `HyperCause::code()` maps QuantifierWithoutAlias, QuantifierWithAlias, FiniteDomain, StepLabelInBody, MatchReadsState, AlignSkipEmpty/Unbound/Repeated/Fairness, SingleExistentialMatch, ExecutionNotAnOperation, BodyUnsupported and OutsideHyperClause to `Code::UnsupportedConstruct`, and FR-172/FR-173 require exactly those refusals. Failure: a TC-1026 implementation whose fourth read uses a step label in the body, or any FR-172-AC-2 test, gets `unsupported_construct` on a relation/hyper clause, so either AC-7 or FR-172 fails. The intent is "not refused as an unbuilt family at S2"; say that (no `FormsCause::NoDispatchEntry` or `UnrepresentedConstruct` for the clause), not a ban on the catalog code. | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:101; spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md:30 | wrong-requirement |
| FND-002 | medium | Item 8 ("refused with its typed hyper cause instead") and AC-7 ("refused with its typed hyper cause") misdescribe #971 for `hyper` clauses. `check_behaviours` pushes the body's operator and interval faults (`checked_operators` into `temporal_causes`, mapped to `Found::Temporal`), fairness resolution faults and `ProfileSelection` as `CheckCause::Temporal`, not `CheckCause::Hyper`. Failure: a hyper clause with a bad interval operator is refused with a TemporalCause that is not `NotEmitted`; an AC-7 test asserting `CheckCause::Hyper` fails on correct code, and a reader concludes every temporal cause on a hyper clause means "checked". Say "with the typed cause of its fault (a hyper cause, or a temporal cause for its temporal operators and profile), never `NotEmitted`". | spec/functional/FR-362-bind-a-checked-unit-to-its-source-by-digest.md:70-71,101 | wrong-requirement |
| FND-003 | low | TC-1026 step 5 leaves the fixture's preconditions and the fourth read's oracle open: a relation "whose body checks" needs a selected model with step operations and one temporal profile, a hyper clause needs domain `behaviours` with `of` aliases (the deleted #971 fixture used `over traces in M::Execution bounded 2`, which #971 refuses `FiniteDomain`), and "the typed hyper cause" names no cause or span, so any HyperCause passes. Name the body fault (for example `BodyMissing` at the read) and the model and profile lines. The oracle does fail if a relation is still refused `unsupported_construct` at S2. | spec/test-cases/TC-1026-one-source-yields-both-and-a-checked-unit-is-bound-by-digest.md:21,30 | wrong-requirement |

## Verdict

Not mergeable as written: FND-001 is a contradiction a test would hit, and
FND-002 misnames the cause family for hyper clauses. Both are wording fixes in
FR-362 item 8, AC-7 and TC-1026 step 5.

Checked and sound: the `NotEmitted` claim matches #971 (`not_emitted` returns
`CheckCause::Temporal(TemporalCause::NotEmitted { name, span })`, and the
#971 tests expect exactly one such refusal for a checked clause). "No graph and
no type report" for a NotEmitted unit is right: item 3 says the producer
returns the typed refusal of the first stage that refused, item 10 says a
refused unit is an error outcome of `check`, and a checked `temporal` clause
already behaves this way. Deleting the AC-7 retirement clause rather than
leaving a note is coherent: the clause told the moving PR to retire the test,
#971 deleted it, and the AC now states the moved behaviour; the history lives in
git and in #971's gap-analysis FND-005. The TC-1026 step 5 oracle would fail
if a relation were still refused `unsupported_construct` at S2 (first read must
be only `NotEmitted`). FR-171's Description and Use case edits are correct and
remove a "today" clause. Item 9 and AC-6 are in SR-12885.
