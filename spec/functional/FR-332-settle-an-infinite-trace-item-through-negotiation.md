---
id: FR-332
title: "Settle an infinite-trace item only through negotiation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-332: Settle an infinite-trace item only through negotiation

## Description

QSL SHALL send every infinite-trace temporal item to negotiation and settle
it only from a negotiated backend's result (ADR-014 §5 A-5, §6, TR-8). The
request writer SHALL classify the item `unbounded` with no available finite
bound. With no backend that advertises (`temporal-satisfaction`,
`unbounded`) and discharges the infinite-trace form, the item SHALL settle
`unsupported` with a warning naming the capability (QSpec FR-161-AC-7). A
backend's QSpec FR-360 result SHALL map to one O-16
category. S6a evaluation of the clause on a trace (FR-328, FR-329) is
evidence for that trace and never settles the item.

## Use case

An author writes `eventually holds(c.value = 3)` and requests it. Without a
liveness backend, they get `unsupported` and a warning that names
`temporal-satisfaction`, not a silent pass and not a bounded check
presented as a proof. When a liveness backend is registered, the same
request routes to it and its result lands in the matching category.

## Inputs

- The clause's requirement record (FR-326): (`temporal-satisfaction`,
  `Unbounded{[formula]}`).
- The registry of backend descriptors (FR-075) and CG `negotiate_*`
  (ADR-012 §7.1).
- A routed backend's QSpec FR-360 result: label and
  QSpec FR-241 execution.

## Outputs

- A requested item classified `unbounded`, `finite_bound_available: false`.
- A disposition from `negotiate_*`.
- The item's O-16 category.

## Behavior

- The request writer SHALL classify an infinite-trace item `unbounded` with
  `finite_bound_available` false, because no `FiniteBound` can stand for an
  infinite-trace formula (ADR-014 §4, FR-097-AC-3), and SHALL refuse a
  bounded request for it as FR-097-AC-4 states.
- When no registrant advertises `temporal-satisfaction`, the registry SHALL
  pass the empty candidate set unchanged (FR-076), and the item SHALL settle
  `unsupported`, warned, naming `temporal-satisfaction`.
- When the only candidate advertises `temporal-satisfaction` in mode
  `bounded` only, the item SHALL settle `unsupported`, warned,
  `unbounded-extent`, naming the kind, the candidate and its modes (QSpec
  FR-290).
- When a candidate advertises (`temporal-satisfaction`, `unbounded`) and its
  arm discharges the infinite-trace form, the item SHALL settle `supported`
  and route to it; over a model subject that candidate is QSL's
  explicit-state model checker (ADR-018 EN-1, FR-126), whose outcome FR-127
  settles.
- A routed backend's result SHALL map to O-16 as ADR-014 A-5 states:
  `proved` to success, `refuted` to violation, `inconclusive` to
  inconclusive, `unsupported` to unsupported, `failed` with execution
  `resource-incomplete` to incomplete, and every other `failed` to internal
  failure.
- An S6a result for the clause on a trace SHALL be `tested` or violation for
  that trace only; the QSpec FR-331 accounting record SHALL join it to no
  requested item, so it settles no item (ADR-014 §8).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-332-AC-1 | `Reaches` (`eventually holds(c.value = 2)`, infinite-trace) is written as one item classified `unbounded` with `finite_bound_available` false; a bounded request for it with any `FiniteBound` refuses `invalid_runtime_input`/`invalid-value` and writes no item. | Test (TC-842) |
| FR-332-AC-2 | With an empty registry, `Reaches` settles `unsupported` with a warning naming `temporal-satisfaction`; with only a test descriptor advertising (`temporal-satisfaction`, `bounded`), it settles `unsupported`, warned, `unbounded-extent`, naming the kind, the descriptor and its mode; with a test descriptor advertising (`temporal-satisfaction`, `unbounded`) whose arm takes the infinite-trace form, it settles `supported` and routes to that descriptor. | Test (TC-842) |
| FR-332-AC-3 | Each FR-360 label maps to its O-16 category as the Behavior section's ADR-014 A-5 mapping states, including `failed` with `resource-incomplete` to incomplete and `failed` with another execution to internal failure. | Test (TC-842) |
| FR-332-AC-4 | Running `Reaches` at S6a on a lasso where it is true gives `tested`; the accounting record for the AC-2 empty-registry request still holds the item `unsupported`. | Test (TC-842) |

## Dependencies

- ADR-014 §3 TR-8, §4, §5 A-3 and A-5, §6, §8, §10 scenario 1, each as
  amended by ADR-018 §3.
- [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md)
  (extent classification and bounded-request refusals),
  [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-076](FR-076-settle-backend-absence-as-unsupported.md),
  [FR-326](FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md).
- CG `negotiate_*` settles the dispositions; AC-2 runs in a test harness
  downstream of CG with test descriptors, as ADR-014 §6 step 5 states.
- QSpec FR-161-AC-7, FR-290, FR-360, FR-331.

## References

- Linear QSL-384 (spec ticket); QSL-43 (implementation).
- QSpec FR-368 (negotiating temporal model-check items): Linear STD-131
  (QS-9).
