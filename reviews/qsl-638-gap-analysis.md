---
id: SR-1369
title: "QSL-638 gap analysis of PR #651 (FR-093-AC-21, FR-093-AC-22, TC-416 steps 12-13)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b90d17b13227416d2110e5a68778ee6033a21caf; PR #651 diff against origin/main; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (AC-21, AC-22); spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md (steps 12-13); qsl-package/src/emit/tests/owners.rs; qsl-package/src/checked_v2/tests.rs; context: QSpec 65e816f0 dependency-selection-vectors.json (selection-carries-version), domain-package-acme-orders.json, fixtures/positive-two-owners-{a,b}.json"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-416
    type: reviews
---
## Summary

Ticket: QSL-638. PR: quire-spec-language#651 at b90d17b13. This is a planless
check of each AC against its tagged tests (Plan completion: not assessed).
The diff has no `spec/` change.

Trace, per unit:

- **FR-093-AC-21 / TC-416 step 12.** Two tests carry
  `#[trace("FR-093-AC-21", "TC-416")]`.
  `source_declared_nodes_carry_their_source_owner` checks `P`, `Tree` and
  `keep` carry (`a`, `u`), and the `Int[0, 9]`, `collection_bounds`,
  `Sequence<Tree>` and parameter nodes and the `Status` enum's nominal nodes
  carry none. `model_declaration_nodes_and_clause_functions_carry_their_model_owner`
  builds the clause functions with `checked_dispatch_operation` and a
  test-built clause table, and checks that `held` carries (`a`, `u`), the
  model nodes carry a `ModelOwner` with no `version`, the clause functions'
  owner nodes are exactly the three operation members, and the reference
  nodes carry none. `assert_owners` checks for every node of both packages
  that the wire owner and the projection owner equal the checked preimage's
  owner, that FR-322's presence rule holds, that every structural key and
  every group label recomputes from the wire alone, and that IR's v2 reader
  returns `Verified`. The model read gives IR the selected document, so IR
  joins each `ModelOwner`. The binding is correct. One clause is not pinned
  to the AC's own values (FND-001).
- **FR-093-AC-22 / TC-416 step 13.**
  `one_source_under_two_owners_differs_exactly_on_the_owner_keyed_nodes`
  checks every clause: the `Point` ids differ, the `List` labels and both
  member ids differ, `Integer` is equal, the `package_id`s differ, and IR
  admits both. `conformance_the_emitter_reproduces_qspec_two_owner_fixture_nodes`
  checks that every emitted node equals QSpec's published node, member by
  member (QSpec FR-322-AC-53). It reads QSpec by `$QSPEC_DIR` and copies
  nothing. Binding correct, all clauses covered.
- **Identity-preimage reader admits `owner`.** Every `checked_v2` fixture
  node now carries `owner`, and the QSpec fixture reads carry owners, so the
  admission is exercised by the whole `checked_v2` suite.
- **`selection-carries-version` maps to `refused:unknown_member`** (ruled,
  QSpec FR-322-AC-35). `conformance_dependency_selection_vectors` reads the
  QSpec vector and asserts `UnknownMember` at `/lock` and at
  `/identity_preimage`. Covered.
- **Domain package evidence for conformance reads.**
  `supply_qspec_domain_package` supplies QSpec's `acme/orders` document only
  when a `model_selections` row selects its digest. The file exists at QSpec
  65e816f0.

No untested production branch: `wire_owner`'s two arms are both exercised
(source and model tests). The `UnsupportedConstruct` refusal arm is a
one-to-one code mapping that the match needs to compile. No stubs, no
coverage inflation, no production code without an owning requirement.

## Verdict

Coverage is complete for AC-22 and nearly complete for AC-21. One low finding:
AC-21's named `ModelOwner` values for M1 and M5 are checked only by kind and
by equality with the preimage, not against the AC's own values.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-093-AC-21 says the model declaration nodes M1 and M5 carry the `ModelOwner`s `ix://acme/orders/Order` and `ix://acme/orders/Sub`. The test only asserts `owner.kind == "model"` and no `version`. The node values are checked only against the checked preimage (QSL's own derivation) and through IR's owner join, so swapping Order's and Sub's owners in the preimage would not be caught by an assertion naming the AC's values. Fix: assert that the set of `model`/`object_type` owner `node`s equals {`ix://acme/orders/Order`, `ix://acme/orders/Sub`} (and the owner `identity` is `acme/orders`). | qsl-package/src/emit/tests/owners.rs:428-433 |

## Dispositions

Round 1, reviewed at 6f9f8e72cd34981a82734249e32b929e6f3cf346 (fix commits
6c8208363 and 6f9f8e72c; `git diff d91fd5530 6f9f8e72c`). The round adds no
new finding.

FND-001: each model node now asserts `owner.identity == "acme/orders"`. The
owner `node`s are collected across the three roots, and the union must equal
{`ix://acme/orders/Order`, `ix://acme/orders/Sub`}. That still checks
FR-093-AC-21. The AC names the M1 and M5 owners without saying the two share
one package, and each root's package holds only its own model declaration
node. The union check fails if either owner is missing, is wrong, or has an
extra value. Only a swap between roots (the Order roots owning Sub and the
Sub root owning Order) would pass. `assert_owners`' equality with each
package's preimage and IR's owner join make that unlikely, so it is not a
new finding.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6f9f8e72c |
