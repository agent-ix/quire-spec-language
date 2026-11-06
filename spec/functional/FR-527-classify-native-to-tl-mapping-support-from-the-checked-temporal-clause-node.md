---
id: FR-527
title: "Classify native-to-TL mapping support from the checked temporal clause node and retain the authenticated selection"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-515
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-499
    type: references
  - target: ix://agent-ix/quire-contract-ir/FR-350
    type: references
  - target: ix://agent-ix/quire-specification/FR-095
    type: depends_on
---
# FR-527: Classify native-to-TL mapping support from the checked temporal clause node and retain the authenticated selection

## Description

The spine SHALL classify a checked temporal clause against the FR-045 support
table, reading the clause through the FR-515 `TemporalSubjectView`, and SHALL
retain, on every classification, the profile selection the package authenticates
for that clause. The table, its eight rows, its target identities and its
outstanding premises are FR-045's and are not restated here. This requirement
states the input, the surface, the reachability rule, the timed profile that
FR-045's table does not list, and the authenticated selection on the checked
node.

This requirement is the successor of the legacy surface `temporal::{mapping_support_v2,
Classification, AuthenticatedSelection}`, which FR-499 keeps until Contract IR
FR-350 consumes the successor's classification (IR-1005).

## Use case

Contract IR holds a checked package and asks whether a temporal clause `Due` can
be mapped to a TL target. It gets back the target and the premises it must still
discharge, or the dimensions that rule the mapping out, together with the
definition the package selected for `Due`. It does not need the legacy
compiled-protocol package to ask.

## Semantic authority and boundary

QSpec FR-095 owns the correspondence contract, FR-045 owns the table, FR-050 the
binding, FR-515 the view. The legacy classifier read an admitted `/2` package;
this one reads the FR-337 node through the view, so the classification has no
`/1` form. The function sits beside FR-515's views because it needs nothing but
the view, and it keeps the legacy type names so that the Contract IR repoint
changes imports and not meaning.

## Inputs

`qsl_package::checked_views::mapping_support(subject: &TemporalSubjectView<'_>,
surrounding: Closure) -> Result<Classification, MappingRefusal>`.

- `subject`: the view of a `temporal`/`temporal_clause` node (FR-515), from which
  the classifier reads `profile()`, `formula()` and `activation()`.
- `surrounding`: `Complete` or `Open`, the surrounding-execution closure the
  request names. `Complete` is this surface's one spelling of a complete
  surrounding execution; the legacy `temporal::Closure` spells it `Closed` and
  stays until the Contract IR repoint (IR-1005), which maps the legacy value to
  this one. The function takes no decision-scope closure.

`mapping_support` is a QSL owner classifier over its checked view. Constructing
the view and performing classification are excluded from FR-350 component
work counters because this API accepts no caller budget or receipt; FR-350
accounts only the component work it owns. This exclusion does not change any
FR-045 classification result or claim that the owner work was charged to the
caller.

## Outputs

`Classification`: the disposition `Supported { target, total_sample_valuation,
premises }` or `Unsupported { dimensions }`, the retained native subject (the
clause's `CheckedNodeId`, the clause's `profile_selections` row, which is the
profile identity, and the activation), and `selection: AuthenticatedSelection`.
The retained subject holds the clause's name as FR-515's `name()` reads it from
the `temporal_clause_names` row: `Some(name)` for a node with one occurrence and
`None` for a node with several, whose occurrence names FR-515's `name_of` returns.

`AuthenticatedSelection` holds the package identity, the clause's
`CheckedNodeId` and the definition artifact the clause's `profile_selections`
row selects. The lock row is `{role, definition: {authority, identity}}`: it has
no separate revision member, the namespaced revision is the `/vN` suffix of the
definition identity (`quire.temporal.fixed-sample.false-extension/v1`), and the
definition artifact's identity is the `authority` and `identity` pair of the
row. `AuthenticatedSelection` holds no clock parameter map. It has no public
constructor.

`Unmatched` dimensions are `InfiniteTrace`, `FiniteWindow`, `MixedDirection` and
`TimedProfile`: the three FR-045 rows that accumulate or exclude, and the timed
profile.

