---
id: SR-1952003
title: "spec-review/base review of QSL-854 combined PR 1952"
type: SpecReview
analysis: base
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

The two FR errata and TC-1194 preserve artifact and criterion identities and existing relationships. The missing dependency edge now names InvalidSemanticGraph at /dependencies; a missing target remains MissingDeclaration/MissingName. This agrees with the authoritative QSpec graph-stage and protocol rules. The origin variant explicitly replaces its removed trigger capture with deref(view).order. FR-383 only removes the obsolete relationships withholding and states emitted relationship-set order. Its cyclic-component text is untouched.

## Verdict

PASS within the stated changed scope; no defects found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Independent reviewer execution: cargo fmt --check passed; cargo clippy --locked -p qsl-package -p qsl-semantics --all-targets --all-features -- -D warnings passed; cargo test --locked -p qsl-package emit::tests::protocol passed 93 tests, zero ignored; cargo test --locked -p qsl-semantics --lib check::protocol_relationship::tests passed 3 tests. qsl-package --no-default-features check passed. Changed-document quire validation passed: 3/3 grammar-clean, zero grammar findings, with module-discovery advisories. Coder API-surface and red/green mutation logs were inspected; the reviewer did not re-run aggregate or full-workspace gates. Existing string-edge 17 and checked-input 3 findings are baseline exclusions supported by the supplied baseline logs, not new changes.

The FR-383 scope is the relationship-set subobligations of AC-2/AC-13, not the entire protocol feature. FR-056-AC-5 is intake context; existing declaration-admission coverage is retained. FR-218-AC-14 is checked through real intake for canonical keys and ends, missing-name and wrong-kind, plus its existing three-test lane. This review does not discharge every subcase of FR-056 or FR-218. Source-map differences from repeated authored aliases are not a claim of byte-identical entire packages. No full FR-049/QSL-852 or aggregate qualification claim is made.

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
bindings: []
```
