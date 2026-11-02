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
states the input, the surface, the reachability rule and the authenticated
selection on the checked node.

The legacy surface `temporal::{mapping_support_v2, Classification,
AuthenticatedSelection}` is kept until Contract IR repoints (IR-1005, FR-499
ruling 2). This requirement is its successor: Contract IR FR-350 consumes the
successor's classification when it repoints.

## Use case

Contract IR holds a checked package and asks whether a temporal clause `Due` can
be mapped to a TL target. It gets back the target and the premises it must still
discharge, or the dimensions that rule the mapping out, together with the
definition the package selected for `Due`. It does not need the legacy
compiled-protocol package to ask.

## Semantic authority and boundary

QSpec FR-095 owns the correspondence contract, FR-045 owns the table, FR-050 the
binding, FR-515 the view. The legacy classifier read an admitted `/2` package;
this one reads the FR-337 node through the view, so the classification keeps no
`/1` form.

The function's crate and names are an inference: it sits beside FR-515's views
because it needs nothing but the view, and keeps the legacy type names so that
the Contract IR repoint changes imports, not meaning.

## Inputs

`qsl_package::checked_views::mapping_support(subject: &TemporalSubjectView<'_>,
surrounding: Closure) -> Result<Classification, MappingRefusal>`.

- `subject`: the view of a `temporal`/`temporal_clause` node (FR-515), from which
  the classifier reads `profile()`, `formula()` and `activation()`.
- `surrounding`: `Complete` or `Open`, the surrounding-execution closure the
  request names. The function takes no decision-scope closure.

## Outputs

`Classification`: the disposition `Supported { target, total_sample_valuation,
premises }` or `Unsupported { dimensions }`, the retained native subject (the
clause's `CheckedNodeId`, name, profile identity and activation), and
`selection: AuthenticatedSelection`.

`AuthenticatedSelection` holds the package identity, the clause's
`CheckedNodeId`, the definition identity and namespaced revision of the clause's
`profile_selections` row, and the identity of the definition artifact the row
selects. It holds no clock parameter map. It has no public constructor.

`Unmatched` dimensions are `InfiniteTrace`, `FiniteWindow` and `MixedDirection`,
the three FR-045 rows that accumulate or exclude.

`MappingRefusal` is `NoProfileSelection` (the lock has no `profile_selections`
row whose definition equals the clause's `temporal_profile` law, FR-515).

## Behavior

- The classifier SHALL decide the disposition from the clause's selected profile,
  the operator kinds in `formula()` and `surrounding` alone, as FR-045 states.
- The classifier SHALL NOT consult a backend capability report, an installed TL
  version, a syntax match or a historical result.
- The classifier SHALL read `formula()` as the operators reachable from the
  clause root, and a connective (`not`, `and`, `or`, `implies`) SHALL carry the
  direction of the operators beneath it and be neither past nor future itself.
- The classifier SHALL treat `eventually`, `always`, `until` and `release` as
  bounded future operators, and `once`, `historically`, `since` and `triggered`
  as bounded past operators (QSpec FR-092), `once[1,1]` being strong previous.
- The classifier SHALL treat a clause with no bounded operator under either
  false-extension profile as FR-045's last row.
- The classifier SHALL set `total_sample_valuation` for FR-045's fixed-sample
  future row and for no other row.
- The classifier SHALL return every matched dimension of the accumulating rows
  and no summary cause.
- The classifier SHALL retain the clause's name, profile identity and activation
  for an unsupported result.
- The classifier SHALL NOT substitute a profile, clock or reduced form.
- The classifier SHALL NOT report a clause as supported because its formula bytes
  match a TL formula.
- The classifier SHALL retain `selection` on every result, supported or not, and
  `selection` SHALL be constructible only by the classifier.
- The classifier SHALL give the same disposition to the same clause whatever the
  package's route: the in-process package and the package read back with the
  reader.
- When the node is not a temporal clause, `mapping_support` is not reachable:
  `temporal_subject` refuses it with `UnresolvedClause` (FR-515), and a view of a
  clause whose lock has no matching selection returns `NoProfileSelection`.
- An operator index outside the clause cannot occur, because the view lists only
  operators of the clause (FR-515), so the legacy out-of-range-reaches-nothing rule
  has no successor case.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-527-AC-1 | For one clause per row of FR-045's table, each matching that row and no other, `mapping_support` returns that row's disposition and TL target. | Test (TC-1322) |
| FR-527-AC-2 | A finite-window clause and a clause with a past and a future operator return `Unsupported`, and each of the four past operators alone under each false-extension profile returns `Supported` with `mltl.origin-complete-history/v1`. | Test (TC-1322) |
| FR-527-AC-3 | A finite-window clause that also mixes a past and a future operator returns `Unsupported` naming `FiniteWindow` and `MixedDirection`, and an infinite-trace clause returns `Unsupported` naming `InfiniteTrace`. | Test (TC-1322) |
| FR-527-AC-4 | `not once[0,1] holds(p)` classifies as past-only, `holds(p) implies eventually[0,2] holds(q)` as future-only, and `once[0,1] holds(p) implies eventually[0,2] holds(q)` as mixed-direction. | Test (TC-1322) |
| FR-527-AC-5 | A clause `holds(p)` with no bounded operator, under each false-extension profile, returns `Supported` with the closed-trace target for `Complete` and the online-prefix target for `Open`. | Test (TC-1322) |
| FR-527-AC-6 | The fixed-sample future clause returns `total_sample_valuation` true under both `surrounding` values, and the event-position future clause returns it false. | Test (TC-1322) |
| FR-527-AC-7 | An unsupported result holds the clause's `CheckedNodeId`, name, profile identity and activation equal to the values read from the view, and the checked package is unchanged by classification. | Test (TC-1322) |
| FR-527-AC-8 | A supported result names the exact TL target and the eight outstanding premises of FR-045, and no field of it claims a correspondence record. | Test (TC-1322) |
| FR-527-AC-9 | Two clauses with equal formula bytes under different profiles receive different classifications. | Test (TC-1322) |
| FR-527-AC-10 | `mapping_support` takes no decision-scope closure: a compile-fail test passing one fails to compile, and changing `surrounding` changes the target of FR-045's future rows and nothing else. | Test (TC-1322) |
| FR-527-AC-11 | Every result's `selection` holds the package identity, the clause, and the definition identity, revision and artifact identity equal to the values the test reads from the lock's `profile_selections` row, and holds no clock parameter map. | Test (TC-1322) |
| FR-527-AC-12 | A compile-fail test that constructs `AuthenticatedSelection` outside the classifier fails to compile. | Test (TC-1322) |
| FR-527-AC-13 | The classification of each of **R**'s three clauses from the in-process package equals the classification from the package read back, and classifying twice gives equal results. | Test (TC-1322) |
| FR-527-AC-14 | A view of a clause whose lock has no `profile_selections` row for its `temporal_profile` law returns `NoProfileSelection`, and a node that is not a temporal clause is refused by `temporal_subject` with `UnresolvedClause`. | Test (TC-1322) |
| FR-527-AC-15 | The classification code reads no value except the view and `surrounding`: no backend capability report, TL version or formula bytes. | Inspection |

## Dependencies

- FR-045-AC-1 to AC-6 (the legacy criteria this requirement succeeds), FR-515
  (the view), FR-337 (the node), FR-050 (the binding and selection).
- Contract IR FR-350 and IR-1005 consume the classification; they repoint from
  `temporal::mapping_support_v2` to `mapping_support`.
- Does not depend on FR-522 to FR-526: classification takes no trace.

## References

- Linear QSL-1504; deletion-table rows B27, B28, B29, B30.
