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
layer selection, the mapping of FR-341's classes onto FR-453's results, the
candidate set of each layer's witness item, the request of that item
through a negotiation function the caller supplies, and the report through
FR-344.

Like FR-340's gate, it is a behavioural comparison. It records no
requirement record and settles no claim; the only backend request it makes
is the negotiation of each layer's witness item, and only when its caller
supplies a negotiation function. Negotiation is quire-contract-codegen's
`negotiate_*` (QSpec FR-331-AC-4), on which QSL takes no dependency
(FR-280-AC-3), so QSL's own run of the gate supplies none and quire-integration
runs the negotiation half (Overlap).

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
- If the corpus holds at least one entry, and any of the five layers has no
  `witness` entry, or any of the five edges has no `case` entry or no
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
  read each disposition from the negotiation function's result.
- With a negotiation function, the gate SHALL give each layer FR-453's
  witness result: `holds` when every `witness` entry for the layer
  classifies `admitted` and its item settles `supported` under at least one
  candidate, and `regression` otherwise, naming the layer and either the
  compile codes or every candidate's backend identity and disposition (or
  the empty candidate set).
- Without a negotiation function, the gate SHALL submit no request and give
  each layer the witness result `holds` when every `witness` entry for the
  layer classifies `admitted` and its candidate set is non-empty, and
  `regression` otherwise, naming the layer and either the compile codes or
  the empty candidate set.
- The gate SHALL give each edge FR-453's proper-subset result: `holds` when
  every `distinguishing` entry for the edge classifies `prohibited` on the
  parent side, naming the parent layer, and `admitted` on the child side,
  and `regression` naming the edge otherwise.

### Comparison core and compile seam

- The gate's comparison core SHALL be the public function
  `xtask::refinement::layering::compare` of the `xtask` package's library
  target. It takes the corpus directory, the compile function (a unit's
  source, packages, dependency input and limits to its FR-278 result), the
  backend registry snapshot (`qsl_route::Registry`) and an optional
  negotiation function, and returns FR-344's report value: every case,
  layer, edge and tool-failure result and the verdict. It SHALL write
  nothing to the process streams.
- The negotiation function SHALL take one `WitnessRequest`, a public type of
  the same module holding the candidate (`qsl_route::Candidate`) whose
  backend it names, the layer's `witness_request` kind and extent and the
  witness entry's compiled clause, and return that request's
  `qsl_route::Disposition`.
- The module SHALL expose `REAL_CORPUS`, the path of
  `tests/fixtures/refinement/layering/` in the source tree the `xtask`
  package is built from, so a dependent built from a QSL revision reads that
  revision's corpus.
- The `xtask` package SHALL take no dependency, in any table, on a
  quire-contract-codegen crate (FR-280-AC-3).
- `xtask refinement layering` SHALL pass `compare` the FR-278 composition
  and QSL's registry snapshot, and no negotiation function, then render the
  report and exit by FR-344.
- A test substitutes a wrapper that returns a chosen result for a named unit,
a test registry and a test negotiation function that settles each request
by a chosen disposition per backend; the wrapper and the test negotiation
function live only in the test module of the gate's crate
(`#[cfg(test)]`), so no build of `xtask` or of the compiler carries a fault
hook.

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
| FR-345-AC-4 | Over a layering corpus with cases on each of QSpec FR-453's edges E1 to E5, one seeded case per edge that its parent refuses and its child admits fails the gate naming that case and its edge, and every other case holds or is not applicable. | Test (TC-868) |
| FR-345-AC-5 | A case whose two units differ outside the header identity string, and a case naming state core and complete model, are each `tool failure` naming the case; verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-6 | With a test negotiation function, each of the five layers admits its witness unit and the witness item, requested with the layer's `witness_request` kind and extent once from each candidate backend by name, settles `supported` under at least one candidate; with two candidates of which one supports and one does not, the layer holds. A layer that refuses its witness, or whose item settles `supported` under no candidate, gives a `regression` naming the layer and the codes or each candidate's identity and disposition; verdict violation, exit 10. With no negotiation function, the negotiation function is never called, every layer whose witness unit admits and whose candidate set is non-empty holds, and a layer whose candidate set is empty gives a `regression` naming the layer and the empty candidate set; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-7 | On each of E1 to E5, the parent prohibits the distinguishing unit naming the parent layer and the child admits it; a parent that admits it gives a `regression` naming the edge; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-8 | An empty corpus gives one tool-failure result naming the corpus; a corpus with no `witness` entry for state graph and no `distinguishing` entry for E5 gives exactly two tool-failure results naming state graph / `witness` and E5 / `distinguishing`; an `entry.json` with an extra member `expected` gives one tool-failure result naming its path and the defect while the other entries still compile; each verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-9 | `make refinement-layering` runs the gate over `tests/fixtures/refinement/layering/` with no negotiation function and passes, every layer's witness unit admitting with a non-empty candidate set in QSL's registry snapshot; removing one `witness` entry from that corpus makes the target fail. | Test (TC-868) |
| FR-345-AC-10 | A test of the root package, which reaches `xtask` only through its dev dependency, calls `xtask::refinement::layering::compare` over `REAL_CORPUS` with the FR-278 composition, QSL's registry snapshot and a negotiation function defined in that test that settles every request `supported`, and gets verdict success with every layer, case and edge result as `make refinement-layering` reports them and every layer's witness item requested once per candidate; the same call with no negotiation function makes no call to a negotiation function and returns the report `make refinement-layering` writes; no byte reaches stdout or stderr from either call; and `cargo tree -p xtask -e normal,build,dev` lists no quire-contract-codegen crate. | Test (TC-868) |

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
witness item settles `supported` under at least one candidate. That run
backs FR-345-AC-6 over real backends and the witness half of FR-345-AC-9.

## References

- ADR-017 §2 RF-3, RF-4, RF-5; §6 Q-5.
- QSpec FR-453 (STD-119, owner ruling option A: every AD-003 layer
  header-selectable, all five edges compared); QSpec FR-452, FR-290 and the
  V1-TOOL-012 re-trace (STD-117).
- Implementation ticket QSL-39; specification ticket QSL-387.
