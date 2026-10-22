---
id: SR-13780
title: 'Spec review of PR #1183: type of self in terminal when P (STD-160), spec-integrity analysis'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@788935b87; git diff origin/main...788935b87 (PR #1183, branch std-160-spec, merge base 7286be314): spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md, spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md; ticket STD-160'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-124
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-519
  type: reviews
---

## Summary

Ticket: STD-160. PR: quire-spec-language#1183, head 788935b87. Method: spec-review with the spec-integrity sub-analysis. Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

The PR adds one behaviour bullet to FR-124 that types `self` in a `terminal when P` predicate from the model alone, adds FR-124-AC-5 to AC-7, adds an FR-391 `depends_on` edge, and extends TC-519 with steps 4 to 6. The diff touches nothing under `docs/` and no test-parsed vector table.

Gates at head: `make check-spec-validation` exit 0, with no warning on FR-124 or TC-519; `tools/check-index-completeness.sh` exit 0; `quire validate` on both touched files passes. The PR is QSL-only, so the QSpec gates do not apply.

Ids: no new artifact ids. FR-124-AC-5 to AC-7 are unique in the repository. TC-519 steps 4 to 6 are new.

Behaviour probe. A throwaway test was added to the `qsl-semantics` `it` suite in a detached review worktree, run with `--features test-support`, and then deleted. It used TC-458's `ConfigVersion` fixture: one type, a copy with `populations: []`, and `with_unrelated_population` for several types. Results at head:

| Case | Current result |
| --- | --- |
| one type, `self.versionNumber = 1000` | `When`, `self: Reference<ConfigVersion>` |
| one type, `{ true }` | `When` |
| one type, `forall(c in Config::config_history : ...)` | check refusal `missing_declaration`/`missing-name`, `MissingSegment` at `Config::config_history` |
| no population, `self` or `{ true }` | assembly refusal `missing_declaration`/`missing-name` (`UnresolvedTerminalState`) at the `on` alias |
| several types, `self`, `{ true }`, `forall`/`exists` over both populations | assembly refusal `ambiguous_declaration`/`ambiguous-name` (`AmbiguousTerminalState`) at the `on` alias, before `P` is checked |
| several types, the same `forall` body as an invariant on `Config::ConfigVersion` | checks; a bound variable over `Config::others` reading `versionNumber` refuses `ill_typed`/`type-mismatch`, so bound variables take their population's type |

So AC-5 matches current behaviour. AC-6 and AC-7 describe required work in the assembler (`terminal_declarations` in qsl-semantics/src/check/assemble.rs), in population-name resolution for the terminal predicate, and in the evaluator (qsl-eval/src/simulation/model/terminal.rs `read` requires a `Reference` `self` parameter and returns BROKEN without one). The PR body's "Remaining work" names only the checker side.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-124 behaviour bullet "The checker SHALL give `self` in `P` a type from the model alone" | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:90-101 | examined |
| FR-124-AC-5 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:149 | examined |
| FR-124-AC-6 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:150 | examined |
| FR-124-AC-7 | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:151 | examined |
| FR-124 relationships (FR-391 edge) and Dependencies | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:18-19,153-161 | examined |
| TC-519 scope, steps 4-6, expected results | spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md:17,32-37,52-57 | examined |
| FR-391 Description (population position) and Resolution | spec/functional/FR-391-name-a-population-in-a-state-clause.md:37-45,104-121 | examined |
| FR-104 `self` typing and context population | spec/functional/FR-104-check-state-clauses.md:127-137,286-297 | context_only |
| ADR-018 DL-1 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:563 | context_only |
| QSpec FR-366 terminal predicate | quire-specification spec/functional/temporal/FR-366-check-deadlock-freedom.md:57-61 | context_only |
| terminal resolution and evaluation code | qsl-semantics/src/check/assemble.rs:3740-3840, qsl-semantics/src/check/terminal.rs, qsl-eval/src/simulation/model/terminal.rs:103-150 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-124-AC-6 depends on `forall(c in M::counters : ...)` resolving in a `terminal when` predicate, but FR-391 defines a population position as a unit-level state clause body (invariant, precondition, postcondition, step clause) or a `holds` atom. A terminal predicate is not in that list, and FR-391 is not amended. The probe confirms it: even in the one-type fixture, `terminal ... when { forall(c in Config::config_history : ...) }` refuses `missing_declaration`/`missing-name` (`MissingSegment`), while the same body as an invariant checks. As written, FR-391 makes AC-6 unbuildable and contradicts the new FR-124 bullet. FR-391's population-position definition needs to name the `terminal when` predicate, with a matching AC. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:150; spec/functional/FR-391-name-a-population-in-a-state-clause.md:37-45 |
| FND-002 | medium | "the model declares exactly one object type" has no stated basis. The implementation counts the canonical member types of the model's populations (`population_objects`), not declared object types. A model can declare an object type that no population covers: the fixture with `populations: []` declares `ConfigVersion` and is refused `UnresolvedTerminalState`. Under the new text it has one object type, so `self` would type and `P` would hold vacuously. A model with two declared types, only one in a population, splits the same way. The bullet needs to say which count it means: declared object types, or the member types of the model's populations. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:92-95 |
| FND-003 | medium | How a `P` that does not read `self` is evaluated is unspecified, and the two branches disagree. In a one-type model the bullet makes every `P` hold when it holds for every object of `T`, and makes a state with no object of `T` satisfy `P`. `terminal when { false }`, or `exists(c in M::counters : c.value = 3)`, is then intended at a state with no counter. The same `P` in a two-type model has no `self` and must be read once at the state, as QSpec FR-366 states ("a terminal state that satisfies `P` is intended"). The evaluator today reads `P` once per object of `self`'s type, and returns BROKEN when there is no `Reference` `self` parameter. So a self-free `P` also needs evaluator work, which the PR body's remaining work omits. The bullet should state that a `P` not reading `self` is read once at the state, or that the per-object reading applies only when `P` reads `self`. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:93-101 |
| FND-004 | low | The new `depends_on` FR-391 edge has no entry in `## Dependencies`, which lists FR-104, FR-107, FR-120, FR-097 and FR-103 but not FR-391. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:153-161 |
| FND-005 | low | For several object types, AC-7 picks `missing_declaration`/`missing-name` at `self`. Current code refuses `ambiguous_declaration`/`ambiguous-name` (`AmbiguousTerminalState`), and FR-104 refuses `ambiguous-name` when a clause's type is covered by several populations. "missing" is defensible, since `self` is simply not bound, but the bullet gives no reason for preferring it over the ambiguity code the checker and FR-104 use for the analogous case. A reader of the old test will expect `ambiguous-name`. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:151 |
| FND-006 | low | AC-6 and TC-519 step 5 give the two-type model no universes, initial values or operations, so which states are terminal, and which of them the AC claims are intended, is not fixed. AC-5 gives `{c1, c2}`. Name the universes (for example `{c1}`, `{g1}`) and the operations, or state that the classification is checked over constructed terminal states. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:150; spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md:34-35 |
| FND-007 | low | AC-5 to AC-7 write the declaration as `terminal when { ... }`. Under AC-4 that form fails to parse (it needs `terminal <name> using <alias> on <model> when <block>;`). AC-1 already abbreviates, but the new refusal ACs locate a span (`at self`), so the full declaration text matters to the test author. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:149-151 |

## Verdict

Not mergeable as written. FND-001 is a high finding: FR-391 does not make a terminal predicate a population position, so AC-6's quantifier form is refused by the spec that FR-124 now depends on, and by the current checker. FND-002 and FND-003 are medium ambiguities that two implementers would resolve differently. The lead default itself is coherent: no new syntax, the one-type reading unchanged, quantifiers for several types, and the alternatives are named in the PR body. The wording is behaviour-only, with no changelog or status language. The ids are unique, and every new AC has a TC-519 step. Apart from FR-391, no statement elsewhere in `spec/` (outside `reviews/`) contradicts the new text. ADR-018 DL-1, ADR-021 SYM-3/SYM-8, TC-566 and FR-382 say nothing about `self` typing.
