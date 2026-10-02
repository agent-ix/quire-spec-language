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

A package, an outcome or a retained graph; a view selector; the source map;
limits and `&Cancel` (FR-275).

## Outputs

`Staged<View>`, or a refusal naming the selector that matched nothing.

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

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-297-AC-1 | The package view of FR-027-AC-10's library request shows its `package_id`, the `test/geometry` dependency with its `package_id`, its exported declarations and its requirement records, each equal to the package's own. | Test (TC-782) |
| FR-297-AC-2 | The node view selected by `seven`'s occurrence key shows kind function, its result type and the source region of its declaration, and that region's bytes equal the declaration's text in the source; a selector naming no node refuses, naming the selector and the `package_id`. | Test (TC-782) |
| FR-297-AC-3 | The counterexample view of FR-281-AC-2's record shows each assignment under its parameter name and the function's `QualifiedName`; the view of FR-290-AC-1's record shows the `trusted` label. | Test (TC-782) |
| FR-297-AC-4 | The trace view of FR-283-AC-2's lasso shows the four positions' states, the loop start at position 2 and the deciding position; the state-graph view of an FR-120 simulation run shows exactly the states and transitions the run retained. The outcome view of FR-286-AC-2's refusal shows its cause, catalog code and `Locus`. | Test (TC-782) |

## Dependencies

- ADR-029 IN-1: the views.
- ADR-011 FB-01: no reparsing.
- [FR-095](FR-095-occurrence-keyed-source-map-and-locus.md): the source map.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the call shape.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md): simulation runs.
- [FR-290](FR-290-settle-plugin-results-as-typed-terminal-records.md): the `trusted` label.
- QSpec FR-300: the `inspect` operation.

## References

- QSL-390 (ARCH-50), QSL-393 (V1-A06): inspection.
- QSpec FR-300 (STD-141): the QSpec half.
