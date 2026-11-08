---
id: FR-526
title: "Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
---
# FR-526: Chain a temporal evidence result to its direct predecessor and refuse a relation that names another obligation or changes nothing

## Description

When a temporal evidence request corrects an earlier result, the spine evaluator
retains the exact direct predecessor and gives the result the next lineage
number. It refuses a predecessor that is not the latest result of the same
obligation instance, a request that leaves the evaluation input unchanged, and a
request whose correspondence differs from the predecessor's. The predecessor
stays unchanged.

This is the spine successor of FR-052-AC-12 and of the lineage, correction and
semantic-trigger-identity rows of the legacy `ProtocolLedger`. FR-516 gives the
relation its three forms and the first refusals; this requirement states the
whole rule.

It is also the spine successor of FR-052-AC-15, AC-16, AC-19 and AC-20, the
subject, support, predecessor and limit criteria of the retired native
owner-report route: every result names the checked subject it was evaluated for,
its support names only evidence the request admitted, its predecessor authority
is the evaluator-written ledger entry, and the chain is bounded by FR-523's
`lineage` counter.

## Use case

A monitor decided an obligation on the evidence it had, then receives a late
observation. It evaluates again, naming the first result as the one it
corrects. The evaluator accepts the correction because the evidence changed and
the first result is the latest of that obligation. A resend of the same evidence,
a correction naming a result that a later correction already superseded, or a
correction naming another obligation's result, is refused, and the first result is
not touched.

## Semantic authority and boundary

FR-052 owns the correction relation and FR-053 the trigger identity.

The legacy `ProtocolLedger` enforced "the latest result of this obligation" from
history it kept. A stateless call cannot know the latest result, so the spine
keeps that history in the caller-held `EvidenceLedger` of FR-524, in the way
FR-502 keeps progress in the `ProgressLedger`. The ledger's `results` table holds,
for each obligation instance (the package identity, the clause, the clause
occurrence and the trigger identity), one flat entry for the latest result: its `result_identity`, its
`input_identity` (FR-522: the identity over the evidence members only, which excludes the relation and the ledger), its correspondence
identity and its lineage. Entries hold no recursive chain and no result bytes.
Only the evaluator writes an entry.

## Inputs

`relation` and `ledger` of `EvidenceRequest` (FR-516). `relation` is `Original`,
`Superseding(prev)` or `Invalidating(prev)`, with `prev` the `result_identity` of
the predecessor.

## Outputs

`lineage: u64`: 0 for `Original`, the predecessor entry's lineage plus 1 otherwise.
`relation` echoed with `prev`. `input_identity`. The updated `ledger`.

`subject: ResultSubject`: the checked subject of the call, with the members
`package_identity`, `package_version` (the checked package format version the
`EvidencePackage` was emitted or read at), `clause` (the clause `CheckedNodeId`),
`occurrence` (the selected `{ role, ordinal }`, FR-516 `occurrence`; for a node
with one occurrence, that occurrence) and `source_map` (the
`CheckedSourceMapEntry` of the selected occurrence, FR-515 `source_span`). The
`correspondence` echoed byte for byte (FR-516) is the observation reference, and
its `authority_revision` is the revision reference.

Each refusal below is `Err(CallFailure::Admission)` with an empty charge log, no
result and the `cause` named in the criterion. A lineage stop is an `Incomplete`
result (FR-516: `Ok`, `activation = Unavailable`, `incomplete` naming `lineage`,
no truth), not an error (FR-523).

## Behavior

- The evaluator SHALL derive truth, settlement and support from the request's own
  inputs for `Original`, `Superseding` and `Invalidating` alike.
- `Superseding` records that the result replaces the predecessor, and the
  evaluator SHALL retain that kind in the result.
- `Invalidating` records that the result retracts the predecessor, and the
  evaluator SHALL retain that kind in the result.