`MappingRefusal` is `NoProfileSelection`: the `profile_selections` row of the
view names a definition identity that is none of the five temporal profiles the
`TemporalTrace` family admits (FR-234, FR-326).

## Behavior

- The classifier SHALL decide the disposition from the clause's selected profile,
  the operator kinds in `formula()` and `surrounding` alone, as FR-045 states.
- The classifier SHALL NOT consult a backend capability report, an installed TL
  version, a syntax match or a historical result.
- The classifier SHALL read every operator in `formula()` as reachable from the
  clause root, since FR-515 lists only the operators of the clause.
- The classifier SHALL NOT treat `not`, `and`, `or` and `implies` as past or
  future operators, since `formula()` lists the `quire.op.temporal.*` applications
  only.
- The classifier SHALL treat `eventually`, `always`, `until` and `release` as
  bounded future operators, and `once`, `historically`, `since` and `triggered`
  as bounded past operators (QSpec FR-092), `once[1,1]` being strong previous.
- The classifier SHALL treat a clause with no bounded operator under either
  false-extension profile as FR-045's last row.
- The classifier SHALL classify a clause selected under
  `quire.temporal.timed/v1` (FR-234) as `Unsupported` naming `TimedProfile`.
  FR-234 admits the timed profile at check, but FR-045's table lists four
  profiles and no TL target evaluates exact rational time, so the classification
  is sound and incomplete: it claims no support and no correspondence.
- The classifier SHALL name `MixedDirection` beside `TimedProfile` when the
  formula of a timed clause holds a past and a future operator.
- The classifier SHALL set `total_sample_valuation` for FR-045's fixed-sample
  future row and for no other row.
- The classifier SHALL return every matched dimension of the accumulating rows
  and no summary cause.
- The classifier SHALL retain the clause's name, profile identity and activation
  for an unsupported result.
- The classifier SHALL NOT substitute a profile, clock or reduced form.
- The classifier SHALL NOT report a clause as supported because its formula bytes
  match a TL formula.
- The classifier SHALL retain `selection` on every result, supported or not.
- `selection` SHALL be constructible only by the classifier.
- The classifier SHALL give the same disposition to the same clause whatever the
  package's route. The two routes are the IR reader applied to the bytes the
  unit's emitter returns in process, and the IR reader applied to the bytes of
  the handoff tree the writer wrote (FR-517); the reader is the one entry that
  produces a `CheckedPackageV2`.
- The classifier SHALL return `NoProfileSelection` for a view whose profile row
  names a definition identity outside the five admitted profiles.
- When the node is not a temporal clause, `mapping_support` is not reachable:
  `temporal_subject` refuses it with `UnresolvedClause` (FR-515).
- A package whose lock has no `profile_selections` row for a temporal clause's
  `temporal_profile` law is refused by the reader, so no view of such a clause
  exists and `NoProfileSelection` does not stand for that case;
  `temporal_subject`'s `UnresolvedClause` for it is a re-check that no reader
  output reaches.
- An operator index outside the clause cannot occur, because the view lists only
  operators of the clause (FR-515), so the legacy out-of-range-reaches-nothing rule
  has no successor case.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-527-AC-1 | For one clause per row of FR-045's table, each matching that row and no other, `mapping_support` returns that row's disposition and TL target. | Test (TC-1322) |
