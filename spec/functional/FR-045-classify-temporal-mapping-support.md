---
id: FR-045
title: "Classify native-to-TL temporal mapping support without a bridge"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-specification/FR-095, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
---
# FR-045: Classify native-to-TL temporal mapping support without a bridge

## Description

When a caller requests a native-to-TL mapping for an admitted temporal
declaration, the compiler SHALL classify that request against the reviewed
correspondence support table and return either a supported classification with
its required premises, or an unsupported classification naming every unmatched
semantic dimension against the retained native subject.

## Semantic authority and boundary

The support table is owned by `ix://agent-ix/quire-specification/FR-095`. This
requirement implements the classification only; it emits no TL formula, no
valuation request and no correspondence record, and it establishes no bridge.

The classification SHALL be a total function of three inputs and nothing else:
the declaration's selected profile, the operator kinds reachable from its root,
and the surrounding-execution closure named in the request. The first two come
from the admitted declaration; the third comes from the request, because the
support table keys on it. It SHALL NOT consult a backend capability report, an
installed TL version, a syntax match or a historical result. This keeps the classification decidable in this repository
while `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL capability
remain outstanding; the emission half of FR-095 stays outside this scope.

### Open question referred to the IR/TL owners

FR-095's support table uses two axis vocabularies in adjacent rows: its first row
keys on "complete execution" and its second on "open prefix". This requirement
normalizes both to surrounding-execution closure, which is unambiguous for the
first row. For the second it is a **selection**, not a restatement: the target's
own name, `mltl.online-prefix/v1`, and FR-091's "open decision scope" language
both point instead at decision-scope openness, and
[FR-043](./FR-043-evaluate-bounded-native-temporal.md)-AC-10 forbids substituting
one axis for the other. Reading that row as decision-scope openness would change
which target an open-scoped, completely-executed declaration maps to. The
selection is recorded on compiler
[#38](https://github.com/agent-ix/quire-spec-language/issues/38) for the IR/TL
owners to rule on, and SHALL be changed to match that ruling.

## Inputs

An admitted temporal declaration: its selected profile identity and revision, and
the operator kinds reachable from its root. From the request: the
surrounding-execution closure the mapping is asked for. Decision-scope closure is
a separate axis under
[FR-043](./FR-043-evaluate-bounded-native-temporal.md)-AC-10 and is not
substituted for it.

## Outputs

`Supported` with the required bridge premises named, or `Unsupported` carrying
every unmatched semantic dimension, the retained native declaration subject, its
selected profile identity and its activation record.

## Behavior

The classifier SHALL use exactly this table, which restates the reviewed
correspondence disposition in
`ix://agent-ix/quire-specification/FR-095` at baseline
`4d6230eb8aa9766ff3017360962f2d6368d74cb3`. The rows are evaluated in order and
the first two are mutually exclusive with the last two, so the table is total:

| Native request | TL target | Disposition |
| --- | --- | --- |
| Timestamped-event finite window | none | Unsupported on the finite-window dimension; no index conversion is inferred, and this subsumes the source table's separate finite-window-against-index-profile row |
| Any bounded past operator reachable from the root | none | Unsupported on the past-operator dimension, until a separately reviewed TL past profile exists |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, complete surrounding execution | `mltl.closed-trace/v1` | Supported, subject to every outstanding bridge premise |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, open surrounding execution | `mltl.online-prefix/v1` | Supported, subject to every outstanding bridge premise |
| Fixed-sample false-extension, at least one bounded future operator reachable and no past operator | the same two targets, selected by surrounding-execution closure | Supported only with one total valuation per required sample and retained epoch, period and unit correspondence |
| Either false-extension profile, no bounded temporal operator reachable at all | the same two targets, selected by surrounding-execution closure | Supported; a formula with no bounded operator imposes no additional TL obligation |

The first two rows accumulate: a declaration matching both SHALL return both
dimensions. The last four are mutually exclusive, so every admitted declaration
matches exactly one row and each row is reachable by some declaration.

The classifier SHALL name every unmatched dimension on an unsupported result,
rather than a single summary cause.

The classifier SHALL name the TL target identity of a supported classification
exactly as the table gives it, and SHALL record the source table's baseline
revision alongside it, so a later revision of that table is a visible change
rather than silent staleness.

The classifier SHALL retain the native declaration subject, its selected profile
identity and its activation record on every unsupported result, and SHALL NOT
substitute a profile, clock or reduced native form.

The classifier SHALL NOT report a declaration as supported because its formula
bytes match a TL formula; equal formula bytes establish no definition selection.

For a supported classification, the classifier SHALL name the premises the
absent bridge would still have to discharge: total Boolean predicate projection,
preserved source and clause identity, preserved model, type and predicate
bindings, preserved evaluation anchor and immutable capture environment,
preserved clock and observation binding, preserved interval, preserved closure
and history premises, and the result dimensions the selected TL wire does not
encode. Naming them is not discharging them.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-045-AC-1 | Every row of the support table is reachable and is exercised by an admitted declaration that matches it and no other, returning that row's disposition and TL target; the classification is derived only from the selected profile, the reachable operator kinds and the requested surrounding-execution closure, with no backend capability report, installed version, syntax match or decision-scope closure consulted. | Test (TC-125); Inspection |
| FR-045-AC-2 | A timestamped-event request and a bounded-past request each return unsupported naming the finite-window and past-operator dimensions respectively, and emit no substitute formula, profile or clock. | Test (TC-125) |
| FR-045-AC-3 | An unsupported result retains the native declaration subject, its selected profile identity and revision, and its activation record; a source-valid declaration is unchanged by the classification. | Test (TC-125) |
| FR-045-AC-4 | An unsupported result names every unmatched dimension rather than one summary cause, and a declaration unmatched on two dimensions names both. | Test (TC-125) |
| FR-045-AC-5 | A supported classification names its outstanding bridge premises, its TL target identity and the source table's baseline revision, and asserts no correspondence; two declarations with equal temporal formula bytes but different selected profiles do not receive the same classification. | Test (TC-125) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) supplies
  the admitted declaration, its profile selection and its operation graph.
- **Peer**: [FR-043](./FR-043-evaluate-bounded-native-temporal.md) evaluates
  natively and is unaffected by this classification.
- **Blocked, recorded as remaining work on compiler #38**: the emission half of
  `ix://agent-ix/quire-specification/FR-095` — the TL formula, valuation request
  and correspondence record — depends on `quire-contract-ir#63`,
  `quire-contract-ir#64` and actual TL capability, and is not implemented here.
