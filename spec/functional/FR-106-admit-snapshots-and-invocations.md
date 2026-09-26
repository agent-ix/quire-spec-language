---
id: FR-106
title: "Read and admit state snapshots and invocations as the spine clause-execution input"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-106: Read and admit state snapshots and invocations as the spine clause-execution input

## Description

QSL SHALL define the spine clause-execution input: a snapshot document, an
invocation document and a clause selection. It SHALL read the documents by
`sha256-jcs` digest and admit them, against a compiled package and the
selected clause, into one typed observation set that S6a evaluates (FR-107).
Admission SHALL settle every input defect before S6a, as one typed refusal or
incomplete result with no truth value (ADR-011 E6: bad arguments are refused
at admission).

This is the E6 input ADR-011 §5 says native-run/1 has no spine equivalent for.
Native `run` reads `native-state-input/1` artifacts typed on IR and native
model types (`src/runtime/input.rs`), which retire with M-6c (ADR-012 §15.8).
The spine has the model-layer admission already (`model::population::
admit_binding`, `admit_invocation` with its frame check, `ObjectEnvironment`),
but no document form, no per-observation field values, and a frame check
that compares only reference-valued fields (`PopulationMember::field_values`,
`qsl-semantics/src/model/population.rs:466-475`).

## Inputs

- The compiled `CheckedPackage` and the selected checked state clause.
- A snapshot provision and an invocation provision: maps from `sha256-jcs`
  digest to document bytes.
- A `ClauseSelection`.
- `ObservationLimits`: document bytes (default 1 MiB), JSON nesting depth
  (default 64), objects per document (default 10,000) and values per
  document (default 100,000).

## Outputs

- An `AdmittedObservations` value: for an invariant, one current observation
  and the self object; for a precondition or postcondition, the pre and post
  observations, the self object, the parameter values, the result value or
  none, and the admitted invocation delta. Each observation holds its
  `PopulationBinding`s and an `ObjectEnvironment` with every field value of
  every object, and keeps its document identity and digest.
- Or an `AdmissionFailure`: `Refused(RefusalRecord)` or
  `Incomplete(RefusalRecord)`, each with a catalog code, a cause and the input
  path (document identity, population, object key, field) it names.

## Document forms

