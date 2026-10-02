---
id: FR-345
title: "Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: traces_to
  - target: ix://agent-ix/quire-specification/AD-003
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-453
    type: depends_on
---
# FR-345: Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal

## Description

QSL SHALL provide `xtask refinement layering <corpus>`, a test gate that
implements QSpec FR-453's profile-layering gate for QSL's compiler (ADR-017
RF-3; QSpec capability row V1-TOOL-012). QSpec FR-453 owns the five AD-003
`requires` edges, the two-unit case, the comparison rules and the
user-subset check. This requirement fixes what QSL builds: the corpus
reader, the compile of each unit through spine `compile` under FR-110's
layer selection, the mapping of FR-341's classes onto FR-453's results, and
the request of each layer's witness item through capability negotiation,
and the report through FR-344.

Like FR-340's gate, it is a behavioural comparison. It records no
requirement record and settles no claim; the only backend request it makes
is the negotiation of each layer's witness item.

## Inputs

- `<corpus>`: one directory. Each immediate subdirectory is one entry,
  holding an `entry.json` file and the files it names.
- `entry.json`, a JSON object with exactly these members:
  - `kind`: `case`, `witness` or `distinguishing`;
  - for `case` and `distinguishing`: `parent` and `child`, each `{path,
    authority, identity, revision_namespace, revision}`, a unit source and
    its four FR-001 labels; for `witness`: `unit`, of the same shape, and
    `layer`, the layer identity the unit's header names;
  - `packages` and `dependencies`: the FR-056 package files and FR-099
    dependency input files the entry's units use;
  - optionally `limits`: the compile limit set spine `compile` takes.

  Every file is named by a path relative to the entry's directory.

## Outputs

One case result per `case` entry, one layer result per layer and one edge
result per edge, and one tool-failure result per malformed entry, missing
entry or empty corpus, all passed to FR-344's report.

## Behavior

### Reading the corpus

- The gate SHALL read every byte a run uses from the files the corpus's
  `entry.json` files name, and SHALL read no environment variable, clock,
  search location or path outside the entry's directory.
- If an `entry.json` cannot be read, is not a JSON object, holds a member
  this requirement does not list for its `kind` or lacks one it lists, or
  names a path that is absolute or resolves outside the entry's directory,
  then the gate SHALL record one tool-failure result naming the
  `entry.json` path and the defect, compile none of its units, and continue
  with the other entries.
- If a `case` or `distinguishing` entry's parent and child units differ in
  any byte outside the identity string of their header `profile`
  declaration, or their header identities are not the parent and child of
  one QSpec FR-453 edge, then the gate SHALL record one tool-failure result
  naming the entry. If a `witness` unit's header names a layer other than
  its `layer`, or names more than one layer, the same holds.
- If the corpus holds no entry, then the gate SHALL record exactly one
  tool-failure result naming the corpus path, and no other result.
- If the corpus holds at least one entry, and any of the five layers has no
  `witness` entry, or any of the five edges has no `case` entry or no
  `distinguishing` entry, then the gate SHALL record one tool-failure
  result per missing item, naming the layer or edge and the missing kind.
  An empty or partial corpus never reports success.

### Compiling and comparing

- The gate SHALL compile each unit through `qsl_replay::spine::compile`
  with the entry's packages, dependency input and limits (each unstated
  limit at its published default), and classify each result by FR-341.
  Both units of every `case` and `distinguishing` entry are compiled by the
  running build; no outcome is read from the corpus.
- The gate SHALL map each `case` entry's parent class `R` and child class
  `C` to QSpec FR-453's case result: FR-341's `refused` is FR-453's typed
  refusal, `prohibited` is a prohibition, `admitted` is admitted,
  `unsupported` and `incomplete` are unsupported and incomplete, and
  `tool failure` is a tool failure. Reported as:

| `R` | `C` | Case result |
| --- | --- | --- |
| `tool failure` | any | `tool failure` |
| any | `tool failure` | `tool failure` |
| `refused` | `refused` | `holds`, reporting both sides' codes |
| `refused` | `admitted` or `prohibited` | `regression` |
| `refused` | `unsupported` | `unresolved (unsupported)` |
| `refused` | `incomplete` | `unresolved (incomplete)` |
| `incomplete` | any | `unresolved (incomplete)` |
| otherwise | any | `not applicable` |