- When a request carries `Superseding` or `Invalidating` without a `ledger`, the
  evaluator SHALL refuse it with cause `no-ledger`.
- The evaluator SHALL look up the ledger's entry for the request's package, clause,
  occurrence and trigger identity.
- When the ledger holds no entry for that key, the evaluator SHALL refuse the
  relation with cause `predecessor-unknown`.
- When `prev` is not the `result_identity` of that entry, the evaluator SHALL refuse
  the relation with cause `predecessor-not-latest`.
- When the request's `input_identity` equals the entry's, the evaluator SHALL
  refuse the relation with cause `unchanged-input`.
- When the request's correspondence differs from the entry's, the evaluator SHALL
  refuse the relation with cause `correspondence-changed`.
- The evaluator SHALL compute `lineage` with checked `u64` arithmetic.
- When the sum does not fit or exceeds the `lineage` ceiling (FR-523), the
  evaluator SHALL return an `Incomplete` result naming `lineage` that carries the
  request's `ledger` unchanged, with no entry recorded for it.
- When a call succeeds, the evaluator SHALL record the result's entry as the
  latest for its key, including for `Original`.
- The evaluator SHALL leave every earlier result and the replaced entry's value as
  the caller holds them.
- An `Original` request SHALL start a new chain at lineage 0 even when the ledger
  holds an entry for its key.
- Equal requests with equal relations and equal ledgers SHALL give equal results
  and equal ledgers.
- The `EvidenceLedger` SHALL have no public constructor of entries.
- A trigger identity that differs from the predecessor's has no entry under the
  request's key and is refused with `predecessor-unknown`, so a correction cannot
  change the trigger identity (FR-053).
- An occurrence that differs from the predecessor's has no entry under the
  request's key and is refused with `predecessor-unknown`, so a correction cannot
  move to another occurrence of the same checked declaration.

### Result subject and support

- Every `Ok` result, an `Incomplete` one included, SHALL carry `subject`, read
  from the package and the request's occurrence selection, never from a request
  member that names it. `subject` adds no member to `result_identity` beyond
  those of FR-522.
- The result SHALL echo `correspondence` byte for byte (FR-516), and QSL SHALL
  treat it as opaque: it defines no revision or provenance shape and derives no
  identity from bytes, hashes or digests.
- Each `SupportRef` of `support` SHALL name a pair of the FR-501 decision support:
  its `position` the ordinal of an observed position of the request in canonical
  order, its `observation` that position's `observation` byte for byte, and its
  `atom` an atom index for which the request admitted a valuation at that
  position (the assessed instance's valuation, for an instance-bound atom,
  FR-524).
- `support` SHALL hold one entry per such pair: two atoms of one position are two
  entries, and no pair appears twice. The request admits at most one valuation per
  position, atom and instance (its valuations are maps keyed by atom), so a
  repeated identical evidence row has no request form and no multiplicity beyond
  one per pair.
- A decision-support pair with no admitted valuation, at a silent sample (FR-516)
  or at an observed position whose valuation of the atom is absent, SHALL have no
  `support` entry; the absent valuation is reported by the `Missing` non-value or
  the `Indeterminate` cause of FR-516, not as support.
- The request SHALL have no `native_valuations` member: that byte member carried
  the retired native route's duplicate-preserving relation and is removed with it,
  so it is neither a request member nor a member of `input_identity`.
- These items change the current evidence route: its result has no `subject`
  member, its support keeps a pair at an observed position whose valuation is
  absent and does not remove a repeated pair, and its request carries
  `native_valuations`.

### Predecessor authority

- The predecessor authority of a correction SHALL be the ledger entry the
  evaluator wrote for the predecessor's call. A `prev` value, a caller-held
  result, or a ledger returned by another call grants none.
- The `EvidenceLedger` SHALL have no public operation that changes an entry's
  `result_identity`, `input_identity`, correspondence or lineage.
