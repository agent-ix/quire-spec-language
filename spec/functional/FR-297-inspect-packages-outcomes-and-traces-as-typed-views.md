---
id: FR-297
title: "Inspect packages, nodes, outcomes, counterexamples, traces and state graphs as typed views"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-029
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-029
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-095
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-290
    type: references
  - target: "ix://agent-ix/quire-specification/FR-300"
    type: depends_on
---
# FR-297: Inspect packages, nodes, outcomes, counterexamples, traces and state graphs as typed views

## Description

`inspect` returns typed views (ADR-029 IN-1). It lives in QSL layer P, crate
`qsl-inspect`, above the qualified core. Each view locates source through the
occurrence-keyed source map (FR-095), never by reparsing text (ADR-011
FB-01).

| View | Shows |
| --- | --- |
| Package | `package_id`, dependency closure, model selections, exported declarations, requirement records, the v2 capability report |
| Node | A checked node by `NodeKey` or occurrence key: kind, type, source regions, requirements |
| Outcome | Item, disposition, terminal result, typed cause, catalog code, `Locus` |
| Counterexample | Decoded assignments under the parameter names and the function's `QualifiedName`, with the `trusted` label where it applies (FR-290) |
| Trace | A finite trace or a lasso: positions, loop start, the state at each position, the deciding position |
| State graph | The states and transitions a simulation (FR-120) or model-check run retained |

## Inputs

A typed request containing a package, outcome, counterexample, trace or
retained graph and its view selector; the package's associated source map;
limits and `&Cancel`
([FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md)).
Package and node inputs carry checked package authority. An outcome, trace,
counterexample or graph input carries the producing operation's typed value;
serialized JSON and rendered text do not establish that authority.

The selector identifies one of the six views above. A node selector contains
either a `NodeKey` scoped to the selected package or an `OccurrenceKey` in
that package's source map. It does not accept a printed declaration name as
a substitute for either identity.

## Outputs

`Result<Staged<View>, StageFailure<InspectCause>>`, following
[FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md).
`View` has distinct typed variants for the six rows above. Its fields retain
the owning types for package and node identity, values, terminal records,
source regions and loci; display strings do not replace them.

Inspection refusals distinguish an incompatible input/view selection, an
unknown node, an unknown occurrence, unavailable retained content and
unresolvable source correspondence. They retain the requested selector and,
when the request selects a package, its `package_id`. Cancellation, limits
and faults use the existing stage failure categories. A refusal or stopped
operation returns no successful `View`.

## Behavior

- The `inspect` operation shall build the selected view from the typed
  package, outcome or graph and its source map.
- Each view shall locate source regions through the occurrence-keyed source
  map, and shall not reparse source text.
- If a node selector names no node of the package, then `inspect` shall
  refuse naming the selector and the package's `package_id`.
- The counterexample view shall show each assignment under its parameter's
  declared name and the function's `QualifiedName`, and the `trusted` label
  when the record carries it.

### Typed projection and selection

- The `inspect` operation SHALL project only the supplied typed value; it
  SHALL NOT execute, analyze, monitor, replay or re-check a package to fill
  missing view content.
- When a selector is incompatible with the supplied input, `inspect` SHALL
  refuse with an incompatible-selection cause. A typed request may instead
  exclude this combination at construction; it SHALL NOT silently select a
  different view.
- When a node selector contains a `NodeKey`, `inspect` SHALL resolve it
  through the selected package's checked node identity and emission mapping.
  A wire node id SHALL NOT be converted into a `NodeKey`.
- When an occurrence selector names a mapped node with an absent role or
  ordinal, `inspect` SHALL refuse with an unknown-occurrence cause distinct
  from an unknown-node cause.

### Source correspondence

- When a node is selected by occurrence, its view SHALL retain exactly that
  occurrence's regions in their stored order, including repeated regions.
- When a node is selected by `NodeKey`, its view SHALL retain every mapped
  occurrence separately, keyed by its role and ordinal, with each occurrence's
  regions in their stored order. It SHALL NOT pick one representative source
  occurrence for a shared node.
- When an input carries a `Locus`, the view SHALL retain that locus unchanged.
  A region locus denotes that region; an occurrence locus resolves through
  the associated package source map using
  [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md).
  An artifact locus retains its artifact position and denotes no source
  region. An absent locus remains absent.
- If a required occurrence cannot be resolved through the associated map,
  then `inspect` SHALL return the typed source-correspondence refusal and
  SHALL NOT invent a region or consult another package's map.
- The `inspect` operation SHALL require no source bytes to construct these
  regions. Source snippets and line/column rendering belong to
  [FR-298](FR-298-render-outcomes-and-views-from-typed-values.md).

### Result fidelity and bounded work

- The package view SHALL preserve the complete dependency closure, model
  selections, exported declarations, requirement records and capability
  report of the supplied package, including empty collections.
