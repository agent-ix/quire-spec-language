---
id: SR-7859
title: "QSL-722 PR 790 code review: TC-235 M04, M06, M08, M09"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@dcc72d4cb537b0d3b2b07c353d913e7b60015830; qsl-semantics/src/model/intake.rs, qsl-semantics/src/model/normalize.rs, qsl-semantics/tests/it/tc235_business_meanings.rs, qsl-semantics/tests/it/tc235_state_machine.rs"
review_set: subset
---

## Summary

Ticket: QSL-722. PR 790 (base `impl/business-model-meanings`) changes intake
and normalization so the refusals for TC-235 M04, M06, M08 and M09 match
FR-208. Rust-review lane folded in. Every production change matches its
FR-208 clause and TC-235 row. The open findings are in test strength: two
production changes can be reverted without failing a test, and the rewritten
M08/M09 helper no longer checks any locus.

## Verdict

CONDITIONAL. The production changes are correct against the spec:

- The event-type operation is now `invalid_model_binding`/`malformed-declaration`
  naming the operation node. This matches FR-208-AC-4 and M04 run 4.
- An `occurrenceField`, `persists` or `members` entry that names nothing is now
  `missing_declaration`/`missing-name` at the member's pointer on the type.
  This matches FR-208-AC-4, -7 and -8, M04 run 3, M08 run 5 and M09 run 5. A
  declared field that belongs to no ancestor still falls through to
  malformed-declaration, as M18 run 3 requires.
- An unresolved transition reference now names the machine, as M05 and M06
  require ("all naming `ix://test/orders/OrderLifecycle`").
- The required-member refusals are now merged with the intake refusals and
  stable-sorted by node. Intake refusals were already sorted by the same
  `refusal_node` key (intake.rs:3856/3982), so the merge only inserts the
  machine's own refusals at its node position. It does not reorder other
  refusals, so FR-154's node-order and table-order rules hold.
- A duplicate state or transition within one machine is now
  `conflicting-binding`. This matches the FR-208 Names rule, FR-208-AC-5 and
  M06 runs 5 and 6. The two-owner case in M19 still reports
  `derivation-conflict` (normalize.rs:1534-1583, FR-208-AC-12), because
  subtype states and transitions carry their own identities.

The test-side change for the persistence interface also matches the spec: it
now has an effective identity and no object universe. FR-322-AC-25 needs a
`persistence_interface` model node to declare operations. FR-208 denies an
effective type only to a namespace (AC-8). Only a process has an object
universe.

## Examined Scope

