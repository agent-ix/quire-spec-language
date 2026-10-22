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

## New findings (disposition pass 1)

Disposition pass 1 at head ce7fddd31 (fix commit ce7fddd31 on top of merge 74cbbb7b3; the head contains origin/main). Gates at head: `make check-spec-validation` exit 0 with no warning on FR-124, FR-391, TC-519 or TC-1076; `tools/check-index-completeness.sh` exit 0; `quire validate` on the four touched spec files passes. The diff touches only `spec/` and `reviews/`, nothing under `docs/` and no test-parsed vector table. The probed code (assemble.rs terminal resolution, check/terminal.rs, qsl-eval terminal.rs, terminal_declarations.rs) is unchanged between 788935b87 and ce7fddd31, so the earlier probe results stand and were not re-run.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | FR-391 adds the `terminal when` predicate to the population-position definition in its Description only. Its normative bullets still name state clauses and `holds` atoms alone: the observation bullet ("`current` in an invariant and in a `holds` atom"), the Evaluation SHALL ("in a state clause (FR-107) and in a `holds` atom") and the Requirements SHALL ("a domain of the clause's requirement record ...; for a `holds` atom, of its temporal clause's record"). A terminal declaration has no requirement record of its own, so which record gains the named population as a domain (for example the deadlock-freedom item's `Unbounded{domains}`) is unstated, and FR-391-AC-9 ("with `counters` as a domain read at `current`") has no requirement behind it. Extend the three bullets to the terminal predicate. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:146-148,172-176,188-190 |
| FND-009 | medium | "Where the member types of the model's populations ... are exactly one object type `T`" admits a model with two populations of `T` (for example `counters` and `archive` of `Counter`, the TC-1076 fixture). The classifier then evaluates `P` "once for each object of `T`'s population", which names no single population. Two implementers would read it as the union of both populations or as a refusal like FR-104's `ambiguous-name` for a clause type covered by several populations. State which. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:90-92,103-104 |
| FND-010 | medium | TC-1076 row 11 classifies a "terminal state" over `counters` holding `a`, `b` with `v` 2 and 1 as deadlocked. In the row's fixture (FR-155-AC-1's `Counter`, `bump` with pre `self.v < 2`) that state has `bump(b)` enabled, so it is not terminal and cannot be deadlocked (FR-124: `P` classifies terminal states only). The fixture also carries `Widget`/`grow`, whose enabledness bears on terminality. Either evaluate `P` directly at the two snapshots (true, then false), or choose states that are terminal in the fixture. | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:33 |
| FND-011 | low | FR-124-AC-8's clause "with universe `{c1}`, `... exists(c in M::counters : c.value = 3) ...` is evaluated once at each terminal state" asserts an evaluation count with no observable outcome, and TC-519 step 7 gives no expected result for `{c1}`. Only the `{}` case distinguishes once-per-state from per-object. State the `{c1}` outcome (the terminal state `c1.value = 3` is intended) or drop the clause. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:162; spec/test-cases/TC-519-terminal-declarations-and-the-deadlock-freedom-item.md:41-43,65-66 |

## New findings (disposition pass 2)

