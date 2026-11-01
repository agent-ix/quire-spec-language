---
id: SR-1952002
title: "gap-analysis review of QSL-854 combined PR 1952"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-854 QSL-1141 QSL-1370; changed qsl-package, qsl-semantics and FR-383/FR-449/TC-1194"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-449
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: references
---

## Summary

Ticket: QSL-854; combined QSL-1141 and QSL-1370. Frozen candidate d75ff9342c73685a370163badb1eec7559263e95; diff origin/main...HEAD at base 6b0252c94. No applicable AssuranceProfile artifact exists in spec/. No reviewed branch files were edited.

The scoped manual AC-to-tag-to-body reconciliation found FR-449 AC-1 through AC-6 all exercised by nonignored real W tests. W is actually admitted by the I2 reader and every fixture compares in-process rows to the IR relation. Exact clause/trigger/temporal identities, origin, empty and duplicate requirements, two temporal rows, irrelevant clocks, recompilation, comments, and four typed reader refusals are asserted. Relationship aliases project to sorted/deduplicated model-node keys using real intake, not manufactured checked records. No production stub or ownerless behavior was found in the diff.

## Verdict

PASS within the stated changed scope; no defects found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Independent reviewer execution: cargo fmt --check passed; cargo clippy --locked -p qsl-package -p qsl-semantics --all-targets --all-features -- -D warnings passed; cargo test --locked -p qsl-package emit::tests::protocol passed 93 tests, zero ignored; cargo test --locked -p qsl-semantics --lib check::protocol_relationship::tests passed 3 tests. qsl-package --no-default-features check passed. Changed-document quire validation passed: 3/3 grammar-clean, zero grammar findings, with module-discovery advisories. Coder API-surface and red/green mutation logs were inspected; the reviewer did not re-run aggregate or full-workspace gates. Existing string-edge 17 and checked-input 3 findings are baseline exclusions supported by the supplied baseline logs, not new changes.

The FR-383 scope is the relationship-set subobligations of AC-2/AC-13, not the entire protocol feature. FR-056-AC-5 is intake context; existing declaration-admission coverage is retained. FR-218-AC-14 is checked through real intake for canonical keys and ends, missing-name and wrong-kind, plus its existing three-test lane. This review does not discharge every subcase of FR-056 or FR-218. Source-map differences from repeated authored aliases are not a claim of byte-identical entire packages. No full FR-049/QSL-852 or aggregate qualification claim is made.

Reconciliation: scoped manual acceptance-criteria/tag/test-body reconciliation, per dispatch. The computed whole-repository matrix was not invoked because the dispatch excludes its known hang; no engine-matrix or evidence-store completeness is claimed.

Plan completion: not assessed

Semantic agreement was checked in the code-review lane; the optional independent whole-repository semantic fan-out did not run.

Methods skipped: dependency (no changed relationship edges), domain-object review (no changed objects), risk-complexity and scope-boundary (no changed allocation or operating scope), evidence advisor and criterion-strength (no new verification-method strategy; no Jev analysis requested). The review used the applicable base, integrity, EARS and failure-domain lenses.

## Examined scope

