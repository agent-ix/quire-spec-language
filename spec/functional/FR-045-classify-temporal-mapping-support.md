---
id: FR-045
title: "Classify native-to-TL temporal mapping support without a bridge"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
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

The disposition SHALL be a total function of three inputs and nothing else:
the declaration's selected profile, the operator kinds reachable from its root,
and the surrounding-execution closure named in the request. The first two come
from the admitted declaration; the third comes from the request, because the
support table keys on it. It SHALL NOT consult a backend capability report, an
installed TL version, a syntax match or a historical result. This keeps the classification decidable in this repository
while `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL capability
remain outstanding; the emission half of FR-095 stays outside this scope.

### Closure axis ruling

FR-095's support table uses two axis vocabularies in adjacent rows: its first row
keys on "complete execution" and its second on "open prefix". The IR/TL owner
ruled on this in
[`quire-contract-ir#64`](https://github.com/agent-ix/quire-contract-ir/issues/64#issuecomment-5649214124):
surrounding-execution closure, complete execution against open prefix, selects
the TL row, and decision-scope closure is not the selector. Both FR-095 rows
therefore key on surrounding-execution closure. The native result model keeps
both axes and removes neither; the ruling decides only which axis selects the TL
row, so [FR-043](./FR-043-evaluate-bounded-native-temporal.md)-AC-10 still forbids
substituting one axis for the other.

### Authenticated classification

A compiled-protocol `/1` package carries no authenticated temporal definition
selection, so a classification made through it retains none. Through a strict
`quire.compiled-protocol/2` package under
[FR-050](./FR-050-publish-authenticated-temporal-artifacts.md), the classifier
SHALL resolve the selection through the declaration's admitted temporal binding,
SHALL refuse a declaration with no binding or whose binding selects a definition
other than the declaration's own, and SHALL retain the selection it resolved: the
admitted package digest, the declaration, the definition identity and namespaced
revision, and the digest of the exact definition artifact the package selects.
The strict reader already refuses a temporal declaration with no binding and a
binding that selects another definition, so both refusals are inspected
re-checks rather than reachable test paths. The
retained selection SHALL NOT be constructible outside the compiler, and SHALL
carry no clock parameter map, because classification takes no trace and so
authenticates none. Only the selection is sealed: the disposition and retained
native subject stay plain data, so a consumer that needs the disposition bound to
the selection re-classifies from the package the selection names. Authentication
SHALL NOT change the disposition: the same declaration classifies identically
through either version.

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
| FR-045-AC-6 | A declaration classified through a strict `/2` package returns the same disposition and retained native subject as through `/1` under both surrounding-execution closures, and additionally retains the authenticated package digest, declaration, definition identity, namespaced revision and definition artifact digest; a `/1` classification retains no authenticated selection, a `/2` request for a declaration outside the package or for a non-temporal declaration is refused, and the classifier refuses a temporal declaration with no binding or a binding that selects a definition other than the declaration's own (Inspection; unreachable after the strict reader). | Test (TC-125); Inspection |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) supplies
  the admitted declaration, its profile selection and its operation graph.
- **Upstream**: [FR-050](./FR-050-publish-authenticated-temporal-artifacts.md)
  supplies the strict `/2` package whose temporal binding authenticates the
  definition selection an authenticated classification retains.
- **Ruling**: `quire-contract-ir#64` fixes surrounding-execution closure as the
  TL row selector.
- **Peer**: [FR-043](./FR-043-evaluate-bounded-native-temporal.md) evaluates
  natively and is unaffected by this classification.
- **Blocked, recorded as remaining work on compiler #38**: the emission half of
  `ix://agent-ix/quire-specification/FR-095` — the TL formula, valuation request
  and correspondence record — depends on `quire-contract-ir#63`,
  `quire-contract-ir#64` and actual TL capability, and is not implemented here.

## Status

The reviewed correspondence table this requirement restates is retained as
the `SUPPORT_TABLE` constant in `src/temporal/mapping.rs`, tagged with the
same `ix://agent-ix/quire-specification/FR-095` baseline
(`4d6230eb8aa9766ff3017360962f2d6368d74cb3`) this requirement's Behavior
section cites; the classifier reads that constant, never a duplicated copy.
The baseline stays pinned to that revision until
`ix://agent-ix/quire-specification#112` lands a reviewed revision of the
source table; only a landed revision moves this pin.
