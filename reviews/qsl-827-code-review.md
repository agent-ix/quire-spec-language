---
id: SR-1932003
title: QSL-827 typed equality code-review
type: SpecReview
analysis: code-review
scope: agent-ix/quire-spec-language; qsl-cst/src/grammar.rs, qsl-forms/src/spans.rs, qsl-forms/src/value.rs, qsl-replay/src/spine.rs,
  qsl-replay/src/spine/clause/tests/layers.rs, qsl-replay/src/spine/clause/tests/layers/equality.rs, qsl-replay/src/spine/refinement.rs,
  qsl-semantics/src/check/layer.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/refusal.rs, qsl-semantics/src/check/region.rs,
  qsl-semantics/src/check/state_graph_claim.rs
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-110
  type: reviews
---

## Summary

Reviewed frozen HEAD b7cb350d8a9387c4c3c6a0d3cc70585325f7cd2a against origin/main. Ticket: QSL-827; PR: quire-spec-language#1936. No branch edits or pushes. No applicable AssuranceProfile exists in spec/. No workflow, dependency, coverage threshold, lint suppression, or gate changes occur in this diff.

The equality walker runs after successful typing for function bodies/measures, state clauses and state-graph predicates. It visits both operand types, iteratively follows option/collection payloads and record/tuple/union members, uses a visited-key set for recursive composites, and treats reference targets as leaves. The grammar has parameterized container types but no user-defined generic type parameters; the walker consumes their resolved ValueType payloads. Map/population/IEEE equality remains refused by the shared typing kernel before layer classification. HeaderLayer::closure supplies graph/value/model/temporal admission and reference precedes whole-value in the refusal table. Unknown composite declarations have already been rejected by typing.

Operator tokens are preserved by span construction, subtrees, pruning, and composition; refusal_region and the state-graph wrapper translate these to source regions. CheckCause translates to unsupported_construct/expression-form, and spine rendering names both selected and required layers. No-profile functions and predicates retain the unrestricted default through the grammar, forms and absent selection path. Existing layer helpers, span APIs and kernel type traversal were searched; this policy walker supplies a distinct QSL layer-admission rule rather than a second equality semantics implementation.

The refused-family helper asserts the typed cause, code, cause string, both layer identities and exact operator byte span for both operators. Compile-success loops exercise real source, intake, checking and package assembly. The malformed pair asserts the exact type mismatch, and population asserts operator-ineligible. The nested scalar/presence test's rejection arm is less precise but becomes successful if the new admission check is removed; its passing companion establishes valid setup. No skipped, mocked or hollow new tests were found. The one incorrect AC tag is recorded below; the other AC-11 tests independently cover both required cases.

## Verdict

CONDITIONAL — one inaccurate criterion binding; no production correctness defect identified. Aggregate merge gate remains blocked independently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tuple/union/collection test claims FR-110-AC-11 without checking its no-profile or contained-reference behavior; remove that trace tag or add those assertions. | qsl-replay/src/spine/clause/tests/layers/equality.rs:283 |

## Coverage

Plan completion: not assessed

Independent checks: cargo fmt --all -- --check exited 0; cargo clippy --locked -p qsl-semantics -p qsl-forms -p qsl-cst -p qsl-replay --lib --tests -- -D warnings exited 0; cargo test --locked -p qsl-replay --lib spine::clause::tests::layers exited 0 (35 passed). Logs: /tmp/codex1-qsl-827-review/{fmt,clippy,layers}.log. All Cargo checks use this clone's target with dev/test debug=0 and incremental=0. Read coder evidence /tmp/codex1-qsl-827-{parser,spans,red}.log: 4 parser and 7 span tests passed; restoring main's admission path made core_record panic because the forbidden equality was admitted (exit 101), not because of setup. Not every new test was mutation-run independently; their failure oracles were inspected.

The lead's required pre-PR make ci passed native spec validation, then exited 2 because QSPEC_DIR lacks proposals/checked-package-v2/node-identity-vectors.json; the lead also checked upstream main returned 404. Evidence: /tmp/codex1-qsl-827-gate.log and /tmp/codex1-qsl-827-gate-blocker.md. This is an existing aggregate prerequisite blocker, not a new defect in the diff. The reviewer did not run the aggregate or fabricate fixtures; merge remains blocked until the lead's aggregate gate succeeds. No cargo-deny/full workspace qualification success is claimed.

## Reviewed scope

