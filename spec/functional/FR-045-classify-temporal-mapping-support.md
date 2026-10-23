---
id: FR-045
title: "Classify native-to-TL temporal mapping support and establish the TL correspondence"
type: FR
relationships:
  - { target: ix://agent-ix/quire-spec-language/US-004, type: implements }
  - { target: ix://agent-ix/quire-spec-language/FR-042, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-050, type: depends_on }
  - { target: ix://agent-ix/quire-spec-language/FR-043, type: references }
  - { target: ix://agent-ix/quire-specification/FR-095, type: depends_on }
  - { target: ix://agent-ix/quire-specification/FR-048, type: references }
  - { target: ix://agent-ix/tl-syntax/FR-011, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-012, type: depends_on }
  - { target: ix://agent-ix/tl-mltl/FR-007, type: depends_on }
---
# FR-045: Classify native-to-TL temporal mapping support and establish the TL correspondence

## Description

When a caller requests a native-to-TL mapping for an admitted temporal
declaration, the compiler SHALL classify that request against the reviewed
correspondence support table and return either a supported classification with
its required premises, or an unsupported classification naming every unmatched
semantic dimension against the retained native subject. For a supported
classification of a declaration of a strict `quire.compiled-protocol/2`
package, the compiler SHALL generate the TL formula and valuation request of
`ix://agent-ix/quire-specification/FR-095`, have each owner reader admit them,
and join the native and TL results through one source/result-correspondence
record (the bridge profile `quire.native-temporal-correspondence/v1`).

## Semantic authority and boundary

The support table and the correspondence meaning are owned by
`ix://agent-ix/quire-specification/FR-095`. This requirement classifies a
request, and for a supported classification it establishes the correspondence
("Correspondence" below): it generates the TL artifacts, discharges every
premise the classification names, and joins the two results. It owns the
correspondence, the construction and the agreement decision. It owns neither
evaluator: FR-043 evaluates natively, and the TL owners (tl-syntax, tl-mltl and
quire-mltl) read and evaluate the TL artifacts. The compiler invokes no TL
evaluator. A classification alone asserts no correspondence.

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

An admitted temporal declaration: its selected profile identity, and
the operator kinds reachable from its root. From the request: the
surrounding-execution closure the mapping is asked for. Decision-scope closure is
a separate axis under
[FR-043](./FR-043-evaluate-bounded-native-temporal.md)-AC-10 and is not
substituted for it.

For the correspondence: the strict `/2` package and the declaration, the
`Supported` classification made through `mapping_support_v2` with its retained
authenticated selection, the FR-043 observation trace the native evaluation
reads, the exact TL owner contract selections (formula, trace or history,
request and result mapping), and caller-configured limits. For the join: the
validated projection, the FR-043 `evaluate_v2` report for the same declaration
and trace, the TL owner's mapped result for the generated request, and, for a
corrected pair, the prior validated join.

## Outputs

`Supported` with the required bridge premises named, or `Unsupported` carrying
every unmatched semantic dimension, the retained native declaration subject, its
selected profile identity and its activation record. For a supported
classification: a projection decision holding the generated TL formula, the
valuation request and the correspondence record, or a typed non-admitted
decision; and a join decision over the two results.

## Behavior

The classifier SHALL use exactly this table, which restates the reviewed
correspondence disposition in
`ix://agent-ix/quire-specification/FR-095`. The rows are evaluated in order and
the first two are mutually exclusive with the last five, so the table is total:

| Native request | TL target | Disposition |
| --- | --- | --- |
| Timestamped-event finite window | none | Unsupported on the finite-window dimension; no index conversion is inferred, and this subsumes the source table's separate finite-window-against-index-profile row |
| Both a bounded past and a bounded future operator reachable from the root | none | Unsupported on the mixed-direction dimension; no TL profile evaluates both directions in one formula |
| Event-position or fixed-sample false-extension, at least one bounded past operator reachable and no future operator | `mltl.origin-complete-history/v1`, with formula `tl-syntax.formula/v2` and catalog `tl-syntax.past-operators/v1` | Supported, subject to every bridge premise, an origin-complete history among them |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, complete surrounding execution | `mltl.closed-trace/v1` | Supported, subject to every outstanding bridge premise |
| Event-position false-extension, at least one bounded future operator reachable and no past operator, open surrounding execution | `mltl.online-prefix/v1` | Supported, subject to every outstanding bridge premise |
| Fixed-sample false-extension, at least one bounded future operator reachable and no past operator | the same two targets, selected by surrounding-execution closure | Supported only with one total valuation per required sample and retained epoch, period and unit correspondence |
| Either false-extension profile, no bounded temporal operator reachable at all | the same two targets, selected by surrounding-execution closure | Supported; a formula with no bounded operator imposes no additional TL obligation |

The first two rows accumulate: a declaration matching both SHALL return both
dimensions. The last five are mutually exclusive, so every admitted declaration
matches exactly one row and each row is reachable by some declaration.

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

## Correspondence

### API

```text
temporal::project_tl(&protocol_artifact::v2::AdmittedPackage, declaration,
  &MappingSupport, &Trace, TlSelection, Limits) -> TlProjectionDecision
temporal::read_tl_projection(bytes, ExpectedTlProjection, Limits)
  -> Result<ValidatedTlProjection, TlDecision>
temporal::join_tl(&ValidatedTlProjection, Option<&temporal::Report>,
  Option<&TlMappedResult>, Option<&ValidatedTlJoin>, Limits) -> TlJoinDecision
temporal::read_tl_join(bytes, ExpectedTlJoin, Limits)
  -> Result<ValidatedTlJoin, TlDecision>
```

No entry point accepts a parser, evaluator, callback, trust flag or raw
mirrored owner record.

### Admission and premises

- `project_tl` SHALL accept only a `Supported` classification whose retained
  authenticated selection names the package digest, declaration, definition
  identity and revision of the supplied `/2` package, and SHALL take the TL
  target exactly from the classification. An `Unsupported` classification
  SHALL yield `unsupported` naming every dimension the classification names,
  with no formula, request or record.
- `project_tl` SHALL discharge every premise the classification names. It
  SHALL refuse, with the premise as the typed cause and no partial artifact, a
  trace whose profile, clock name or parameter map differs from the package's
  authenticated binding (FR-043 `/2` authentication), a `holds` leaf whose
  valuation is not a total Boolean, or a capture, anchor or interval that
  differs from the admitted declaration.
- Decision-scope closure, surrounding-execution closure, assessment
  execution, input completeness and availability SHALL stay independent
  assertions. Surrounding-execution closure alone selects between
  `mltl.closed-trace/v1` and `mltl.online-prefix/v1`. A closed but incomplete
  trace yields no false-padded request.
- For a past target, the trace's lower boundary SHALL be an authoritative
  execution origin (FR-043 Inputs). A history-cutoff boundary SHALL yield
  `incomplete` with cause `history-not-origin-complete`, not `unsupported`,
  and authorizes no pre-origin false extension.

### Formula and valuation construction

- Construction SHALL traverse the declaration's closed temporal operation
  graph in left-to-right postorder, keep every occurrence without
  deduplication, assign contiguous operand-before-consumer ids, and keep every
  inclusive `[a, b]` with `0 <= a <= b <= u32::MAX`.
- Each `holds` leaf handle SHALL map to one TL proposition id distinct from
  every native predicate, signal and capture identity. A numeric, nullable,
  unavailable or pending value SHALL NOT enter the proposition population.
- For every required position and proposition, the projection SHALL take
  exactly one explicit Boolean valuation cell from the trace position's
  valuation map, bound to that position, anchor, capture and clock
  coordinate. A false cell is explicit even where the TL wire encodes it by
  omission. A missing required cell yields `incomplete`. A duplicate or
  foreign cell refuses before either evaluator input exists.
- A future-only declaration SHALL emit `tl-syntax.formula/v1` and a
  `tl-mltl.trace/v1` request under the classification's target, with the
  lower-bound convention and false-extension rule of QSpec FR-091.
- A past-only declaration SHALL emit `tl-syntax.formula/v2` under
  `tl-syntax.past-operators/v1`, and `tl-mltl.position-history/v1` with
  `tl-mltl.history-requirement/v1` under `mltl.origin-complete-history/v1`.
- A fixed-sample declaration SHALL retain the authenticated epoch, period and
  unit, report an absent sample as `incomplete`, and refuse a changed period
  or an event-count reinterpretation even where the formula bytes match.
- Each generated artifact SHALL be admitted by its owner's public reader
  before the projection is returned. The formula, valuation, trace or
  history, request and correspondence identities SHALL each hash their own
  canonical bytes, and none substitutes for another. The native source and
  bridge-definition digests SHALL use QSpec FR-201's `raw-artifact-digest`.
- The correspondence record SHALL preserve FR-095's complete key: source
  digest and clause span, clause identity, language and profile definition,
  model, type and predicate bindings, anchor, captures, clock and observation
  binding, interval, closure and history premises, the TL formula, profile
  and proposition identities, and the bridge definition. Its derived
  reference SHALL change with every key mutation and SHALL NOT reuse a native
  or TL identity.

### Join

- `join_tl` SHALL verify the native report (FR-043 `evaluate_v2`, carrying
  its authenticated binding) and the TL mapped result against the projection,
  and then compare normalized structures, never owner label strings.
- The decision SHALL be one row of this table:

| Validated mapped state | Join decision |
| --- | --- |
| both values equal and every applicable premise and support equal | `agreement` / `equal-final`, that Boolean |
| both pending under an open decision scope, equal | `agreement` / `equal-pending`, no Boolean |
| a required result or producer unavailable | `unavailable` / `not-compared` |
| either result unsupported | `unsupported` / `not-compared` |
| either result incomplete | `incomplete` / `not-compared` |
| either result failed | `failed` / `not-compared` |
| either result refused, or a stale or wrong subject | `refused` / `not-compared` |
| unequal values or premises, contradicted deciding support, or an identity collision | `conflict` / `mismatch` |

- A non-final decision SHALL carry no Boolean and a nonempty ordered cause
  set. Assessment execution, both progress and closure axes, settlement
  basis, decision support, completeness and supersession SHALL be preserved,
  and omitting or mutating any of them SHALL NOT yield `agreement`.
- A corrected pair SHALL require the strict-read prior join whose native and
  TL source-result identities equal the two direct predecessors. A mixed
  original and corrected pair, a missing prior join or a predecessor mismatch
  SHALL be `conflict`. A correction never rewrites a prior join.

### Strict reading and limits

`read_tl_projection` and `read_tl_join` SHALL reject unknown, duplicate,
missing or out-of-order members, noncanonical or trailing bytes, identity or
digest mismatches and cross-wired positions, scopes or clocks. Limits SHALL
bound bytes, depth, nodes, positions, valuations, support, causes and total
work, each charged before the work it pays for. A limit reached exactly
admits, and one past it returns a non-admitted decision with no partial
artifact.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-045-AC-1 | Every row of the support table is reachable and is exercised by an admitted declaration that matches it and no other, returning that row's disposition and TL target; the classification is derived only from the selected profile, the reachable operator kinds and the requested surrounding-execution closure, with no backend capability report, installed version, syntax match or decision-scope closure consulted. | Test (TC-125); Inspection |
| FR-045-AC-2 | A timestamped-event request returns unsupported naming the finite-window dimension, and a request reaching both a bounded past and a bounded future operator returns unsupported naming the mixed-direction dimension; neither emits a substitute formula, profile or clock. A past-only declaration under each false-extension profile is supported with target `mltl.origin-complete-history/v1`. | Test (TC-125) |
| FR-045-AC-3 | An unsupported result retains the native declaration subject, its selected profile identity, and its activation record; a source-valid declaration is unchanged by the classification. | Test (TC-125) |
| FR-045-AC-4 | An unsupported result names every unmatched dimension rather than one summary cause, and a declaration unmatched on two dimensions names both. | Test (TC-125) |
| FR-045-AC-5 | A supported classification names its outstanding bridge premises, and its TL target identity, and asserts no correspondence; two declarations with equal temporal formula bytes but different selected profiles do not receive the same classification. | Test (TC-125) |
| FR-045-AC-6 | A declaration classified through a strict `/2` package returns the same disposition and retained native subject as through `/1` under both surrounding-execution closures, and additionally retains the authenticated package digest, declaration, definition identity, namespaced revision and definition artifact digest; a `/1` classification retains no authenticated selection, a `/2` request for a declaration outside the package or for a non-temporal declaration is refused, and the classifier refuses a temporal declaration with no binding or a binding that selects a definition other than the declaration's own (Inspection; unreachable after the strict reader). | Test (TC-125); Inspection |
| FR-045-AC-7 | Over a strict `/2` package with the event-position declaration `always[0,1] holds(p)` classified `Supported`, `project_tl` emits a `tl-syntax.formula/v1` formula and a `tl-mltl.trace/v1` request that their owner readers admit, and on every member of QSpec FR-095-AC-1's exhaustive bounded-future formula and trace population `join_tl` over the `evaluate_v2` report and the TL result returns `agreement` with the native truth. | Test (TC-1240) |
| FR-045-AC-8 | The one-position `always[0,1] p` vector rejects a finite-window-to-`mltl.closed-trace/v1` equivalence claim; an `Unsupported` classification (timestamped-event, mixed-direction or infinite-trace) yields `unsupported` with no formula; a past-only `once[0,2] holds(p)` declaration emits `tl-syntax.formula/v2` and a position history, and with a history-cutoff trace boundary yields `incomplete`, `history-not-origin-complete`. | Test (TC-1240) |
| FR-045-AC-9 | Mutating the profile, source span, predicate binding, clock, capture, interval, closure, history or bridge identity changes the correspondence reference and names that axis; a numeric, nullable or pending leaf value cannot become a proposition; the formula, valuation, request and record identities each change only with their own inputs. | Test (TC-1240) |
| FR-045-AC-10 | A fixed-sample declaration's projection retains the authenticated epoch, period and unit; a trace with a changed period refuses even with equal formula bytes; an absent required sample yields `incomplete`. | Test (TC-1240) |
| FR-045-AC-11 | Each join-table row is reached; every non-final decision carries no Boolean and a nonempty cause set; omitting any preserved axis from either result cannot yield `agreement`; a corrected pair without the prior join, or with a predecessor mismatch, is `conflict`. | Test (TC-1240) |
| FR-045-AC-12 | A missing, duplicate or foreign valuation cell, an owner-reader rejection, a noncanonical projection or join, and each limit reached exactly and one past it give one deterministic decision, the over-limit and malformed cases non-admitted with no partial artifact and no evaluator invoked by the compiler. | Test (TC-1240) |

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
- **Peer**: FR-043's `evaluate_v2` report is the native result the join
  reads.
- **TL owners**: tl-syntax FR-011 and FR-012 (the formula editions and the
  past-operator catalog), tl-mltl FR-007 (trace and history wire) and
  quire-mltl's request and result mapping read and evaluate the generated
  artifacts.
