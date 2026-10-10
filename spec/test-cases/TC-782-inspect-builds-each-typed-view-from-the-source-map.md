---
id: TC-782
title: "inspect builds each typed view from the source map"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-297
    type: verifies
---
# TC-782: inspect builds each typed view from the source map

## Description

Verify that the six typed inspection views preserve their producing values,
source correspondence and result status, including selection errors and
stopped requests.

Scope: [FR-297](../functional/FR-297-inspect-packages-outcomes-and-traces-as-typed-views.md)
AC-1 through AC-10. This is the authored acceptance procedure; passing
evidence requires executable tests over the public `qsl-inspect` API. A
missing producer, including E11 for retained state graphs, leaves its checks
pending.

## Test Procedure

Use real checked packages and typed producer records. Capture the expected
fields directly from those inputs before inspection; do not use rendered
text or a second call to `inspect` as the oracle. Supply default sufficient
limits and an uncancelled handle except in the explicit stopped cases.

1. Build the package view of
   [FR-027](../functional/FR-027-export-compiled-native-package.md) AC-10's
   library request. Repeat with a checked package having no dependencies or
   requirement records. Include a package with model selections and a
   capability report populated by its producer.
2. Build the node view selected by `seven`'s occurrence key over
   `tests/fixtures/spine-compile.native`; then select a node key the package
   lacks. Compare the returned region with the declaration's independently
   located source bytes.
3. Build the counterexample views of
   [FR-281](../functional/FR-281-analyze-claims-with-in-process-engines.md)
   AC-2's record and
   [FR-290](../functional/FR-290-settle-plugin-results-as-typed-terminal-records.md)
   AC-1's record.
4. Build the trace view of
   [FR-283](../functional/FR-283-monitor-a-supplied-trace-offline.md)
   AC-2's lasso, the state-graph view of an
   [FR-120](../functional/FR-120-simulate-a-checked-package-s-state-family.md)
   simulation run, and the outcome view of
   [FR-286](../functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md)
   AC-2's refusal.
5. Use a checked node with two source occurrences at distinct role/ordinal
   pairs; give one occurrence two ordered regions. Inspect by `NodeKey`,
   then by each `OccurrenceKey`. Select an absent ordinal, an absent role,
   and a node outside the selected package. Include repeated regions in one
   occurrence to check that projection preserves multiplicity.
6. Inspect outcomes carrying each of `Locus::Region`, `Locus::Occurrence`,
   `Locus::Artifact`, and no locus. Include a resolvable occurrence with two
   regions and an unresolvable occurrence. Supply no source bytes; for the
   unresolvable case, also make the same occurrence available only in an
   unrelated package map which the request does not select.
7. Inspect a finite trace with a repeated state and no loop start, the lasso
   from step 4, a partial retained graph, an explicitly retained empty graph,
   and a result which retained no graph. Include a trace with no deciding
   position so that absence is compared directly.
8. Inspect refused, incomplete, unsupported and trusted terminal records.
   Capture each input's item, disposition, terminal value, typed cause,
   catalog code, optional locus and trust basis before the call.
9. Cover every incompatible pairing of the six view selectors and supported
   typed input categories. For combinations excluded by the public request
   types, use compile-fail cases with compiling positive controls; for
   representable combinations, call `inspect` and check its typed refusal.
   Add compile-fail controls passing raw bytes, rendered text and unchecked
   syntax where package/node inspection requires checked authority.
10. Repeat each successful request with equal limits. Call with a precancelled
    handle, then with each inspection limit one below the corresponding
    counter reached by a successful control and at that counter. Check the
    operation's accounting for work in predecessor stages.

Tag the tests `#[trace("TC-782", "<AC id>")]`.

## Expected Results

- Step 1: it shows the `package_id`, the `test/geometry` dependency with its `package_id`, the exported declarations and the requirement records, each equal to the package's.
- Step 2: kind function, the result type and a source region whose bytes equal the declaration's text; the unknown selector refuses naming the selector and the `package_id`.
- Step 3: each assignment under its parameter name and the function's `QualifiedName`; the second shows the `trusted` label.
- Step 4: four positions' states, loop start at position 2 and the deciding position; exactly the retained states and transitions; the cause, catalog code and `Locus`.
- Step 1 also preserves model selections and the capability report exactly;
  empty collections remain empty.
- Step 5: node selection retains both separately keyed occurrences;
  occurrence selection retains only the selected regions in their original
  order and multiplicity. Absent role/ordinal yields unknown occurrence;
  the foreign node yields unknown node. Each refusal retains selector and
  package identity and returns no view.
- Step 6: the original locus is preserved. Region and resolvable occurrence
  loci denote exactly the expected regions; artifact and absent loci denote
  none. An unresolvable occurrence refuses without substituting the
  unrelated map or a synthesized region. Source bytes are not needed.
- Step 7: position count, order, repeated states, loop start and deciding
  position (including absence) equal the producer's values. The partial
  graph retains every retained state and transition with original identities
  and endpoints and remains marked partial. Empty retained graphs succeed;
  unretained graphs refuse as unavailable-retained-content. No producer is
  rerun to obtain additional graph content.
- Step 8: every captured field equals its input field. Trusted stays trusted;
  refused, incomplete and unsupported dispositions are unchanged.
- Step 9: every incompatible pair fails construction or refuses as
  incompatible selection; none falls back to another view. Negative
  authority controls fail to compile and their checked-input controls
  compile.
- Step 10: repeated calls yield equal views; precancellation returns
  cancelled and limit exhaustion identifies its configured limit, both
  without a successful or truncated view. The inclusive boundary succeeds.
  Successful inspections do no work in source parsing, checking, execution,
  analysis, monitoring or replay stages.