Both documents are UTF-8 JSON objects with closed member sets. The document's
digest is `sha256-jcs`: SHA-256 over the RFC 8785 encoding of the parsed
document (FR-056's rule), so formatting does not change it. Integers are
decimal strings, because RFC 8785 reads a JSON number as an IEEE double.

Snapshot, format `quire.state.snapshot/v1`:

```json
{
  "format": "quire.state.snapshot/v1",
  "identity": {"authority": "agent-ix", "identity": "ix://example/config-version/current/healthy-parent", "revision_namespace": "example", "revision": "1"},
  "observation": "current",
  "model": {"identity": "example/config-version", "version": "1", "digest": "sha256-jcs:<hex>"},
  "populations": [{
    "population": "ix://example/config-version/config_history",
    "complete": true,
    "objects": [
      {"key": "root", "type": "ix://example/config-version/ConfigVersion",
       "fields": {"versionNumber": {"integer": "1"}, "parent": {"absent": {}}}},
      {"key": "child", "type": "ix://example/config-version/ConfigVersion",
       "fields": {"versionNumber": {"integer": "2"},
                  "parent": {"present": {"reference": {"population": "ix://example/config-version/config_history", "key": "root"}}}}}
    ]
  }]
}
```

- `observation` is `current`, `pre` or `post`.
- `population` and `type` are declaration identities of the selected
  package; `fields` is keyed by member name.
- A value is exactly one of `{"boolean": true|false}`,
  `{"integer": "<decimal>"}`, `{"absent": {}}`, `{"present": <value>}`,
  `{"reference": {"population": ..., "key": ...}}` and
  `{"collection": [<value>, ...]}`. The field's declared type decides what a
  value means; a collection takes its kind (sequence, set, bag, ordered set)
  from the declaration.

Invocation, format `quire.state.invocation/v1`:

```json
{
  "format": "quire.state.invocation/v1",
  "identity": {"authority": "agent-ix", "identity": "ix://example/config-version/invocation/changed-version", "revision_namespace": "example", "revision": "1"},
  "model": {"identity": "example/config-version", "version": "1", "digest": "sha256-jcs:<hex>"},
  "context": "ix://example/config-version/ConfigVersion",
  "operation": "attemptUpdate",
  "self": {"population": "ix://example/config-version/config_history", "key": "child"},
  "pre": {"identity": {"authority": "agent-ix", "identity": "...", "revision_namespace": "example", "revision": "1"}, "digest": "sha256-jcs:<hex>"},
  "post": {"identity": {"...": "..."}, "digest": "sha256-jcs:<hex>"},
  "parameters": {},
  "result": {"boolean": true},
  "created": [],
  "deleted": []
}
```

- `result` is a value or JSON `null`; `created` and `deleted` list
  `{population, key}` object references.

`ClauseSelection` names the clause by its `QualifiedName` and one
observation: `Current { snapshot: DocumentRef, self: ObjectRef }` or
`Invocation { invocation: DocumentRef }`, where `DocumentRef` is the four
FR-001 labels and the `sha256-jcs` digest.

## Behavior

Admission SHALL apply these checks in this order and SHALL stop at the first
that fails, returning its one record.

1. **Read.** Each selected document SHALL be present in its provision under
   its digest, SHALL decode under `ObservationLimits`, SHALL have the expected
   `format`, only known members and four non-blank labels equal to the
   selection's, and SHALL digest to the selected digest. Failures:
   `Incomplete` with `unavailable_observation`/`missing-required-artifact`
   (absent: missing observation evidence is incomplete input, QSpec state
   contract), and refusals `stage_limit_exceeded` with
   `input-bytes-exceeded` or `nesting-depth-exceeded` (limits),
   `unknown_wire` (format), `invalid_runtime_input`/`malformed-json`,
   `unknown-member` or `missing-member` (shape),
   `invalid_source_identity` (a blank label), `stale_dependency`/
   `byte-digest-mismatch` (digest or labels). An invocation's `pre` and
   `post` references are read by the same rule from the snapshot provision.
2. **Selection form.** `Current` SHALL select an invariant and `Invocation` a
   precondition or postcondition. Otherwise `wrong_snapshot`/
   `wrong-observation`.
3. **Observation role.** The current snapshot SHALL say `current`, and an
   invocation's `pre` and `post` snapshots `pre` and `post`. Otherwise
   `wrong_snapshot`/`wrong-observation`.
4. **Model.** Each document's `model` SHALL equal the package's model
   selection for the clause's alias. Otherwise `invalid_model_binding`/
   `wrong-model-selection`.
5. **Operation.** An invocation's `context` and `operation` SHALL be the
   clause's. Otherwise `wrong_snapshot`/`wrong-invocation`.
6. **Populations and values.** Each population SHALL name a population of the
   package, each object type SHALL be one of its member types (FR-084), keys
   SHALL be unique within a population, and each object's fields SHALL be
   exactly its type's declared fields, each value of its declared type and
   within its declared bounds. Failures: `invalid_runtime_input` with
   `wrong-role-mapping` (population or type), `conflicting-identity`
   (duplicate key), `missing-member` or `unknown-member` (fields),
   `wrong-value-kind` (value kind) and `invalid-value` (an integer outside its
   `Int[lower, upper]`, such as `1001` for `versionNumber`).
7. **Completeness.** Every population the clause requires, that is the
   population holding `self` and each population a reference-typed field of
   a required population's type targets, SHALL be marked `complete`.
   Otherwise the result is `Incomplete` with `incomplete_population`/
   `incomplete-scope`, and no dangling check runs over that population.
8. **Closure.** Every reference into a complete population SHALL name one of
   its keys. Otherwise `dangling_reference`/
   `absent-target-in-complete-population`, naming the reference and the
   population.
9. **Self.** `self` SHALL name an object of the current snapshot, or of the
   pre snapshot and, for a postcondition, of the post snapshot. Otherwise
   `invalid_runtime_input`/`wrong-role-mapping`.
10. **Parameters and result.** An invocation's `parameters` SHALL be exactly
    the operation's declared parameters, each of its declared type; `result`
    SHALL be a value of the declared result type when the operation declares
    one and `null` otherwise. Failures: `invalid_runtime_input` with
    `missing-member`, `unknown-member`, `wrong-value-kind` or
    `invalid-value`.
11. **Frame and delta.** For an invocation, `admit_invocation` SHALL run over
    the two observations with the operation's `OperationEffect` (FR-103). The
    frame check SHALL compare every declared field of every surviving object,
    scalar and reference alike, and SHALL refuse a change to a field outside
    `modifies` with `frame_violation`/`unauthorized-change`. A declared
    `created`/`deleted` list that disagrees with the computed delta SHALL
    refuse `population_delta_mismatch`/`delta-disagreement`.

- Admission SHALL read no path, environment variable, clock or search
  location. The same package, provisions, selection and limits SHALL give
  the same result.
- The document reader is a string edge (ADR-012 §9): `format`,
  `observation`, member names and value tags are converted once, at read, to
  closed enums or typed identities.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-106-AC-1 | The healthy-parent current snapshot above, selected with `self` = `child` for `ParentOrder`, admits: one observation with population `config_history` complete, objects `root` and `child` with `versionNumber` 1 and 2, `child.parent` present and naming `root`, and the snapshot's identity and digest retained. Re-serializing the document with other whitespace and member order gives the same digest and the same admitted value. | Test (TC-464) |
| FR-106-AC-2 | The changed-version invocation (pre: `child` at 2; post: `child` at 3; result `true`; no parameters, created or deleted) selected for `VersionUnchanged` admits, with the pre and post observations distinct and `result` true. | Test (TC-464) |
| FR-106-AC-3 | Each check 1 to 10 has one case that fails only it and returns exactly its code and cause: an absent document, a 1 MiB + 1 byte document, `format` `native-state-input/1`, an extra member, a blank `authority`, a document edited but kept under its original selected digest, `Current` selecting `VersionUnchanged`, a `pre` document selected as current, another model digest, operation `other`, a duplicate `root` key, `versionNumber` `{"boolean": true}`, `versionNumber` `"1001"`, `versionNumber` `"-1"`, `self` = `ghost`, and a `result` of `null` for `attemptUpdate`. | Test (TC-465) |
| FR-106-AC-4 | The incomplete-population snapshot (`complete: false`, `child.parent` naming `missing`) gives `Incomplete` with `incomplete_population`/`incomplete-scope` and no dangling refusal; the same snapshot with `complete: true` gives `Refused` with `dangling_reference`, naming `missing` and `config_history`. | Test (TC-465) |
| FR-106-AC-5 | The forbidden-parent-change invocation (post sets `child.parent` absent) refuses `frame_violation`/`unauthorized-change` naming `child` and `parent`. A package whose `attemptUpdate` frame modifies only `parent`, with an invocation that changes only `versionNumber`, refuses `frame_violation`/`unauthorized-change` naming `versionNumber` (a scalar field). An invocation declaring `created: [child]` refuses `population_delta_mismatch`/`delta-disagreement`. | Test (TC-465) |
| FR-106-AC-6 | Running admission twice over the same inputs gives equal results, and admission builds its result without reading the filesystem (the provisions are in-memory maps; a test with no files on disk passes). | Test (TC-464) |

## Dependencies

- FR-084 and FR-089 (population binding and identity), FR-103 (operation
  effect), FR-001 (the four labels), FR-056 (the `sha256-jcs` rule).
- QSpec FR-153 and `state-contract.md` (anchors, completeness and closure),
  `native-diagnostics.md` revision `1-draft.7` (every code and cause above).
