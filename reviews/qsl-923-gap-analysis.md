---
id: SR-13657
title: "Gap analysis of QSL-923 PR #1136 against FR-126 (property automaton), FR-176, FR-177, FR-178 and TC-601, TC-602, TC-603"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8ec77cb39b263b36b726d1b2230d5f94518bb513; PR #1136 (git diff origin/main...8ec77cb39): qsl-eval/src/automaton/formula.rs, qsl-eval/tests/it/property_automata.rs, qsl-analyze/tests/it/{hyper,hyper_witness,hyper_projected}.rs; spec read: FR-126 (Property automaton section), FR-176, FR-177, FR-178, TC-601, TC-602, TC-603"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-601
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-602
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-603
    type: reviews
---
# SR-13657: Gap analysis of QSL-923 PR #1136

## Summary

Ticket: QSL-923. The scope is the PR diff. Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0, at 8ec77cb39. Plan completion: not assessed.

The PR changes no criterion and adds no AC-level tag. Its new tests carry TC-only tags, `#[trace("TC-601")]`, `("TC-602")` and `("TC-603")`, which is the repo's convention in the hyper suites. The matrix for FR-126, FR-176, FR-177 and FR-178 is the same as on main. FR-176-AC-1 and AC-2, FR-177-AC-7 to AC-12, FR-178-AC-7 to AC-9, and FR-126-AC-2, 4, 6-9 and 13 are untagged both before and after this PR, so they are outside this diff.

The ticket's acceptance asks for four things. (1) Fix the translation: done (SR-13656). (2) Tests for both shapes across the HP-2, HP-3, HP-5 and HP-6 callers: partly done (FND-002). (3) An oracle test of independent prefix semantics over random small bodies against the automaton: done, `the_automaton_of_a_random_body_accepts_exactly_the_lassos_of_its_semantics` and three sibling tests, which kill every non-equivalent mutant. (4) A check that no merged HP-2, HP-3 or HP-6 conclusion moved: done by this review's probes (SR-13656 Summary), with 0 discrepancies at this head and many at the merge base.

The requirement the fix restores is FR-126's property automaton. It says the engine "SHALL build a deterministic bad-prefix monitor: the automaton of the formula itself" and, for Liveness, "SHALL build a generalized Büchi automaton for the negation of the formula". ADR-023 HC-2 and HC-4 hand those automata to HP-3 and HP-2. No acceptance criterion binds the translation's exactness. The PR's oracle tests bind TC-601, which covers HP-2 (FR-176-AC-1 to AC-4) and not the translation (FND-001, FND-003).

Semantic check (intent, test and code) for the changed triple: the code matches FR-126's intent for every body the oracle covers. The property tests exercise the real translator, and no seam is doubled. The hyper tests exercise the real product, but two of them do not reach the changed rule (SR-13656 FND-001).

## Verdict