| FR-527-AC-2 | A finite-window clause returns `Unsupported` naming `FiniteWindow`, and a clause with a past and a future operator returns `Unsupported` naming `MixedDirection`. | Test (TC-1322) |
| FR-527-AC-3 | Each of the four past operators alone, under each false-extension profile, returns `Supported` with `mltl.origin-complete-history/v1`. | Test (TC-1322) |
| FR-527-AC-4 | A finite-window clause that also mixes a past and a future operator returns `Unsupported` naming `FiniteWindow` and `MixedDirection`, and an infinite-trace clause returns `Unsupported` naming `InfiniteTrace`. | Test (TC-1322) |
| FR-527-AC-5 | `once[0,1] holds(p)` classifies as past-only, `eventually[0,2] holds(q)` as future-only, and a formula holding both `once[0,1]` and `eventually[0,2]` as mixed-direction; the connectives `not`, `and`, `or` and `implies` change none of the three. | Test (TC-1322) |
| FR-527-AC-6 | A clause `holds(p)` with no bounded operator, under each false-extension profile, returns `Supported` with the closed-trace target for `Complete` and the online-prefix target for `Open`. | Test (TC-1322) |
| FR-527-AC-7 | The fixed-sample future clause returns `total_sample_valuation` true under both `surrounding` values, and the event-position future clause returns it false. | Test (TC-1322) |
| FR-527-AC-8 | An unsupported result holds the clause's `CheckedNodeId`, profile row and activation equal to the values read from the view, and the checked package is unchanged by classification. | Test (TC-1322) |
| FR-527-AC-9 | A supported result names the exact TL target and the eight outstanding premises of FR-045, and no field of it claims a correspondence record. | Test (TC-1322) |
| FR-527-AC-10 | Two clauses with equal formula bytes under different profiles receive different classifications. | Test (TC-1322) |
| FR-527-AC-11 | A compile-fail test that passes a decision-scope closure to `mapping_support` fails to compile. | Test (TC-1322) |
| FR-527-AC-12 | Changing `surrounding` changes the target of FR-045's future rows and nothing else. | Test (TC-1322) |
| FR-527-AC-13 | Every result's `selection` holds the package identity, the clause, and the definition `authority` and `identity` (the `/vN` suffix being the revision) equal to the values the test reads from the lock's `profile_selections` row, and holds no clock parameter map. | Test (TC-1322) |
| FR-527-AC-14 | A compile-fail test that constructs `AuthenticatedSelection` outside the classifier fails to compile. | Test (TC-1322) |
| FR-527-AC-15 | The classification of each temporal clause of each of **R**'s three packages, from the reader applied to the bytes the unit's emitter returns without a file, equals the classification from the reader applied to the bytes of the written handoff tree. | Test (TC-1322) |
| FR-527-AC-16 | A view whose profile row names a definition identity outside the five admitted temporal profiles returns `NoProfileSelection`. | Test (TC-1322) |
| FR-527-AC-17 | A node that is not a temporal clause is refused by `temporal_subject` with `UnresolvedClause`. | Test (TC-1322) |
| FR-527-AC-18 | A unit that selects a temporal profile outside the five admitted identities is refused at check with FR-110's `UnsupportedSelection`, so no clause node exists for `mapping_support` to classify. | Test (TC-1322) |
| FR-527-AC-19 | Classifying one clause twice gives equal results. | Test (TC-1322) |
| FR-527-AC-20 | The classification code reads no value except the view and `surrounding`: no backend capability report, TL version or formula bytes. | Inspection |
| FR-527-AC-21 | The reader refuses a package whose lock has no `profile_selections` row for a temporal clause's `temporal_profile` law, so the package yields no `TemporalSubjectView`. | Test (TC-1322) |
| FR-527-AC-22 | A clause selected under `quire.temporal.timed/v1` returns `Unsupported` naming `TimedProfile` and no TL target. | Test (TC-1322) |
| FR-527-AC-23 | A timed clause holding a past and a future operator returns `Unsupported` naming `TimedProfile` and `MixedDirection`. | Test (TC-1322) |
| FR-527-AC-24 | Over a package whose clause `Gamma` has one occurrence, a supported result and an unsupported result (AC-22's timed clause named `Delta`, with one occurrence) hold the retained name `Some("Gamma")` and `Some("Delta")` respectively, each equal to `name()` of the view; over FR-337-AC-16's package of the two structurally equal clauses `Alpha` and `Beta`, the result holds `None`, equal to `name()` of the view. | Test (TC-1322) |
| FR-527-AC-25 | Calling `mapping_support` performs no caller-budget or receipt operation and leaves FR-350 component counters unchanged; it returns the same classification for the same checked view and `surrounding` value. | Test |

## Dependencies

- FR-045-AC-1 to AC-6 (the legacy criteria this requirement succeeds), FR-515
  (the view), FR-337 (the node), FR-050 (the binding and selection).
- Contract IR FR-350 and IR-1005 consume the classification; they repoint from
  `temporal::mapping_support_v2` to `mapping_support`.
- Does not depend on FR-522 to FR-526: classification takes no trace.

## References

- Linear QSL-1504; deletion-table rows B27, B28, B29, B30.
