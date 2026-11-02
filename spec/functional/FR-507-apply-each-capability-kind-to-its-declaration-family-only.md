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
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
---
# FR-507: Apply each capability kind to its declaration family only

## Description

FR-057-AC-11 requires each of the twelve capability kinds to apply to exactly
one declaration family, and a request for a kind on a declaration of another
family to be recorded as inapplicable, naming the family. The composed linker
did this for the seven protocol kinds (`finite-replay`, `global-conformance`,
`monitorability`, `local-projection`, `refinement`, `realizability`,
`composition`) in `Family::Protocol` and `Disposition::InapplicableCapability`,
tested by TC-115. The spine has the closed family catalogue `FamilyKind`
(ADR-012) and the `Capability` type with its FR-290 family projection
(FR-057-AC-13), but no request is checked against the family of the claim it
names, and `qsl-route` only registers advertisements. This requirement is the
spine successor.

## Inputs

- An FR-281 `ClaimItem`: `claim {node, occurrence}` and `kind`.
- The checked package, from which the claim node's declaration family is read.

## Outputs

- For an applicable item, the item is routed as FR-281 states.
- For an inapplicable item, a terminal record with disposition
  `invalid-request`, diagnostic `invalid_capability`/`inapplicable-kind`
  carrying the kind and the claim node's family, and no engine run.

## Behavior

The family of a claim is the family of the declaration that holds the claim
node, spelled as FR-290 spells it. The applicable family of each kind is:

| Kind | Declaration family |
| --- | --- |
| `value-validity` | `Value` (`value`) |
| `operation-contract` | `StateModel` (`state-model`) |
| `temporal-satisfaction` | `TemporalTrace` (`temporal-trace`) |
| `finite-replay`, `global-conformance`, `monitorability`, `local-projection`, `refinement`, `realizability`, `composition` | `ProtocolClause` (`protocol`) |
| `probabilistic-satisfaction` | the probabilistic claim form (FR-186), family `probabilistic` |
| `schedulability` | the task-set and schedulability claim forms (FR-246), family `real-time` |

This table replaces the two `none` rows of FR-057's applicability table: since
FR-186 and FR-246 exist, the probabilistic and real-time families have
declaration forms.

- `analyze` SHALL check each item's kind against its claim's family before
  negotiation consumes the item, and SHALL settle an item whose kind does not
  apply as `invalid-request` with `invalid_capability`/`inapplicable-kind`,
  naming the kind and the family, and no engine SHALL run it.
- An inapplicable item SHALL keep its `request_index` and its place among the
  records (FR-281), and SHALL NOT withhold, alter or re-check the declaration
  body, which has already reached its family checker.
- An item whose kind applies SHALL be unaffected by the check, whatever the
  disposition the caller supplied.
- A `required` inapplicable request SHALL make a complete aggregate result
  unavailable, joined from the FR-331 accounting records on request index
  (FR-036-AC-6).
- The family projection SHALL be total: every `Capability` variant has an entry
  in the table above, and a variant added without one fails to compile
  (FR-063).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-507-AC-1 | For each of the twelve kinds and each of the six families the table names, exactly the tabled (kind, family) pairs are applicable: of the 72 pairs, 12 are applicable and 60 settle `invalid-request` with `invalid_capability`/`inapplicable-kind` naming the kind and the claim's family. | Test (TC-1302) |
| FR-507-AC-2 | A required `operation-contract` item on a state clause's claim is applicable and runs; a `finite-replay` item (not required) on the same claim settles `invalid-request` naming `state-model`; the state declaration's clause is still checked and its record unchanged. | Test (TC-1302) |
| FR-507-AC-3 | Each of the seven protocol kinds on a `protocol` declaration's claim is applicable; each of the seven on a state claim, a value function claim and a temporal clause claim settles `invalid-request` naming `state-model`, `value` and `temporal-trace` respectively. | Test (TC-1302) |
| FR-507-AC-4 | A `probabilistic-satisfaction` item on a probabilistic claim and a `schedulability` item on a task-set claim are applicable; the same kinds on a state claim settle `invalid-request` naming `state-model`. | Test (TC-1302) |
| FR-507-AC-5 | Two items, `operation-contract` on a state claim (`supported`) and `finite-replay` required on the same claim, give two records at indices 0 and 1 in caller order; the aggregate is unavailable, and an engine ran only for index 0. | Test (TC-1302) |
| FR-507-AC-6 | A `Capability` variant with no applicable-family entry does not compile under `--cfg seam_probe`. | Test (TC-1302) |

## Dependencies

- **Upstream:** FR-057 (kinds, FR-057-AC-13 projection), FR-281 (items and
  records), FR-290 (families).
- **Contract change required (QSpec FR-271/272):** the cause
  `inapplicable-kind` under `invalid_capability` must be added to the QSpec
  catalog; until then the criteria that name it cannot pass. If QSpec instead
  names an existing code, this requirement takes that code and the criteria are
  reworded, not dropped.
