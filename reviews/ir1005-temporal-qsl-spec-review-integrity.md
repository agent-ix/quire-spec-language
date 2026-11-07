---
id: SR-20261106200933
title: "integrity review of IR-1005 temporal bridge spec (quire-spec-language)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; IR-1005; spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md; spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md"
review_set: subset
---

## Summary

Ticket IR-1005, PR 2479. Reviewed the temporal bridge specification change using the integrity method.

## Verdict

**CONDITIONAL** — One procedure-table formatting defect requires correction.

Checked consistency of all added paragraphs and criteria against FR-515, FR-527, FR-052 and the actual current Contract IR FR-350 contract. Original declaration/role/ordinal occurrence keys and selected_model_packages admitted identity order are preserved. No source aliases are inferred. Native classifier/report ownership, integration projection/strict reader/join ownership and TL admission/evaluation ownership remain separated. Existing requirement→stakeholder links and verification methods remain, and no implementation sequence or duplicated authority is introduced.

No active AssuranceProfile document applies to these changed artifacts. No Cargo tests or final aggregate gate ran in this review; authored acceptance procedures remain planned. Actual owner-source inspection is not runtime evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-1322 inserts a blank line before new procedure rows 27 and 28, detaching them from the table header. | spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:44 |

## Examined Scope

