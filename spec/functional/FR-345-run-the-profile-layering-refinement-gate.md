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
  - target: ix://agent-ix/quire-integration/IT-015
    type: references
---
# FR-345: Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal

## Description

QSL SHALL provide `xtask refinement layering <corpus>`, a test gate that
implements QSpec FR-453's profile-layering gate for QSL's compiler (ADR-017
RF-3; QSpec capability row V1-TOOL-012). QSpec FR-453 owns the six AD-003
`requires` edges, the two-unit case, the comparison rules and the
user-subset check. This requirement fixes what QSL builds: the corpus
reader, the compile of each unit through spine `compile` under FR-110's
layer selection, the mapping of FR-341's classes onto FR-453's results, and
the report through FR-344. Its comparison core optionally negotiates each
layer's witness item when its caller supplies a negotiation function.

Like FR-340's gate, it is a behavioural comparison. It records no
requirement record and settles no claim. When its caller supplies a
negotiation function, its only backend request is negotiation of each
layer's witness item; without one, it makes no backend request.

## Inputs

- `<corpus>`: one directory. Each immediate subdirectory is one entry,
  holding an `entry.json` file and the files it names.
- `entry.json`, a JSON object with exactly these members:
  - `kind`: `case`, `witness` or `distinguishing`;
  - for `case` and `distinguishing`: `parent` and `child`, each `{path,
    authority, identity}`, a unit source and its two FR-001 labels; for `witness`: `unit`, of the same shape, and
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
- If the corpus holds at least one entry, and any of the six layers has no
  `witness` entry, or any of the six edges has no `case` entry or no
  `distinguishing` entry, then the gate SHALL record one tool-failure
  result per missing item, naming the layer or edge and the missing kind.
  An empty or partial corpus never reports success.

### Compiling and comparing

- The gate SHALL compile each unit through the FR-278 composition
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

- For each `witness` entry the unit admits, the gate SHALL take the
  registry's QSpec FR-290 candidate set for the layer's `witness_request`
  kind (QSpec `header_selectable_layers` row).
- When its caller supplies a negotiation function, the gate SHALL submit the
  entry's one clause in one QSpec FR-331 request per candidate, each naming
  that candidate's backend, with the `witness_request` kind and extent, and
  read each disposition from the function's result (QSpec FR-331-AC-4).
- With a negotiation function, the gate SHALL give each layer FR-453's
  witness result: `holds` when every `witness` entry for the layer
  classifies `admitted` and its item settles `supported` under at least one
  candidate, and `regression` otherwise, naming the layer and either the
  compile codes or every candidate's backend identity and disposition (or
  the empty candidate set).
- Without a negotiation function, the gate SHALL submit no request and SHALL
  NOT report `holds` for a witness item that was not requested. For each
  layer whose witness unit classifies `admitted` and whose candidate set is
  non-empty, it SHALL report `unresolved (unsupported)` with reason
  `not-negotiated`, naming the layer and candidate backends. If the witness
  unit does not classify `admitted`, it SHALL report `regression` naming the
  layer and compile codes; if its candidate set is empty, it SHALL report
  `regression` naming the layer and empty candidate set. The unresolved
  result gives FR-344's unsupported verdict and exit 21, so the no-negotiation
  path cannot pass the gate without evaluating any witness.
- The gate SHALL give each edge FR-453's proper-subset result: `holds` when
  every `distinguishing` entry for the edge classifies `prohibited` on the
  parent side, naming the parent layer, and `admitted` on the child side,
  and `regression` naming the edge otherwise.

### Comparison core and compile seam

- The gate's comparison core SHALL be the public function
  `xtask::refinement::layering::compare` of the `xtask` package's library
  target. It takes the corpus directory, the compile function, QSL's backend
  registry snapshot and an optional negotiation function, and returns
  FR-344's report value: every case, layer, edge and tool-failure result and
  the verdict. It SHALL write nothing to process streams.
