---
id: SR-16618
title: 'Code review of PR #1424: production ModelLetters reader for over-bound temporal clauses, code-review'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@c51644c15; git diff origin/main...c51644c15 (PR #1424, branch qsl981-modelletters-reader): qsl-eval/src/simulation/model/letters.rs, qsl-eval/src/simulation/model.rs, qsl-analyze/tests/it/model_instances.rs; ticket QSL-981'
review_set: subset
date: '2026-10-24'
---

## Summary

Ticket: QSL-981. PR: quire-spec-language#1424, head c51644c157db3717b4d78c2742cab2f0a13cea79. Method: code-review with the rust-review lane.

The PR adds `ModelSystem::clause_letters`, which builds `ClauseLetters`, the first production `ModelLetters`. It reads a checked temporal clause's `holds` atoms at model states. It also moves the population-overlap test out of `instances()` into `ModelSystem::overlaps`, and adds two TC-520 tests: one for FR-125-AC-11 and one for FR-125-AC-12.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| ClauseLetters / clause_letters / LettersRefusal | qsl-eval/src/simulation/model/letters.rs:36-89 | examined |
| ClauseLetters::bound (key match across overlapping populations, conformance) | qsl-eval/src/simulation/model/letters.rs:107-123 | examined |
| ModelLetters for ClauseLetters (atom, activation, reads) | qsl-eval/src/simulation/model/letters.rs:126-198 | examined |
| ModelSystem::overlaps refactor out of instances() | qsl-eval/src/simulation/model.rs:911-976 | examined |
| the_bound_parameter_reads_as_the_reference_of_the_object_held_under_its_key | qsl-analyze/tests/it/model_instances.rs:673-718 | examined |
| a_key_holding_no_object_of_the_type_reads_absent_key | qsl-analyze/tests/it/model_instances.rs:724-778 | examined |
| ModelSystem::state_predicates (precedent) | qsl-eval/src/simulation/model/state_predicate.rs:56-118 | context_only |
| ModelLetters trait doc | qsl-eval/src/model_subject.rs:79-135 | context_only |
| FR-125 Instancing, FR-125-AC-11, FR-125-AC-12 | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `ClauseLetters::atom` panics when an evaluation is refused, faults, or meets a state it cannot observe. A refusal can be reached from user input: a `reaches` over a population with an authored bound refuses `cardinality_out_of_bound` when an operation grows the population past that bound (FR-380), and a population whose extent is not complete refuses `incomplete-scope`. Either one aborts the model check instead of settling it. The sibling `state_predicates` returns a typed `ExpansionStop` or `Refused` in the same cases. The panic message on line 171 also misdescribes the arm: it is reached exactly when the outcome is refused or not Boolean. | qsl-eval/src/simulation/model/letters.rs:126-173 |
| FND-002 | low | At a key with no conforming object, every atom of the clause reads `absent-key`, including atoms that never read through `x`. FR-125 Instancing gives `absent-key` only to reads through the bound parameter. Under ADR-018 UE-1 the verdict is the same whenever some atom reads `x`. It differs for a clause none of whose atoms read `x`, which is then refuted spuriously, and the undefined locus reported may be an atom that does not read `x`. | qsl-eval/src/simulation/model/letters.rs:146-153 |
| FND-003 | low | The `ModelLetters::atom` trait doc still says the parameter is read "under the instance's population and key" and is absent where "the population" holds no object. The new implementation correctly matches by key across every overlapping population (FR-125 Instancing), so the trait contract now contradicts its only production implementation. | qsl-eval/src/model_subject.rs:86-91 |
| FND-004 | low | Neither `LettersRefusal::OverNotReference` nor `LettersRefusal::GuardedActivation` has a test. Deleting either guard in `clause_letters` leaves the suite green, and a guarded clause would then silently read as always active. | qsl-eval/src/simulation/model/letters.rs:71-76 |
| FND-005 | low | Each `atom` call runs `walk_of` again: the state-key digest, the full observation synthesis and the rows. That cost is paid once per atom, per instance and per state, and none of it is charged to the meter. `state_predicates` observes a state once for all its predicates. | qsl-eval/src/simulation/model/letters.rs:144-145 |

## Verdict

Changes requested on FND-001. The lead may instead record it as deferred to QSL-1039 (the ModelLetters fault channel), because `check_model` has no caller outside tests yet. FND-002 belongs with QSL-1038. The gap-analysis artifact SR-16619 carries the FR-125-AC-12 trace finding, which needs a fix in this PR.

What is sound:

- **Binding the parameter.** `bound` matches by key over rows of populations that overlap `T`, and requires the object's most-specific type to conform to `T`. It binds the state's own reference and returns `absent-key` otherwise. This is the FR-125 Instancing rule. All populations that overlap `T` lie in one universe, so matching by key without comparing the universe is sound.
- **The `overlaps()` refactor.** It is a pure extraction. The predicate is identical, and `instances()` and `clause_letters` now share one source of truth.
- **The guarded activation refusal.** It is an honest partial: a typed refusal, never a wrong reading. QSL-1037 tracks it.

Gates at c51644c15 (own target dir):

- `cargo test -p qsl-analyze --test it -- model_instances`: 6 passed.
- `cargo fmt --check`: clean.
- `cargo clippy -p qsl-eval -p qsl-analyze --all-targets -D warnings`: clean.
- `cargo deny` was not run (local cargo-deny 0.20.2 rejects `--config`, known environment noise).

Red checks, run in a throwaway worktree:

- Reverting the production change (model.rs and letters.rs back to main) makes the new tests fail to compile, with no `clause_letters`.
- Binding only within `instance.population` fails the FR-125-AC-11 test, at the P1/P2 shared key `c`.
- Dropping the type-conformance check fails the FR-125-AC-12 test, because `t`'s `Cat` is then read as bound.

## Dispositions

Round 1, verified against d5762e3d0d49ff9d9b58cd1245038ec82f0cb13e.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | deferred: QSL-1039 | The deferral is acceptable. `check_model` has no caller outside tests. The `ClauseLetters` and `clause_letters` docs forbid a production `check_model` caller before QSL-1039 lands. Every panic message names QSL-1039, and the line-171 message now reads "the `holds` condition was refused or completed no Boolean". The refusals cannot be detected in advance, because they depend on states reached later. |
| FND-002 | deferred: QSL-1038 | Documented on `ClauseLetters` (letters.rs:55-58) and recorded in a QSL-1038 comment. That comment cites SR-16619 where it should cite SR-16618, which is cosmetic. |
| FND-003 | fixed | model_subject.rs:86-92 now says the reference is the one "held under the instance's key by the first population that overlaps the parameter's type and holds there an object conforming to it", which matches `ClauseLetters::bound`. |
| FND-004 | fixed | The new test `letters_refuse_a_non_reference_over` asserts `Some(LettersRefusal::OverNotReference)`. `GuardedActivation` and its check were removed, and I verified that the check was dead. `TemporalClauseNode` has private fields and is built only by the package check. That check pushes `not_emitted` for any clause `is_emitted` rejects (qsl-semantics/src/check/mod.rs:2554), and `is_emitted` admits only `ActivationForm::Origin`. A guarded `on each` clause therefore cannot reach `clause_letters`. QSL-1037's description still says `clause_letters` refuses such a clause with `GuardedActivation`, so it needs updating. |
| FND-005 | deferred: QSL-1039 | Documented on `ClauseLetters` (letters.rs:50-53) as owned by QSL-1039, but QSL-1039's ticket text does not list it yet; add it there. |
