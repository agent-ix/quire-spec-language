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
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-106: Read and admit state snapshots and invocations as the spine clause-execution input

## Description

When the run entry receives a clause selection, the admission stage SHALL
admit the selected snapshot and invocation documents into one typed
observation set for S6a, or return exactly one refusal or incomplete result
with no truth value. It reads each document by `sha256-jcs` digest and admits
it against the compiled package and the selected clause (FR-107, FR-109). Admission
settles every input defect before S6a (ADR-011 E6: bad arguments are refused
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

- An `AdmittedObservations` value: for an invariant, one current observation,
  its anchor and the self object; for a precondition or postcondition, the
  pre and post observations, the self object, the parameter values, the
  result value or none, and the admitted invocation delta. Each observation
  holds its `PopulationBinding`s and an `ObjectEnvironment` with every field
  value of every object, and keeps its document identity and digest.
- Or an `AdmissionFailure`: `Refused(RefusalRecord)` or
  `Incomplete(RefusalRecord)`, each with a catalog code, a cause and the input
  path (document identity, population, object key, field) it names.

## Document forms

Both documents are UTF-8 JSON objects with closed member sets. A document's
digest is `sha256-jcs`: SHA-256 over the RFC 8785 encoding of the parsed
document, computed by `quire-canonical` (FR-056's rule), so formatting does
not change it. Integers are decimal strings in FR-038's integer spelling (an
optional leading `-`, no `+`, no leading zero, `0` for zero and never `-0`),
because RFC 8785 reads a JSON number as an IEEE double.

Snapshot, format `quire.state.snapshot/v1`:

```json
{
  "format": "quire.state.snapshot/v1",
  "identity": {"authority": "agent-ix", "identity": "ix://example/config-version/current/healthy-parent", "revision_namespace": "example", "revision": "1"},
  "observation": "current",
  "anchor": {"kind": "handler", "name": "validate"},
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

- `observation` is `current`, `pre` or `post`. `anchor` is present exactly
  when `observation` is `current`: the named initialization or handler
  observation the snapshot was taken at (`kind` `initialization` or
  `handler`), as QSpec's state contract requires of an invariant.
- `population` and `type` are declaration identities of the selected
  package; `fields` is keyed by member name.
- A value is exactly one of `{"boolean": true|false}`,
  `{"integer": "<decimal>"}`, `{"absent": {}}`, `{"present": <value>}`,
  `{"reference": {"population": ..., "key": ...}}` and
  `{"sequence": [<value>, ...]}`. There is no set, bag or ordered-set value:
  those collections are outside QSpec's selected state extension.

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
observation: `Current { snapshot: DocumentRef, anchor, self: ObjectRef }` or
`Invocation { invocation: DocumentRef }`, where `DocumentRef` is the four
FR-001 labels and the `sha256-jcs` digest, and `anchor` is `{kind, name}`.

## Behavior

Admission runs the numbered checks in order and stops at the first failing
condition, returning its one record. Inside a check, the conditions run in the
order listed. Where a check walks the document, it walks populations in
document order, objects in document order within a population, and fields in
the object type's declared field order, and it reports the first failing
object and field.

A **required population** is the population holding `self`, and, repeatedly,
every population named by a `reference` value in any field of any object of a
required population, in the observations the clause reads.

1. Read check. For each selected document (the snapshot, or the invocation and
   then its `pre` and `post` snapshots):
   1. If no document is in its provision under the selected digest, then
      admission SHALL return `Incomplete` with `unavailable_observation`/
      `missing-required-artifact` (missing observation evidence is
      incomplete input, QSpec state contract).
   2. If the bytes exceed the document byte limit, or a value is nested
      deeper than the depth limit, then admission SHALL refuse
      `stage_limit_exceeded` with `input-bytes-exceeded` or
      `nesting-depth-exceeded`.
   3. If the document's `sha256-jcs` digest differs from the selected digest,
      then admission SHALL refuse `stale_dependency`/`byte-digest-mismatch`.
      Bytes that do not parse as JSON are digested raw, as FR-056 does, and
      so refuse here. This check comes before any member is read.
   4. If `format` is not the expected format, then admission SHALL refuse
      `unknown_wire`/`unsupported-wire`.
   5. If a required member is missing, then admission SHALL refuse
      `invalid_runtime_input`/`missing-member`, naming the first missing
      member in the member order above.
   6. If an unknown member is present, then admission SHALL refuse
      `invalid_runtime_input`/`unknown-member`, naming the first in document
      order.
   7. If a label is empty or only whitespace, then admission SHALL refuse
      `invalid_runtime_input`/`invalid-value` at that label.
   8. If the four labels differ from the selection's, then admission SHALL
      refuse `stale_dependency`/`revision-mismatch`, naming both.
2. Selection form check. If `Current` selects a precondition or postcondition,
   or `Invocation` an invariant, then admission SHALL refuse
   `wrong_snapshot`/`wrong-observation`.
3. Observation role and anchor check. If the current snapshot does not say
   `current`, or an invocation's `pre` or `post` snapshot does not say `pre`
   or `post`, then admission SHALL refuse `wrong_snapshot`/
   `wrong-observation`. If the current snapshot's `anchor` differs from the
   selection's, then admission SHALL refuse `wrong_snapshot`/`wrong-anchor`,
   naming both.
4. Model check. If a document's `model` differs from the package's model
   selection for the clause's alias, then admission SHALL refuse
   `invalid_model_binding`/`wrong-model-selection`.
5. Operation check. If an invocation's `context` or `operation` differs from
   the clause's, then admission SHALL refuse `wrong_snapshot`/
   `wrong-invocation`.
6. **Populations and values**, per object in walk order:
   1. If the population names no population of the package, or the object's
      type is not one of its member types (FR-084), then admission SHALL
      refuse `invalid_runtime_input`/`wrong-role-mapping`.
   2. If the object's type declares a set, bag or ordered-set field, then
      admission SHALL refuse `unknown_required_feature`/
      `unsupported-feature` at that field.
   3. If an object's key repeats an earlier key of its population, then
      admission SHALL refuse `invalid_runtime_input`/`conflicting-identity`
      at the later object.
   4. If a declared field is missing, then admission SHALL refuse
      `invalid_runtime_input`/`missing-member`; if an undeclared field is
      present, `invalid_runtime_input`/`unknown-member`.
   5. For each field in declared order: if the value's kind does not fit the
      declared type, then admission SHALL refuse `invalid_runtime_input`/
      `wrong-value-kind`; if an integer is not in FR-038's spelling, or lies
      outside its `Int[lower, upper]` (such as `"1001"` for
      `versionNumber`), then admission SHALL refuse `invalid_runtime_input`/
      `invalid-value`.
7. Completeness check. If a required population is not marked `complete`, then
   admission SHALL return `Incomplete` with `incomplete_population`/
   `incomplete-scope`, naming the first such population in walk order.
   Admission SHALL skip the dangling check over an incomplete population.
8. Closure check. If a `reference` value names a key absent from its complete
   population, then admission SHALL refuse `dangling_reference`/
   `absent-target-in-complete-population`, naming the first such reference
   in walk order and its population.
9. Self check. If the selected self object is absent from the current
   snapshot, from the pre snapshot, or (for a postcondition) from the post
   snapshot, then admission SHALL refuse `invalid_runtime_input`/
   `wrong-role-mapping`.
10. Parameters and result check. If an invocation's `parameters` lack a declared
    parameter or hold an undeclared one, then admission SHALL refuse
    `invalid_runtime_input`/`missing-member` or `unknown-member`; if a value
    does not fit its declared type, `wrong-value-kind` or `invalid-value`, by
    check 6's value rule. If the result member is `null` for an
    operation that declares a result, or a value for one that declares none,
    then admission SHALL refuse `invalid_runtime_input`/`missing-member` or
    `unknown-member`.
11. Frame and delta check. For an invocation, admission SHALL run the frame
    check (`admit_invocation` with the operation's `OperationEffect`,
    FR-103), which SHALL compare every declared field of every surviving object, scalar and
    reference alike, and SHALL report the first violation in this order:
    1. an object of the pre snapshot absent from the post snapshot that the
       frame's `deletes` does not grant, in pre document order:
       `frame_violation`/`unauthorized-change`;
    2. an object of the post snapshot absent from the pre snapshot that
       `creates` does not grant, in post document order:
       `frame_violation`/`unauthorized-change`;
    3. a changed field of a surviving object outside `modifies`, objects in
       pre document order and fields in declared order:
       `frame_violation`/`unauthorized-change`;
    4. a repeated identity in `created` or `deleted`, an identity in both,
       then a declared `created` or `deleted` list that differs from the
       computed one: `population_delta_mismatch`/`delta-disagreement`.

- Admission SHALL read no path, environment variable, clock or search
  location. The same package, provisions, selection and limits SHALL give
  the same result.
- The document reader is a string edge (ADR-012 §9): `format`,
  `observation`, `anchor.kind`, member names and value tags are converted
  once, at read, to closed enums or typed identities.
- Every code and cause above is in QSpec `native-diagnostics.md` revision
  `1-draft.7`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-106-AC-1 | The healthy-parent current snapshot above, selected with anchor `handler validate` and `self` = `child` for `ParentOrder`, admits: one observation with population `config_history` complete, objects `root` and `child` with `versionNumber` 1 and 2, `child.parent` present and naming `root`, and the snapshot's identity and digest retained. Re-serializing the document with other whitespace and member order gives the same digest and the same admitted value. | Test (TC-464) |
| FR-106-AC-2 | The changed-version invocation (pre: `child` at 2; post: `child` at 3; result `true`; no parameters, created or deleted) selected for `VersionUnchanged` admits, with the pre and post observations distinct and `result` true. | Test (TC-464) |
| FR-106-AC-3 | Each condition of checks 1 to 6, 9 and 10 has one case that fails only it and returns exactly its code and cause, as TC-465 lists: among them an absent document, a 1 MiB + 1 byte document, a value nested 65 deep, bytes edited under their original digest, `format` `native-state-input/1` (`unknown_wire`/`unsupported-wire`), a missing `populations`, an extra member, a blank `authority`, another `revision`, `Current` selecting `VersionUnchanged`, a `pre` document selected as current, anchor `handler other`, another model digest, operation `other`, an unknown population, a duplicate `root` key, a set-typed field, `versionNumber` `{"boolean": true}`, `"01"`, `"-1"` and `"1001"`, `self` = `ghost`, and a `result` of `null` for `attemptUpdate`. | Test (TC-465) |
| FR-106-AC-4 | The incomplete-population snapshot (`complete: false`, `child.parent` naming `missing`) gives `Incomplete` with `incomplete_population`/`incomplete-scope` and no dangling refusal; the same snapshot with `complete: true` gives `Refused` with `dangling_reference`, naming `missing` and `config_history`. A second, incomplete population that no reference value names does not make a healthy case incomplete. | Test (TC-465) |
| FR-106-AC-5 | The forbidden-parent-change invocation (post sets `child.parent` absent) refuses `frame_violation`/`unauthorized-change` naming `child` and `parent`. A package whose `attemptUpdate` frame modifies only `parent`, with an invocation that changes only `versionNumber`, refuses `frame_violation`/`unauthorized-change` naming `versionNumber` (a scalar field). An invocation declaring `created: [child]` refuses `population_delta_mismatch`/`delta-disagreement`. | Test (TC-465) |
| FR-106-AC-6 | Running admission twice over the same inputs gives equal results, and admission builds its result without reading the filesystem (the provisions are in-memory maps; a test with no files on disk passes). | Test (TC-464) |
| FR-106-AC-7 | Multi-defect documents report the first defect by the order above: bytes edited under their original digest that also add an unknown member refuse `byte-digest-mismatch`; a snapshot with `root.versionNumber` `"-1"` and `child.versionNumber` `"1001"` refuses `invalid-value` at `root`; an invocation whose post both deletes `root` and changes `child.parent` refuses the deletion. | Test (TC-465) |

## Dependencies

- FR-084 and FR-089 (population binding and identity), FR-103 (operation
  effect), FR-001 (the four labels), FR-056 (the `sha256-jcs` rule and its
  digest-first order), FR-038 (integer spelling).
- QSpec FR-153 and `state-contract.md` (anchors, completeness and closure),
  `native-diagnostics.md` revision `1-draft.7` (every code and cause above).
