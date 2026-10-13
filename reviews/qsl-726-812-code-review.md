---
id: SR-9008
title: "Code review — QSL-726 AC-2 operation-body intake (PR #812)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0d399b03502e622104231c200cc2b2b7eb0a4172; PR #812 diff against merge-base dea94c20c20c9b8416d9d57249b235bfe42b149a: qsl-cst (grammar.rs, token.rs, cst.rs), qsl-forms (dispatch.rs, protocol_clause.rs, syntax.rs, value.rs), qsl-semantics (check/assemble.rs, check/operation_body.rs, check/check.rs, model/normalize.rs, model/intake.rs), qsl-foundation/src/diagnostic.rs, qsl-replay/src/spine.rs, src/complete/editor.rs, docs/native-error-codes.md, tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---

## Summary

Ticket: QSL-726, slice QSL-INPUT-AC-2. Code review with the rust-review lane folded in. The review covered the S1 `OperationBody` production and the `body` reservation, S2 `OperationBodyForm`, the S3 resolution of body targets and unit pre/post clauses to the declaring operation's `DeclarationKey`, `EffectiveView::with_operation_bodies`, the new `conflicting_declaration` code, and the replay message arm.

## Verdict

**PASS with low findings.** The behaviour is correct.

- S1 matches the QSpec complete-V1 `operation-body` production exactly. `body` is in QSpec's complete-facet reserved list. It is reserved only in the complete grammar, and member-name positions still accept it.
- No `.native` file, test source string or fixture uses `body` as a binder. The root `it` suite (986), the crate suites and clippy `-D warnings` are green in the gate logs.
- `first` is in the base vocabulary, so TC-987's label rename is required.
- A body target resolves through the operation table to the declaring operation's key. An inherited operation is keyed to its declarer. Duplicates are detected per key after resolution, naming the label spans in source order. A missing operation refuses `missing_declaration`.
- QSpec FR-151 and QSL FR-302 and FR-246 all mandate `conflicting_declaration`/`duplicate-declaration`, so the new code is justified. No production match is exhaustive over `Code` without it.
- GitHub reports the PR MERGEABLE/CLEAN against the moved base. #810 touches different hunks of normalize.rs, and #811 touches different hunks of spine.rs.

The findings are design debt and hygiene only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `PackageDeclarations::state_clause_operations` is a public `BTreeMap<usize, DeclarationKey>` indexing into the public `state_clauses` Vec. Any reorder or filter of `state_clauses` silently desyncs the join. The resolved key belongs on the clause itself (for example on `ClauseOperation`, which bodies already share). | qsl-semantics/src/check/check.rs:333-341; qsl-semantics/src/check/assemble.rs:2472-2558 |
| FND-002 | low | `with_operation_bodies` runs for every selected model on every compile, even when the body set is empty. Each run clones the domain package and rebuilds its `ModelIndex`, which is uncharged work. The comment above the loop says "A model no body names keeps the view normalization built", which the code does not do. | qsl-semantics/src/check/assemble.rs:2329-2349; qsl-semantics/src/model/normalize.rs:610-621 |
| FND-003 | low | `has_body` is now held twice: in `package.selected_views` and in the rebuilt `package.models` `AdmittedModel`. Neither has a production reader yet. Tests assert only `selected_views`, so dropping the `admitted` rebuild fails no test. | qsl-semantics/src/check/assemble.rs:2342-2349; qsl-semantics/tests/it/operation_bodies.rs:101-114 |
| FND-004 | low | The two new qsl-forms tests are tagged `FR-102-AC-4`, but they assert `OperationBodyForm` content (label, alias, segments, parameters, spans). AC-4 is the thin-dispatch-arm property, and the existing `dispatch_entry_is_a_single_thin_call` already covers the new `Body` arm. | qsl-forms/tests/it/protocol_clause_forms.rs:1190-1250 |
| FND-005 | low | `CATALOG_CATEGORIES` is documented as the `quire.native.diagnostics/v1` causes table "in the catalog's row order". The QSpec native-diagnostics catalog (QSpec FR-271/FR-272) has no `conflicting_declaration` row, although QSpec FR-151 mandates the code. The new row has no upstream catalog entry. | qsl-foundation/src/diagnostic.rs:935-972 |

## Review Notes

The following were judged against the AC and recorded as clean:

- `body` was not entered into the clause/function namespace. FR-104 scopes that namespace to clauses and functions, FR-102 says the label is local, and FR-109 is silent.
- A target with other than three segments refuses as `UnresolvedTypeName` (`missing_declaration`/`missing-name`). That is consistent with FR-104's name-resolution refusals and FR-302's missing-operation rule.
- The new `ModelType` refusal on the pre/post clause path is reachable only when an assembler invariant breaks (`runtime_invariant`, as `ModelType` already is). The root and crate suites show no regression.