- The negotiation function SHALL take one `WitnessRequest`, the candidate
  (`qsl_route::Candidate`) whose backend it names, the layer's
  `witness_request` kind and extent, and the witness entry's compiled clause,
  and return that request's `qsl_route::Disposition`.
- The module SHALL expose `REAL_CORPUS`, the path of
  `tests/fixtures/refinement/layering/` in the source tree the `xtask`
  package is built from.
- The `xtask` package SHALL take no dependency, in any table, on a
  quire-contract-codegen crate (FR-280-AC-3).
- `xtask refinement layering` SHALL pass `compare` the FR-278 composition
  and QSL's registry snapshot, and no negotiation function, then render the
  report and exit by FR-344.
- A test substitutes a wrapper that returns a chosen result for a named
  unit, a test registry and a test negotiation function that settles each
  request by a chosen disposition per backend; the wrapper and test
  negotiation function live only in the test module of the gate's crate
  (`#[cfg(test)]`), so no build of `xtask` or of the compiler carries a
  fault hook.

### Report and where it runs

- The gate SHALL report and exit by FR-344, over every case, layer, edge
  and tool-failure result.
- Each case SHALL be named by its two units' `RawSourceRef`s and its edge's
  two layer identities, never by a display string or a directory name.
- The make target `refinement-layering`, which `make ci` runs, SHALL run
  `cargo run --package xtask -- refinement layering
  tests/fixtures/refinement/layering/` over the real corpus, with no
  negotiation function, and SHALL fail when the gate's exit code is not 0
  (FR-344 "Invocation and exit").

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-345-AC-1 | For every combination of `R` and `C` class, the comparison returns the result the table gives; each expected result in the test is a literal, not computed by the gate's code. | Test (TC-867) |
| FR-345-AC-2 | A case whose parent compile refuses with a typed refusal and whose child compile admits is a `regression` naming both units' `RawSourceRef`s, the edge's two identities and the parent's codes; FR-344 reports verdict violation, exit 10. | Test (TC-867) |
| FR-345-AC-3 | A case whose parent class is `prohibited` and whose child admits is `not applicable`. | Test (TC-867) |
| FR-345-AC-4 | Over a layering corpus with cases on each of QSpec FR-453's edges E1 to E6, one seeded case per edge that its parent refuses and its child admits fails the gate naming that case and its edge, and every other case holds or is not applicable. | Test (TC-868) |
| FR-345-AC-5 | A case whose two units differ outside the header identity string, and a case naming state core and complete model, are each `tool failure` naming the case; verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-6 | With a test negotiation function, each of the six layers admits its witness unit and witness item; the item is requested once by backend name from each candidate, using that layer's `witness_request` kind and extent, and settles `supported` under at least one candidate. The temporal witness is a unit selecting only `quire.temporal.complete/v1` with a timed state model, a `Clock` field, a reset, a guard, a time invariant and an `invariant` clause, and its request is `operation-contract`/`bounded`. With two candidates, one supporting and one not supporting the item, the layer holds. A layer that refuses its witness, or whose item settles `supported` under no candidate, gives a `regression` naming the layer and the codes or each candidate's identity and disposition; verdict violation, exit 10. With no negotiation function, no request is made and no unrequested witness is reported `holds`: each admitted witness with a non-empty candidate set is `unresolved (unsupported)`, reason `not-negotiated`, naming the layer and candidate backends; FR-344 gives verdict unsupported, exit 21. A refused witness or empty candidate set gives a `regression` naming the layer and compile codes or empty candidate set; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-7 | On each of E1 to E6, the parent prohibits the distinguishing unit naming the parent layer and the child admits it; a parent that admits it gives a `regression` naming the edge; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-8 | An empty corpus gives one tool-failure result naming the corpus; a corpus with no `witness` entry for state graph and no `distinguishing` entry for E5 gives exactly two tool-failure results naming state graph / `witness` and E5 / `distinguishing`; an `entry.json` with an extra member `expected` gives one tool-failure result naming its path and the defect while the other entries still compile; each verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-9 | `make refinement-layering` runs the gate over `tests/fixtures/refinement/layering/` with no negotiation function; when each witness unit admits and its candidate set is non-empty, each layer is `unresolved (unsupported)` with reason `not-negotiated`, the verdict is unsupported and the target fails with FR-344 exit 21. Removing one `witness` entry from that corpus makes the target fail with tool-failure exit 30. | Test (TC-868) |
| FR-345-AC-10 | A root-package test calls `xtask::refinement::layering::compare` over `REAL_CORPUS` with the FR-278 composition, QSL's registry snapshot and a test negotiation function that settles every request `supported`, and receives verdict success with the same case, layer and edge results as the no-negotiation `make refinement-layering` run, except that all six negotiated witness results are `holds` rather than `unresolved (unsupported)`; each layer's witness item is requested once per candidate. The same call with no negotiation function makes no request and returns the report `make refinement-layering` writes, with verdict unsupported, exit 21, and no witness result `holds`. Neither call writes to stdout or stderr, and `cargo tree -p xtask -e normal,build,dev` lists no quire-contract-codegen crate. | Test (TC-868) |
| FR-345-AC-11 | Over a layering corpus that holds E6's `case`, `distinguishing` and `witness` entries (the temporal layer's witness is a timed state model with a clock field, a reset, a guard, a time invariant and one `invariant` clause), E6's both-refuse case holds, its `distinguishing` entry classifies `prohibited` on the parent side naming `quire.model.complete/v1` and `admitted` on the child side, and the temporal witness settles `supported` under a candidate that advertises `operation-contract` in `bounded` mode; a corpus without E6's `distinguishing` entry or without the temporal `witness` entry gives one tool-failure result for each missing item, naming E6 / `distinguishing` or `quire.temporal.complete/v1` / `witness`. | Test (TC-868) |
| FR-345-AC-12 | Over the real backend registry, with quire-contract-codegen's `negotiate_*` as the negotiation function, `compare` over `REAL_CORPUS` gives each of the six layers a `holds` witness result: its witness unit admits and its witness item, requested once per candidate by backend name, settles `supported` under at least one candidate. The temporal witness is the timed model of AC-6 and its request is `operation-contract`/`bounded`. A seeded unsupported witness gives a `regression` naming the layer and each candidate's identity and disposition, and a removed witness entry gives a tool-failure result. | Integration test (quire-integration IT-015 / quire-integration TC-095) |