```yaml
scope:
  - {id: FR-208-AC-4, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "an identity field, an operation or an occurrence field of another type is invalid_model_binding/malformed-declaration, and an occurrence field naming no field is missing_declaration/missing-name."}
  - {id: FR-208-AC-5, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "No operations, no states or no transitions ... is invalid_model_binding/malformed-declaration; a transition from, to, trigger or guard naming nothing is missing_declaration/missing-name; a repeated state is invalid_model_binding/conflicting-binding."}
  - {id: FR-208-AC-7, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "A field, no operations, an empty persists or a persists entry that is not an object type is invalid_model_binding/malformed-declaration; an entry naming nothing is missing_declaration/missing-name; an operation contract or a quire clause is unsupported_construct/declaration-form."}
  - {id: FR-208-AC-8, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "yields exactly one original declaration with its members and vocabulary and no effective type ... a member naming nothing is missing_declaration/missing-name"}
  - {id: FR-208-AC-12, path: spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: examined, excerpt: "a state or transition declared by two owners ... is invalid_model_binding/derivation-conflict"}
  - {id: FR-154-node-order, path: spec/functional/type-model/FR-154-admit-domain-package-model.md, role: examined, excerpt: "Intake reports every declaration refusal in node order, and then no later phase runs. Within one node, every check of the table that fails reports, in table order"}
  - {id: FR-322-AC-25, path: spec/objects/interfaces/FR-322-checked-package-artifact.md, role: context_only, excerpt: "An operation member resolves only against a declaring model node of semantic_form object_type, process, state_machine or persistence_interface"}
  - {id: TC-235-M04, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 3, 21 charges: missing_declaration/missing-name naming OrderPlaced, member path TypeDefinition.occurrenceField, absent node OrderPlaced/at; run 4, 22 charges: malformed-declaration naming OrderPlaced/replay, member path TypeDefinition.operations[0]"}
  - {id: TC-235-M06, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 1, 19 charges: malformed-declaration at TypeDefinition.operations, then missing-name at TypeDefinition.transitions[0].trigger, [1].trigger and [2].trigger, all naming ix://test/orders/OrderLifecycle; ... run 5 conflicting-binding at TypeDefinition.states[3]; run 6 conflicting-binding at TypeDefinition.transitions[3]"}
  - {id: TC-235-M08, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 3, 21 charges: malformed-declaration at TypeDefinition.persists; run 4, 21 charges: malformed-declaration at TypeDefinition.persists[0]; run 5, 21 charges: missing-name at TypeDefinition.persists[0], absent node ix://test/orders/Invoice"}
  - {id: TC-235-M09, path: spec/test-cases/TC-235-business-construct-meaning-binding.md, role: examined, excerpt: "run 3: malformed-declaration naming OrderManagement, which follows Billing in node order, at TypeDefinition.members[0]; run 5: missing-name naming OrderManagement at TypeDefinition.members[3], absent node Ghost"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The machine-node rename for unresolved transition references (intake.rs:1768-1787) has no test that fails without it. The M06 run 1 and run 2 assertions and the M05 assertions check the missing-name refusals only through `detail.contains("TypeDefinition.transitions")`, `.from`/`.to` or `"OrderLifecycle"`, and `"OrderLifecycle"` is also a substring of the old transition node `.../OrderLifecycle/transition0`. If the change is reverted, the sort still puts the machine's malformed refusal first, so every test stays green. `IntakeMissingDeclaration` already has typed `node` and `path` fields: assert `node == OrderLifecycle` and the TC's `transitions/[k]/trigger`, `from` and `to` pointers in order. The same gap applies to M06 runs 5 and 6, which find `states[3]` and `transitions[3]` through the detail text instead of the typed `ConflictingBinding.key`. | qsl-semantics/tests/it/tc235_state_machine.rs:139-200; qsl-semantics/tests/it/tc235_state_machine.rs:28-61; qsl-semantics/src/model/intake.rs:1768-1787 |
| FND-002 | medium | The rewritten `assert_malformed_refusal` dropped the member-path check without putting a typed one in its place. It now asserts only the node. As a result, M08 run 2 (`operations` empty), run 3 (`persists` empty) and run 4 (`persists[0]` is `Money`) all reduce to the same assertion: one malformed-declaration on `OrderStore`. A refusal at the wrong member, or a swapped refusal, would still pass. M09 run 3 lost its `members[0]` locus in the same way. The sibling `assert_malformed_at` keeps the path check through the detail text. Keep that check here too, or add a typed member path to the cause (see the gap-analysis note on whether the spec requires that). | qsl-semantics/tests/it/tc235_business_meanings.rs:586-598; qsl-semantics/tests/it/tc235_business_meanings.rs:1320-1330 |
| FND-003 | medium | The machine-node rename widens a suppression. `read_domain_package` drops every reader refusal whose node is in `semantic_refused_nodes` (intake.rs:3840-3855). An unresolved transition reference now puts the machine node in that set, where before it put the transition node. A reader-level refusal on the state machine node itself is now silently dropped whenever any transition reference is unresolved, which breaks FR-154's rule that every failing check reports. This was traced by reading the code only; no case has been reproduced. | qsl-semantics/src/model/intake.rs:1768-1787; qsl-semantics/src/model/intake.rs:3840-3855 |
| FND-004 | low | The new `state_machine_required_members` was inserted between `validate_process_meanings` and its doc comment ("Process subtype identity and ancestry checks are normalization decisions ..."). That comment now documents the wrong function, and `validate_process_meanings` has no doc. | qsl-semantics/src/model/normalize.rs:1614-1621 |
| FND-005 | low | `if !transition_reference && !occurrence_reference { type_identity_segment(...)?; }` exempts `persists` and `members` from the foreign/native identity guard. Only `occurrenceField` needed the exemption, because its field-path id fails `type_identity_segment`. A foreign-package or native id in `persists` or `members` is now reported as a missing name of this package, which contradicts the comment directly above the check. Restrict the exemption to `occurrenceField`. | qsl-semantics/src/model/intake.rs:1734-1739 |