**CONDITIONAL**: two medium findings and one low finding. No untagged criterion is introduced. FND-001 needs a spec cycle (/specify) and is flagged to the lead, not guessed at here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The translation's exactness has no acceptance criterion. FR-126's Property automaton section requires a bad-prefix monitor of the formula itself and a generalized Büchi automaton of its negation. Both automata were wrong on main for a body with a state-formula disjunct, yet no FR-126 criterion, and no TC, states that a property automaton accepts exactly the lassos that satisfy (or, for the negation, violate) its formula. None names disjunctions, normal form or the body's own automaton (HC-2). That is why the defect had no failing test, and why the PR's oracle tests have no criterion to bind. Needs a spec cycle: an FR-126 AC (with a TC) for translation exactness over every mode the translator offers (clause, body, negation, negation in normal form). The PR's property tests then retag to it. Flagged to the lead per the spec-need rule, not authored here. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:154-190; qsl-eval/tests/it/property_automata.rs:940-1092 |
| FND-002 | medium | The ticket asks for tests of both shapes across the HP-2, HP-3 and HP-6 callers (HP-5 lives on #1131). Shape 2, the `negation-interned` fault, has no entry-point test at all, although it reproduces on main through HP-2 (with fairness), HP-3 and HP-6 (SR-13656 FND-002). Shape 1 has one effective HP-2 test (`a_negated_body_is_violated_by_the_behaviour_that_never_sets_the_output`), one HP-3 test and one HP-6 test. The other two HP-2 tests are inert (SR-13656 FND-001). There is also no repository brute-force oracle for HP-2 or HP-3 verdicts with `or` bodies, so the check in ticket item (4) lives only in review probes. Fix: add the SR-13656 FND-001 and FND-002 bodies as entry-point tests. Optionally adopt a small joint-lasso brute force for HP-2 and HP-3 like this review's probe, which over 1120 random cases each catches 1 false Holds, 23 false Violated and 10 faults on main and none at this head. | qsl-analyze/tests/it/hyper.rs:744-878; qsl-analyze/tests/it/hyper_witness.rs:149-181; qsl-analyze/tests/it/hyper_projected.rs:132-155 |
| FND-003 | low | The four new property_automata.rs tests carry `#[trace("TC-601")]`, but TC-601 is "EN-1 checks a universal hyperproperty over the self-composition product" (scope FR-176-AC-1 to AC-4). These tests check the automaton translation (`translate`, `translate_negation`, in clause, normal-form and itself modes) against lasso semantics, and never run an HP-2 product. The binding is wrong, as the coder's own judgement-call note concedes. Fix: retag them when FND-001's criterion exists. Until then, drop the TC-601 tag rather than bind it to the wrong test case. | qsl-eval/tests/it/property_automata.rs:940, 968, 1021, 1048; spec/test-cases/TC-601-universal-hyperproperty-by-self-composition.md:9-15 |

## Coverage

- Matrix (scoped): FR-126, FR-176, FR-177 and FR-178 rows are unchanged by the PR. No criterion in scope is newly untagged and none is newly tagged.
- Bindings examined:
  - property_automata.rs TC-601 tags: wrong (FND-003).
  - hyper.rs TC-601 tags: right scope (HP-2), but two of the three tests are inert.
  - hyper_witness.rs TC-602 tag: right.
  - hyper_projected.rs TC-603 tag: right.
- Reverse gap: the formula.rs change is owned by FR-126's Property automaton section (and ADR-018 SM-6). No unowned code.
- Stubs and hollow evidence: none in source. Two hollow HP-2 tests (SR-13656 FND-001).
- Semantic review: performed for the changed triple only (dispatched reviewer; no interactive opt-in available).
- Plan completion: not assessed

## Dispositions

Round 1, reviewed at `e3d9a51a5767c4bdd006917341e0749d381c96c6`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | e3d9a51a5 adds no FR-126 criterion, and no follow-up ticket is named. The translation's exactness still has no acceptance criterion. It needs a /specify cycle in this PR, or a tracked follow-up ticket, after which it can be recorded as deferred. That is the lead's call, not the coder's or the reviewer's. |
| FND-002 | fixed | e3d9a51a5 |
| FND-003 | fixed | e3d9a51a5 |

FND-002: entry-point tests now cover both shapes on every caller in scope:

- HP-2: the table test, `a_negated_body...` and the swap test (Terminal repro plus held quotient).
- HP-3: `a_witness_body_with_a_nested_state_disjunction_is_decided`.
- HP-6: `a_projected_body_with_a_nested_state_disjunction_is_decided`.

Each fails on the merge base on the defect's own symptom (SR-13656 Dispositions). After-excerpt: `fn a_witness_body_with_a_nested_state_disjunction_is_decided()`.

FND-003: the four `#[trace("TC-601")]` attributes and the `use ix_trace_rs::trace;` import are removed from qsl-eval/tests/it/property_automata.rs. After-excerpt: `#[test]\nfn the_automaton_of_a_random_body_accepts_exactly_the_lassos_of_its_semantics() {`.