## Dependencies

- QSpec FR-290 (capability kinds) and FR-331 (the request and
  `negotiate_*`) for the witness item; FR-280-AC-3 (no QSL dependency on
  quire-contract-codegen).
- FR-341 (compile classification), FR-344 (report, verdict and exit),
  FR-001 (source labels), FR-056 and FR-099 (package and dependency input).
- FR-110 (layer selection: a header naming any AD-003 layer resolves to it,
  and S3 admits under its admitted-form set).
- QSpec AD-003 (the profile hierarchy and its `requires` edges) and QSpec
  FR-453 (the edges, the two-unit case and the user-subset check).

## Overlap

quire-integration runs the negotiation half of the gate. It takes QSL's
`xtask` package as a dev dependency, from the same QSL source as its other
QSL dependencies, and its test calls `xtask::refinement::layering::compare`
over `REAL_CORPUS` with
the FR-278 composition, the real backend registry and quire-contract-codegen's
`negotiate_*` as the negotiation function, and fails unless every layer's
witness item settles `supported` under at least one candidate. That run is
specified by quire-integration IT-015 and quire-integration TC-095, and it is FR-345-AC-12; it
backs FR-345-AC-6 over real backends; unlike the no-negotiation make target
in FR-345-AC-9, this integration run evaluates each witness request.

## References

- ADR-017 §2 RF-3, RF-4, RF-5; §6 Q-5.
- QSpec FR-453 (STD-119, owner ruling option A: every AD-003 layer
  header-selectable, all six edges compared); QSpec FR-452, FR-290 and the
  V1-TOOL-012 re-trace (STD-117).
- Implementation ticket QSL-39; specification ticket QSL-387.