```yaml
scope:
- id: FR-527-AC-1
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: For one clause per row of FR-045's table, each matching that row and no
    other, `mapping_support` returns that row's disposition and TL target.
- id: FR-527-AC-2
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A finite-window clause returns `Unsupported` naming `FiniteWindow`, and
    a clause with a past and a future operator returns `Unsupported` naming `MixedDirection`.
- id: FR-527-AC-3
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Each of the four past operators alone, under each false-extension profile,
    returns `Supported` with `mltl.origin-complete-history/v1`.
- id: FR-527-AC-4
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A finite-window clause that also mixes a past and a future operator returns
    `Unsupported` naming `FiniteWindow` and `MixedDirection`, and an infinite-trace
    clause returns `Unsupported` naming `InfiniteTrace`.
- id: FR-527-AC-5
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: '`once[0,1] holds(p)` classifies as past-only, `eventually[0,2] holds(q)`
    as future-only, and a formula holding both `once[0,1]` and `eventually[0,2]` as
    mixed-direction; the connectives `not`, `and`, `or` and `implies` change none
    of the three.'
- id: FR-527-AC-6
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A clause `holds(p)` with no bounded operator, under each false-extension
    profile, returns `Supported` with the closed-trace target for `Complete` and the
    online-prefix target for `Open`.
- id: FR-527-AC-7
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: The fixed-sample future clause returns `total_sample_valuation` true under
    both `surrounding` values, and the event-position future clause returns it false.
- id: FR-527-AC-8
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: An unsupported result holds the clause's `CheckedNodeId`, profile row and
    activation equal to the values read from the view, and the checked package is
    unchanged by classification.
- id: FR-527-AC-9
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A supported result names the exact TL target and the eight outstanding
    premises of FR-045, and no field of it claims a correspondence record.
- id: FR-527-AC-10
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Two clauses with equal formula bytes under different profiles receive different
    classifications.
- id: FR-527-AC-11
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A compile-fail test that passes a decision-scope closure to `mapping_support`
    fails to compile.
- id: FR-527-AC-12
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Changing `surrounding` changes the target of FR-045's future rows and nothing
    else.
- id: FR-527-AC-13
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Every result's `selection` holds the package identity, the clause, and
    the definition `authority` and `identity` (the `/vN` suffix being the revision)
    equal to the values the test reads from the lock's `profile_selections` row, and
    holds no clock parameter map.
- id: FR-527-AC-14
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A compile-fail test that constructs `AuthenticatedSelection` outside the
    classifier fails to compile.
- id: FR-527-AC-15
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: For each temporal clause of each of **R**'s three packages, the reader
    applied to the unit emitter's bytes and the reader applied to the handoff tree's
    bytes produce equal support, retained subject, package name, clause and definition
    artifact. Each classification belongs only to its own admission and clones retaining
    that admission; their independently issued bindings are unequal.
- id: FR-527-AC-16
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A view whose profile row names a definition identity outside the five admitted
    temporal profiles returns `NoProfileSelection`.
- id: FR-527-AC-17
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A node that is not a temporal clause is refused by `temporal_subject` with
    `UnresolvedClause`.
- id: FR-527-AC-18
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A unit that selects a temporal profile outside the five admitted identities
    is refused at check with FR-110's `UnsupportedSelection`, so no clause node exists
    for `mapping_support` to classify.
- id: FR-527-AC-19
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Classifying one clause twice gives equal results.
- id: FR-527-AC-20
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: 'The classification code reads no value except the view and `surrounding`:
    no backend capability report, TL version or formula bytes.'
- id: FR-527-AC-21
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: The reader refuses a package whose lock has no `profile_selections` row
    for a temporal clause's `temporal_profile` law, so the package yields no `TemporalSubjectView`.
- id: FR-527-AC-22
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A clause selected under `quire.temporal.timed/v1` returns `Unsupported`
    naming `TimedProfile` and no TL target.
- id: FR-527-AC-23
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: A timed clause holding a past and a future operator returns `Unsupported`
    naming `TimedProfile` and `MixedDirection`.
- id: FR-527-AC-24
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: Over a package whose clause `Gamma` has one occurrence, a supported result
    and an unsupported result (AC-22's timed clause named `Delta`, with one occurrence)
    hold the retained name `Some("Gamma")` and `Some("Delta")` respectively, each
    equal to `name()` of the view; over FR-337-AC-16's package of the two structurally
    equal clauses `Alpha` and `Beta`, the result holds `None`, equal to `name()` of
    the view.
- id: FR-527-AC-25
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: context_only
  excerpt: The signature of `mapping_support` takes no budget, limit or receipt parameter
    and returns none; `MappingRefusal` holds no work-limit variant; over a view built
    at the FR-515 work-unit default exactly, it returns the same classification on
    a second call, and a view one expansion past that default is refused at view construction
    (`ViewRefusal::FormulaTooLarge`), never by `mapping_support`.
- id: FR-527-AC-26
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: 'For a supported and an unsupported clause, classify the same producer
    bytes admitted twice independently: package names, content, disposition and retained
    clause facts are equal, but each selection''s owner binding accepts its original
    admission and its unchanged clone, refuses the other admission, and makes the
    classifications unequal. Moving the original package preserves binding membership.
    Dropping the source package and subject borrow does not prevent an owned selection
    from retaining and comparing its owner binding.'
- id: FR-527-AC-27
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: Inspection of the temporal bridge input contract identifies the original
    checked package/view/classification APIs, distinct borrowed-view and opaque-admission
    membership checks, full reclassification before trusting public data, and separate
    QSL/native, integration/bridge and TL owner responsibilities; it introduces no
    new carrier, callback evaluator or content-derived admission identity.
- id: FR-527-AC-28
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: For two declaration occurrences sharing one clause node, the classification
    retains the node and no single name, while the bridge input's independently selected
    occurrence resolves its own name and clock by declaration/role/ordinal. Selected
    model package identities retain the checked owner's order and no authored model
    alias map is inferred. A changed public classification field is unequal to reclassification
    of the original subject even when its sealed selection is unchanged.
- id: FR-527
  path: spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: 'The QSL input contract for Contract IR FR-350''s bridge SHALL be the original

    `CheckedPackageV2`, its FR-515 `TemporalSubjectView`, and this classification.

    The subject''s `belongs_to(package)` checks the actual borrowed package; the

    classification''s `belongs_to(package)` checks the opaque owner admission that

    its sealed selection retains. These checks are distinct. An unchanged clone

    may retain the classification''s admission without being the package from which

    a particular subject view was borrowed. Neither package-name equality nor equal

    producer bytes SHALL replace either check.


    The disposi'
- id: TC-1322
  path: spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md
  role: examined
  excerpt: '| Row | AC | Input | Expected |

    | --- | --- | --- | --- |

    | 1 | 1 | one clause per FR-045 table row | each row''s disposition and TL target
    |

    | 2 | 2 | a finite-window clause; a clause with a past and a future operator |
    `Unsupported` naming `FiniteWindow`; `Unsupported` naming `MixedDirection` |

    | 3 | 3 | each of `once`, `historically`, `since`, `triggered` alone under both
    false-extension profiles | `Supported`, `mltl.origin-complete-history/v1` |

    | 4 | 4 | a finite-window clause with mixed directions; an infinite-trace clause
    | `FiniteWindow` and `MixedDirection`; `InfiniteTrace` |

    | 5 | 5 |'
- id: FR-350
  path: agent-ix/quire-contract-ir/spec/temporal/functional/FR-350-native-temporal-tl-correspondence.md
  role: context_only
  excerpt: The correspondence record SHALL retain the package name, declaration and
    clause names, selected source `{authority, identity}` and span, selected model
    package identities from `CheckedPackageV2::selected_model_packages` in its admitted
    identity order, model/type/predicate bindings, anchor, captures, clock and observation
    binding, every interval, closure/history premises, the exact owner-admitted TL
    formula, request, trace/history and valuation values, their public versions and
    the bridge definition `{authority, identity}`.
```

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | At 3723bbea5b377ac1d627d6309fbd3c5d5d8a5c1e the blank line is removed; procedure rows 26, 27 and 28 are contiguous below the single table header. Rechecked the exact one-line diff and selected-document validation. |

Latest disposition verdict: **PASS**. FND-001 is fixed; no open findings remain.
