---
id: FR-045
title: "Classify native-to-TL temporal mapping support"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-specification/FR-095, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
  - { target: ix://agent-ix/quire-spec-language/ADR-011, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-011, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-013, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-014, type: depends_on }
  - { target: ix://agent-ix/tl-mltl/FR-018, type: depends_on }
  - { target: ix://agent-ix/quire-mltl/FR-001, type: depends_on }
  - { target: ix://agent-ix/quire-mltl/FR-003, type: depends_on }
  - { target: ix://agent-ix/quire-contract-ir/FR-350, type: references }
  - { target: ix://agent-ix/quire-specification/FR-092, type: depends_on }
---
# FR-045: Classify native-to-TL temporal mapping support

## Description

When a caller requests a native-to-TL mapping for an admitted temporal
declaration, the QSL compiler SHALL classify the request against the reviewed
support table and return either a supported classification with its outstanding
bridge premises or an unsupported classification naming every unmatched
semantic dimension and retaining the native subject. For a strict
`quire.compiled-protocol/2` package, the classification SHALL retain the
selection authenticated through QSL FR-050. Contract IR FR-350 consumes this
classification to construct and validate the TL correspondence.

## Semantic authority and boundary

QSpec FR-095 owns the correspondence meaning and support table. This QSL
requirement owns `mapping_support_v2` and the authenticated native classification
that Contract IR FR-350 consumes. A supported classification names outstanding
premises; it does not claim that a formula, valuation request or correspondence
record has been emitted. Contract IR owns that construction and the result join.
Neither QSL nor Contract IR owns the TL or native evaluator.

The disposition SHALL be a total function of three inputs and nothing else:
the declaration's selected profile, the operator kinds reachable from its root,
and the surrounding-execution closure named in the request. The first two come
from the admitted declaration; the third comes from the request, because the
support table keys on it. It SHALL NOT consult a backend capability report, an
installed TL version, a syntax match or a historical result, so the
classification is decided from the admitted package and the request alone,
whatever TL release or bridge is installed.

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
admitted package identity, the declaration, the definition identity and namespaced
revision, and the identity of the exact definition artifact the package selects.
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

An admitted temporal declaration with its selected profile identity and
reachable temporal operator kinds; the requested surrounding-execution closure;
and, for a strict `/2` package, the package's authenticated temporal binding.
Decision-scope closure remains a separate axis under QSL FR-043-AC-10 and SHALL
NOT be substituted for surrounding-execution closure.

## Outputs

A `Supported` classification with the required bridge premises and exact TL
target identity, or an `Unsupported` classification carrying every unmatched
semantic dimension, the retained native declaration subject, its selected
profile identity and its activation record. A strict `/2` classification also
retains the authenticated package identity, declaration, definition identity and
revision, and exact definition artifact identity. The classification makes no
claim that the Contract IR bridge has emitted or joined an artifact.

## Behavior

The classifier SHALL use exactly this table, which restates the reviewed
correspondence disposition in
`ix://agent-ix/quire-specification/FR-095`. The admitted temporal profiles are
`quire.temporal.event-position.false-extension/v1`,
`quire.temporal.fixed-sample.false-extension/v1`,
`quire.temporal.timestamped-event.finite-window/v1` and
`quire.temporal.infinite-trace/v1`. The rows are evaluated in order, and the
first three are mutually exclusive with the last five, so the table is total
over those four profiles:

| Native request | TL target | Disposition |
| --- | --- | --- |
| `quire.temporal.infinite-trace/v1` | none | Unsupported on the infinite-trace dimension; there is no bounded TL formula to generate, and QSpec FR-161 analyzes the request natively |
| Timestamped-event finite window | none | Unsupported on the finite-window dimension; no index conversion is inferred, and this subsumes the source table's separate finite-window-against-index-profile row |
| Both a bounded past and a bounded future operator reachable from the root | none | Unsupported on the mixed-direction dimension; no TL profile evaluates both directions in one formula |
| Event-position or fixed-sample false-extension, at least one bounded past operator reachable and no future operator | `mltl.origin-complete-history/v1`, with formula `tl-syntax.formula/v2` and catalog `tl-syntax.past-operators/v1` | Supported, subject to every bridge premise, an origin-complete history among them |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, complete surrounding execution | `mltl.closed-trace/v1` | Supported, subject to every outstanding bridge premise |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, open surrounding execution | `mltl.online-prefix/v1` | Supported, subject to every outstanding bridge premise |
| Fixed-sample false-extension, at least one bounded future operator reachable and no past operator | the same two targets, selected by surrounding-execution closure | Supported only with one total valuation per required sample and retained epoch, period and unit correspondence |
| Either false-extension profile, no bounded temporal operator reachable at all | the same two targets, selected by surrounding-execution closure | Supported; a formula with no bounded operator imposes no additional TL obligation |

The first three rows accumulate: a declaration matching more than one SHALL
return every matched dimension. The last five are mutually exclusive, so every admitted declaration
matches exactly one row and each row is reachable by some declaration.

The bounded-past operator set is exactly the one defined by QSpec FR-092. The
past-only row covers each member of that set; QSpec FR-092 defines strong
previous as `once[1,1]`. A request reaching any bounded past operator and any
bounded future operator matches the mixed-direction row.

The classifier SHALL name every unmatched dimension on an unsupported result,
rather than a single summary cause.

The classifier SHALL name the TL target identity of a supported classification
exactly as the table gives it.

The classifier SHALL retain the native declaration subject, its selected profile
identity and its activation record on every unsupported result, and SHALL NOT
substitute a profile, clock or reduced native form.

The classifier SHALL NOT report a declaration as supported because its formula
bytes match a TL formula; equal formula bytes establish no definition selection.

For a supported classification, the classifier SHALL name the premises the
bridge has to discharge: total Boolean predicate projection,
preserved source and clause identity, preserved model, type and predicate
bindings, preserved evaluation anchor and immutable capture environment,
preserved clock and observation binding, preserved interval, preserved closure
and history premises, and the result dimensions the selected TL wire does not
encode. Naming them is not discharging them.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-045-AC-1 | Every row of the support table is reachable and is exercised by an admitted declaration that matches it and no other, returning that row's disposition and TL target; classification consults only the selected profile, reachable operator kinds and requested surrounding-execution closure. | Test (TC-125); Inspection |
| FR-045-AC-2 | Timestamped-event and mixed-direction requests return unsupported with every unmatched dimension; each bounded past operator defined by QSpec FR-092, alone under both false-extension profiles, is supported only with the `mltl.origin-complete-history/v1` target. | Test (TC-125) |
| FR-045-AC-3 | An unsupported result retains the native declaration subject, selected profile identity and activation record; a source-valid declaration is unchanged by classification. | Test (TC-125) |
| FR-045-AC-4 | An unsupported result names every unmatched dimension rather than one summary cause, including a declaration unmatched on two dimensions. | Test (TC-125) |
| FR-045-AC-5 | A supported classification names its exact TL target and every outstanding bridge premise, and does not claim correspondence; equal formula bytes under different selected profiles do not receive the same classification. | Test (TC-125) |
| FR-045-AC-6 | A strict `/2` classification equals the `/1` disposition and retained native subject under both surrounding-execution closures, and additionally retains the authenticated package identity, declaration, definition identity, namespaced revision and exact definition-artifact identity; a `/1` classification retains no authenticated selection; a `/2` request for an out-of-package or non-temporal declaration, a temporal declaration without a binding, or a binding that selects another definition is refused (the latter strict-reader defects are inspected as unreachable re-checks). | Test (TC-125); Inspection |

## Dependencies

- QSpec FR-095 owns the correspondence support meaning; QSpec FR-092 defines
  the complete bounded-past operator set and history semantics.
- QSL FR-050 supplies the strict `/2` package and authenticated temporal
  binding; QSL FR-043 supplies the native `temporal::evaluate_v2` report consumed by the
  bridge.
- Contract IR FR-350 consumes this classification and owns the TL projection,
  owner-reader validation, correspondence record and result join.
- TL owners supply formula, history/trace, request and mapped-result contracts
  to Contract IR; QSL classifies without consulting their installed versions.
