---
id: SR-12146
title: "QSL-557 PR #974 gap analysis: protocol, memory and refinement-row grammar criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@458459d34cc8c020cfba66243e3ab42045622d34; FR-211-AC-1, FR-211-AC-4, FR-212-AC-2, FR-219-AC-1, FR-220-AC-1, FR-220-AC-2, FR-136-AC-1, FR-214-AC-1, FR-214-AC-3, FR-214-AC-4; qsl-cst/tests/it/protocol_memory_refinement.rs, qsl-forms/tests/it/protocol_control_forms.rs, qsl-semantics/src/check/protocol_controls.rs, qsl-cst/src/grammar.rs"
review_set: subset
---

## Summary

Ticket: QSL-557. PR quire-spec-language#974. This is a planless gap analysis scoped to the diff, so plan completion was not assessed. The criteria were read from `spec/functional/FR-136`, `FR-211`, `FR-212`, `FR-214`, `FR-219` and `FR-220`, and compared with QSpec's shared grammar and FR-375 (cited by id). Coverage was measured with `quire matrix --format tsv` (quire 0.36.2) on the reviewed head: all nine criteria in FND-001 report `tagged`, and their only binders are this PR's tests in `qsl-cst/tests/it/protocol_memory_refinement.rs` and `qsl-forms/tests/it/protocol_control_forms.rs:519`.

The ticket asks for tests that "back tokens for FR-211..220, FR-136". None of those FRs has a parse-level criterion. Every acceptance criterion in them is an S3 or runtime behaviour (TC-541, TC-656, TC-657, TC-659, TC-664, TC-665). This PR adds parse tests and tags them with nine of those criteria. On `origin/main`, no test carried any of the nine tags.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Coverage inflation. Nine S3 or runtime criteria (FR-211-AC-1, FR-211-AC-4, FR-212-AC-2, FR-219-AC-1, FR-220-AC-1, FR-220-AC-2, FR-136-AC-1, FR-214-AC-1, FR-214-AC-4) are now `#[trace]`-tagged by parse or form tests that cannot fail on those criteria' behaviour: package identities, deadlock verdicts, fairness sets, `Sc`/`Tso` resolution, access classification, `ill_typed` refusals, `CheckedRefinement.steps`, `Visible`/`Internal`/`Either`. That behaviour is not implemented, and the PR's own semantics test shows S3 refusing these constructs as `not-yet-implemented`. So the matrix reports unmet criteria as covered, which hides exactly what AGENTS.md says to keep visible for the later testing campaign. | qsl-cst/tests/it/protocol_memory_refinement.rs:83,117,130,152,161,184,211,223,271; qsl-forms/tests/it/protocol_control_forms.rs:517 |
| FND-002 | medium | FR-214-AC-1 and FR-214-AC-3 write rows the grammar refuses. `step inc -> add()` has a bare `add` with no receiver expression, but `step-target` is `operation-name '(' expr {',' step-argument} ')'`, with `operation-name` being `M::T::op`. `step Fill::Both.fork -> stutter` uses a postfix `.fork`, but `protocol-step-source` puts the step-kind word first (`fork Fill::Both`). The implemented grammar follows QSpec, so TC-659 cannot pass as the AC is written. This is a spec-versus-grammar conflict of the same kind as the FR-220-AC-2 `relaxed` fence already routed to the planner. | spec/functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md:76-78; qsl-cst/src/grammar.rs:1643-1684 |
| FND-003 | low | Nothing pins that a `refinement` declaration fails to compile. It is refused today only by construction: S2's closed `from_spelling` has no `refinement` entry, so it gives `NoDispatchEntry`. A future stub entry could let it through with no test failing. The `memory sc` and `memory ra` S3 refusals are not exercised either (only `tso` is). They take the same code path. | qsl-forms/src/dispatch.rs:412-436; qsl-semantics/src/check/protocol_controls.rs:886-913 |
| FND-004 | low | `a_refinement_parses_its_step_rows` asserts `StepSource` and `StepTarget` by count only. It does not check which alternative each row took (`memory` versus a step-kind word plus qualified name; `node` versus an abstract application), so a grammar that picked the wrong alternative with the same span would still pass. | qsl-cst/tests/it/protocol_memory_refinement.rs:250-261 |

## Verdict

FIX FND-001 before merge. The change is small: remove the nine AC ids from the `#[trace]` tags, or replace them with a doc comment saying no criterion is claimed (the SR-12128 precedent), or add a parse-level criterion to the QSL spec first and tag that. FND-002 belongs to the planner, together with the FR-220-AC-2 conflict. FND-003 and FND-004 are recorded lows.

No code defect was found in the behaviour the tests do cover:

- No construct compiles silently as `sc`. `memory tso`, `ordering seq_cst` and `fence F ordering seq_cst` each refuse `unsupported_construct`/`not-yet-implemented` with the construct label asserted (TC-1065 case table). `memory sc` and `memory ra` take the same unconditional `MemoryClause` push.
- Parse oracles are strong: exact node text per production, in order, and an exact offending token plus `InvalidSyntax` for each refusal (`pso`, a fence with no ordering at `;`, a missing `->`, a missing target, an application with no parentheses).
- The `terminal` contextual-literal rule from #961 is unchanged, and the existing forms test with `sequence fork` and `attempt terminal` still exercises it.
- Known and routed, not re-raised: FR-220-AC-2 against QSpec's `fence-order`, which excludes `relaxed`; and FR-375's population, object, history and fairness rows, which are not parsed.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 067a8e327: every `#[trace]` and every AC id is gone from the parse tests in `qsl-cst/tests/it/protocol_memory_refinement.rs` and from `a_fence_and_a_memory_clause_build_beside_their_controls`, and each doc comment now reads "Verifies the parse only and claims no criterion." `git grep` on 067a8e327 finds none of the nine AC ids in any `.rs` file, so the matrix has no binder left for them. |
| FND-002 | deferred | A spec-side conflict (FR-214-AC-1/AC-3 row spellings against QSpec's `protocol-step-source`/`step-target`), outside this grammar PR. No routing ticket exists yet; the leader routes it to the planner beside STD-167 (the FR-220-AC-2 relaxed fence). |
| FND-003 | fixed | 067a8e327: `a_refinement_declaration_refuses_the_unit_with_no_dispatch_entry` (qsl-forms/tests/it/source_grammar_vectors.rs) asserts `NoDispatchEntry{spelling: "refinement"}`, `UnsupportedConstruct` and the declaration span. The TC-1065 table now runs `memory sc`, `memory tso` and `memory ra`, each expected to refuse `parallel memory`. |
| FND-004 | fixed | 067a8e327: `a_refinement_parses_its_step_rows` now asserts the exact `StepSource` and `StepTarget` text per row, the two `OperationName` nodes (application targets only), and the `QualifiedName` list with step-kind words and `memory` rows outside the name, so a wrong alternative changes an asserted list. |
