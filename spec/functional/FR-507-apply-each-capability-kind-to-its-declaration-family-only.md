---
id: FR-507
title: "Apply each capability kind to its declaration family only"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-281
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-063
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-272
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-271
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# FR-507: Apply each capability kind to its declaration family only

## Description

A request item's kind applies only to the declaration family that holds its
claim. QSpec FR-290 fixes the closed vocabulary of seven declaration families,
the kinds that apply to each, and the cause `invalid_capability`/
`inapplicable-kind` (QSpec FR-271, FR-272) for an item whose kind does not
apply. A family may admit more than one kind, and a kind may apply to more
than one family: one declaration may hold claims of two kinds, as a `relation`
clause with an implementation binding does. QSpec FR-290 names this
requirement as QSL's producer mapping from its declarations and claim forms to
those labels.

The composed linker did this for the seven protocol kinds in
`Family::Protocol` and `Disposition::InapplicableCapability`, tested by
TC-115. The spine has the closed family catalogue `FamilyKind` (ADR-012) and
the `Capability` type with its FR-290 kind-group projection (FR-057-AC-13),
but no request is checked against the family of the claim it names, and
`qsl-route` only registers advertisements. This requirement is the spine
successor.

## Inputs

- An FR-281 `NegotiatedItem`: its `ClaimItem` (`request_index`,
  `claim {node, occurrence}`, `kind`, and the temporal member's `item_kind`
  when it has one), and the disposition the caller's `negotiate_*` gave it.
- The checked package, from which the claim node's tag, `semantic_form` and
  occurrence role are read.

## Outputs

- For an applicable item, the item is settled and routed as FR-281 states.
- For an inapplicable item, a terminal record with disposition
  `invalid-request`, settled by that disposition, carrying the diagnostic
  `invalid_capability`/`inapplicable-kind` with payload `request_index`,
  `capability` and `family`, no result and no engine run.

## Behavior

### Families and their kinds

The declaration families are QSpec FR-290's seven labels, and each admits
exactly these kinds:

| Declaration family | Kinds that apply |
| --- | --- |
| `value` | `value-validity` |
| `state-model` | `operation-contract` |
| `temporal-trace` | `temporal-satisfaction` |
| `relation-claim` | `temporal-satisfaction`, `operation-contract` |
| `protocol` | `finite-replay`, `global-conformance`, `monitorability`, `local-projection`, `refinement`, `realizability`, `composition` |
| `probabilistic` | `probabilistic-satisfaction` |
| `real-time` | `schedulability` |

Each family's kinds are exactly the kinds the claim forms of that family's
rows in "QSL claim forms and their families" below require. `finite-replay`
applies to `protocol` only, because only a choreography is replayed;
`state-model` admits `operation-contract` only.

### QSL claim forms and their families

The family is a property of the claim, not of its kind. This table is QSL's
producer mapping. Each row gives the kind its claim form requires, which is
the kind FR-057's claim-form table gives where that table has the form:

| QSL claim form | Required kind | Declaration family |
| --- | --- | --- |
| Boolean clause over pure expressions, including one holding a `case` expression | `value-validity` | `value` |
| Scalar operation application in a `Value` function body (FR-062), one claim per occurrence | `value-validity` | `value` |
| State precondition, postcondition and invariant (FR-104) | `operation-contract` | `state-model` |
| Frame obligation (`modifies`, `creates`, `deletes`) | `operation-contract` | `state-model` |
| Temporal clause under a finite-trace or infinite-trace profile, including over a timed (FR-231) or hybrid (FR-447) subject | `temporal-satisfaction` | `temporal-trace` |
| State-graph claim (FR-165) | `temporal-satisfaction` | `temporal-trace` |
| `hyper` clause (FR-173) | `temporal-satisfaction` | `temporal-trace` |
| Derived deadlock-freedom item over a model (FR-124) or protocol (FR-211) subject, derived time-lock-freedom item (FR-232), derived race-freedom item (FR-224) | `temporal-satisfaction` | `temporal-trace` |
| Model claim of a `relation` clause over model operations (FR-173) | `temporal-satisfaction` | `relation-claim` |
| Code claim of that `relation` clause when every operation has an implementation binding (FR-180) | `operation-contract` | `relation-claim` |
| Whole refinement between two model subjects (FR-144), including one whose abstract side is a protocol (FR-147) | `temporal-satisfaction` | `relation-claim` |
| Per-step and initial-state obligation of a model refinement (FR-146); refinement between two operation contracts or state models | `operation-contract` | `relation-claim` |
| Replay of one finite execution against protocol semantics | `finite-replay` | `protocol` |
| Refinement between two protocols | `refinement` | `protocol` |
| Other protocol analysis claim | its `protocol`-family kind | `protocol` |
| Probabilistic claim (FR-186) | `probabilistic-satisfaction` | `probabilistic` |
| `schedulable` or `response` claim over a task set (FR-246) | `schedulability` | `real-time` |
| `case` exhaustiveness obligation over a `SumCase`, abstraction relation (QSpec FR-353) | none | none: no request item |