- A clone of a returned ledger, and a ledger moved into a later request, SHALL give
  the same admission as the ledger itself.
- `evaluate_temporal_evidence` SHALL return an opaque `EvaluatedEvidence` that
  uniquely owns the complete original `EvidenceResult`. Its `result()` borrows
  that result immutably, including checked subject, truth, support, settlement
  and ledgers; `predecessor()` borrows this issued result's `ResultIdentity` for
  the next correction. The current previous link remains in `relation`.
- Only the evaluator SHALL issue this carrier. It SHALL expose no public
  constructor, deserialization construction, mutable borrow or consuming escape
  of its original data. Reading the result or predecessor SHALL neither evaluate
  again nor charge work. A cloned read snapshot is ordinary caller data and
  SHALL NOT substitute for the issued carrier.
- `result_identity` and `matches_result_identity` bind input members only;
  neither authenticates caller-authored truth, support, settlement or ledger.
  The carrier binds the admitted request and checked subject to the actual
  evaluator output by retaining that output privately. This adds no serializer,
  predecessor bytes, result-byte storage or recursive chain.

### Lineage limit

- The chain is bounded by FR-523's `lineage` counter, charged before any instance
  is created or any position is scanned.
- When the counter refuses the charge, the evaluator SHALL return the
  `Incomplete` of FR-523 with `LimitReached { limit, value, setting }` naming
  `lineage`, `value` the ceiling as given and `setting` `temporal_evidence.lineage`.
- That result SHALL carry the attempted `lineage` (the predecessor entry's lineage
  plus 1) and the request's `relation` with its `prev`. When that sum does not fit
  in `u64`, the result's `lineage` SHALL be 0, the value carried with no lineage,
  and the `LimitReached` names `lineage` as above.
- The stopped result's `ledger` SHALL equal the request's, with no entry for the
  stopped call.
- The caller's `Meter` SHALL keep every charge earlier calls made: a lineage stop
  never resets or lowers it (FR-523 "Stop scope").
- No other predecessor limit applies. The retired native route charged work and
  storage for copying and comparing retained predecessor bytes. A spine ledger
  entry holds no result bytes and no chain (above), and admission compares a fixed
  set of identity members whose request bytes FR-523's `request_bytes` and
  `member_bytes` already bound, so the spine has no such resource to bound.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-526-AC-1 | An `Original` request returns `lineage` 0 and relation `Original`, and the returned ledger holds its entry for the request's key. | Test (TC-1321) |
