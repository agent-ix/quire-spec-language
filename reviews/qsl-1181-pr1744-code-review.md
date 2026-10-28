---
id: SR-1744001
title: "code review (with rust-review) of PR #1744 (QSL-1181 slice 6): refinement adverse corpus"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1744 slice commit 8d1753244 only (4 files; the #1737 merge excluded): qsl-package emit/tests/refinement_adverse.rs, emit/tests/admission_corpus.rs, emit/tests.rs, qsl-semantics check/refinement/internal_list.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1744001: code review of PR #1744 (QSL-1181 slice 6)

## Summary

Ticket: QSL-1181. Reviewed only the slice's own commit, 8d1753244 (4 files, tests only). The #1737 merge is excluded.

How each test asserts code, cause and locus:
- `rows_without_a_node_form_refuse_the_refinement_at_its_declaration`
  - Code and cause: `unsupported_construct`/`not-yet-implemented`.
  - Locus: the `RefinementNotEmitted` span runs from `refinement R using` to the closing `}`, with exactly one refusal.
  - Mutant: dropping `NodeKind::Seq` from `rows_have_nodes` fails the `seq` case, so each case is caught by its own row form.
- `a_refinement_over_an_unwritten_clause_refuses_naming_the_construct`
  - Each of the 5 constructs (`RepeatDecisionReadsBody`, `PostTestRepeat`, `VisibleEntryNotAnExpression`, `RepeatStepsThroughRepeat`, predicate-joined `parallel`) gives exactly two refusals.
  - The clause refusal equals `not_emitted(construct)` at `protocol Impl`.
  - The refinement refusal is `RefinementCause::Check` of the same cause, located at the first step row's exact text.
- `a_refinement_row_or_header_key_of_the_wrong_kind_or_absent_refuses_at_its_member`
  - Starts from an admitted baseline, with every identity recomputed.
  - The wrong-kind `concrete_type` and `semantic_type` cases assert code, cause and member path.
  - The absent-`abstract` case asserts code and cause but no path (FND-001). The probed verdict is `MissingDeclaration at /semantic_graph/nodes/10/body/abstract (MissingName)`, so the assertion is a one-line fix.
- `an_each_clause_without_its_trigger_binder_is_an_internal_fault` asserts the exact `InternalFault` subsystem and invariant, plus the `origin` control.

Rust:
- `refinement_verdict` rekeys from the closed `quire.refinement-node/v1` preimage and refreshes the frame identity. The new `pub(super)` items are test-module scoped.
- `cargo clippy -p qsl-semantics -p qsl-package --all-features --all-targets -D warnings` and fmt: exit 0.

Gates at 8d1753244 (locked-build, jobs 4, debug info off): the 4 new tests pass (3 in qsl-package lib, 1 in qsl-semantics lib).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The absent local `abstract` case asserts code and cause but not "at the member": add `verdict.contains(&at("body/abstract"))` | FR-375-AC-21 |

## Verdict

Mergeable once FND-001 and the gap-analysis findings (SR-1744002) are fixed.

## Dispositions

Round 1, at f8925a6b9 (fix 48914b039 plus the IR-1093 follow-up). Every finding is fixed:
- SR-1744001 FND-001: the `body/abstract` locus is now asserted.
- SR-1744002 FND-001: `an_out_of_order_row_refuses_at_its_first_offending_entry` refuses at `objects/0/fields/1`.
- SR-1744002 FND-002: the ignores cite IR-1093, which is open.
- SR-1744002 FND-003: the external-key `MissingSelection` case is tested at `body/abstract`.

Verdict: mergeable.
