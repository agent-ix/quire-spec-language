---
id: TC-449
title: "The request builder writes one item per requirement record"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-449: The request builder writes one item per requirement record

## Description

Verify that `route`'s request builder turns a checked package's requirement
records into requested items with no caller-supplied item list: one item per
record, in record key order, each with its occurrence key, node, kind,
extent classification, unbounded domains, result bound and candidate
outcome. Scope: FR-075-AC-8.

Catches a builder that merges two records at one node into one item, drops
an item whose candidate set is empty or whose extent is unbounded, orders
items by node or by check order instead of by occurrence key, or takes the
extent from anywhere but the record.

## Test Procedure

1. Check `function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }`.
   Build a registry with one backend advertising `value-validity` in
   `bounded` mode. Run the builder over the package's requirement records
   with no named backend.
2. Check `function big using v(n: Integer): Integer pure { n + 1 }` and
   run the builder over its records with the same registry. Write a bounded
   follow-up for its one item as a new request with a fresh
   `RequestWriter`, passing the item's occurrence key to `bounded_item`
   and supplying `IntegerRange{0, 100}` for the item's one unbounded
   `DomainKey`. Finish the follow-up request and count its items for that
   occurrence key.
3. Run the builder over step 1's records with a registry whose one backend
   advertises only `operation-contract`.
4. Run the builder over step 1's records with the step 1 registry and a
   named backend `missing` that the registry does not hold.
5. Check a unit whose only function body is `b and c` over Boolean
   parameters and run the builder over its records.
6. Check FR-062's RR-16 unit and run the builder over its records with the
   step 1 registry.

## Expected Results

- Step 1: three items, request indices 0, 1 and 2, in the bytewise order of
  their occurrence keys. The two items for the `+` node carry the same node
  and distinct occurrence keys. Every item is `value-validity`, classified
  `bounded`, carries no unbounded domain and its record's result bound,
  and its candidate set is exactly the one backend.
- Step 2: one item, classified `unbounded` with `finite_bound_available`
  true, carrying one unbounded domain: kind `Integer`, keyed by `n`'s
  parameter node and the empty path. The follow-up request holds one item,
  request index 0, classified `bounded`, carrying the occurrence key
  passed to `bounded_item`, which equals the original item's. The
  follow-up request holds exactly one item for that occurrence key, so the
  record has a single follow-up settlement.
- Step 3: three items, each with an empty candidate set.
- Step 4: three items, each carrying the unknown-backend marker naming
  `missing`.
- Step 5: no items.
- Step 6: two items at the one `+` node, with distinct occurrence keys,
  carrying result bounds `Int[0, 10]` and `Int[0, 20]` in key order.