A refinement declaration is a model refinement whatever its abstract side:
its items are `temporal-satisfaction` and `operation-contract` on
`relation-claim`, never `refinement`. The `refinement` kind applies only to a
refinement between two protocols, which a protocol declaration holds.

A claim transferred through a refinement (QSpec FR-380) is planner-local and
is not a request item, so it takes no family and is never checked.

### Where the family is read

`analyze` SHALL read an item's family from the request and the checked
package, by the rule QSpec FR-290 gives `negotiate_*`:

- An item whose `item_kind` is not `formula` takes its family from the
  `item_kind`: `deadlock-freedom`, `time-lock-freedom` and `race-freedom` are
  `temporal-trace`, and `refinement` is `relation-claim`, whatever node the
  item is anchored on. A derived item anchored on a protocol clause node is
  therefore `temporal-trace`.
- Every other item takes the family of the checked-package node its `claim`
  names, by that node's tag, `semantic_form` and occurrence role, through QSpec
  FR-290's node-to-family table.
- A claim that names a node of no row of that table, or a node absent from the
  package, has no family. Every kind is inapplicable to it, and the refusal's
  payload has no `family` member.

`analyze` SHALL NOT read the family from the item's kind, from the kind's
FR-290 kind group (FR-057-AC-13), or from QSL's `FamilyKind`.

### The refusal

If an item's kind does not apply to its claim's family, then `analyze` SHALL
settle the item `invalid-request` with `invalid_capability`/
`inapplicable-kind`. Its payload is:

- `request_index`, the item's request index;
- `capability`, the item's kind label;
- `family`, the claim's family label, absent when the claim has no family.

No engine SHALL run an inapplicable item, and no other kind SHALL be
substituted for its kind.

`analyze` SHALL NOT refuse `inapplicable-kind` an item whose kind is the
required kind of its claim form's row in "QSL claim forms and their
families".

- **Order.** QSpec FR-290 makes this check `negotiate_*`'s first item rule
  after kind admission: it comes before the evidence-kind, `not-statistical`,
  `absent-extent` and candidate-table rules. `analyze` receives items the
  caller has already settled (FR-281), so it SHALL apply the check before it
  routes an item, whatever disposition the caller supplied, and before
  FR-281's `unknown-backend` and `inconsistent-candidates` checks, as FR-281's
  rule order states. An inapplicable
  item therefore settles `inapplicable-kind` whatever its extent, evidence
  kind, statistical method, candidates or named backend, and an item settled
  `Supported` runs on no engine when its kind does not apply. A conforming
  `negotiate_*` has already given that item the same disposition and
  diagnostic, so the record is the same either way.
- **Record.** An inapplicable item SHALL keep its `request_index` and its
  place among the records (FR-281). Its terminal record is its
  `invalid-request` disposition (`settled_by` `disposition`, QSpec FR-331),
  with no result entry, artifact or counterexample.
- **Scope.** The refusal settles only its own item. It SHALL NOT withhold,
  alter or re-check the declaration body, which has already reached its
  family checker, and the request's other items settle independently.
- An item whose kind applies SHALL be unaffected by the check, whatever the
  disposition the caller supplied.
- A `required` inapplicable item SHALL make a complete aggregate result
  unavailable, joined from the FR-331 accounting records on request index
  (FR-036-AC-6).