- For each `witness` entry the unit admits, the gate SHALL submit its one
  clause as an item of a QSpec FR-331 request whose required FR-290 kind
  and extent classification are the layer's `witness_request` in QSpec's
  `header_selectable_layers` row, with the registry's candidate set, and
  read the item's disposition from `negotiate_*` (QSpec FR-331-AC-4).
- The gate SHALL give each layer FR-453's witness result: `holds` when every
  `witness` entry for the layer classifies `admitted` and its item settles
  `supported`, and `regression` otherwise, naming the layer and either the
  compile codes or the item's disposition and candidate backend
  identities.
- The gate SHALL give each edge FR-453's proper-subset result: `holds` when
  every `distinguishing` entry for the edge classifies `prohibited` on the
  parent side, naming the parent layer, and `admitted` on the child side,
  and `regression` naming the edge otherwise.

### Compile seam

The gate's comparison core SHALL take the compile function and the backend
registry snapshot as parameters. `xtask refinement layering` passes
`qsl_replay::spine::compile` and QSL's registry snapshot and nothing else.
A test substitutes a wrapper that returns a chosen result for a named unit,
and a test registry; the wrapper lives only in the test module of the gate's crate
(`#[cfg(test)]`), so no build of `xtask` or of the compiler carries a fault
hook.

### Report and where it runs

- The gate SHALL report and exit by FR-344, over every case, layer, edge
  and tool-failure result.
- Each case SHALL be named by its two units' `RawSourceRef`s and its edge's
  two layer identities, never by a display string or a directory name.
- The repository's local test target SHALL run the gate over
  `tests/fixtures/refinement/layering/`, the real corpus, and SHALL fail
  when the gate's exit code is not 0.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-345-AC-1 | For every combination of `R` and `C` class, the comparison returns the result the table gives; each expected result in the test is a literal, not computed by the gate's code. | Test (TC-867) |
| FR-345-AC-2 | A case whose parent compile refuses with a typed refusal and whose child compile admits is a `regression` naming both units' `RawSourceRef`s, the edge's two identities and the parent's codes; FR-344 reports verdict violation, exit 10. | Test (TC-867) |
| FR-345-AC-3 | A case whose parent class is `prohibited` and whose child admits is `not applicable`. | Test (TC-867) |
| FR-345-AC-4 | Over a layering corpus with cases on each of QSpec FR-453's edges E1 to E5, one seeded case per edge that its parent refuses and its child admits fails the gate naming that case and its edge, and every other case holds or is not applicable. | Test (TC-868) |
| FR-345-AC-5 | A case whose two units differ outside the header identity string, and a case naming state core and complete model, are each `tool failure` naming the case; verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-6 | Each of the five layers admits its witness unit and the witness item, requested with the layer's `witness_request` kind and extent, settles `supported`; a layer that refuses its witness, or a witness item that settles `unsupported`, gives a `regression` naming the layer and the codes or the disposition and candidates; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-7 | On each of E1 to E5, the parent prohibits the distinguishing unit naming the parent layer and the child admits it; a parent that admits it gives a `regression` naming the edge; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-8 | An empty corpus gives one tool-failure result naming the corpus; a corpus with no `witness` entry for state graph and no `distinguishing` entry for E5 gives exactly two tool-failure results naming state graph / `witness` and E5 / `distinguishing`; an `entry.json` with an extra member `expected` gives one tool-failure result naming its path and the defect while the other entries still compile; each verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-9 | The local test target runs the gate over `tests/fixtures/refinement/layering/` and passes; removing one `witness` entry from that corpus makes the target fail. | Test (TC-868) |

## Dependencies

- QSpec FR-290 (capability kinds) and FR-331 (the request and
  `negotiate_*`) for the witness item.
- FR-341 (compile classification), FR-344 (report, verdict and exit),
  FR-001 (source labels), FR-056 and FR-099 (package and dependency input).
- FR-110 (layer selection: a header naming any AD-003 layer resolves to it,
  and S3 admits under its admitted-form set).
- QSpec AD-003 (the profile hierarchy and its `requires` edges) and QSpec
  FR-453 (the edges, the two-unit case and the user-subset check).

## References

- ADR-017 §2 RF-3, RF-4, RF-5; §6 Q-5.
- QSpec FR-453 (STD-119, owner ruling option A: every AD-003 layer
  header-selectable, all five edges compared); QSpec FR-452, FR-290 and the
  V1-TOOL-012 re-trace (STD-117).
- Implementation ticket QSL-39; specification ticket QSL-387.

## Status

Specified; not yet implemented -- TC-867 and TC-868 planned.