| FR-526-AC-2 | A `Superseding(r1)` request with changed inputs, the ledger from `r1`'s call, and the same package, clause, trigger and correspondence returns `lineage` 1 and relation `Superseding` naming `r1.result_identity`. | Test (TC-1321) |
| FR-526-AC-3 | After the AC-2 call, `r1` is byte-equal to its value before the call and the returned ledger holds the new entry as the latest. | Test (TC-1321) |
| FR-526-AC-4 | An `Invalidating(r1)` request under the conditions of AC-2 returns relation `Invalidating` and the same `lineage`. | Test (TC-1321) |
| FR-526-AC-5 | The truth, settlement and support of the AC-4 result equal those of a `Superseding(r1)` request with the same inputs. | Test (TC-1321) |
| FR-526-AC-6 | A `Superseding(r1)` request and an `Invalidating(r1)` request with `r1`'s `input_identity` each return `Err(CallFailure::Admission)` with cause `unchanged-input` and an empty charge log. | Test (TC-1321) |
| FR-526-AC-7 | A `Superseding` request without a `ledger` returns `Err(CallFailure::Admission)` with cause `no-ledger`. | Test (TC-1321) |
| FR-526-AC-8 | With a ledger holding `r1` and `r2` (`r2` superseding `r1`), a `Superseding(r1)` request returns `Err(CallFailure::Admission)` with cause `predecessor-not-latest`, and a `Superseding(r2)` request is accepted. | Test (TC-1321) |
| FR-526-AC-9 | A request for another clause of the same package naming `r1`, and a request for the same clause in another package naming `r1`, each return `Err(CallFailure::Admission)` with cause `predecessor-unknown`. | Test (TC-1321) |
| FR-526-AC-10 | A request whose trigger identity differs from `r1`'s in one byte, naming `r1`, returns `Err(CallFailure::Admission)` with cause `predecessor-unknown`. | Test (TC-1321) |
| FR-526-AC-11 | A request whose `correspondence` differs from `r1`'s, naming `r1`, returns `Err(CallFailure::Admission)` with cause `correspondence-changed`. | Test (TC-1321) |
| FR-526-AC-12 | With the `lineage` ceiling set to 2, the third correction of a chain (lineage 3) returns an `Ok` `Incomplete` result naming `lineage` with `activation = Unavailable` and no truth, the returned ledger holding no entry for it, and with the ceiling set to 3 it completes. | Test (TC-1321) |
| FR-526-AC-13 | The evaluator computes `lineage` from the predecessor entry's lineage with checked `u64` arithmetic (`u64::checked_add` in the lineage counter, which has no wrapping or saturating form), so an entry at `u64::MAX` (unreachable by calls) cannot yield a smaller lineage. | Inspection (`checked_add` in the lineage counter) |
| FR-526-AC-14 | A chain of three corrections `r1`, `r2`, `r3` returns `lineage` 1, 2, 3, each naming its direct predecessor, with `r1` and `r2` unchanged after `r3`. | Test (TC-1321) |
| FR-526-AC-15 | An `Original` request over a ledger that holds an entry for its key returns `lineage` 0 and replaces that entry. | Test (TC-1321) |
| FR-526-AC-16 | The same request, relation and ledger evaluated twice give equal results and equal ledgers, charge log included. | Test (TC-1321) |
| FR-526-AC-17 | A compile-fail test that constructs an `EvidenceLedger` entry outside the evaluator fails to compile. | Test (TC-1321) |
| FR-526-AC-18 | An empty trigger identity is refused at construction of `SemanticTriggerIdentity`. | Test (TC-1321) |
| FR-526-AC-19 | A non-empty trigger identity is echoed byte for byte by every result of a three-correction chain. | Test (TC-1321) |
| FR-526-AC-20 | For each of **R**'s three packages (FR-517), an `Original` call returns a result whose `subject` has `package_identity`, `package_version`, `clause`, `occurrence` and `source_map` equal to the values the test reads from the package and the selected occurrence (FR-515 `source_span`), and whose `correspondence` equals the request's member for member, `authority_revision` included; a call stopped `Incomplete` (FR-523-AC-5) carries the same `subject`. | Test (TC-1321) |
| FR-526-AC-21 | Over the twin package of FR-516-AC-32 (one node, occurrences 7 and 3), two calls with equal positions, valuations and correspondence, each with the `binding` of its own occurrence's clock row, selecting occurrences 3 and 7 return results whose `subject` agrees in `package_identity`, `package_version` and `clause` and differs in `occurrence`. | Test (TC-1321) |
| FR-526-AC-22 | In a unit the test authors under the event-position profile (**R** holds no such clause), over the clause `on origin eventually[0,1] (holds(a) and holds(b))` with `a` and `b` true at position 0 and false at position 1, `Complete { through: 1 }` and `Closed` scope, the result is `True` and `support` is exactly the two entries (position 0, atom of `a`) and (position 0, atom of `b`), each naming position 0's `observation` byte for byte; a reading that merges entries by position, or by observation, gives one entry and fails this criterion. | Test (TC-1321) |
| FR-526-AC-23 | Over AC-22's clause with `b`'s valuation at position 0 withdrawn, and over a fixed-sample clause whose decision support holds a pair at a silent sample (FR-516-AC-44), no `support` entry names a pair the request admitted no valuation for, and every entry of every result of TC-1321 names a request position and an atom with an admitted valuation there. | Test (TC-1321) |
| FR-526-AC-24 | With `r1` from one request and `r1'` from a request that differs from it in one valuation so that their truths differ, a `Superseding(r1)` request carrying the ledger returned by `r1'`'s call returns `Err(CallFailure::Admission)` with cause `predecessor-not-latest`, and a `Superseding(r1')` request with changed inputs carrying that ledger is accepted. | Test (TC-1321) |
| FR-526-AC-25 | Over the twins of AC-21, with `r3` the result for occurrence 3, a `Superseding(r3)` request selecting occurrence 7 with `r3`'s ledger and changed inputs returns `Err(CallFailure::Admission)` with cause `predecessor-unknown`, and `r3` and its ledger are unchanged. | Test (TC-1321) |
| FR-526-AC-26 | A `Superseding(r1)` request carrying a clone of `r1`'s returned ledger, and one carrying that ledger moved out of `r1`, each return a result equal to the request carrying the ledger itself; a compile-fail test that sets an entry's `result_identity`, `input_identity`, correspondence or lineage outside the evaluator fails to compile. | Test (TC-1321) |
| FR-526-AC-27 | A `Superseding(r1)` request with changed inputs completes with the `lineage` ceiling 1, and with the ceiling 0 returns an `Ok` `Incomplete` result with `LimitReached` naming `lineage`, `value` 0 and setting `temporal_evidence.lineage`, result `lineage` 1, relation `Superseding` naming `r1.result_identity`, `activation = Unavailable`, no truth, `usage.instances` 0 and `usage.positions` 0, and a `ledger` equal (`==`) to the request's. | Test (TC-1321) |
| FR-526-AC-28 | A `Meter` charged `U` work units by one call and then passed to AC-27's stopped call reports, after the stop, `U` plus the units of the stopped call's charge log, never less than `U`; a third call on the same meter continues from that total. | Test (TC-1321) |
| FR-526-AC-29 | Over the clause of AC-1, a request that is AC-1's with one extra `valuations` entry keyed by an atom index the formula does not have (such as 99) at position 1 returns `Err(CallFailure::Admission)` with cause `valuation-atom-unknown`, catalog code `invalid_runtime_input`/`invalid-value` at member `valuations` and fields `atom` (that index) and `position` `1`, and an empty charge log; the same key alone at position 0 names `position` `0`; the same key in an `instance_valuations` map names member `instance_valuations`; and AC-1's request without the key is evaluated, so a stray atom key never yields a result whose `input_identity` differs from the request without it (FR-516 `valuation-atom-unknown`). | Test (TC-1321) |
| FR-526-AC-30 | A real checked-package temporal evaluation returns an evaluator-issued immutable full-result carrier. Repeated immutable result and predecessor borrows retain the original truth, support, settlement, checked subject and ledger without further meter charges. A changed-input correction names the original carrier's identity, changes actual truth, support, settlement and latest ledger, advances lineage, and leaves the original unchanged, including after mutation of a cloned read snapshot. Compile-fail tests reject constructing the carrier, mutating its original result and substituting a mutated snapshot for an issued carrier. | Test (TC-1321) |

## Dependencies

- FR-052-AC-12 and the `ProtocolLedger` rows (the legacy behavior this requirement
  succeeds), FR-053 (identity), FR-522 (`input_identity`, the authenticated
  binding), FR-523 (`lineage`), FR-524 (the `EvidenceLedger` value), FR-515
  (`source_span`, the occurrence source-map entry of `subject`).
- FR-052-AC-15, AC-16, AC-19 and AC-20 (the retired native owner-report criteria
  AC-20 to AC-28 succeed).
- FR-502 is the pattern of the caller-held ledger.
- FR-516 carries the relation and the entry and is a reference, not a dependency,
  of this requirement.

## References

- Linear QSL-1503.