```yaml
scope:
- id: FR-449-AC-1
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: 'Over fixture **W** (TC-1061: `activation on each (opened: M::OrderStarted)`,
    `requires temporal OrderClosureBounded`), the map has exactly one row: the `OrderFlow`
    clause key, `Each` with `opened`''s node key, and `OrderClosureBounded`''s node
    key.'
- id: FR-449-AC-2
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: '**W** with `activation on origin` and the `order` capture rebound from
    `opened.order` to `deref(view).order` (the origin activation introduces no trigger
    binder) gives one row carrying `Origin` and no trigger; **W** with no `requires
    temporal` gives an empty map; **W** with a second `requires temporal OrderClosureBounded`
    gives AC-1''s map.'
- id: FR-449-AC-3
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: '**W** with a second temporal clause `OrderPaidBounded` and `requires temporal
    OrderPaidBounded` gives two rows in ascending node id order of the two temporal
    clause nodes; **W** with a temporal clause `RefundBounded` on the clock `refund-clock`
    that the protocol does not require gives AC-1''s map.'
- id: FR-449-AC-4
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: The map of **W** derived from the in-process package equals the map derived
    from the package read back through the I2 reader; compiling **W** twice, and compiling
    **W** reformatted with a comment added, give equal maps.
- id: FR-449-AC-5
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: '**W**''s emitted package edited so that the `requirements` entry, or separately
    the `each` trigger, names an existing node absent from the clause''s `dependencies`
    is refused by the I2 reader with `invalid_semantic_graph` at the clause''s `/dependencies`.
    Separately, each reference retargeted to a key present in the clause''s `dependencies`
    but naming no node is refused with `missing_declaration`/`missing-name` at the
    target. Neither refusal returns a map.'
- id: FR-449-AC-6
  path: spec/functional/FR-449-derive-the-control-to-temporal-activation-map-from-the-protocol-nodes.md
  role: examined
  excerpt: '**W** with `requires temporal Missing`, where the unit declares no temporal
    clause `Missing`, is refused at S3 with `missing_declaration`/`missing-name` at
    the name, with no package and no map.'
- id: FR-383-AC-2
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: The clause node of **W** has the thirteen arguments in order. With `activation
    on origin` it carries `origin`, no trigger and no guard; with `on each` it carries
    the trigger binder node and, only when a guard is written, the guard; `on each`
    with no guard and `on each ... when (true)` emit different clause keys. `relationships`
    holds the model relationship node and no alias, `requirements` holds the temporal
    clause node, `terminal` and `scheduling` are present exactly when written, and
    a protocol with none of them has empty sets and absent optionals.
- id: FR-383-AC-13
  path: spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md
  role: examined
  excerpt: Two `relationship` declarations binding one model relationship emit one
    `relationships` entry; two equal `related by` clauses on one `send`, two equal
    `visible` entries and `requires temporal Due` written twice each emit one entry;
    each such source emits the same bytes as the source without the repeat.
- id: FR-056-AC-5
  path: spec/functional/FR-056-admit-domain-package-model-declarations.md
  role: context_only
  excerpt: Each relationship member uses FR-154's canonical relationship-slot identity
    and yields one `relationship` export with its name and span; a relationship member
    missing either identity or source span refuses `invalid_model_binding`/`malformed-declaration`;
    a relationship member or reference to a node absent from the package refuses `missing_declaration`/`missing-name`;
    any declaration refusal leaves the whole package unadmitted with every refusal
    reported in node order.
- id: FR-218-AC-14
  path: spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md
  role: examined
  excerpt: '`relationship Pipe = M::pipe;` over an exported relationship binds the
    checked relationship to the export''s key and the types of its two ends; `M::nothing`
    refuses `missing_declaration`/`missing-name` at the name; `M::out`, a port, and
    `M::Node`, an object type, each refuse `invalid_model_binding`/`wrong-export`
    at the name retaining required kind `relationship`, the actual kind and the export''s
    declaration key; two relationships `Pipe` refuse `ambiguous_declaration`/`ambiguous-name`
    at both.'
- id: QSpec-FR-464-AC-13
  path: /Users/peter/dev/quire-specification/spec/objects/interfaces/FR-464-checked-package-protocol-node-bodies.md
  role: context_only
  excerpt: 'The activation relation of an admitted protocol is derived from its clause
    node''s `activation` and `requirements` alone: `activation on each (opened)` with
    `requires temporal OrderClosureBounded` gives exactly one row, the clause key,
    `each` with `opened`''s node id and `OrderClosureBounded`''s node id; under `activation
    on origin` the one row carries `origin` and no trigger; a protocol with no requirement
    has no row; a second `requires temporal OrderClosureBounded` leaves one row; a
    second required temporal clause gives two rows in ascending node-id order; a temporal
    clause that shares the prot'
- id: QSpec-FR-322-AC-96
  path: /Users/peter/dev/quire-specification/spec/objects/interfaces/FR-322-checked-package-artifact.md
  role: context_only
  excerpt: 'Every node of every published positive fixture carries exactly the `dependencies`
    list that FR-322 `dependencies` rules 1 to 4 rebuild from the node as written,
    or, for a node form whose body artifact states its own list, that list: in `positive-all-families.json`
    the `expression`/`reference` node and the `correspondence`/`source_locus` node
    each list their body target, and the `bounded_domain`/`integer_range` node lists
    its `semantic_type`; in `positive-operation-identities.json` the `bounded_domain`/`model_population`
    node lists its `semantic_type` alone, not its `max` literal''s `type`. The '
- id: qsl-package/src/protocol_activation.rs
  path: qsl-package/src/protocol_activation.rs
  role: examined
  excerpt: '//! FR-449: the activation relation projected from the in-process protocol
    nodes.'
- id: qsl-package/src/lib.rs
  path: qsl-package/src/lib.rs
  role: examined
  excerpt: '#![cfg_attr(not(feature = "std"), no_std)]'
- id: qsl-package/src/emit/tests/reachability.rs
  path: qsl-package/src/emit/tests/reachability.rs
  role: examined
  excerpt: '//! FR-365-AC-5, FR-365-AC-6 (TC-1047): a checked `reaches` over a relationship'
- id: qsl-package/src/emit/tests/protocol_requirements.rs
  path: qsl-package/src/emit/tests/protocol_requirements.rs
  role: examined
  excerpt: '//! FR-383-AC-2, AC-13: a protocol''s requirements set names the emitted'
- id: qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs
  path: qsl-package/src/emit/tests/protocol_requirements/fixture_w.rs
  role: examined
  excerpt: //! The complete TC-1061 W choreography used by TC-1194's activation cases.
- id: qsl-semantics/src/check/lowering/model.rs
  path: qsl-semantics/src/check/lowering/model.rs
  role: examined
  excerpt: '//! FR-094: model-owned nodes, the `Reference<T>` and'
- id: qsl-semantics/src/check/lowering/protocol.rs
  path: qsl-semantics/src/check/lowering/protocol.rs
  role: examined
  excerpt: '//! FR-092''s protocol nodes: the `protocol`/`protocol_clause` node each'
- id: qsl-semantics/src/check/protocol_relationship/tests.rs
  path: qsl-semantics/src/check/protocol_relationship/tests.rs
  role: examined
  excerpt: //! obligation tests, extended with TC-1056 fixture P's object type `Peer`,
bindings:
- test_id: protocol_requirements_reference_deduplicated_temporal_clause_nodes
  ac_id: FR-383-AC-2
  trace: correct
- test_id: protocol_requirements_reference_deduplicated_temporal_clause_nodes
  ac_id: FR-383-AC-13
  trace: correct
- test_id: protocol_requirements_reference_deduplicated_temporal_clause_nodes
  ac_id: FR-449-AC-2
  trace: correct
- test_id: requirement_wire_mutations_refuse
  ac_id: FR-383-AC-2
  trace: correct
- test_id: requirement_wire_mutations_refuse
  ac_id: FR-383-AC-13
  trace: correct
- test_id: requirement_order_does_not_change_the_emitted_set
  ac_id: FR-383-AC-13
  trace: correct
- test_id: requirement_order_does_not_change_the_emitted_set
  ac_id: FR-449-AC-3
  trace: correct
- test_id: a_function_can_reference_a_protocol_with_a_temporal_requirement
  ac_id: FR-383-AC-13
  trace: correct
- test_id: a_protocol_requirement_changes_the_clause_key_and_empty_set_has_no_row
  ac_id: FR-383-AC-13
  trace: correct
- test_id: a_protocol_requirement_changes_the_clause_key_and_empty_set_has_no_row
  ac_id: FR-449-AC-2
  trace: correct
- test_id: a_required_clause_relation_carries_the_each_trigger
  ac_id: FR-449-AC-1
  trace: correct
- test_id: activation_rows_ignore_unrequired_temporals_and_source_trivia
  ac_id: FR-449-AC-3
  trace: correct
- test_id: activation_rows_ignore_unrequired_temporals_and_source_trivia
  ac_id: FR-449-AC-4
  trace: correct
- test_id: a_missing_required_temporal_refuses_at_its_authored_name
  ac_id: FR-449-AC-6
  trace: correct
- test_id: activation_targets_must_exist_and_be_clause_dependencies
  ac_id: FR-449-AC-5
  trace: correct
- test_id: activation_targets_must_exist_and_be_clause_dependencies
  ac_id: FR-383-AC-2
  trace: correct
- test_id: activation_targets_must_exist_and_be_clause_dependencies
  ac_id: FR-383-AC-13
  trace: correct
- test_id: complete_w_activation_map_matches_read_back
  ac_id: FR-449-AC-1
  trace: correct
- test_id: complete_w_activation_map_matches_read_back
  ac_id: FR-449-AC-2
  trace: correct
- test_id: complete_w_activation_map_matches_read_back
  ac_id: FR-449-AC-3
  trace: correct
- test_id: complete_w_activation_map_matches_read_back
  ac_id: FR-449-AC-4
  trace: correct
- test_id: complete_w_activation_map_matches_read_back
  ac_id: FR-449-AC-5
  trace: correct
- test_id: origin_w_rebinds_the_capture_without_a_trigger
  ac_id: FR-449-AC-2
  trace: correct
- test_id: origin_w_rebinds_the_capture_without_a_trigger
  ac_id: FR-449-AC-4
  trace: correct
- test_id: w_relationships_are_model_nodes_sorted_and_deduplicated
  ac_id: FR-383-AC-2
  trace: correct
- test_id: w_relationships_are_model_nodes_sorted_and_deduplicated
  ac_id: FR-383-AC-13
  trace: correct
- test_id: w_missing_required_temporal_refuses_before_a_package_exists
  ac_id: FR-449-AC-6
  trace: correct
- test_id: a_canonical_relationship_binds_through_real_model_intake
  ac_id: FR-056-AC-5
  trace: correct
- test_id: a_canonical_relationship_binds_through_real_model_intake
  ac_id: FR-218-AC-14
  trace: correct
```
