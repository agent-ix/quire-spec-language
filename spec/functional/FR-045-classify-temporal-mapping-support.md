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

The classification SHALL be a total function of the declaration's selected
profile, its temporal operator set and its decision-scope closure. It SHALL NOT
consult a backend capability report, an installed TL version, a syntax match or a
historical result. This keeps the classification decidable in this repository
while `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL capability
remain outstanding; the emission half of FR-095 stays outside this scope.

## Inputs

An admitted temporal declaration: its selected profile identity and revision, the
operator kinds reachable from its root, and the requested decision-scope closure
of the mapping request.

## Outputs

`Supported` with the required bridge premises named, or `Unsupported` carrying
every unmatched semantic dimension, the retained native declaration subject, its
selected profile identity and its activation record.

## Behavior

The classifier SHALL use exactly this table, which restates the reviewed
correspondence disposition:

| Native request | Disposition |
| --- | --- |
| Event-position false-extension, bounded future operators only, closed decision scope | Supported, subject to every bridge premise |
| Event-position false-extension, bounded future operators only, open prefix | Supported, subject to every bridge premise |
| Fixed-sample false-extension, bounded future operators only | Supported only with one total valuation per required sample and retained epoch, period and unit correspondence |
| Timestamped-event finite window, any request | Unsupported; no index conversion is inferred |
| Any bounded past operator, any profile | Unsupported until a separately reviewed TL past profile exists |
| Finite-window future semantics against an index-based TL profile | Unsupported, because that profile extends atoms false beyond closure |

The classifier SHALL name every unmatched dimension on an unsupported result,
rather than a single summary cause.

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
| FR-045-AC-1 | Every row of the support table is exercised by an admitted declaration and returns that row's disposition; the classification is derived from the declaration alone, with no backend capability report, installed version or syntax match consulted. | Test (TC-125), Inspection |
| FR-045-AC-2 | A timestamped-event request and a bounded-past request each return unsupported naming the finite-window and past-operator dimensions respectively, and emit no substitute formula, profile or clock. | Test (TC-125) |
| FR-045-AC-3 | An unsupported result retains the native declaration subject, its selected profile identity and revision, and its activation record; a source-valid declaration is unchanged by the classification. | Test (TC-125) |
| FR-045-AC-4 | An unsupported result names every unmatched dimension rather than one summary cause, and a declaration unmatched on two dimensions names both. | Test (TC-125) |
| FR-045-AC-5 | A supported classification names its outstanding bridge premises and asserts no correspondence; two declarations with equal temporal formula bytes but different selected profiles do not receive the same classification. | Test (TC-125) |

## Dependencies

- **Upstream**: [FR-042](./FR-042-publish-compiled-protocol-artifacts.md) supplies
  the admitted declaration, its profile selection and its operation graph.
- **Peer**: [FR-043](./FR-043-evaluate-bounded-native-temporal.md) evaluates
  natively and is unaffected by this classification.
- **Blocked, recorded as remaining work on compiler #38**: the emission half of
  `ix://agent-ix/quire-specification/FR-095` — the TL formula, valuation request
  and correspondence record — depends on `quire-contract-ir#63`,
  `quire-contract-ir#64` and actual TL capability, and is not implemented here.