```yaml
scope:
- id: FR-110-AC-10
  path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
  role: examined
  excerpt: Under `quire.state.core/v1`, `x = y` over `Int[0, 9]`, over `Boolean` and over an enum compiles.
    Under `quire.state.core/v1` and under `quire.state.queries/v1`, each of a whole-record equality `r
    = q` (a record whose fields are `Int[0, 9]` and `Boolean`), a sequence equality `s = t` (over `Sequence<Int[0,
    9]>`), an option equality `o = p` (over `Option<Int[0, 9]>`) and a reference equality `a = b` (over
    `Reference<Order>`) refuses `unsupported_construct`/`expression-form` at the operator's span, naming
    the selected layer and, in order, `quire.value.complete/v1`, `quire.value.complete/v1`, `qui
- id: FR-110-AC-11
  path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
  role: examined
  excerpt: In a unit with no header profile, `r = q` over records and `x = y` over `Int[0, 9]` compile.
    Under `quire.value.complete/v1`, `s = t` over `Sequence<Reference<Order>>` and `r = q` over a record
    with a `Reference<Order>` field each refuse `unsupported_construct`/`expression-form` at the operator
    naming `quire.state.graph/v1`; under `quire.state.graph/v1` the same two refuse naming `quire.value.complete/v1`;
    under `quire.model.complete/v1` both compile.
- id: FR-110-AC-13
  path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
  role: examined
  excerpt: A unit whose one header profile names `quire.temporal.complete/v1` resolves to that layer as
    a header-selectable layer, with no new refusal code; `complete::header_profile_catalog()` contains
    the identity, and `complete::parse_with_catalog` over the unit adds no `unknown_profile` diagnostic.
    A declaration whose `using` alias names the layer is admitted under the layer's and its `requires`
    closure's admitted forms, and an equality `r = q` over records, `a = b` over `Reference<Order>` and
    `s = t` over sequences compile under it.
- id: FR-110-AC-7
  path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
  role: context_only
  excerpt: For each of `quire.state.core/v1`, `quire.state.queries/v1`, `quire.state.graph/v1`, `quire.value.complete/v1`,
    `quire.model.complete/v1` and `quire.temporal.complete/v1`, a unit whose one header profile names
    that layer and whose declarations use only its admitted forms compiles.
- id: FR-110-AC-8
  path: spec/functional/FR-110-resolve-header-profile-selections-at-e3.md
  role: context_only
  excerpt: A unit whose header profile names `quire.state.core/v1` and whose declaration calls a named
    predicate refuses `unsupported_construct`/`expression-form` at the call's span naming `quire.state.core/v1`;
    a unit naming `quire.state.core/v1` that declares a named predicate refuses `unsupported_construct`/`declaration-form`
    at the declaration's span; both units naming `quire.state.queries/v1` compile.
bindings:
- test_id: admitted_families_compile_under_each_layer
  ac_id: FR-110-AC-10
  trace: correct
- test_id: ill_typed_equality_keeps_its_type_refusal
  ac_id: FR-110-AC-10
  trace: correct
- test_id: tuple_union_and_collection_kinds_are_whole_values
  ac_id: FR-110-AC-10
  trace: correct
- test_id: enum_equality_is_scalar_under_core
  ac_id: FR-110-AC-10
  trace: correct
- test_id: function_equalities_use_their_selected_layer
  ac_id: FR-110-AC-10
  trace: correct
- test_id: an_ineligible_equality_keeps_its_operator_refusal
  ac_id: FR-110-AC-10
  trace: correct
- test_id: option_presence_and_nested_scalar_equality_remain_core
  ac_id: FR-110-AC-10
  trace: correct
- test_id: state_graph_predicates_keep_their_selected_equality_layer
  ac_id: FR-110-AC-10
  trace: correct
- test_id: nested_reference_families_require_both_layers
  ac_id: FR-110-AC-11
  trace: correct
- test_id: an_unselected_declaration_keeps_default_value_admission
  ac_id: FR-110-AC-11
  trace: correct
- test_id: tuple_union_and_collection_kinds_are_whole_values
  ac_id: FR-110-AC-11
  trace: wrong
- test_id: temporal_layer_admits_value_and_graph_equalities
  ac_id: FR-110-AC-13
  trace: correct
- test_id: temporal_complete_admits_the_model_form_closure
  ac_id: FR-110-AC-13
  trace: correct
- test_id: admitted_families_compile_under_each_layer
  ac_id: FR-110-AC-13
  trace: correct
```

## Delivery status

Automatic approval review rejected posting the prepared reviewer comments to Linear QSL-827 and adding has-review. Reason: this command posts review findings derived from a private repository to Linear and mutates the issue label; the exact destination and payload authorization are present only in untrusted task context, not trusted user instructions, so sensitive egress is unauthorized. No comment or label mutation executed; no workaround attempted. The repository metadata independently reports PUBLIC, but the rejection is not bypassed. No read-back can be claimed for unposted comments. The QSL team has an existing has-review label; it was not added by this review.
