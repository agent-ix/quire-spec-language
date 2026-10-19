---
id: SR-13353
title: "Spec review of PR #1068: base review"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7fe9aa88d; git diff refs/remotes/origin/main...7fe9aa88d (PR #1068); spec/functional/FR-133-*.md, spec/test-cases/TC-534-*.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-133
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-534
    type: reviews
---

## Summary

Ticket: QSL-530. Base spec review of `git diff refs/remotes/origin/main...7fe9aa88d` (FR-133, TC-534), against FR-126 (bindings tie-break, CX-5 loop), ADR-018 CX-5, ADR-019 SV-6, TC-216, FR-120 and FR-125, and PR #1047 (qsl-530-pr4 at 5bbe8d1b1). Measured: `make check-spec-validation` exit 0, no warning on either changed file; `tools/check-index-completeness.sh` exit 0; `quire validate` on this file exit 0. Ran `cargo test -p qsl-analyze --test it strong_hint` on 5bbe8d1b1: 5 passed, including `a_weak_refutation_names_the_strong_each_that_would_exclude_it`, which asserts the lasso's first step is `upd(b)` and the hint's identity is `a`'s update at position 0. AC-2 re-derived by hand: FR-126 'Bindings and ranking' gives a tie in length to the earlier binding `a`; for `c = a` the negation automaton's accepting SCC holds `a` at 0, every `upd(a)` edge leaves it, so CX-5's walk from `(0,0)` discharges the weak whole `attemptUpdate` obligation with the nearest edge `upd(b)` and closes with two more `upd(b)` steps (FR-126-AC-1's loop). `attemptUpdate` is taken, so no Whole hint; `upd(a)` is enabled at loop position 0 and never taken, so one `{Strong, attemptUpdate, Each}` hint, identity `upd(a)`, position 0. AC-4 vector checked by hand: `inc` (0x69) < `setTo` (0x73) in UTF-8 bytes, `T1 < T2`, so `T1.inc, T1.setTo, T2.bump`; all three are Whole hints since none is taken. The definition agrees with TC-216 (no display name enters an ordering) and with FR-126's 'never hash-map iteration order' rule. No AC renumbered; FR-133-AC-4 collides with nothing on main or open PRs; no other open PR touches FR-133 or TC-534. Behaviour text, not changelog.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-133 canonical declaration order bullet | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-AC-2 | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| FR-133-AC-4 | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md | examined |
| TC-534 scope | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| TC-534 step 4 | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| TC-534 expected step 2 | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| TC-534 expected step 4 | spec/test-cases/TC-534-strong-fairness-hint.md | examined |
| FR-126-AC-1 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| FR-126 bindings tie-break bullet | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| FR-126 canonical loop bullet | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| ADR-018 CX-5 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | context_only |
| ADR-019 SV-6 | spec/decisions/ADR-019-strong-fairness.md | context_only |
| TC-216 | spec/test-cases/TC-216-display-name-never-affects-identity.md | context_only |
| FR-120 declaration identity text | spec/functional/FR-120-simulate-a-checked-package-s-state-family.md | context_only |
| FR-125 canonical instance order | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md | context_only |
| PR #1047 ModelSystem::operations | quire-spec-language@5bbe8d1b1:qsl-eval/src/simulation/model.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-4 cannot tell declaring-type order from admitting-type order. FR-133 orders hints by the declaring type of each operation, but AC-4's vector has every operation declared on the type that admits it (T1 declares inc and setTo, T2 declares bump), the one shape where declaring-type and admitting-type order agree. PR #1047's ModelSystem::operations() (qsl-eval/src/simulation/model.rs at 5bbe8d1b1) iterates applications keyed by the admitting type's EffectiveId, names ascending within each, deduplicated on (declaring, name) with the first admitting type winning, so an operation a subtype inherits from a supertype is placed in the admitting type's group. That implementation passes AC-4 and TC-534 step 4 yet violates the FR-133 statement whenever an inherited operation's declaring type sorts differently from its first admitting type. The declaring-type wording is the right rule: an operation's identity (FR-120 operation text, FairnessOperation) is its declaring type plus name, and that order does not move when the subject admits another subtype. Add a leg to AC-4 and TC-534 step 4 with an operation declared on a supertype T0 and admitted only through a subtype, placing T0 in the identity order so the two orders differ, and route the sort fix to PR #1047 (sort operations() by (declaring, name)). Derived from reading the code; the five strong_hint tests at 5bbe8d1b1 pass, none of them with an inherited operation. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:95 |
| FND-002 | medium | 'canonical identity order' of a type is not defined anywhere. The new definition orders operations 'by the canonical identity order of their declaring type', but no other artifact defines a canonical identity order of types (grep over spec/: the phrase occurs only in FR-133). Two readings give different orders: the EffectiveId digest bytes (quire.model.effective-declaration/v2), which PR #1047 sorts by, and the declaration identity text of the type's DeclarationKey (FR-120 'Identities'), which FR-125 uses to order populations. A digest order and a JCS text order of the same keys disagree in general, so two implementers can emit the hints in different orders and both claim conformance; AC-4's premise 'T1 < T2' does not pin which key it means either. Name the key explicitly, for example 'ascending by the declaring type's EffectiveId bytes' or 'by the declaring type's declaration identity text (FR-120)', and cite where it is defined. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:62-66 |
| FND-003 | low | 'canonical declaration order' collides with the corpus's source 'declaration order'. Across the spec 'declaration order' means source order (FR-120 fields and parameters 'in declaration order', FR-101 arguments, FR-093 record slots), while this term is defined to exclude source position. A reader meeting 'canonical declaration order' without the parenthetical will read source order, which is exactly what AC-4 rules out. Rename it, for example 'canonical operation order', and keep the definition with it. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:62-66 |
| FND-004 | low | AC-4's vector leaves the loop's taken operation unstated. A lasso that leaves inc, setTo and bump enabled and untaken cannot be the terminal stutter loop (FR-133 emits nothing there, and it is entered only at a state where nothing is enabled), so its loop takes some fourth operation. AC-4 does not name that operation or require it to get no hint, while TC-534's expected result says 'three hints'. If the fixture's loop operation leaves an enabled identity untaken it gets an Each hint and the count is four. State the loop operation and that each of its enabled identities is taken, or say the three named hints appear in that relative order. | spec/functional/FR-133-name-the-strong-constraint-that-would-exclude-a-refutation.md:95 |

## Verdict

Findings: 2 medium, 2 low. Clean units: FR-133-AC-2 and TC-534 step 2 now agree with FR-126-AC-1 and with ADR-018 CX-5 (re-derived and run); TC-534's scope and step 4 trace AC-4 one to one. On the author's open concern: the spec wording (declaring type) is the right rule and should stay; PR #1047's `ModelSystem::operations()` orders by admitting type and needs the fix, and AC-4 needs an inherited-operation leg so a test can catch it (FND-001). FND-002 should be fixed before merge, since the order key is otherwise implementer's choice. PR #1047's test `hints_come_in_the_models_operation_order` says the order 'is not defined further'; that comment goes stale with this PR. Mergeable after FND-001 and FND-002; FND-003/004 may be fixed in the same round or accepted.
