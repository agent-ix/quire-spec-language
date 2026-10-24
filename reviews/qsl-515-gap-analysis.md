---
id: SR-12064
title: "QSL-515 PR #930 gap analysis: FR-110 layer selection (AC-1, AC-7, AC-8, AC-9), FR-361 and FR-341-AC-6"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@49ad03552456bed91a194ca204bbf0bc31eeb683; FR-110-AC-1, FR-110-AC-7, FR-110-AC-8, FR-110-AC-9, FR-341-AC-6, FR-361 (status text); qsl-semantics/src/check/layer.rs, qsl-semantics/src/value/layer.rs, qsl-semantics/src/value/definition.rs, qsl-package/src/emit.rs, qsl-replay/src/spine/clause/tests/layers.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/tests/it/complete_value_lock.rs, spec/functional/FR-110-resolve-header-profile-selections-at-e3.md, spec/functional/FR-361-gate-model-constructs-on-the-model-profile-selection.md"
review_set: subset
---

## Summary

Ticket: QSL-515. PR quire-spec-language#930. Planless, diff-scoped gap
analysis. Plan completion: not assessed. Criteria were traced to tagged tests
by reading. No matrix was run (read-only brief). QSpec is private and is cited
by id only.

Bindings examined (all tagged TC-490 unless noted):

- FR-110-AC-1: `the_value_layer_selects_state_core_and_the_qualification_rows`
  (spine compile; the lock holds exactly state core, value complete and the
  `always` rows) and `the_qualification_rows_follow_the_value_complete_layer`.
- FR-110-AC-7: `each_layer_compiles_its_own_forms_and_locks_its_closure` (all
  five layers through spine compile; the layer identities in the lock are
  checked against the closure, and for a non-value layer the `root` row
  identity is absent), `the_requires_closure_of_each_layer_is_computed`
  (closure hard-coded per layer), and
  `the_value_layer_admits_functions_and_the_package_records_its_layers`.
- FR-110-AC-8: `a_named_predicate_call_is_outside_state_core` and
  `a_named_predicate_declaration_is_outside_state_core` (spine compile; code,
  cause, span text and layer; both sides of the criterion), plus the assembler
  tests `a_named_predicate_declaration_is_outside_state_core_and_inside_state_queries`,
  `an_expression_form_of_a_wider_layer_is_refused_at_its_span`,
  `a_function_declaration_is_outside_state_queries`,
  `reachability_is_outside_state_queries` and
  `a_layer_admits_the_forms_of_its_closure_and_no_wider_layer`.
- FR-110-AC-9: `conformance_layer_rows_match_qspec_complete_value_lock`
  checks identity, requires, admitted-form file, witness request and
  `package_selection` against QSpec. It is skipped when `QSPEC_DIR` is unset,
  which `make conformance` requires.
- FR-341-AC-6: `a_layer_refusal_is_prohibited` (TC-861).
- FR-361: the status paragraph now says FR-110 layer selection is built and
  the FR-361 gate is not. That matches the code: model constructs are `None`
  in the classifier.
- FR-110 status text: the "not yet implemented" sentence is removed, which
  matches the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Most of the classifier's arms have no test. A test covers integer `div`/`rem`, named predicate calls, `reaches` and the declaration forms. Nothing covers `filter`/`map`/`count`/`sum` (SequenceQuery), `flatMap`, record/collection literals, `case`, `convert`, `flatten`, `fold`/`reduce`, or a non-predicate function call in a body. If any of these arms were mapped to `None` (admitted everywhere), state core would admit it and no test would fail. That breaks FR-110's "S3 never admits a form under a wider layer". | qsl-semantics/src/check/layer.rs:148-161 |

## Verdict

PASS. Every criterion in scope has a tagged test whose oracle can fail.
AC-1 and AC-7 check the lock's layer rows independently of the code under
test, and AC-8 checks both sides with code, cause, span and layer. FND-001 is
a medium: the per-form classification of the wider layers' expression forms
is not exercised. It is recorded under the major-only bar.

Not raised (below the bar): FR-110 "Outputs" still says "no later stage reads
a resolved profile". The emitter now reads `CheckedGraph::header_layers` to
write `definition_selections`, so that sentence is stale.