Disposition pass 2 at head f8de46a6e (fix commit f8de46a6e on top of ce7fddd31; the head contains origin/main). Gates at head: `make check-spec-validation` exit 0 with no warning on FR-124, FR-391, TC-519 or TC-1076; `tools/check-index-completeness.sh` exit 0; `quire validate` on the four touched spec files passes. The diff touches only `spec/` and `reviews/`, nothing under `docs/` and no test-parsed vector table. FR-124-AC-9 and FR-124-AC-10 are unique in the repository and in every open PR (only #1190, std-protocol-234, touches FR-391 and TC-1076, editing FR-391-AC-6 and row 8, with no id overlap). The ambiguity rule matches FR-104: "If `T` conforms to a member type of two or more populations, then ... the checker SHALL refuse `ambiguous_declaration`/`ambiguous-name` at the clause's `on`, naming those populations in ascending `DeclarationKey` order", and FR-104-AC-5 names `archive` and `config_history` the same way FR-124-AC-9 names `archive` and `counters`. FR-124-AC-10's `2 / (2 - self.value)` follows the corpus's division-by-zero fixtures for undefined terminal predicates (TC-537 step 4, FR-128-AC-6): values 2, 1, 3 and 0 give `Undefined`, false, true and true, so the three expected outcomes follow the combination rule. No statement elsewhere in `spec/` (outside `reviews/`) contradicts the new text.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | medium | The ambiguous-name bullet's condition "Where exactly one object type `T` is a member type of two or more populations" overlaps the missing-name bullet's "Where those populations cover no object type or several". A model with `Counter` in `counters` and `archive` and `Gate` in `gates` satisfies both: exactly one type is covered twice, and the populations cover two types. One bullet refuses `self` with `ambiguous-name` naming `archive` and `counters`, the other with `missing-name`. FR-124-AC-9 uses a one-type model, so no AC decides the mixed case. Tie the ambiguity condition to the one-type case (for example "Where the member types of the model's populations are exactly one object type `T` and two or more populations cover `T`"), and add the mixed model to an AC. | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:94-103,170 |
| FND-013 | low | TC-1076 row 11 says "read the deadlock-freedom item's requirement", but FR-124 adds a `DeadlockFreedom` item only "per distinct model subject among the request's temporal items", and the row names no request and no temporal item over the `Counter` subject. As written there is no item whose requirement the row can read. Name the request, for example one temporal item over the `Counter` subject, as FR-124-AC-2 does. | spec/test-cases/TC-1076-a-state-clause-names-a-population.md:33 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: FR-391 Description now reads "the predicate of a `terminal when` declaration (FR-124), which reads `current`, or a `holds` atom of a temporal clause"; FR-391-AC-9 and TC-1076 row 11 added, TC-1076 scope extended to AC-9. The normative bullets of FR-391 were not extended (new FND-008), and row 11 has its own defect (new FND-010). |
| FND-002 | fixed | Round 1: "Where the member types of the model's populations (the types its populations cover, not every declared object type) are exactly one object type `T`, the checker SHALL type `self` in `P` as `Reference<M::T>`." |
| FND-003 | fixed | Round 1: "When `P` does not read `self`, the deadlock classifier SHALL evaluate `P` once at the terminal state (QSpec FR-366)."; the per-object reading is limited to "When `P` reads `self`"; FR-124-AC-8 and TC-519 step 7 test it; PR body remaining work now names the evaluator. |
| FND-004 | fixed | Round 1: Dependencies now ends "[FR-391](FR-391-name-a-population-in-a-state-clause.md) (a population name in a terminal predicate)." |
| FND-005 | fixed | Round 1: the bullet states the reason: "no single type is the one `self` stands for, so `self` is not bound (the ambiguity is the declaration's, not a name resolving to several declarations)." |
| FND-006 | fixed | Round 1: AC-6 names `Counter` universe `{c1}`, `Gate` (`open: Boolean`, universe `{g1}`), `inc` as in AC-1 and `release` (pre `not self.open`, post `self.open`), and "no terminal state is reachable otherwise"; TC-519 step 5 matches. |
| FND-007 | fixed | Round 1: AC-5 to AC-8 use `terminal Done using v on M when { ... };`, as do TC-519 steps 4, 6 and 7. |
| FND-008 | fixed | Round 2: FR-391's observation bullet reads "`current` in an invariant, in a `holds` atom and in the predicate of a `terminal when` declaration"; Evaluation reads "in a state clause (FR-107), in a `holds` atom and in the predicate of a `terminal when` declaration (FR-124)"; Requirements reads "for a `terminal when` predicate, of the model subject's deadlock-freedom item, FR-124", whose `Unbounded{domains}` requirement FR-124's Outputs define. |
| FND-009 | fixed | Round 2: typing now needs "exactly one population covers `T`", and a new bullet refuses `self` with `ambiguous_declaration`/`ambiguous-name` at `self` when `T` is a member type of two or more populations, naming them in ascending `DeclarationKey` order, as FR-104 does for an `on` type; FR-124-AC-9 and TC-519 step 8 test it. The new condition overlaps the missing-name bullet in a mixed model (new FND-012). |
| FND-010 | fixed | Round 2: TC-1076 row 11 now reads "evaluate its predicate over `counters` holding `a`, `b` with `v` 2 and 2, and with `v` 2 and 1", expected "the predicate is true, then false", with no claim about terminality. The row's item read names no request (new FND-013). |
| FND-011 | fixed | Round 2: FR-124-AC-8 reads "with universe `{c1}` and `c1.value = 3` the terminal state is intended", and TC-519 step 7 expects the same. |