- The outcome view SHALL preserve the selected item's disposition, terminal
  result, typed cause, catalog code and optional locus. Inspection SHALL NOT
  strengthen a result's certification or turn a refused, incomplete or
  unsupported result into a successful result.
- The trace view SHALL preserve the input's position order and multiplicity,
  states, optional loop start and optional deciding position. A finite trace
  SHALL retain absence of a loop start, and inspection SHALL NOT append a
  closing state to a lasso.
- The state-graph view SHALL contain exactly the states and transitions
  retained by its producer, including their identities and endpoints. A
  partial retained graph remains partial; its producer's completion status
  SHALL remain visible. If no graph was retained, then `inspect` SHALL
  refuse with unavailable-retained-content; a retained empty graph remains
  a successful empty view.
- The `inspect` operation SHALL apply the caller's limits and cancellation
  through [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md)
  and [FR-276](FR-276-cancel-a-lifecycle-operation.md). It SHALL NOT return
  an unmarked truncated view when inspection stops.

The state-graph contract depends on the E11 retained-graph producer. Its
absence does not authorize a replacement graph, a new execution, or a claim
that the state-graph acceptance is implemented. Rendering and CLI dispatch
remain the separate contracts in
[FR-298](FR-298-render-outcomes-and-views-from-typed-values.md) and
[FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-297-AC-1 | The package view of FR-027-AC-10's library request shows its `package_id`, the `test/geometry` dependency with its `package_id`, its exported declarations and its requirement records, each equal to the package's own. Model selections and the capability report equal the package's own; a package with no dependencies or requirements retains those empty collections. | Test |
| FR-297-AC-2 | The node view selected by `seven`'s occurrence key shows kind function, its result type and the source region of its declaration, and that region's bytes equal the declaration's text in the source; a selector naming no node refuses, naming the selector and the `package_id`. | Test |
| FR-297-AC-3 | The counterexample view of FR-281-AC-2's record shows each assignment under its parameter name and the function's `QualifiedName`; the view of FR-290-AC-1's record shows the `trusted` label. | Test |
| FR-297-AC-4 | The trace view of FR-283-AC-2's lasso shows the four positions' states, the loop start at position 2 and the deciding position; the state-graph view of an FR-120 simulation run shows exactly the states and transitions the run retained. The outcome view of FR-286-AC-2's refusal shows its cause, catalog code and `Locus`. | Test |
| FR-297-AC-5 | For one checked node with two distinct occurrences, one holding two regions, `NodeKey` selection retains both occurrences and all regions; occurrence selection retains only that occurrence's regions in stored order. An absent role/ordinal refuses as unknown occurrence; a node outside the selected package refuses as unknown node. Neither returns a view. | Test |
| FR-297-AC-6 | An outcome with a region locus preserves it; one with an occurrence locus resolves exactly the associated map's regions while retaining the original locus; an unmapped occurrence refuses. An artifact locus preserves its artifact position with no source region; an absent locus remains absent. These inspections succeed without source bytes whenever correspondence is resolvable. | Test |
| FR-297-AC-7 | A finite trace preserves its positions, repeated states and absence of a loop start. A lasso retains its original position count and loop start. A retained partial graph preserves its exact states, transitions and partial status; retained empty and unretained graphs respectively produce an empty view and unavailable-retained-content refusal. | Test |
| FR-297-AC-8 | Views of refused, incomplete, unsupported and trusted terminal records preserve each record's disposition, terminal value, cause, code and optional locus as present, including the trust basis. No view promotes any of those records to a certified proof. | Test |
| FR-297-AC-9 | Each incompatible input/view combination is either unconstructible through the public typed request API or returns an incompatible-selection refusal naming the selector. Raw bytes, rendered text and unchecked syntax cannot construct a package/node inspection request; compiling controls use the checked input types. | Test |
| FR-297-AC-10 | Repeated equal requests with equal limits and an uncancelled handle yield equal views. A precancelled request returns cancelled with no view; a request stopped by a configured inspection limit names that limit and returns no view. Successful inspection reports no work in the stages that produced its input. | Test |

## Dependencies

- ADR-029 IN-1: the views.
- ADR-011 FB-01: no reparsing.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): the source map.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- [FR-276](FR-276-cancel-a-lifecycle-operation.md) and [FR-277](FR-277-bound-every-lifecycle-operation-by-caller-limits.md): cancellation and bounded inspection work.
- [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): outcome fields retain their existing meaning.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): simulation runs.
- [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md): the `trusted` label.
- QSpec FR-300: the `inspect` operation.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06): inspection.
- QSL-600 (IN1): typed views; LC4 supplies typed outcomes and E11 supplies retained state graphs.
- QSpec FR-300 (STD-141): the QSpec half.
