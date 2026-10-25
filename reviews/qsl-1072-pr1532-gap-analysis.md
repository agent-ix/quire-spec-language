---
id: SR-17211
title: "QSL-1072 PR #1532 gap analysis: FR-487, FR-172-AC-5/7/8 and FR-315-AC-43 matrix rows"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@95a7da3c2; git diff origin/main...HEAD (PR #1532, branch qsl-1072-lowering-code); spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md (AC-5, AC-7, AC-8), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md (AC-43), spec/test-cases/TC-1264-hyper-atoms-and-match-conjuncts-are-lowered-and-evaluated.md"
review_set: subset
date: "2026-10-25"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1264
    type: references
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1532 at 95a7da3c2. This is a planless run
scoped to the criteria the PR delivers or owes. `quire matrix --scope .` at the head
reads `tagged` for FR-487-AC-1 to AC-6, AC-10 and AC-11, and for FR-172-AC-5 (TC-597),
FR-172-AC-7 and FR-172-AC-8. So SR-17186 FND-001's set is closed except for FR-487-AC-9.
FR-487-AC-7, AC-8, AC-12 and FR-315-AC-43 read `untagged`. They are deferred by tickets
(QSL-1083 for AC-7/8/43, blocked on QSL-1010; QSL-1084 for AC-12), and the PR's "Not
delivered" section says so, so they are not findings here. FR-487-AC-9 also reads
`untagged`, although only its evaluation half is deferred (QSL-1085).

Reverse gap: every new production item (`AtomSlot`, `atom_slots`/`condition`/
`component_reads`, `CheckedStepPredicate::slots`/`condition`, `HyperSlot`, `MatchPart`,
`MatchIncomplete`, `MatchReading::Refused { conjunct, outcome }`, `evaluate_hyper_holds`,
`evaluate_step_predicate`, `CheckedHyperLetters`, `CheckedStepMatch`, `Halt::Atom`,
`LettersRefusal::ComponentsMismatch`, `CheckedQuantifier::is_universal`) is owned by
FR-487's Outputs or "The clause seams". No stubs: no `todo!`, `unimplemented!` or
placeholder return is in the diff. The rewritten fixtures (`1 / x = 1` changed to
`1 mod x = 0`, `v.l @ b = 2` changed to `v.l @ b != v.l @ b`) keep each test's truth
table over the vault's values, and the `1 mod` form is what FR-176-AC-4 states. The
`hyper_world` source-text seams are replaced by the checked seams, which strengthens
those tests.

## Verdict

**FAIL** by the mechanical rule, for one criterion with no deferral: FR-487-AC-9 is
untagged. The other findings are coverage inflation (`low`). Plan completion: not
assessed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-487-AC-9 is `untagged`. Only its evaluation half (and `HyperSlot::Absent`) is deferred to QSL-1085. The static half ("S3 records component 0 for the read of `next` and for the read of `l` through `r`") can be tested on this PR, and no test reads `component_reads()` for the `let`/`deref` atom. The PR body defers only "FR-487-AC-9 evaluation half", which implies the static half is delivered. | spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md:440, qsl-semantics/tests/it/hyper_clauses.rs:1536 |
| FND-002 | low | Coverage inflation: FR-172-AC-7 is tagged `tagged` by a test that does not exercise its first sentence (the two-object-type receiver refusal, deferred to QSL-1085). FR-487-AC-5 is tagged by a test without its `sqrt`/`Refused` case (QSL-1084). FR-487-AC-2 is tagged by a test that asserts the refusal at the conjunct span where the AC says "at the label" (span narrowing, QSL-1085). The matrix shows these rows as covered while their deferred halves are open, and no matrix status records the deferral. | qsl-semantics/tests/it/hyper_clauses.rs:1508, qsl-analyze/tests/it/hyper_lowering.rs:217, qsl-semantics/tests/it/hyper_clauses.rs:1360 |

## Coverage

- Criteria in scope: 16 (FR-487-AC-1 to AC-12, FR-172-AC-5, AC-7, AC-8, FR-315-AC-43).
  Tagged: 11. Untagged: 5. Of those, 4 are deferred by ticket (FR-487-AC-7, AC-8, AC-12,
  FR-315-AC-43) and 1 is not (FR-487-AC-9).
- Tagged but partial: FR-172-AC-7, FR-487-AC-2, FR-487-AC-5 (FND-002). The weak or
  mutant-surviving tests behind tagged rows are recorded in the code review, SR-17210
  (FND-008 to FND-013).
- Trace tags: all new tests use `#[trace("TC-1264", "FR-487-AC-n")]` or
  `#[trace("TC-597", "FR-172-AC-5")]`, with no `/// Trace:` lines. The added
  `#[trace("TC-601", "FR-176-AC-2")]` now binds a test whose fixture matches that AC's
  text.
- Semantic review: not run as a separate pass. The intent-to-test checks done in the
  code review stand in for it.
- Plan completion: not assessed

## New findings (disposition pass 1)

Found in the fix PR, quire-spec-language#1539 at 278643086.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | `an_atom_may_call_a_declared_function` is tagged `#[trace("TC-1264", "FR-487-AC-1")]`, but it asserts nothing FR-487-AC-1 states (slots, condition shape, component reads, re-check equality). It checks that a call in an atom checks, which is FR-487 "Dependencies" and FR-172 "Body expressions", neither of which has a criterion. The binding is wrong. | qsl-semantics/tests/it/hyper_clauses.rs:1722 |
| FND-004 | low | FR-487-AC-9 is now `tagged` by `an_indexed_let_and_deref_read_one_component`, which covers only its checking half. The evaluation half and `HyperSlot::Absent` are deferred to QSL-1085. This is the same partial-tag inflation that FND-002 asked to remove for FR-487-AC-2, FR-487-AC-5 and FR-172-AC-7, so the fix applies the rule inconsistently. | qsl-semantics/tests/it/hyper_clauses.rs:1647 |

## Dispositions

Disposition pass 1, over quire-spec-language#1539 at 278643086, where `quire matrix --scope .` reads `tagged` for FR-487-AC-1 to AC-6, AC-9, AC-10, AC-11 and FR-172-AC-5, AC-7, AC-8.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `an_indexed_let_and_deref_read_one_component` reads `component_reads()` of the `(let r = v.parent in present(r) and deref(value(r)).versionNumber = 1) @ t` atom: every read is component 0 under `@ a` and component 1 under `@ b`. FR-487-AC-9 now reads `tagged`. |
| FND-002 | still-open | The `#[trace]` attributes were reduced to `#[trace("TC-1264")]`, but the doc lines `/// FR-487-AC-2: …` (hyper_clauses.rs:1382 and 1579), `/// FR-487-AC-5: …` (hyper_lowering.rs:235) and `/// FR-172-AC-7: …` (hyper_clauses.rs:1530) still bind. `quire matrix` at 278643086 still reads FR-487-AC-2, FR-487-AC-5 and FR-172-AC-7 as `tagged`, with those tests as binders, while their deferred halves (QSL-1084, QSL-1085) are open. Confirmed by experiment: rewording the FR-172-AC-7 doc line alone turns that row `untagged`. Either reword the doc lines so that they do not lead with the criterion id, or keep the tags and record the deferral. |