- The applicability table SHALL be total over kinds: every `Capability`
  variant is listed for at least one family, and a variant added without an
  entry fails to compile (FR-063).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-507-AC-1 | For each of the twelve kinds and each of the seven families, exactly the tabled (kind, family) pairs are applicable: of the 84 pairs, 14 are applicable and 70 settle `invalid-request` with `invalid_capability`/`inapplicable-kind` and payload `request_index`, `capability` (the kind label) and `family` (the claim's family label), with no engine run. | Test (TC-1302) |
| FR-507-AC-2 | A required `operation-contract` item on a state clause's claim is applicable and runs; a `finite-replay` item (not required) on the same claim settles `invalid-request` with `family` `state-model`; the state declaration's clause is still checked and its record unchanged. | Test (TC-1302) |
| FR-507-AC-3 | Each of the seven protocol kinds on a `protocol` declaration's claim is applicable; each of the seven on a state claim, a value function claim and a temporal clause claim settles `invalid-request` with `family` `state-model`, `value` and `temporal-trace` respectively. | Test (TC-1302) |
| FR-507-AC-4 | A `probabilistic-satisfaction` item on a probabilistic claim and a `schedulability` item on a task-set claim are applicable; the same kinds on a state claim settle `invalid-request` with `family` `state-model`. | Test (TC-1302) |
| FR-507-AC-5 | Two items, `operation-contract` on a state claim (`supported`) and `finite-replay` required on the same claim, give two records at indices 0 and 1 in caller order; the aggregate is unavailable, and an engine ran only for index 0. | Test (TC-1302) |
| FR-507-AC-6 | A `Capability` variant with no applicable-family entry does not compile under `--cfg seam_probe`. | Test (TC-1302) |
| FR-507-AC-7 | For one `relation` clause whose every operation has an implementation binding, its model claim requested as `temporal-satisfaction` and its code claim requested as `operation-contract` both apply, on family `relation-claim`. For one model refinement, its whole-refinement item (`temporal-satisfaction`) and each per-step and initial-state obligation (`operation-contract`) apply on `relation-claim`. A `global-conformance` item on the relation clause's claim settles `inapplicable-kind` with `family` `relation-claim`. | Test (TC-1302) |
| FR-507-AC-8 | The derived deadlock-freedom, time-lock-freedom and race-freedom items, `temporal-satisfaction`, apply on `temporal-trace` read from their `item_kind`, including a deadlock-freedom and a race-freedom item anchored on a protocol clause node. A `global-conformance` item on that protocol clause node's own claim applies on `protocol`. | Test (TC-1302) |
| FR-507-AC-9 | An item whose claim names a node absent from the package, or a node of no row of QSpec FR-290's node-to-family table, settles `inapplicable-kind` for each of the twelve kinds, and its payload has `request_index` and `capability` and no `family` member. | Test (TC-1302) |
| FR-507-AC-10 | An item whose kind does not apply, handed to `analyze` settled `Supported` on a held engine advertising its (kind, mode), `Supported` on a candidate `engines` does not hold, `Supported` on a held engine that does not advertise its (kind, mode), `RequiresBound`, `Unsupported`, `invalid-request` with `absent-extent`, or `invalid-request` with `ambiguous-backend`, settles `invalid-request` with `inapplicable-kind` in every case, never `unknown-backend`, `inconsistent-candidates` or the handed disposition, and no engine runs (FR-281's order). | Test (TC-1302) |
| FR-507-AC-11 | For each row of "QSL claim forms and their families" that requires a kind, an item of that row's kind on such a claim is not refused `inapplicable-kind`, and reads that row's family. This covers at least a Boolean value clause (an `expression` node, occurrence role `claim` and role `declaration`), a scalar operation application in a `Value` function body, a frame obligation (`state`/`operation_anchor`, role `anchor`), a state-graph claim, a `hyper` clause, a temporal claim over a timed subject and one over a hybrid subject, and a refinement whose abstract side is a protocol. | Test (TC-1302) |

## Dependencies

- **Upstream:** FR-057 (kinds, claim-form table, FR-057-AC-13 kind-group
  projection), FR-281 (items and records), QSpec FR-290 (the seven
  declaration families, their kinds, the node-to-family and `item_kind`
  tables, and the check's order), QSpec FR-271 and FR-272 (the
  `invalid_capability`/`inapplicable-kind` cause and its payload keys), QSpec
  FR-331 (request position and disposition record).
- The QSpec contract change this requirement needed, the cause
  `inapplicable-kind` under `invalid_capability`, is in QSpec
  (quire-specification#608).
