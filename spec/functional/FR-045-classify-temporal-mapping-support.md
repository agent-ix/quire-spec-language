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
  - { target: ix://agent-ix/quire-spec-language/ADR-011, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-011, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-013, type: depends_on }
  - { target: ix://agent-ix/tl-syntax/FR-014, type: depends_on }
  - { target: ix://agent-ix/tl-mltl/FR-018, type: depends_on }
  - { target: ix://agent-ix/quire-mltl/FR-001, type: depends_on }
  - { target: ix://agent-ix/quire-mltl/FR-003, type: depends_on }
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
  &MappingSupport, &Trace, TlSelection, TlLimits) -> TlProjectionDecision
temporal::read_tl_projection(bytes, ExpectedTlProjection, TlLimits)
  -> Result<ValidatedTlProjection, TlDecision>
temporal::join_tl(&ValidatedTlProjection, Option<&temporal::Report>,
  Option<&TlMappedResult>, Option<&ValidatedTlJoin>, TlLimits) -> TlJoinDecision
temporal::read_tl_join(bytes, ExpectedTlJoin, TlLimits)
  -> Result<ValidatedTlJoin, TlDecision>
```

No entry point accepts a parser, evaluator, callback, trust flag or raw
mirrored owner record.

- `TlSelection { formula, trace, request, result_map }` names the exact TL
  owner contracts, each a `DefinitionRef`: `formula` is
  `tl-syntax.formula/v1` (future) or `tl-syntax.formula/v2` (past, with the
  `tl-syntax.past-operators/v1` catalog); `trace` is `tl-mltl.trace/v1`
  (future) or `tl-mltl.position-history/v1` (past); `request` is the
  quire-mltl request contract (quire-mltl FR-001); `result_map` is
  `tl-mltl.contract-ir-result-map/v1` (quire-mltl FR-003). A selection whose
  members disagree with the classification's target refuses.
- `ExpectedTlProjection { package, declaration, selection, reference }`
  holds the `/2` package digest, the declaration index, the `TlSelection`
  and the expected correspondence reference, each supplied independently of
  the bytes read.
- `ExpectedTlJoin { reference, native, tl, predecessor }` holds the
  correspondence reference, the digest of the native report's canonical
  bytes, the digest of the TL mapped result's bytes, and the prior join's
  reference or none.
- `TlMappedResult` is quire-mltl FR-003's strict-read
  `tl-mltl.contract-ir-result-map/v1` view; no other TL result is admitted.
- `TlLimits` is described in "Strict reading and limits".

### Dependencies of the correspondence

The correspondence is the one QSL stage that reads TL owner artifacts. The
root crate's `temporal` module SHALL depend on `tl-syntax`, `tl-mltl` and
`quire-mltl` for their public readers, for this stage only; no other QSL
crate or module SHALL depend on them (ADR-011 §7.1). None of the three
depends on QSL, so the edges add no cycle. QSL owns the native subject, the
`/2` package and the native evaluator, and the TL crates own their readers,
so the stage that joins the two sits where both are reachable without
copying either owner's wire.

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
  `incomplete` with cause `missing-history` (QSpec FR-092), not `unsupported`,
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
- The rows of this table SHALL be evaluated in order and the first row the
  pair matches SHALL be the decision; the cause set SHALL list the cause of
  every row the pair matches, in table order:

| Validated mapped state | Join decision |
| --- | --- |
| either result refused, or a stale or wrong subject | `refused` / `not-compared` |
| either result failed | `failed` / `not-compared` |
| a required result or producer unavailable | `unavailable` / `not-compared` |
| either result unsupported | `unsupported` / `not-compared` |
| either result incomplete | `incomplete` / `not-compared` |
| both results values or both pending, with unequal values or premises, contradicted deciding support, or an identity collision | `conflict` / `mismatch` |
| both values equal and every applicable premise and support equal | `agreement` / `equal-final`, that Boolean |
| both pending under an open decision scope, equal | `agreement` / `equal-pending`, no Boolean |

- A non-final decision SHALL carry no Boolean and a nonempty ordered cause
  set. Assessment execution, both progress and closure axes, settlement
  basis, decision support, completeness and supersession SHALL be preserved,
  and omitting or mutating any of them SHALL NOT yield `agreement`.
- A corrected pair SHALL require the strict-read prior join whose native and
  TL source-result identities equal the two direct predecessors. A mixed
  original and corrected pair, a missing prior join or a predecessor mismatch
  SHALL be `conflict`. A correction never rewrites a prior join.

### Wire

Every record below SHALL be a JSON object written in RFC 8785 canonical form
by `quire-canonical` (ADR-013 §2): members in RFC 8785 order, integers as
JSON numbers, digests as 64 lowercase hex characters, owner bytes as
unpadded base64url strings, absent values as `null`. A reader SHALL reject
any byte sequence that is not the canonical form of an admitted record.

The **correspondence record** of profile
`quire.native-temporal-correspondence/v1` holds exactly these members:

| Member | Value |
| --- | --- |
| `anchor` | the declaration's activation anchor kind (FR-044) |
| `bindings` | one `{leaf, predicate, proposition}` per `holds` leaf in ascending leaf handle: the leaf handle, the leaf's checked predicate name, the TL proposition id |
| `bridge` | `"quire.native-temporal-correspondence/v1"` |
| `captures` | one `{index, value}` per capture in declaration order, `value` the capture's canonical value |
| `clause` | `{declaration, identity}`: the declaration index and the clause's declared name |
| `clock` | `{name, parameters}`: the emitted clock binding name and the authenticated parameter map |
| `closure` | `{surrounding}`: `"open"` or `"closed"` |
| `definition` | `{identity, revision}` of the selected profile definition |
| `history` | `"execution-origin"` or `"history-cutoff"` for a past target, `null` for a future target |
| `intervals` | every `[a, b]` in postorder |
| `package` | the `/2` package digest |
| `source` | `{digest, span}`: the native source's `raw-artifact-digest` and the clause's `[start, end]` byte span |
| `target` | `{formula, formula_edition, profile, request, trace}`: the generated formula's and request's digests, the formula edition, the TL target profile and the trace or history contract |

The **correspondence reference** SHALL be `{domain, digest}` with `domain`
`"quire.native-temporal-correspondence/v1"` and `digest` the SHA-256 of the
record's canonical bytes. The **projection** (`quire.native-temporal-projection/v1`)
holds `correspondence` (the record), `formula` and `request` (the owner bytes),
`reference` and `version`. The **join** (`quire.native-temporal-join/v1`)
holds `causes` (in table order), `decision`, `native` and `tl` (the digests of
the two results' bytes), `outcome`, `predecessor` (the prior join's
reference, or `null`), `reference` (the correspondence reference), `value`
(`true`, `false` or `null`) and `version`.

### Strict reading and limits

`read_tl_projection` and `read_tl_join` SHALL reject unknown, duplicate,
missing or out-of-order members, noncanonical or trailing bytes, identity or
digest mismatches and cross-wired positions, scopes or clocks.

`TlLimits` SHALL hold seven caller-configured limits, each a request setting
with the published default below (QSpec FR-461). Each is charged before the
work it pays for. A limit reached exactly admits; one past it SHALL stop with
`incomplete`, cause `limit-reached{limit, value, setting}`, and no partial
artifact:

| Limit | Setting | Default | Counts |
| --- | --- | --- | --- |
| `max_bytes` | `temporal.correspondence.max_bytes` | 16,777,216 | bytes read or written per record |
| `max_nodes` | `temporal.correspondence.max_nodes` | 65,536 | formula nodes |
| `max_positions` | `temporal.correspondence.max_positions` | 1,048,576 | trace positions |
| `max_valuations` | `temporal.correspondence.max_valuations` | 16,777,216 | valuation cells |
| `max_support` | `temporal.correspondence.max_support` | 1,048,576 | decision-support entries compared |
| `max_causes` | `temporal.correspondence.max_causes` | 64 | causes on one decision |
| `max_work` | `temporal.correspondence.max_work` | 4,294,967,296 | total work units, one per node, position, cell or support entry visited |

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
| FR-045-AC-8 | The one-position `always[0,1] p` vector rejects a finite-window-to-`mltl.closed-trace/v1` equivalence claim; an `Unsupported` classification (timestamped-event, mixed-direction or infinite-trace) yields `unsupported` with no formula; a past-only `once[0,2] holds(p)` declaration emits `tl-syntax.formula/v2` and a position history, and with a history-cutoff trace boundary yields `incomplete`, cause `missing-history`. | Test (TC-1240) |
| FR-045-AC-9 | Mutating the profile, source span, predicate binding, clock, capture, interval, closure, history or bridge identity changes the correspondence reference and names that axis; a numeric, nullable or pending leaf value cannot become a proposition; the formula, valuation, request and record identities each change only with their own inputs. | Test (TC-1240) |
| FR-045-AC-10 | A fixed-sample declaration's projection retains the authenticated epoch, period and unit; a trace with a changed period refuses even with equal formula bytes; an absent required sample yields `incomplete`. | Test (TC-1240) |
| FR-045-AC-11 | Each join-table row is reached; every non-final decision carries no Boolean and a nonempty cause set; omitting any preserved axis from either result cannot yield `agreement`; a corrected pair without the prior join, or with a predecessor mismatch, is `conflict`. | Test (TC-1240) |
| FR-045-AC-12 | A missing, duplicate or foreign valuation cell, an owner-reader rejection, a noncanonical projection or join, and each limit reached exactly and one past it give one deterministic decision, the over-limit and malformed cases non-admitted with no partial artifact and no evaluator invoked by the compiler. | Test (TC-1240) |
| FR-045-AC-13 | The correspondence record with `anchor` `origin`, one binding `{leaf: 0, predicate: "p", proposition: 0}`, no captures, `clause` `{declaration: 0, identity: "Always"}`, `clock` `{name: "steps", parameters: {sequence_authority: "seq"}}`, `closure` `closed`, `definition` `{identity: "quire.temporal.event-position.false-extension/v1", revision: "1"}`, `history` `null`, `intervals` `[[0, 1]]`, `package` 64 `0`s, `source` `{digest: 64 1s, span: [0, 22]}` and `target` `{formula: 64 2s, formula_edition: "tl-syntax.formula/v1", profile: "mltl.closed-trace/v1", request: 64 3s, trace: "tl-mltl.trace/v1"}` encodes to 860 canonical bytes whose SHA-256 is `848ccf3418350f00e1252d5050c5fbbc995d29bac807905d239d0f93edf168e0`; with `intervals` `[[0, 2]]` the digest is `fde423e1ad82e45a46622ed8daaa81f86df504512db1a7bb2d6467b3aed29e1e`; the same record with its members reordered, or with whitespace, refuses at `read_tl_projection`. | Test (TC-1240) |
| FR-045-AC-14 | A pair matching more than one join row returns the first row's decision with every matched row's cause in table order: a refused TL result beside unequal premises is `refused` with causes `refused` then `mismatch`, and an unsupported native result beside an incomplete TL result is `unsupported` with causes `unsupported` then `incomplete`. Each `TlLimits` limit admits at its default value and stops one past it with `limit-reached` naming the limit, its value and its `temporal.correspondence.*` setting. | Test (TC-1240) |

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
- **TL owners**: tl-syntax FR-011 (past-operator semantics and catalog),
  FR-013 and FR-014 (the formula editions and their strict artifacts),
  tl-mltl FR-018 (the `trace/v1`, `position-history/v1` and
  `history-requirement/v1` wire), quire-mltl FR-001 (request and result
  admission) and FR-003 (the `tl-mltl.contract-ir-result-map/v1` result
  mapping) read and evaluate the generated artifacts.
- **Architecture**: ADR-011 §7.1 records the root crate's edges to
  `tl-syntax`, `tl-mltl` and `quire-mltl`.
