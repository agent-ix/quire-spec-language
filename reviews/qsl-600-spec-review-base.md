---
id: SR-4950
title: spec-review/base review of QSL-600 typed inspection
type: SpecReview
analysis: base
scope: agent-ix/quire-spec-language@331ae4dfc47168c8f717b26f6c28066584b61233; spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md;
  spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
review_set: subset
---

## Summary

Ticket: QSL-600. Static review of PR #712. FAIL: nine acceptance procedures are coherent, but the AC-3 procedure selects a view its producer cannot supply. Authored procedures cover AC-1 through AC-10; executable coverage is pending, and this spec deliverable cannot close QSL-600 implementation acceptance.

## Examined scope

```yaml
scope:
- id: FR-297-AC-1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: The package view of FR-027-AC-10's library request shows its `package_id`, the `test/geometry` dependency
    with its `package_id`, its exported declarations and its requirement records, each equal to the package's own.
    Model selections and the capability report equal the package's own; a package with no dependencies or requirements
    retains those empty collections.
- id: FR-297-AC-2
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: The node view selected by `seven`'s occurrence key shows kind function, its result type and the source
    region of its declaration, and that region's bytes equal the declaration's text in the source; a selector naming
    no node refuses, naming the selector and the `package_id`.
- id: FR-297-AC-3
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: The counterexample view of FR-281-AC-2's record shows each assignment under its parameter name and the
    function's `QualifiedName`; the view of FR-290-AC-1's record shows the `trusted` label.
- id: FR-297-AC-4
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: The trace view of FR-283-AC-2's lasso shows the four positions' states, the loop start at position 2
    and the deciding position; the state-graph view of an FR-120 simulation run shows exactly the states and transitions
    the run retained. The outcome view of FR-286-AC-2's refusal shows its cause, catalog code and `Locus`.
- id: FR-297-AC-5
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: For one checked node with two distinct occurrences, one holding two regions, `NodeKey` selection retains
    both occurrences and all regions; occurrence selection retains only that occurrence's regions in stored order.
    An absent role/ordinal refuses as unknown occurrence; a node outside the selected package refuses as unknown
    node. Neither returns a view.
- id: FR-297-AC-6
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: An outcome with a region locus preserves it; one with an occurrence locus resolves exactly the associated
    map's regions while retaining the original locus; an unmapped occurrence refuses. An artifact locus preserves
    its artifact position with no source region; an absent locus remains absent. These inspections succeed without
    source bytes whenever correspondence is resolvable.
- id: FR-297-AC-7
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: A finite trace preserves its positions, repeated states and absence of a loop start. A lasso retains
    its original position count and loop start. A retained partial graph preserves its exact states, transitions
    and partial status; retained empty and unretained graphs respectively produce an empty view and unavailable-retained-content
    refusal.
- id: FR-297-AC-8
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: Views of refused, incomplete, unsupported and trusted terminal records preserve each record's disposition,
    terminal value, cause, code and optional locus as present, including the trust basis. No view promotes any of
    those records to a certified proof.
- id: FR-297-AC-9
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: Each incompatible input/view combination is either unconstructible through the public typed request API
    or returns an incompatible-selection refusal naming the selector. Raw bytes, rendered text and unchecked syntax
    cannot construct a package/node inspection request; compiling controls use the checked input types.
- id: FR-297-AC-10
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: Repeated equal requests with equal limits and an uncancelled handle yield equal views. A precancelled
    request returns cancelled with no view; a request stopped by a configured inspection limit names that limit
    and returns no view. Successful inspection reports no work in the stages that produced its input.
- id: FR-297/Inputs/1.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: 'A typed request containing a package, outcome, counterexample, trace or

    retained graph and its view selector; the package''s associated source map;

    limits and `&Cancel`

    ([FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md)).

    Package and node inputs carry checked package authority. An outcome, trace,

    counterexample or graph input carries the producing operation''s typed value;

    serialized JSON and rendered text do not establish that authority.'
- id: FR-297/Inputs/2.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: 'The selector identifies one of the six views above. A node selector contains

    either a `NodeKey` scoped to the selected package or an `OccurrenceKey` in

    that package''s source map. It does not accept a printed declaration name as

    a substitute for either identity.'
- id: FR-297/Outputs/1.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: '`Result<Staged<View>, StageFailure<InspectCause>>`, following

    [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md).

    `View` has distinct typed variants for the six rows above. Its fields retain

    the owning types for package and node identity, values, terminal records,

    source regions and loci; display strings do not replace them.'
- id: FR-297/Outputs/2.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: 'Inspection refusals distinguish an incompatible input/view selection, an

    unknown node, an unknown occurrence, unavailable retained content and

    unresolvable source correspondence. They retain the requested selector and,

    when the request selects a package, its `package_id`. Cancellation, limits

    and faults use the existing stage failure categories. A refusal or stopped

    operation returns no successful `View`.'
- id: FR-297/Behavior/1.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The `inspect` operation shall build the selected view from the typed\n  package, outcome or graph\
    \ and its source map."
- id: FR-297/Behavior/1.2
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- Each view shall locate source regions through the occurrence-keyed source\n  map, and shall not reparse\
    \ source text."
- id: FR-297/Behavior/1.3
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- If a node selector names no node of the package, then `inspect` shall\n  refuse naming the selector\
    \ and the package's `package_id`."
- id: FR-297/Behavior/1.4
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The counterexample view shall show each assignment under its parameter's\n  declared name and the\
    \ function's `QualifiedName`, and the `trusted` label\n  when the record carries it."
- id: FR-297/Behavior/3.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The `inspect` operation SHALL project only the supplied typed value; it\n  SHALL NOT execute, analyze,\
    \ monitor, replay or re-check a package to fill\n  missing view content."
- id: FR-297/Behavior/3.2
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When a selector is incompatible with the supplied input, `inspect` SHALL\n  refuse with an incompatible-selection\
    \ cause. A typed request may instead\n  exclude this combination at construction; it SHALL NOT silently select\
    \ a\n  different view."
- id: FR-297/Behavior/3.3
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When a node selector contains a `NodeKey`, `inspect` SHALL resolve it\n  through the selected package's\
    \ checked node identity and emission mapping.\n  A wire node id SHALL NOT be converted into a `NodeKey`."
- id: FR-297/Behavior/3.4
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When an occurrence selector names a mapped node with an absent role or\n  ordinal, `inspect` SHALL\
    \ refuse with an unknown-occurrence cause distinct\n  from an unknown-node cause."
- id: FR-297/Behavior/5.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When a node is selected by occurrence, its view SHALL retain exactly that\n  occurrence's regions\
    \ in their stored order, including repeated regions."
- id: FR-297/Behavior/5.2
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When a node is selected by `NodeKey`, its view SHALL retain every mapped\n  occurrence separately,\
    \ keyed by its role and ordinal, with each occurrence's\n  regions in their stored order. It SHALL NOT pick\
    \ one representative source\n  occurrence for a shared node."
- id: FR-297/Behavior/5.3
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- When an input carries a `Locus`, the view SHALL retain that locus unchanged.\n  A region locus denotes\
    \ that region; an occurrence locus resolves through\n  the associated package source map using\n  [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md).\n\
    \  An artifact locus retains its artifact position and denotes no source\n  region. An absent locus remains\
    \ absent."
- id: FR-297/Behavior/5.4
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- If a required occurrence cannot be resolved through the associated map,\n  then `inspect` SHALL return\
    \ the typed source-correspondence refusal and\n  SHALL NOT invent a region or consult another package's map."
- id: FR-297/Behavior/5.5
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The `inspect` operation SHALL require no source bytes to construct these\n  regions. Source snippets\
    \ and line/column rendering belong to\n  [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md)."
- id: FR-297/Behavior/7.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The package view SHALL preserve the complete dependency closure, model\n  selections, exported declarations,\
    \ requirement records and capability\n  report of the supplied package, including empty collections."
- id: FR-297/Behavior/7.2
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The outcome view SHALL preserve the selected item's disposition, terminal\n  result, typed cause,\
    \ catalog code and optional locus. Inspection SHALL NOT\n  strengthen a result's certification or turn a refused,\
    \ incomplete or\n  unsupported result into a successful result."
- id: FR-297/Behavior/7.3
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The trace view SHALL preserve the input's position order and multiplicity,\n  states, optional loop\
    \ start and optional deciding position. A finite trace\n  SHALL retain absence of a loop start, and inspection\
    \ SHALL NOT append a\n  closing state to a lasso."
- id: FR-297/Behavior/7.4
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The state-graph view SHALL contain exactly the states and transitions\n  retained by its producer,\
    \ including their identities and endpoints. A\n  partial retained graph remains partial; its producer's completion\
    \ status\n  SHALL remain visible. If no graph was retained, then `inspect` SHALL\n  refuse with unavailable-retained-content;\
    \ a retained empty graph remains\n  a successful empty view."
- id: FR-297/Behavior/7.5
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: "- The `inspect` operation SHALL apply the caller's limits and cancellation\n  through [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md)\n\
    \  and [FR-276](FR-276-cancel-a-lifecycle-operation.md). It SHALL NOT return\n  an unmarked truncated view when\
    \ inspection stops."
- id: FR-297/Behavior/8.1
  path: spec/functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md
  role: examined
  excerpt: 'The state-graph contract depends on the E11 retained-graph producer. Its

    absence does not authorize a replacement graph, a new execution, or a claim

    that the state-graph acceptance is implemented. Rendering and CLI dispatch

    remain the separate contracts in

    [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md) and

    [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md).'
- id: TC-782/step-1
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "1. Build the package view of\n   [FR-027](../functional/FR-027-export-compiled-native-package.md) AC-10's\n\
    \   library request. Repeat with a checked package having no dependencies or\n   requirement records. Include\
    \ a package with model selections and a\n   capability report populated by its producer."
- id: TC-782/step-2
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "2. Build the node view selected by `seven`'s occurrence key over\n   `tests/fixtures/spine-compile.native`;\
    \ then select a node key the package\n   lacks. Compare the returned region with the declaration's independently\n\
    \   located source bytes."
- id: TC-782/step-3
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "3. Build the counterexample views of\n   [FR-281](../functional/FR-281-analyze-claims-with-in-process-engines.md)\n\
    \   AC-2's record and\n   [FR-290](../functional/FR-290-settle-plugin-results-as-typed-terminal-records.md)\n\
    \   AC-1's record."
- id: TC-782/step-4
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "4. Build the trace view of\n   [FR-283](../functional/FR-283-monitor-a-supplied-trace-offline.md)\n\
    \   AC-2's lasso, the state-graph view of an\n   [FR-120](../functional/FR-120-simulate-a-checked-package-s-state-family.md)\n\
    \   simulation run, and the outcome view of\n   [FR-286](../functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md)\n\
    \   AC-2's refusal."
- id: TC-782/step-5
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "5. Use a checked node with two source occurrences at distinct role/ordinal\n   pairs; give one occurrence\
    \ two ordered regions. Inspect by `NodeKey`,\n   then by each `OccurrenceKey`. Select an absent ordinal, an\
    \ absent role,\n   and a node outside the selected package. Include repeated regions in one\n   occurrence to\
    \ check that projection preserves multiplicity."
- id: TC-782/step-6
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "6. Inspect outcomes carrying each of `Locus::Region`, `Locus::Occurrence`,\n   `Locus::Artifact`, and\
    \ no locus. Include a resolvable occurrence with two\n   regions and an unresolvable occurrence. Supply no source\
    \ bytes; for the\n   unresolvable case, also make the same occurrence available only in an\n   unrelated package\
    \ map which the request does not select."
- id: TC-782/step-7
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "7. Inspect a finite trace with a repeated state and no loop start, the lasso\n   from step 4, a partial\
    \ retained graph, an explicitly retained empty graph,\n   and a result which retained no graph. Include a trace\
    \ with no deciding\n   position so that absence is compared directly."
- id: TC-782/step-8
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "8. Inspect refused, incomplete, unsupported and trusted terminal records.\n   Capture each input's item,\
    \ disposition, terminal value, typed cause,\n   catalog code, optional locus and trust basis before the call."
- id: TC-782/step-9
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "9. Cover every incompatible pairing of the six view selectors and supported\n   typed input categories.\
    \ For combinations excluded by the public request\n   types, use compile-fail cases with compiling positive\
    \ controls; for\n   representable combinations, call `inspect` and check its typed refusal.\n   Add compile-fail\
    \ controls passing raw bytes, rendered text and unchecked\n   syntax where package/node inspection requires\
    \ checked authority."
- id: TC-782/step-10
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "10. Repeat each successful request with equal limits. Call with a precancelled\n    handle, then with\
    \ each inspection limit one below the corresponding\n    counter reached by a successful control and at that\
    \ counter. Check the\n    operation's accounting for work in predecessor stages."
- id: TC-782/expected-2
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: 'Step 1: it shows the `package_id`, the `test/geometry` dependency with its `package_id`, the exported
    declarations and the requirement records, each equal to the package''s.'
- id: TC-782/expected-3
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: 'Step 2: kind function, the result type and a source region whose bytes equal the declaration''s text;
    the unknown selector refuses naming the selector and the `package_id`.'
- id: TC-782/expected-4
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: 'Step 3: each assignment under its parameter name and the function''s `QualifiedName`; the second shows
    the `trusted` label.'
- id: TC-782/expected-5
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: 'Step 4: four positions'' states, loop start at position 2 and the deciding position; exactly the retained
    states and transitions; the cause, catalog code and `Locus`.'
- id: TC-782/expected-6
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 1 also preserves model selections and the capability report exactly;\n  empty collections remain\
    \ empty."
- id: TC-782/expected-7
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 5: node selection retains both separately keyed occurrences;\n  occurrence selection retains only\
    \ the selected regions in their original\n  order and multiplicity. Absent role/ordinal yields unknown occurrence;\n\
    \  the foreign node yields unknown node. Each refusal retains selector and\n  package identity and returns no\
    \ view."
- id: TC-782/expected-8
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 6: the original locus is preserved. Region and resolvable occurrence\n  loci denote exactly the\
    \ expected regions; artifact and absent loci denote\n  none. An unresolvable occurrence refuses without substituting\
    \ the\n  unrelated map or a synthesized region. Source bytes are not needed."
- id: TC-782/expected-9
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 7: position count, order, repeated states, loop start and deciding\n  position (including absence)\
    \ equal the producer's values. The partial\n  graph retains every retained state and transition with original\
    \ identities\n  and endpoints and remains marked partial. Empty retained graphs succeed;\n  unretained graphs\
    \ refuse as unavailable-retained-content. No producer is\n  rerun to obtain additional graph content."
- id: TC-782/expected-10
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 8: every captured field equals its input field. Trusted stays trusted;\n  refused, incomplete and\
    \ unsupported dispositions are unchanged."
- id: TC-782/expected-11
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 9: every incompatible pair fails construction or refuses as\n  incompatible selection; none falls\
    \ back to another view. Negative\n  authority controls fail to compile and their checked-input controls\n  compile."
- id: TC-782/expected-12
  path: spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md
  role: examined
  excerpt: "Step 10: repeated calls yield equal views; precancellation returns\n  cancelled and limit exhaustion\
    \ identifies its configured limit, both\n  without a successful or truncated view. The inclusive boundary succeeds.\n\
    \  Successful inspections do no work in source parsing, checking, execution,\n  analysis, monitoring or replay\
    \ stages."
- id: FR-095
  path: spec/functional/FR-095-occurrence-keyed-source-map-and-locus.md
  role: context_only
  excerpt: "The types are layer F's (ADR-011 \xA76.1): `qsl_foundation::source::provenance`\nholds the occurrence\
    \ key, `RawSourceRef`, `SourceRegion` and\n`PackageSourceMap`, and `qsl_foundation::diagnostic` holds `Locus`.\
    \ The\nlayer-4 I2 reader builds the package source map from a verified\n`quire.checked-package/v2` wire's `source_map`\
    \ (ADR-013 C-14)."
- id: FR-275
  path: spec/functional/FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md
  role: context_only
  excerpt: '| Operation | Takes | Returns | Behaviour |

    | --- | --- | --- | --- |

    | `parse` | source bytes and `SourceIdentity` | `Staged<ParsedSource>` | FR-278 |

    | `format` | an admissible `ParsedSource` | `Staged<FormattedSource>`, the formatted bytes | FR-003 |

    | `select` | domain package documents by digest, the unit''s model selections | `Staged<AdmittedModels>` | FR-278
    |

    | `check` | `ParsedSource`, `AdmittedModels`, the dependency input, lock evidence | `Staged<CheckedPackage>`
    | FR-278 |

    | `check_fences` | a spec artifact inventory and the module manifests that type it | `Staged<FenceReport>` |
    FR'
- id: FR-276
  path: spec/functional/FR-276-cancel-a-lifecycle-operation.md
  role: context_only
  excerpt: '`StageFailure` gains the variant `Cancelled(CancelCause)` and `CallFailure`

    gains the variant `Cancelled(CancelCause)` (ADR-029 Amendments, ADR-013 T-4).'
- id: FR-277
  path: spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md
  role: context_only
  excerpt: 'Nesting depth is not a limit. Node, byte and work limits bound every stage,

    and no stage recurses on the native stack (ADR-030 D-1). ADR-030 owns the

    depth rule; this requirement states only that the lifecycle operations take

    their limits from the caller.'
- id: FR-286
  path: spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md
  role: context_only
  excerpt: "- `format`: the document's format identity, `quire-outcome/1`.\n- `operation`: the library operation\
    \ that produced it.\n- `last_stage`: the last ADR-011 stage the operation reached, or `null`\n  when no stage\
    \ is known: a cancelled outcome that stopped before its first\n  stage, or an internal-failure (fault) outcome.\n\
    - `category`: the ADR-013 O-16 category of the whole outcome, as the one\n  `Category` type (FR-285); each item's\
    \ category is the same type.\n- `items`: for `prove`, `analyze` and `monitor`, one entry per requested item\n\
    \  with its terminal record and its category. The member is always p"
- id: FR-290
  path: spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md
  role: context_only
  excerpt: 'This requirement states how a plugin''s result becomes a QSL terminal record:'
- id: FR-298
  path: spec/functional/FR-298-render-outcomes-and-views-from-typed-values.md
  role: context_only
  excerpt: 'Text forms:'
- id: FR-290-AC-1
  path: spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md
  role: context_only
  excerpt: A process-provider FR-331 result of `proved` for a routed item settles a terminal record with value `proved`,
    the provider's `BackendId` and certification `trusted`; the FR-281-AC-1 record's certification is `certified`,
    naming the EN-5 checker; FR-281-AC-4's record from the test engine with no checker has certification `uncertified`,
    naming that engine; a Kani `proved` record built as C-09 settles it carries `certified`.
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-782 step 3 requests a counterexample view from FR-290-AC-1's proved/trusted record, which supplies no counterexample. This contradicts FR-297's supplied-value-only projection and incompatible-selection rules. Use the outcome view for the trusted record and keep the counterexample view on FR-281-AC-2; make FR-297-AC-3 and step 3's expected result explicit. | spec/test-cases/TC-782-inspect-builds-each-typed-view-from-the-source-map.md:39 |

## Verdict

FAIL: nine acceptance procedures are coherent, but the AC-3 procedure selects a view its producer cannot supply. Authored procedures cover AC-1 through AC-10; executable coverage is pending, and this spec deliverable cannot close QSL-600 implementation acceptance.

## Validation and limits

Changed-file validation: 2/2 docs grammar-clean, zero grammar findings. Full spec validation: exit 0, 1536/1544 grammar-clean, 17 warnings in unchanged files. git diff --check passed. Installed quoin 0.28.3 and quire 0.36.2 (engine 0.50.2). Module registration emitted DuplicateArchetype, DuplicateInverseEdge and semantic.inline-data-schema advisories. No builds, lock jobs or executable tests ran. No installed AssuranceProfile applies. Computed quire matrix completed successfully: FR-297-AC-1 through FR-297-AC-10 are all untagged; static Rust trace search likewise found no FR-297 or TC-782 tags. These are explicit implementation obligations retained on QSL-600, not evidence of passing executable acceptance.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 3dd0ed3e81a889a6845f756b2700c9a0824ae1a1: TC-782 step 3 now uses the outcome view for the proved/trusted record; FR-297-AC-3 and step 3's expected result agree. |

## Disposition pass 1

Reviewed 3dd0ed3e81a889a6845f756b2700c9a0824ae1a1. Latest verdict: PASS for this static specification deliverable. The original finding and review verdict above remain the historical review record. No new findings. The counterexample view remains tied to FR-281-AC-2; the trusted plugin record is inspected as an outcome without deriving assignments.

Both changed artifacts passed scoped Quire validation (2/2 grammar-clean; zero grammar findings), and git diff --check passed. Module discovery emitted the same duplicate archetype/inverse-edge and inline-data-schema advisories. No builds, lock jobs or executable tests ran. QSL-600 implementation acceptance and producer-dependent checks remain open; the prior matrix showed all ten criteria untagged.

Fixed TC-782 step 3:

```text
3. Build the counterexample view of
   [FR-281](../functional/FR-281-analyze-claims-with-in-process-engines.md)
   AC-2's record. Build the outcome view of
   [FR-290](../functional/FR-290-settle-plugin-results-as-typed-terminal-records.md)
   AC-1's `proved`/`trusted` plugin record, which carries no counterexample
   assignments.
```
