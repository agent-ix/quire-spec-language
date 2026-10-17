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
  - target: ix://agent-ix/quire-semantic-value/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-364
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-207
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-208
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
- `ObservationLimits`: document bytes (default 1 MiB), objects per
  document (default 10,000) and values per document (default 100,000),
  each named by its setting
  ([FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md)).
  Documents are read through `quire-canonical`'s shared reader at any
  nesting depth
  ([FR-261](FR-261-read-other-untrusted-json-at-any-depth.md)); the byte,
  object and value limits bound the read. Each relationship instance counts
  as one value against the values-per-document limit.

## Outputs

- An `AdmittedObservations` value: for an invariant, one current observation,
  its anchor and the self object; for a precondition selected by `PreCall`,
  the pre observation, the self object and the parameter values; for a
  precondition selected by `Invocation`, the same pre-call observation from
  the invocation's pre snapshot, `self` and parameters; for a postcondition
  selected by `Invocation`, the pre and post observations, the self object, the parameter values, the result value or
  none, and the admitted invocation delta. The value also names the selected
  invocation by its four identity labels, whatever kind `self` is, for a
  postcondition and for a precondition selected by `Invocation`, and none for
  an invariant over a current snapshot and for a precondition selected by
`PreCall`. FR-107 roots
  value paths at it. Each observation
  holds its `PopulationBinding`s and an `ObjectEnvironment` with every field
  value of every object and the relationship extents of the snapshot
  (FR-364), and keeps its document identity and digest. Admission also
  issues each observation's population manifest and bundle (FR-364): one row
  per population declaration of the package, `complete` exactly when the
  snapshot lists the population `"complete": true`, so a declared population
  the snapshot does not list is an incomplete row; the manifest is closed by
  exhausting that traversal of the package's declarations and by nothing the
  document says.
- For an invariant of a state machine, selected by `Context` (below): the
  machine's one context in the selected side of the invocation (its `pre` or
  its `post` snapshot), the invocation's identity labels, and no self object,
  parameter values or result. A state machine's context has no object identity
  (QSpec FR-208, State-machine context anchors), so an observation holds it as
  a value of the machine's effective fields next to its objects and
  relationships; for a precondition or postcondition of a state machine
  operation the same context stands where the self object stands for an object
  operation, and the self object is none. The context is not an object of the
  `ObjectEnvironment`, has no `PopulationBinding` and is never the target of a
  `reference`.
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
  "identity": {"authority": "agent-ix", "identity": "ix://example/config-version/current/healthy-parent"},
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
- `invocation` is optional on a `pre` or `post` snapshot: the four identity
  labels (`authority`, `identity`, `revision_namespace`, `revision`) of the
  invocation document the snapshot observes. It carries no digest, because
  the invocation's digest covers the snapshot's own. A snapshot without it
  is unbound to any invocation.
- `population` and `type` are declaration identities of the selected
  package; `fields` is keyed by member name.
- `relationships` is optional on a snapshot. Each entry has exactly `relationship`, `complete` and
  `instances`. `relationship` is the declaration identity of a navigation
  relationship of the selected package
  (`ix://<package>/relationship/<Name>-<verb>-<TargetName>`), `complete` is a
  boolean, and `instances` is an array of objects with exactly `source` and
  `target`, each an object reference `{"population": ..., "key": ...}` as in
  `self`. An instance links a source object to a target object through the
  relationship; the entry's order and its instances' order carry no meaning,
  and a link between the same two objects listed twice is two instances. An
  entry with `"complete": true` and `"instances": []` states that the
  relationship has no link in this state. A relationship with no entry is
  unlisted: the snapshot says nothing about it, which is not the same as an
  empty entry (FR-364). A snapshot without `relationships` lists none.

  A snapshot of TC-1035's `orders` model lists two links of
  a `Sys` object `s1` to its `Pump` objects as:

  ```json
  "relationships": [{
    "relationship": "ix://test/orders/relationship/Sys-composition-Pump",
    "complete": true,
    "instances": [
      {"source": {"population": "ix://test/orders/Q", "key": "s1"},
       "target": {"population": "ix://test/orders/Q", "key": "p2"}},
      {"source": {"population": "ix://test/orders/Q", "key": "s1"},
       "target": {"population": "ix://test/orders/Q", "key": "p1"}}
    ]
  }]
  ```
- `contexts` is optional on a `pre` or `post` snapshot and is not a member of
  a `current` snapshot. A **context** is the state of one instance of a state
  machine in this observation: the values of the machine's effective fields,
  which is all QSpec binds (FR-208: no member binds the machine's current
  state, so the document carries no `state`, `states` or `transitions`
  member; a machine's declared states, transitions and guards are model
  declarations and a snapshot restates none of them). Each entry has exactly
  `machine` and `fields`. `machine` is the declaration identity of the
  most-specific state machine of the package the instance is of, and
  `fields` has exactly one entry for every effective field of that machine,
  own and inherited, in the exposed effective field order, keyed by the exact
  field name, each a value of the closed grammar below (an optional field is
  `{"absent": {}}` or `{"present": <value>}`, never omitted). The entries'
  order and the members' order carry no meaning. A context carries no key and
  no population, and its presence in a snapshot says nothing about any
  population. A snapshot without `contexts` holds no context of any machine,
  which is not an empty or default context.

  A `pre` snapshot of TC-1045's `OrderLifecycle` machine holds its context as:

  ```json
  "contexts": [{
    "machine": "ix://test/orders/OrderLifecycle",
    "fields": {"placed": {"boolean": false}, "cancelled": {"boolean": false}}
  }]
  ```
- A value is exactly one of `{"boolean": true|false}`,
  `{"integer": "<decimal>"}`, `{"absent": {}}`, `{"present": <value>}`,
  `{"reference": {"population": ..., "key": ...}}`,
  `{"sequence": [<value>, ...]}`, the record form below, the variant form of
  FR-360 and the four payload object forms below, `{"kind": "string",
  "text": "<any Unicode scalar sequence>"}`, `{"kind": "uuid", "text":
  "<canonical UUID>"}`, `{"kind": "timestamp", "nanoseconds":
  "<canonical decimal>"}` and `{"kind": "text", "text": "<any Unicode scalar
  sequence>"}`. There is no set, bag or ordered-set value:
  those collections are outside QSpec's selected state extension.

A record value is `{"record": {"type": "<declaration identity>",
"fields": {"<field name>": <value>, ...}}}`. The outer object has exactly
the `record` tag; its payload has exactly `type` and `fields`. `type` is the
IR node identity of the value's most-specific admitted record value type,
not a display name or an object reference. `fields` has exactly one entry
for every exposed effective field of that type, own and inherited, keyed by
the exact field name; an optional field is present with `{"absent": {}}`
or `{"present": <value>}`. Its field values use this same closed value
grammar, so nested records, optionals and sequence elements may contain
records. A record's actual type SHALL conform to the field, parameter or
result's declared record type by QSpec FR-208's record specialization rule.
The record carries no object key, population or identity, and an object
`reference` tag cannot stand in for it. Two admitted records compare under
FR-208 by most-specific type and every effective field; JSON member order
does not affect the typed value or the `sha256-jcs` digest.

A **native payload value** is a String, a UUID or a Timestamp, the closed wire
objects of FR-056's K2 canonical native payloads. They take that requirement's
spelling unchanged: the String object is `{"kind": "string", "text":
"ready"}`, whose `text` is any Unicode scalar sequence, the empty one
included; the UUID object is `{"kind": "uuid", "text":
"00112233-4455-6677-8899-aabbccddeeff"}`, whose `text` is 32 lowercase ASCII
hexadecimal digits in groups `8-4-4-4-12` with four ASCII hyphens, and the
Timestamp object is `{"kind": "timestamp", "nanoseconds": "0"}`, whose
`nanoseconds` is a signed count on FR-056's POSIX-epoch scale in the
signed-128 domain, spelled `0`, or an optional `-` then a nonzero digit and
more digits. A value object with a `kind` member is a payload object (the
three native payload objects above and the Text object below); every other
value is a single-tag object. A UUID object has exactly
the members `kind` and `text`, a String object exactly `kind` and `text`, a
Text object exactly `kind` and `text`, and a Timestamp object exactly `kind`
and `nanoseconds`. The payload is a JSON
string, never a JSON number, and the canonical text is the only accepted
spelling of a UUID or a Timestamp: admission never case-folds, trims, inserts
or removes a hyphen, strips a `urn:uuid:` prefix or braces, reads a calendar
or offset text, or accepts an `Integer` for a Timestamp or a `String` for a
UUID. A String payload is the decoded Unicode scalar sequence and has no
narrower grammar: admission retains it exactly and never normalizes, case-folds,
trims, truncates, escapes or repairs it, so the empty String, leading and
trailing whitespace, NUL and other control characters, noncharacters and
text that looks like a UUID all admit unchanged, and a composed and a
decomposed spelling of one character (U+00E9 and U+0065 U+0301) are two
different Strings. A String has no length limit of its own: the document byte
limit and the values-per-document setting are its only bounds. JSON escape
spelling (`\u00e9` or the literal character, a surrogate pair or the literal
character) belongs to the document reader and does not change the payload or
the digest. The one text a document cannot carry is a lone UTF-16 surrogate
escape, and a document with one, or with bytes that are not UTF-8, is a
document the shared reader refuses and check 1.3 settles, before any value is
read. Only a String slot reads a String object and only a Text slot a Text
object; the object `key` of an object, a record's `type`, a variant `case`
and every member name are identities, not String or Text values. UUID admission accepts every 128-bit payload, Nil and Max
included, and retains the version and variant bits. A String, UUID, Timestamp or
Text object counts as one value against the values-per-document setting.
Two admitted values of one native kind are equal exactly when their canonical
payloads are equal (QSpec FR-149; FR-056 for String, whose payloads are equal
when the scalar sequences are identical), so the frame check (check 11)
compares a String, UUID or Timestamp field by payload, as it does a scalar. Payload
objects use the same closed value grammar anywhere a value may stand: a
field, a parameter, a result, a record field, a present option and a
sequence element.

A **Text value** is the object `{"kind": "text", "text": "<any Unicode scalar
sequence>"}`, the value of a field, parameter or result of declared type
`Text[min, max; profile]` (QSpec FR-141; FR-056 admits that type from a scalar
type bound to `Text`). FR-056 and QSpec define no wire object for Text: the
K2 payloads are String, UUID and Timestamp. The Text object is this
grammar's own, spelled with the String object's members, so one envelope rule
reads all four payload objects. `Text` is a distinct type from `String`. Its
`text` is the decoded scalar sequence of the runtime text input, with the
String object's rules for JSON escape spelling, lone surrogate escapes and
bytes that are not UTF-8, which the document reader and check 1.3 settle
before any value is read. The value retains the sequence under the
declared profile, as QSpec FR-141 states: the scalar sequence itself under
`unicode-scalars`, its normalization under `nfc`, `nfd`, `nfkc` and `nfkd`,
and its UTF-8 bytes under `binary-utf8`. Its length is the profile length of
the retained sequence: the scalar count, or the byte count under
`binary-utf8`. Admission normalizes a supplied text under a normalizing
profile and never refuses it for being unnormalized; it never case-folds,
trims, truncates, escapes or repairs a text. The empty text is a Text value
exactly when the declared minimum is `0`. Two Text values of one declared
type are equal exactly when their retained sequences are equal (QSpec FR-141
and FR-149), so the frame check compares a Text field by its retained
sequence: under `nfc`, U+00E9 and U+0065 U+0301 are one value, and under
`unicode-scalars` they are two. The document's digest covers the supplied
text, so two documents holding those spellings have different digests and one
typed value.

A field, parameter or result of declared type `String` admits only a String
object, one of declared type `UUID` only a UUID object, one of declared
type `Timestamp` only a Timestamp object, and one of declared type `Text[min,
max; profile]` only a Text object. A String, UUID, Timestamp or Text object
fits no other declared type, and no other form fits those four. A Text
object does not fit a `String` slot and a String object does not fit a Text
slot. Admission checks a payload object in this order and
refuses the first defect found, at the nested value path and naming the
member where one is listed, with `invalid_runtime_input` (the closed set of
FR-056's runtime-input adapter; the kernel's construction classes are not
catalog codes):

1. A `kind` member that is not a JSON string (a number, `null`, an object)
   refuses `wrong-value-kind`, naming `kind`.
2. A `kind` string other than `string`, `uuid`, `timestamp` and `text`
   (including a letter-case variant such as `String`, `UUID` or `Text`)
   refuses `invalid-value`,
   naming `kind`. The members of such an object are not examined.
3. A missing payload member of the object's kind refuses `missing-member`,
   naming `text` (String, UUID, Text) or `nanoseconds` (Timestamp). So
   `{"kind":"uuid","nanoseconds":"0"}`,
   `{"kind":"string","nanoseconds":"0"}` and
   `{"kind":"text","nanoseconds":"0"}` refuse `missing-member` naming
   `text`.
4. An unknown member of the object's kind refuses `unknown-member`, naming the
   first in document order. Here `nanoseconds` on a String, UUID or Text
   object is unknown.
5. A payload member that is not a JSON string, including a bare JSON number
   (even `0`) or `null`, refuses `wrong-value-kind`, naming the member.
6. A payload kind that differs from the slot's declared type, a payload
   object in a slot of any other declared type, and a single-tag object
   (such as `{"integer": "5"}`) in a `String`, `UUID`, `Timestamp` or `Text`
   slot each refuse `wrong-value-kind`. A String object whose text spells a
   UUID or a Timestamp is still a String object and fits only a `String`
   slot, and a Text object holding the same scalars as a String fits only a
   `Text` slot; no payload is read as another kind.
7. A UUID or Timestamp payload that is not canonical refuses `invalid-value`,
   naming the payload member, with the kernel class `noncanonical-payload`. A
   canonical Timestamp text outside the signed-128 domain refuses
   `invalid-value` with the kernel class `payload-out-of-domain`.
   Noncanonical text takes precedence over the range check. A Text payload
   whose profile length, measured on the retained sequence, lies outside the
   declared `[min, max]` refuses `invalid-value`, naming `text`, the declared
   bounds, the profile and the measured length; QSpec FR-141's text-length
   refusal maps to this cause as the kernel's payload refusals do (FR-056),
   and no value is produced. A String payload has no defect to refuse here,
   because every Unicode scalar sequence is a String.

A repeated member name in a document, a lone surrogate escape in a `text` and
bytes that are not UTF-8 are settled earlier, by check 1.3's shared-reader
rule, and never reach this order.

Invocation, format `quire.state.invocation/v1`:

```json
{
  "format": "quire.state.invocation/v1",
  "identity": {"authority": "agent-ix", "identity": "ix://example/config-version/invocation/changed-version"},
  "model": {"identity": "example/config-version", "version": "1", "digest": "sha256-jcs:<hex>"},
  "context": "ix://example/config-version/ConfigVersion",
  "operation": "attemptUpdate",
  "self": {"population": "ix://example/config-version/config_history", "key": "child"},
  "pre": {"identity": {"authority": "agent-ix", "identity": "..."}, "digest": "sha256-jcs:<hex>"},
  "post": {"identity": {"...": "..."}, "digest": "sha256-jcs:<hex>"},
  "parameters": {},
  "result": {"boolean": true},
  "created": [],
  "deleted": []
}
```

- `result` is a value or JSON `null`; `created` and `deleted` list
  `{population, key}` object references.
- `self` is an object reference for an operation of an object type, and JSON
  `null` for an operation of a state machine, whose receiver is the machine's
  context and has no object identity. The context is the one the `pre` and
  `post` snapshots hold (`contexts`), so the invocation names no instance.

`ClauseSelection` names the clause by its `QualifiedName` and one
observation: `Current { snapshot: DocumentRef, anchor, self: ObjectRef }`,
`PreCall { snapshot: DocumentRef, self: ObjectRef, parameters }` or
`Invocation { invocation: DocumentRef }` or `Context { invocation:
DocumentRef, side }`, where `DocumentRef` is the four
FR-001 labels and the `sha256-jcs` digest, `side` is `pre` or `post`, `anchor` is `{kind, name}`, and
`parameters` maps each declared parameter name to a value in the snapshot
value form, as an invocation's `parameters` member does. `Context` selects the
invariant of a state machine over one side of an invocation: QSpec FR-208-AC-10
holds that invariant in the pre observation and separately in the post
observation of every invocation of the machine's operations, so a caller
selects each side and gets one verdict per selection. For a clause whose
receiver is a state machine's context, `self` of `Current` and of `PreCall` is
none. `ValueInvariants { snapshot: DocumentRef, invocation: Option<DocumentRef> }`
selects no clause: it admits one current snapshot and, when given, one
invocation, for the evaluation of the type-level invariants of every type over
every value they range over (FR-366). The snapshot is read as a `Current`
snapshot is, and its anchor is not compared with any clause's. The invocation
is read as an `Invocation` selection of a postcondition reads it: its `pre` and
`post` snapshots, the `contexts` they hold, the parameters and the result,
admitted by the same checks. Checks that name the selected clause do not apply to it: check 4 compares
the document's model with the package's model, since there is no clause alias;
check 5 has no clause receiver, and check 9's `self` tests do not apply, there being no `self`, but its context rule does: each `pre` and `post` snapshot of the invocation holds exactly one context entry for the one machine the invocation's `context` names (an entry whose machine is that machine or a specialization of it, as check 9 reads it), else `invalid_model_binding`/`wrong-context` naming that machine and the snapshot, or `invalid_runtime_input`/`duplicate-member` at the later entry; no context is required of any other machine, whatever invariants it has, and a context of another machine is validated by check 6 and ignored; check 11's frame check runs on the invocation as it does for a postcondition; and check 7's
required population is every population of the snapshot, there being no
population holding `self`. The invocation's `context` and `operation` are
checked only against the package: `context` names a state machine, an object type or a process of it, and `operation` one of its effective operations, else `wrong_snapshot`/`wrong-invocation`; `ValueInvariants` reads its documents first (check 1: the snapshot, then the invocation and its `pre` and `post`), runs checks 3 to 8 on the current snapshot, then checks 3 to 11 on the invocation, and resolves the invocation's `context` and `operation` (check 5) after its model check (check 4). The admitted value holds no clause
identity and no `self` object; it holds the current observation and, for an
invocation, the observations, parameters, result and contexts as an `Invocation`
selection admits them. `PreCall` is the
observation of a call before the operation runs: the snapshot is the
operation's pre state, and a precondition reads only that state, `self` and
the parameters (FR-104, FR-107), so it has no post snapshot, result or
delta.

## Behavior

Admission runs the numbered checks in order and stops at the first failing
condition, returning its one record. Inside a check, the conditions run in the
order listed. Where a check walks the document, it walks populations in
document order, objects in document order within a population, and fields in
the object type's declared field order, and it reports the first failing
object and field. After the last object, it walks a snapshot's `relationships`
entries in document order, and the instances of each entry in document order,
and then a `pre` or `post` snapshot's `contexts` entries in document order, each
entry's fields in the machine's exposed effective field order.
For each entry it runs checks 6.6 and 6.7, then for each instance of the entry
it runs checks 6.8 and 6.9, and it stops at the first failing condition with
its relationship and zero-based instance index. A defect of an earlier
instance or entry is therefore reported before one of a later instance or
entry, whatever check number each has.

A `Context` selection with `side` `pre` reads the same documents as a
precondition selected by `Invocation` (the invocation and its `pre` snapshot)
and skips the same members and checks, and additionally skips the invocation's
`parameters` (an invariant binds no operation parameter, QSpec FR-208): check
10 does not run for it. A `Context` selection with `side` `post` reads the
documents of a postcondition (the invocation and its `pre` and `post`
snapshots) and runs every check including check 11, because the post side is
evidence only of an invocation whose frame and delta hold. Its parameters and
`result` are validated as for a postcondition, and the clause reads neither.
Its observation is the selected side's snapshot, which roots the clause's
value paths (FR-107).

A precondition's input is the pre-call observation (QSpec FR-046-AC-7,
FR-208-AC-13). For a precondition selected by `Invocation`, the selected
documents are the invocation document and its `pre` snapshot only. Admission SHALL
read the invocation's `self`, `parameters` and `pre` members and that
snapshot. Admission SHALL neither require, read nor check the invocation's `post`,
`result`, `created` or `deleted` members or its `post` snapshot: checks 1,
3, 4, 6 and 10 skip them, and check 11 does not run. A missing, malformed or
out-of-frame post side therefore changes nothing. Its admitted observation
and verdict SHALL equal those of a `PreCall` selection over the same pre
snapshot, `self` and parameters.

The **read contexts** of a clause whose receiver is a state machine's
context are, in each snapshot the clause reads (the `pre` snapshot, and for a
postcondition or `Context` with `side` `post` the `post` snapshot too), the
`contexts` entries whose `machine` is the clause's machine or specializes it.
They are decided from the entries' `machine` member alone, before any check.

A **required population** is the population holding `self`, every
declared population of the package that a reference in a parameter of an
invocation or `PreCall` selection names in the pre snapshot, including one
nested in a record, present optional or sequence, and,
repeatedly, every population named by a `reference` value anywhere in a field of
any object of a required population, and every population named by an end of an
instance of a listed relationship whose other end names an object of a required
population, in the observations the clause reads.
A required population absent from its snapshot is not marked `complete`. A
parameter reference naming no declared population makes no population
required; check 10 settles it.

1. Read check. For each selected document (the snapshot of `Current` or
   `PreCall`; the invocation and then its `pre` and `post` snapshots for a
   postcondition and for `Context` with `side` `post`; the invocation and then
   its `pre` snapshot for a precondition selected by `Invocation` and for
   `Context` with `side` `pre`):
   1. If no document is in its provision under the selected digest, then
      admission SHALL return `Incomplete` with `unavailable_observation`/
      `missing-required-artifact` (missing observation evidence is
      incomplete input, QSpec state contract).
   2. If the bytes exceed the document byte limit, then admission SHALL
      refuse `stage_limit_exceeded`/`input-bytes-exceeded` naming setting
      `observation.input_bytes`. A byte error from check 3's read or
      encoding is this refusal too (FR-259 B4).
   3. If the document's `sha256-jcs` digest differs from the selected digest,
      then admission SHALL refuse `stale_dependency`/`content-mismatch`
      carrying the selected digest and the recomputed digest (QSpec
      FR-272-AC-14).
      Bytes the shared reader refuses (FR-261) are digested raw, as FR-056
      does, and so refuse `stale_dependency`/`byte-digest-mismatch` carrying the selected digest and the raw bytes' own digest;
      admission never refuses them as malformed.
      If a document the reader reads holds a number with no exact RFC 8785
      spelling, by FR-056's rule, then admission SHALL refuse
      `noncanonical_wire`, with cause `inexact-integer` or
      `inexact-number` and the `document_pointer` of the first such
      number in document order, before the digest is compared. The rule
      covers every number of the document, including members admission
      does not otherwise read, such as an invocation's `post`, `result`,
      `created` and `deleted` under a precondition.
      If reading or encoding the document for its digest cannot reserve
      memory, then admission SHALL refuse
      `resource_exhausted`/`allocation-failed`, carrying the bytes requested
      (FR-259 B6). This check comes before any member is read.
   4. If `format` is not the expected format, then admission SHALL refuse
      `unknown_wire`/`unsupported-wire`.
   5. If a required member is missing (for a precondition selected by
      `Invocation` and for `Context` with `side` `pre`, `post`, `result`,
      `created` and `deleted` are not required, and for the latter
      `parameters` is not either), then admission SHALL refuse
      `invalid_runtime_input`/`missing-member`, naming the first missing
      member in the member order above.
   6. If an unknown member is present, then admission SHALL refuse
      `invalid_runtime_input`/`unknown-member`, naming the first in document
      order.
   7. If a label of the document's identity is empty or consists only of
      Unicode `White_Space` scalars, then admission SHALL refuse
      `invalid_source_identity`/`blank-label`. Within this check, the labels
      are checked in the order `authority`, `identity`, and field `label`
      names the first blank one.
2. Selection form check. If `Current` selects a precondition or postcondition,
   `PreCall` selects an invariant or a postcondition, or `Invocation` an
   invariant, then admission SHALL refuse `wrong_snapshot`/
   `wrong-observation`. A `Context` selection of anything but an invariant
   whose receiver is a state machine's context, an invariant of a state
   machine selected by `Current`, `PreCall` or `Invocation`, a `Current` or
   `PreCall` selection that names a `self` for a machine clause, and a `Current`
   or `PreCall` selection of an object-receiver clause that names none, each
   refuse `wrong_snapshot`/`wrong-observation`.
3. Observation role and anchor check. If the current snapshot does not say
   `current`, a `PreCall` snapshot does not say `pre`, an invocation's
   `pre` snapshot does not say `pre`, or (for a postcondition, or a `Context`
   selection with `side` `post`) its `post` snapshot does not say `post`, then admission SHALL refuse `wrong_snapshot`/
   `wrong-observation`. If the current snapshot's `anchor` differs from the
   selection's, then admission SHALL refuse `wrong_snapshot`/`wrong-anchor`,
   naming both.
4. Model check. If a selected document's `model` differs from the package's model
   selection for the clause's alias, then admission SHALL refuse
   `invalid_model_binding`/`wrong-model-selection`.
5. Operation check. If the clause's receiver is an object and an
   invocation's `context` or `operation` differs from the clause's, then
   admission SHALL refuse `wrong_snapshot`/`wrong-invocation`. If the clause is
   a state machine's invariant, precondition or postcondition and the
   invocation's `context` is neither the clause's machine nor a
   specialization of it, or its `operation` is not an effective operation of
   that machine, or (for a precondition or postcondition) is not the clause's
   operation, then admission SHALL refuse `wrong_snapshot`/
   `wrong-invocation`. An invariant names no operation, so `Context`
   accepts any effective operation of the machine (QSpec FR-208-AC-10: every
   invocation of every operation). If a `pre` or `post` snapshot of the invocation
   names an `invocation` whose four labels differ from the selected
   invocation's own identity, then admission SHALL refuse `wrong_snapshot`/
   `wrong-invocation`, naming that snapshot, the selected invocation's
   identity and the one the snapshot names, `pre` before `post`. A snapshot
   that names none is not checked, and a `PreCall` selection has no
   invocation to compare.
6. **Populations and values**, per object of the selected documents in walk
   order:
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
      `invalid-value`. A `reference` value fits a declared `Reference<T>`
      exactly when its target object's type conforms to `T` (QSpec FR-151:
      the same type, or a chain of declared supertypes leads from it to
      `T`).
      For a record value, admission SHALL resolve its `type` to an admitted
      record declaration, check conformance to the expected record type,
      require exactly its effective fields, and validate each field value
      under that field's declared type. A missing field refuses
      `missing-member`, an extra field `unknown-member`, an incompatible
      record type or value tag `wrong-value-kind`, and a malformed scalar
      payload `invalid-value`, all under `invalid_runtime_input` at the
      nested value path. A String, UUID, Timestamp or Text object is checked by the
      payload rule above. The same rule applies to operation parameters and
      results in check 10.
      Inputs are refused, outputs are evidence: this refusal applies to
      every current snapshot, every pre snapshot and every operation
      argument. A postcondition invocation's post snapshot is what the
      subject produced, so, when the caller selects post-state witnessing
      (`ObservationLimits::post_state` is `Witness`), an integer in it
      outside its declared `Int[lower, upper]` is not refused. Admission SHALL
      admit the exact integer, never coerced or clamped.
      Admission SHALL report it as an out-of-range observation naming the object, the
      field (its declaring object type and name), the declared range and the
      observed value, in walk order, with the element's position when the
      field is a sequence. For a nested record field it SHALL also retain
      the FR-207 field, option-value and index steps through the record to
      the integer. An out-of-range element of a post-state sequence
      field is evidence in the same way. The field's declared range is part
      of the operation's contract, so the observation is the witness of a
      violation; the caller settles it. A post-state value of the wrong
      kind, not in FR-038's spelling, or a String, UUID, Timestamp or Text object
      that fails the payload checks above (a Text outside its declared length
      included) is still refused. The default is
      `Refuse`, under which every snapshot refuses, and a `Frame` run's
      post snapshot always refuses. A clause that compares or orders the
      field, or does arithmetic on it, evaluates over the exact integer;
      only a value admission recorded as out of range is exempt from the
      evaluator's declared-type check, and any other operand outside its
      type is still an internal fault.
   6. If a `relationships` entry lacks a member, then admission SHALL refuse
      `invalid_runtime_input`/`missing-member`, naming the first missing of
      `relationship`, `complete` and `instances`. If an entry holds an unknown
      member, then admission SHALL refuse `unknown-member`. If `complete` is
      not a boolean or `instances` is not an array, then admission SHALL
      refuse `wrong-value-kind`.
   7. For each entry: admission SHALL decide FR-364's extent conditions 1 and 2
      at the entry: a `relationship` that names no navigation relationship of
      the package (a connection and an unknown identity included) refuses
      `invalid_runtime_input`/`wrong-role-mapping`, and a second entry for one
      relationship refuses `invalid_runtime_input`/`conflicting-identity` at
      the later entry.
   8. For each instance of an entry, in document order: if a member of the
      instance or of `source` or `target` is missing or unknown, then
      admission SHALL refuse `missing-member` or `unknown-member`, and a
      non-string `population` or `key`, `wrong-value-kind`. If an end's
      `population` names no population of the package, then admission SHALL
      refuse `invalid_runtime_input`/`wrong-role-mapping`, `source` before
      `target`. If an end names an object the snapshot lists, and that
      object's type does not conform to the relationship's end type, then
      admission SHALL refuse `invalid_runtime_input`/`wrong-value-kind`,
      naming the relationship, the instance and the end (FR-364 condition 3).
      An end naming a key the snapshot does not list is not decided here;
      checks 7 and 8 decide it.
   9. If an instance repeats an earlier instance of its entry and the
      relationship's two ends do not both declare `unique: false`, then
      admission SHALL refuse `invalid_runtime_input`/`conflicting-identity`
      at the later instance (FR-364 condition 4). No instance count is
      compared with an end's multiplicity bounds: a count outside `[lower,
      upper]` admits, and navigation refuses it (FR-364).
   10. If a snapshot says `current` and holds `contexts`, then admission SHALL
       refuse `invalid_runtime_input`/`unknown-member` naming `contexts`: a
       current observation names no invocation, so it holds no context. For
       a `contexts` entry of a `pre` or `post` snapshot, in document order:
       if `contexts` is not an array, or `machine` is not a string or `fields`
       not an object, then admission SHALL refuse `wrong-value-kind`; if
       `machine` or `fields` is missing, `missing-member`, naming the first
       missing in that order; if another member is present, `unknown-member`.
   11. For each entry: if `machine` names no state machine of the package (an
       object type, a process, a record value type or an unknown identity
       included), then admission SHALL refuse `invalid_runtime_input`/
       `wrong-role-mapping`; if an earlier entry of the snapshot names the
       same machine, then admission SHALL refuse `invalid_runtime_input`/
       `duplicate-member` at the later entry (QSpec FR-208: one context per
       machine in an observation).
   12. For each entry, over the machine's effective fields in exposed
       effective field order: if the machine declares a set, bag or
       ordered-set field, then admission SHALL refuse `unknown_required_feature`/
       `unsupported-feature` at that field; if a declared field is missing,
       `invalid_runtime_input`/`missing-member` naming it; if a field of
       `fields` is not an effective field of the machine, `unknown-member`
       naming the first in document order (a member named `state` or
       `transitions` is such a field unless the machine declares it); and,
       for each field in order, the value rules of check 6.5 apply to its
       value unchanged: kind fit, integer spelling and range, a record's
       effective fields, the payload order for String, UUID, Timestamp and Text
       objects, and the nested path, which here starts at the context (root
       the machine, then the field). A reference value fits a `Reference<T>`
       field by the conformance rule of 6.5, and its target is resolved in
       check 8 against the objects of the same snapshot. The post-state
       range witness of 6.5 applies to a context field of a `post` snapshot
       exactly as to an object field, and `OutOfRange` names the machine, the
       field, the declared range and the observed value.
7. Completeness check. If a required population is not marked `complete`, then
   admission SHALL return `Incomplete` with `incomplete_population`/
   `incomplete-scope`, naming the first such population in walk order.
   Admission SHALL skip the dangling check over an incomplete population.
   A `reference` value in a field of a read context also makes the population it names required, as a reference value in an
   object field does, because a context has no population of its own to mark
   complete or incomplete.
   An instance end that names a key its population does not list, where the
   population is not marked `complete`, also returns `Incomplete` with
   `incomplete_population`/`incomplete-scope` naming that population, because
   the end has no object to give the type component of its reference key; the
   first such end in walk order is named, after the populations above. (An end
   naming a listed key of an incomplete population admits.)
   An extent's own `complete` flag is not decided here: an entry with
   `"complete": false`, or an unlisted relationship, admits, and navigation
   returns `incomplete_population`/`incomplete-scope` when a clause reads it
   (FR-364).
8. Closure check. If a `reference` value names a key absent from its complete
   population, then admission SHALL refuse `dangling_reference`/
   `absent-target-in-complete-population`, naming the first such reference
   in walk order and its population. After the `reference` values, an instance
   end that names a key absent from its complete population refuses the same
   way, naming the relationship, the instance index, the end and the key and
   population; instances are walked as in check 6 and the ends of one instance
   are checked `source` before `target`. After the instance ends, a `reference` value in a field of the context the
   clause reads (a read context), `pre` before `post`, refuses the same way, naming
   the machine, the field path and the key and population, in effective field
   order. With check 7's rule for an end in an
   incomplete population, no stored end lacks an object: every admitted
   instance is a triple of full references, because only the object supplies
   the type component of its reference key (FR-364). The `resolve` of every
   stored end is therefore `present`; `dangling` and `undecidable` belong to
   model-layer extents. Checks 7 and 8 read the same `complete` flags the
   manifest rows carry.
9. Self and context check. If the clause's receiver is an object and the
   selected self object is absent from the current snapshot, from the pre
   snapshot, or (for a postcondition) from the post snapshot, then
   admission SHALL refuse `invalid_runtime_input`/`wrong-role-mapping`. If
   the clause's receiver is an object and an invocation's `self` is `null`,
   then admission SHALL refuse
   `invalid_runtime_input`/`wrong-value-kind`. For a clause whose receiver is a state machine's
   context, admission checks first that the invocation's `self` is JSON
   `null`, else refuses `invalid_runtime_input`/`wrong-value-kind`; then, for
   each observation the clause reads (the `pre` snapshot of a precondition and
   of a `Context` selection with `side` `pre`; the `pre` and then the `post`
   snapshot of a postcondition and of a `Context` selection with `side`
   `post`), the read contexts of that snapshot: if there is none, then admission SHALL refuse
   `invalid_model_binding`/`wrong-context` naming the machine and the
   snapshot; if there are two (a machine and one of its specializations both
   listed), then admission SHALL refuse `invalid_runtime_input`/
   `duplicate-member` at the later entry (QSpec FR-208: exactly one context in
   each observation). The context found is the clause's `self`; a context of
   another machine in the snapshot is validated by check 6 and then ignored.
   A context is never marked incomplete: a snapshot that lists no context of
   the machine is refused here, not returned `Incomplete`.
10. Parameters and result check. If an invocation's or a `PreCall`
    selection's `parameters` lack a declared
    parameter or hold an undeclared one, then admission SHALL refuse
    `invalid_runtime_input`/`missing-member` or `unknown-member`; if a value
    does not fit its declared type, `wrong-value-kind` or `invalid-value`, by
    check 6's value rule. For a postcondition, if the result member is `null` for an
    operation that declares a result, or a value for one that declares none,
    then admission SHALL refuse `invalid_runtime_input`/`missing-member` or
    `unknown-member`. A `PreCall` selection carries no result. A reference
    value, including one nested in a record, optional or sequence, resolves
    against the pre snapshot's objects for a parameter (the
    `PreCall` snapshot, or the invocation's `pre`) and against the post
    snapshot's for a result. If one names a key absent from its complete
    population, or names a population that snapshot does not list (a
    population the package does not declare included), then
    admission SHALL refuse `dangling_reference`/
    `absent-target-in-complete-population`, naming it and its population
    (check 8's closure rule).
11. Frame and delta check. For an invocation selected for a postcondition or
    by `Context` with `side` `post`, admission SHALL run the frame
    check (`enforce_frame` through `admit_invocation`, with the operation's
    `OperationEffect`, FR-103) once per population, in the pre snapshot's
    population document order and then any population only in the post
    snapshot, in its document order. `admit_invocation` admits one
    population per call, so admission calls it per population and reports
    the first population's first violation. For a population present in
    only one of the two snapshots, admission SHALL pass an empty document
    (no objects) for the missing side, so its objects are creations or
    deletions. The check SHALL compare every declared field of every
    surviving object, scalar, reference and record alike. The frame check SHALL
    compare a changed record value by its most-specific type and the fields
    exposed after QSpec FR-150 normalization, under QSpec FR-208. The frame
    check SHALL decide whether a field changed by comparing the pre and post
    values as admitted typed values under the equality of the field's
    declared type, never the document's raw JSON: JSON member order,
    whitespace, escape spelling and a supplied spelling that admission
    normalizes are no change. The equality of each declared type is: `Boolean`
    the same truth value; `Int[lo, hi]` the same integer; `String` the same
    scalar sequence, with no normalization; `Text[min, max; profile]` the same
    retained sequence under that profile (QSpec FR-141), so U+00E9 and U+0065
    U+0301 are one value under `nfc`, `nfd`, `nfkc` and `nfkd` and two under
    `unicode-scalars` and `binary-utf8`; `UUID` and `Timestamp` the same
    canonical payload; `Reference<T>` the same population and key; `Option<T>`
    both absent, or both present with equal contents; `Sequence<T>` the same
    length with equal elements in order; a record the same most-specific type
    and equal effective fields; a variant the same case and, where it carries
    one, an equal payload. The frame check SHALL apply the
    selected QSpec FR-013 and FR-340 frame grant to
    the changed declared object field. A record value's member comparison
    detects changes to that field without introducing another frame grant.
    Within one population it SHALL report the first violation in this
    order, which is `enforce_frame`'s:
    1. walking the post snapshot's objects in ascending key order: an object
       absent from the pre snapshot that `creates` does not grant, or a
       surviving object whose most-specific type differs between pre and
       post (`FrameTypeChanged`; no frame authorizes a retype):
       `frame_violation`/`unauthorized-change`;
    2. an object of the pre snapshot absent from the post snapshot that the
       frame's `deletes` does not grant, in ascending key order:
       `frame_violation`/`unauthorized-change`;
    3. a changed field of a surviving object outside `modifies`, objects in
       pre document order and fields in ascending `DeclarationKey` order
       (`enforce_frame`'s `BTreeSet`, `qsl-semantics/src/model/
       population.rs:1488-1490`): `frame_violation`/`unauthorized-change`;
    4. over that population's entries of `created` and `deleted`: a repeated
       identity in either, an identity in both, then a list that differs
       from the computed one: `population_delta_mismatch`/
       `delta-disagreement`.
    5. once, after every population, over relationships rather than
       objects. If a relationship that both snapshots list with
       `"complete": true` is not in the frame's `modifies`, and an instance's
       count differs between the pre and post extents, then admission SHALL
       refuse `frame_violation`/`unauthorized-change` for the first such
       instance, naming the relationship, the instance and whether it was
       `added` or `removed`. Relationships are walked in ascending
       `DeclarationKey` order and instances in ascending order of source then
       target reference key. A `Frame` run (FR-115) reports the refusal as the
       evaluated `Link` change of its frame witness. An instance with an end at an object the
       invocation's `deleted` lists is left out, because a deleted object's
       links go with it as its fields do. An instance with an end at a
       created object is an added instance and needs the grant: unlike a
       created object's own fields, a link also changes what the object at
       its other end navigates to, and `creates` grants only the new object. A
       relationship that is not listed complete in both snapshots is not
       compared, because the snapshots do not say whether it changed.
    6. once, after the relationships, for a state machine clause: the pre
       and the post context (the read contexts of check 9) are compared field by field in the
       machine's exposed effective field order, as admitted typed values under
       the equality of the field's declared type, as an object's fields are. A
       pre context and a post context of different most-specific machines are
       a retype, which no frame authorizes: it refuses `frame_violation`/
       `unauthorized-change` naming both machines, before any field is
       compared. A
       changed field that the operation's frame does not grant in `modifies`
       (QSpec FR-013 and FR-340) refuses `frame_violation`/
       `unauthorized-change`, naming the machine and the field. A context
       needs no `creates` or `deletes` grant and has no entry in `created` or
       `deleted`.

- Admission SHALL read no path, environment variable, clock or search
  location. The same package, provisions, selection and limits SHALL give
  the same result.
- The document reader is a string edge (ADR-012 §9): `format`,
  `observation`, `anchor.kind`, member names and value tags are converted
  once, at read, to closed enums or typed identities.
- Value validation SHALL use an explicit work stack, walking populations
  and objects in document order, then relationship entries and their
  instances in document order, then context entries and their fields in
  that order, record fields in exposed effective field
  order and sequence
  elements by ascending index. It SHALL count the record, each option
  wrapper, each sequence and each contained value against the
  declared values-per-document setting (a context entry counts as one value,
  and each of its field values as it would in an object), and stop at that limit before the next
  value with `stage_limit_exceeded`/`node-count-exceeded`, naming the
  setting `observation.values`, the bound and the count the next value would
  have reached (FR-255); finite recursive record
  declarations therefore admit only finite
  supplied values within the byte and value limits, with no call-stack or
  schema-depth limit. A nested refusal SHALL carry its exact FR-207 `field`,
  `option_value` and `index` steps from the root object or invocation binding.
  Invariant evaluation follows QSpec FR-208-AC-10 after admission, once for
  each reachable record value, with the same path; admission does not turn a
  false invariant into an input refusal.
- Admission re-runs the model walks of the selected domain packages. A
  ceiling of `ModelNormalizationLimitsV1` reached there refuses
  `resource_exhausted` with the compile path's cause (FR-255 Behavior 1).
- Every code and cause above is in QSpec `native-diagnostics.md`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-106-AC-1 | The healthy-parent current snapshot above, selected with anchor `handler validate` and `self` = `child` for `ParentOrder`, admits: one observation with population `config_history` complete, objects `root` and `child` with `versionNumber` 1 and 2, `child.parent` present and naming `root`, and the snapshot's identity and digest retained. Re-serializing the document with other whitespace and member order gives the same digest and the same admitted value. Over a package variant where `Sub` specializes `ConfigVersion` and a second population `archive` has member type `Sub`, the same snapshot with `child.parent` naming `k`, a `Sub` object of a complete `archive`, admits with `child.parent` naming `k` typed `Sub`. | Test |
| FR-106-AC-2 | The changed-version invocation (pre: `child` at 2; post: `child` at 3; result `true`; no parameters, created or deleted) selected for `VersionUnchanged` admits, with the pre and post observations distinct and `result` true. | Test |
| FR-106-AC-3 | Each condition of checks 1 to 6, 9 and 10 has one case that fails only it and returns exactly its code and cause, as TC-465 lists: among them an absent document, a 1 MiB + 1 byte document, bytes edited under their original digest, `format` `native-state-input/1` (`unknown_wire`/`unsupported-wire`), a missing `populations`, an extra member, a blank `authority`, `Current` selecting `VersionUnchanged`, a `pre` document selected as current, anchor `handler other`, another model digest, operation `other`, an unknown population, a duplicate `root` key, a set-typed field, `versionNumber` `{"boolean": true}`, `"01"`, `"-1"` and `"1001"`, `self` = `ghost`, a `result` of `null` for `attemptUpdate`, an object of a type that is not a member type of its population, a missing declared field, an undeclared field, `child.parent` naming an object of a type unrelated to `ConfigVersion` (`wrong-value-kind` at `child`, `parent`), and, for an operation `probe(target: ConfigVersion)` with no result, a missing, an unknown and an ill-typed parameter, and a result value on an invocation selected for a postcondition on `probe` (a precondition selected by `Invocation` reads no result, AC-9). | Test |
| FR-106-AC-4 | The incomplete-population snapshot (`complete: false`, `child.parent` naming `missing`) gives `Incomplete` with `incomplete_population`/`incomplete-scope` and no dangling refusal; the same snapshot with `complete: true` gives `Refused` with `dangling_reference`, naming `missing` and `config_history`. A second, incomplete population that no reference value names does not make a healthy case incomplete. | Test |
| FR-106-AC-5 | The forbidden-parent-change invocation (post sets `child.parent` absent) refuses `frame_violation`/`unauthorized-change` naming `child` and `parent`. A package whose `attemptUpdate` frame modifies only `parent`, with an invocation that changes only `versionNumber`, refuses `frame_violation`/`unauthorized-change` naming `versionNumber` (a scalar field). An invocation declaring `created: [child]` refuses `population_delta_mismatch`/`delta-disagreement`. A post snapshot that adds a population `archive` absent from the pre snapshot, holding `a1`, refuses `frame_violation`/`unauthorized-change` naming the creation of `a1` (the pre side is an empty document). | Test |
| FR-106-AC-6 | Running admission twice over the same inputs gives equal results, and admission builds its result without reading the filesystem (the provisions are in-memory maps; a test with no files on disk passes). | Test |
| FR-106-AC-7 | Multi-defect documents report the first defect by the order above: bytes edited under their original digest that also add an unknown member refuse `content-mismatch`; a snapshot with `root.versionNumber` `"-1"` and `child.versionNumber` `"1001"` refuses `invalid-value` at `root`; an invocation whose post both deletes `root` and changes `child.parent` refuses the deletion; over a package where `Sub` specializes `ConfigVersion`, an invocation whose post changes `child`'s type to `Sub` and deletes `root` refuses the type change, naming `child`; an invocation whose pre and post list `archive` before `config_history`, with a change outside the frame in each, refuses naming the `archive` object; a snapshot whose `authority` and `identity` are both blank refuses `invalid_source_identity`/`blank-label` with `label` `authority`. | Test |
| FR-106-AC-8 | Over TC-466 step 3's `probe` unit, `PreCall { snapshot, self: a, parameters: {target: a} }` over the chain `a -> b -> c` pre snapshot, selected for `ReachesTarget`, admits one pre observation, `self` `a`, the parameter `target` naming `a`, no post observation, result or delta, and runs no frame check. The same selection for `VersionUnchanged` refuses `wrong_snapshot`/`wrong-observation`; with a snapshot that says `post` it refuses `wrong_snapshot`/`wrong-observation`; with no `target` it refuses `invalid_runtime_input`/`missing-member`; with `target` naming `ghost`, absent from the complete `config_history`, it refuses `dangling_reference`/`absent-target-in-complete-population`; over the same unit in a package variant where `Sub` specializes `ConfigVersion` and a second population `archive` has member type `Sub` (so `config_history` stays the one population covering `ReachesTarget`), `target` naming `a1` in `archive`, which the snapshot lists with no objects and marks `complete: false`, and which no field of `a`, `b` or `c` reaches, returns `Incomplete` with `incomplete_population`/`incomplete-scope` naming `archive`; with its snapshot marked `complete: false` it returns `Incomplete` with `incomplete_population`/`incomplete-scope`. In that variant, `target` naming a `Sub` object of a complete `archive` admits, typed `Sub`. Over the base `probe` unit plus an object type `Other` unrelated to `ConfigVersion` and a population `others` over it, `target` naming an `Other` object of a complete `others` refuses `invalid_runtime_input`/`wrong-value-kind`; over the variant plus an operation `probeSub(target: Sub)` on `ConfigVersion` and a precondition on it, `PreCall` with `target` naming `a` refuses `invalid_runtime_input`/`wrong-value-kind`. | Test |
| FR-106-AC-9 | The forbidden-parent-change invocation (its post sets `child.parent` absent, outside `attemptUpdate`'s frame) selected for a precondition admits the pre observation, `self` and parameters with no post observation, result or delta and no frame check, and the precondition's verdict equals its verdict under `PreCall` over the same pre snapshot, `self` and parameters. The same invocation whose `post` snapshot is absent from the provision, and the same invocation whose `post` snapshot bytes are malformed (not JSON), each admit and give that same verdict, with no `unavailable_observation`, `byte-digest-mismatch` or other refusal. Beyond check 1.3's number rule (FR-106-AC-11), admission neither requires nor reads the invocation's `post`, `result`, `created` or `deleted` members: the same invocation with those four members removed, and with each of them ill-formed, admits and gives that same verdict, and a `probe` invocation whose `result` is `{"boolean": true}`, selected for `ReachesTarget`, admits and gives `ReachesTarget`'s verdict under `PreCall` over the same pre snapshot, `self` and parameters. | Test |
| FR-106-AC-11 | The authorized-change invocation selected for the precondition `AttemptUpdatePre` refuses `noncanonical_wire` in check 1.3 when its `post` is `9007199254740993` (`inexact-integer`, `document_pointer` `/post`), its `result` is `0.1000000000000000000001` (`inexact-number`, `/result`), its `created` is `[0,1e-400]` (`inexact-number`, `/created/1`) or its `deleted` is `[{"a/b":18446744073709551616}]` (`inexact-integer`, `/deleted/0/a~1b`), although admission reads none of those members further; with those members `2`, `0.1`, `[0,1e-300]` and `[{"a/b":9007199254740992}]` it admits. | Test |
| FR-106-AC-12 | Inputs are refused, outputs are evidence. Over a package whose operation declares a parameter `Int[0, 1000]`, an invocation or `PreCall` argument of `-1` refuses `invalid_runtime_input`/`invalid-value` naming the parameter, both under the default limits and under `PostStateRange::Witness`; `1000` admits. Over the ConfigVersion model's `versionNumber` (`Int[0, 1000]`), a postcondition invocation's post snapshot holding `-1` under `PostStateRange::Witness` admits, and the post observation reports one `OutOfRange` naming the object `child`, the field `versionNumber`, the range `[0, 1000]`, the exact observed -1 and no index; a pre snapshot holding `-1` or `1001` under `Witness` still refuses `invalid-value`. A sequence field of `Int[0, 1000]` elements holding `5`, `1001` in a post snapshot under `Witness` admits exactly and reports the element at index 1; under `Refuse` it refuses. An in-range value reports nothing. | Test |
| FR-106-AC-13 | The default is `Refuse`. The wrapping debit (`child.versionNumber` 0 to -1) refuses `invalid_runtime_input`/`invalid-value` naming `child` and `versionNumber` under `admit_observations` with default limits, through `run_clause`'s `Clause` selection of `VersionUnchanged`, and through `run_clause`'s `Frame` selection of `attemptUpdate`. | Test |
| FR-106-AC-14 | A current snapshot of `Account` whose required `address: Address` field is `{"record":{"type":"ix://example/model/PostalAddress","fields":{"street":{"present":{"record":{"type":"ix://example/model/Street","fields":{"name":{"integer":"7"}}}}},"zip":{"integer":"42"}}}}` admits when `PostalAddress` specializes `Address`, its effective `street` field is optional `Street`, its `zip` field is `Integer`, and `Street.name` is `Integer`. Reordering JSON object members yields the same typed observation and selected digest. The two nested records retain their most-specific types and are reached by distinct FR-207 field and option-value paths. | Test |
| FR-106-AC-15 | A record payload with a missing effective field refuses `invalid_runtime_input`/`missing-member`; one with an extra field refuses `unknown-member`; a `type` naming an object type, an unrelated record subtype, or no selected record refuses `wrong-value-kind`; and an object reference in place of a record refuses `wrong-value-kind`. Each refusal names the first nested path reached in field order, including an index for a sequence element. The same cases in a parameter or result use the invocation binding as the path root. | Test |
| FR-106-AC-16 | A finite `Node { next?: Node }` value nested through 10,000 present `next` fields admits when byte and value limits allow it, with no reader or validator stack-depth refusal. At the exact values-per-document bound the document admits; one more nested value returns `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`, the bound and the count reached, before that value is admitted. A false invariant on one nested `Node` is reported as a logical violation at its FR-207 runtime value path after admission, not as an input refusal. | Test |
| FR-106-AC-17 | An invocation changing `Account.address` from one admitted record value to another admits its frame when the selected `modifies` grant covers that declared object field. Without that grant it refuses `frame_violation`/`unauthorized-change` naming `Account.address`, whether the change is to a record member or its most-specific type. The existing QSpec FR-013 and FR-340 receiver and grant semantics apply. | Test |
| FR-106-AC-18 | Over the TC-1029 fixture, a current snapshot whose `child` holds `token` `{"kind":"uuid","text":"00112233-4455-6677-8899-aabbccddeeff"}` and `stamp` `{"kind":"timestamp","nanoseconds":"0"}` admits both as a UUID and a Timestamp value of the retained canonical payloads; reordering the members of either object, and the whitespace of the document, gives the same typed observation and digest. Nil (`00000000-0000-0000-0000-000000000000`), Max (`ffffffff-ffff-ffff-ffff-ffffffffffff`), a UUID whose version nibble is `0`, outside RFC 9562's version table (`00112233-4455-0677-8899-aabbccddeeff`), and Timestamps `1`, `-1`, `170141183460469231731687303715884105727` and `-170141183460469231731687303715884105728` admit unchanged. A present optional `prior`, each element of a `Sequence<Timestamp>` field `seen`, and the `id` and `at` parameters and a UUID `result` of an invocation admit by the same rule, and each object counts as one value toward `observation.values`. | Test (TC-1029) |
| FR-106-AC-19 | A UUID `text` in uppercase (`00112233-4455-6677-8899-AABBCCDDEEFF`), mixed case, hyphenless, braced, `urn:uuid:`-prefixed, with a moved hyphen, with a non-hex digit, of the wrong length, padded with whitespace or holding a non-ASCII digit refuses `invalid_runtime_input`/`invalid-value` naming `child`, `token` and member `text`, with kernel class `noncanonical-payload`. A Timestamp `nanoseconds` of `-0`, `+1`, `01`, `-01`, the empty string, ` 1`, `1.0`, `1e3` and `2026-10-15T00:00:00Z` refuses `invalid-value` with class `noncanonical-payload`. `170141183460469231731687303715884105728` and `-170141183460469231731687303715884105729` refuse `invalid-value` with class `payload-out-of-domain`; `0170141183460469231731687303715884105728` refuses with `noncanonical-payload`, never the range class. No refusal produces a value or a repaired spelling. | Test (TC-1029) |
| FR-106-AC-20 | Native payload objects refuse in the listed order: `kind` as `1` or `null` refuses `wrong-value-kind` naming `kind`, even beside a missing payload member; `kind` `String`, `UUID` and `uuidv4` refuse `invalid-value` naming `kind`, and `{"kind":"String","nanoseconds":"0"}` refuses the same way, its members unexamined; a UUID object with no `text` (and `{"kind":"uuid","nanoseconds":"0"}`) and a Timestamp object with no `nanoseconds` refuse `missing-member` naming that member; an extra member refuses `unknown-member` naming it; `nanoseconds` as the JSON number `0`, and `text` as `null` or `5`, refuse `wrong-value-kind` naming the member. An object with both an extra member and a noncanonical `text` refuses `unknown-member`. | Test (TC-1029) |
| FR-106-AC-21 | A Timestamp object in the UUID field `token`, a UUID object in the Timestamp field `stamp`, `{"integer":"5"}` in `stamp`, `{"integer":"5"}` and `{"boolean":true}` in `token`, a UUID object in the integer field `versionNumber` and in a `Reference` field, a Timestamp object inside `{"present": ...}` of the optional UUID `prior`, and a UUID object as an element of `seen` each refuse `invalid_runtime_input`/`wrong-value-kind` at the nested path (the sequence element with its index), including a Timestamp object with `nanoseconds` `"01"` in `token` and a UUID object with `text` `"X"` in `stamp`, which are `wrong-value-kind`, never `invalid-value`. The same cases in a parameter or result use the invocation binding as the path root. | Test (TC-1029) |
| FR-106-AC-22 | An invocation changing `token` or `stamp` between two admitted payloads admits its frame when the selected `modifies` grant covers the field and refuses `frame_violation`/`unauthorized-change` naming `child` and the field without it; one whose post holds the same payload, with differently ordered members, is no change. Under `PostStateRange::Witness`, a post snapshot holding a noncanonical UUID or Timestamp still refuses `invalid-value`. With `observation.values` set to the exact count of a document holding a `token` and a `stamp`, the document admits; one less returns `stage_limit_exceeded`/`node-count-exceeded` at the last of those values in walk order. | Test (TC-1029) |
| FR-106-AC-23 | TC-1035's `orders` snapshot of population `Q` (`s1`, `s2`, `p1`, `p2`, `p3`, `t1`) lists `Sys-composition-Pump` complete with instances `s1`-`p2` then `s1`-`p1`, and `Pump-housing-Tank` complete with `p1`-`t1` and `p2`-`t1`. Selected for an invariant on `Sys` with `self` `s1`, it admits one observation whose composition extent reads `targets` `[p1, p2]` and whose housing extent reads `sources` of `t1` `[p1, p2]`; reordering the entries, the instances and the JSON members gives the same digest and the same admitted value. The same snapshot with no `relationships` member admits with both relationships unlisted (`complete` false); with `Sys-composition-Pump` listed `"complete": true` and `"instances": []` it admits with that extent complete and empty. A snapshot listing four instances `s1`-`p1` to `s1`-`p4` through `parts`, whose destination end is `{0,3}`, and one with no `Pump-housing-Tank` instance for `p3` through `home`, whose destination end is `{1,1}`, admit. | Test (TC-1035) |
| FR-106-AC-24 | Each of checks 6.6 to 6.9 has one case that fails only it and returns exactly its code and cause, naming the relationship and the zero-based instance index: an entry without `instances`, an entry with an extra member, a `complete` of `"yes"`; a `relationship` that is a connection, and separately an unknown identity; two entries for one relationship; an instance without `target`, with an extra member, with a numeric `key`; a `population` naming no population of the package; a `Tank` `t1` at the `source` end of `Sys-composition-Pump`, and separately at its `target` end; a repeated `s1`-`p1` instance where the ends are unique. A repeated instance admits as two instances when both ends of the relationship declare `unique: false`, and refuses when only one does. Checks run entry by entry and instance by instance: a repeat at instance 1 with a wrong end type at instance 2 refuses the repeat, and a wrong end type at instance 0 of the first composition entry refuses before a second composition entry does. | Test (TC-1035) |
| FR-106-AC-25 | A target `t9` of a `Pump-housing-Tank` instance that `Q` does not list, with `Q` complete, refuses `dangling_reference`/`absent-target-in-complete-population` naming the relationship, the instance and end `target`, `t9` and `Q`; the same snapshot with `Q` `"complete": false` returns `Incomplete` with `incomplete_population`/`incomplete-scope` and no dangling record. A dangling `reference` value and a dangling instance end in one snapshot report the `reference` value. Over a variant in which the `Tank` object `t1` lives in a second population `Tanks` marked `"complete": false`, an instance `p1`-`t1` makes `Tanks` required and returns `Incomplete` naming `Tanks`, while the same snapshot without the instance admits. An extent listed `"complete": false` admits. An instance end naming an unlisted key of a population that is not complete, with no end in a required population, returns `Incomplete` naming that population; the same end naming a listed key admits. | Test (TC-1035) |
| FR-106-AC-26 | An invocation of the operation `rehome` on `Pump`, selected for its postcondition `RehomePost`, whose pre snapshot lists `Pump-housing-Tank` complete with `p1`-`t1` and `p2`-`t1` and whose post lists it complete with `p1`-`t2` and `p2`-`t1` admits its frame when `rehome` modifies `Pump-housing-Tank`, and refuses `frame_violation`/`unauthorized-change` naming the relationship and the instance `p1`-`t1` `removed` when it does not. A post that deletes `p2` and drops `p2`-`t1` admits under a frame that deletes `Pump` and does not modify the relationship; a post that creates `p3` with `p3`-`t1` under a frame that creates `Pump` and does not modify the relationship refuses naming `p3`-`t1` `added`. A relationship listed in only one of the two snapshots changes nothing. | Test (TC-1035) |
| FR-106-AC-27 | Each instance counts as one value against `observation.values`: a snapshot whose objects and instances reach the bound exactly admits and one more instance returns `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`, the bound and the count reached. An invocation selected for a precondition admits and gives its `PreCall` verdict when its `post` snapshot lists a malformed `relationships` entry. | Test (TC-1035) |
| FR-106-AC-28 | Over the TC-1036 fixture, a current snapshot whose `child` holds `label` `{"kind":"string","text":"ready"}`, an absent `note` and an empty `tags` admits `label` as a String value of the retained scalar sequence; reordering the members of the object, and the whitespace of the document, gives the same typed observation and digest. These `label` texts each admit unchanged: the empty String, `"  ready  "`, `"READY"`, `"\u00e9"` (U+00E9), `"e\u0301"` (U+0065 U+0301), `"00112233-4455-6677-8899-aabbccddeeff"`, `"\u0000"`, `"a\u001fb"`, `"\ufffe"`, `"a;b>>>c"`, a text of 65,536 scalars and the paired escape `"\ud83d\ude00"`, which gives the value and digest of the literal U+1F600. U+00E9 and U+0065 U+0301 are two unequal values with different digests. A present `note`, each element of `tags` and the `name` parameter and the String `result` of an invocation admit by the same rule, and each object counts as one value toward `observation.values`. | Test (TC-1036) |
| FR-106-AC-29 | String objects refuse in the order of the native payload rule: `kind` as `1`, `null` or `{}` refuses `wrong-value-kind` naming `kind`, even beside a missing `text`; `kind` `"String"`, `"STRING"` and `"Text"`, and `{"kind":"blob","value":"x"}`, refuse `invalid-value` naming `kind`, the members unexamined; `{"kind":"string"}` and `{"kind":"string","nanoseconds":"0"}` refuse `missing-member` naming `text`; `{"kind":"string","text":"x","extra":1}` refuses `unknown-member` naming `extra` and `{"kind":"string","text":"x","nanoseconds":"0"}` naming `nanoseconds`; `text` as `5`, `null`, `true`, `["x"]` and `{}` refuses `wrong-value-kind` naming `text`; an object with both an extra member and a `text` of `5` refuses `unknown-member`. No `text` makes a String object refuse `invalid-value`: every Unicode scalar sequence is a String. | Test (TC-1036) |
| FR-106-AC-30 | A String object in the UUID field `token`, the Timestamp field `stamp`, the integer field `versionNumber`, the `Reference` field `parent`, a `{"present": ...}` of the optional UUID `prior` and as an element of `seen`; a UUID object, a Timestamp object, `{"integer":"5"}`, `{"boolean":true}`, `{"absent":{}}` and the bare JSON string `"ready"` in the String field `label`; and a String object whose text is a canonical UUID in `token` each refuse `invalid_runtime_input`/`wrong-value-kind` at the nested path (the sequence element with its index) and read the text as no other kind. `{"kind":"string","text":5}` in `token` refuses `wrong-value-kind` naming `text`, and `{"kind":"string","text":"x","extra":1}` in `versionNumber` refuses `unknown-member`, because both defects come before type fit. The same cases in a parameter or result use the invocation binding as the path root. | Test (TC-1036) |
| FR-106-AC-31 | An invocation changing `label` between two admitted payloads admits its frame when the selected `modifies` grant covers the field and refuses `frame_violation`/`unauthorized-change` naming `child` and `label` without it, including a change from U+00E9 to U+0065 U+0301 and from `ready` to `Ready`; one whose post holds the same payload with differently ordered members, or written `"\u0072eady"`, is no change. Under `PostStateRange::Witness`, a post snapshot holding a `label` of the wrong kind still refuses `wrong-value-kind`. With `observation.values` set to the exact count of a document whose `child` holds a present `note` as its last value in walk order, the document admits; one less returns `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`, the bound and the count the String object inside `note` would have reached. | Test (TC-1036) |
| FR-106-AC-32 | A `label` text holding a lone high surrogate escape `\ud800`, a lone low surrogate escape `\udc00` or the byte `0xC0` refuses `stale_dependency`/`byte-digest-mismatch` at check 1.3 carrying the selected digest and the raw bytes' own digest, before any value is read: the same document with a wrong-kind `token` elsewhere gives that refusal and not `wrong-value-kind`. A `label` text that brings the document over the byte limit refuses `stage_limit_exceeded`/`input-bytes-exceeded` naming `observation.input_bytes` at check 1.2. | Test (TC-1036) |
| FR-106-AC-33 | Over the TC-1038 fixture, a current snapshot whose `child` holds `title` `{"kind":"text","text":"Ready"}`, an absent `subtitle`, `code` `{"kind":"text","text":"abc"}` and an empty `aliases` admits `title` as a Text value of the retained sequence; reordering the members of the object, and the whitespace of the document, gives the same typed observation and digest. These `title` texts each admit: `"a"` (the declared minimum), `"abcdefgh"` (the declared maximum), `"a;b>>>c"`, U+00E9, U+0065 U+0301, `"\u0000"`, and the paired escape `"\ud83d\ude00"`, which gives the value and digest of the literal U+1F600. Under `nfc`, U+00E9 and U+0065 U+0301 retain one sequence and so are one equal value with two different digests, and eight pairs of U+0065 U+0301 (sixteen supplied scalars) retain eight scalars and admit. A present `subtitle`, each element of `aliases` and the `name` argument and the Text `result` of an invocation admit by the same rule, and each Text object counts as one value toward `observation.values`. | Test (TC-1038) |
| FR-106-AC-34 | Text objects refuse in the order of the payload rule: `kind` as `1`, `null` or `{}` refuses `wrong-value-kind` naming `kind`, even beside a missing `text`; `kind` `"Text"`, `"TEXT"` and `{"kind":"blob","value":"x"}` refuse `invalid-value` naming `kind`, the members unexamined; `{"kind":"text"}` and `{"kind":"text","nanoseconds":"0"}` refuse `missing-member` naming `text`; `{"kind":"text","text":"Ready","extra":1}` refuses `unknown-member` naming `extra` and `{"kind":"text","text":"Ready","nanoseconds":"0"}` naming `nanoseconds`; `text` as `5`, `null`, `true`, `["x"]` and `{}` refuses `wrong-value-kind` naming `text`; an object with both an extra member and a `text` of `5` refuses `unknown-member`. | Test (TC-1038) |
| FR-106-AC-35 | A String object in the Text field `title`, as a `{"present": ...}` of the Text field `subtitle` and as an element of `aliases`; a Text object in the String field `label`, the UUID field `token`, the Timestamp field `stamp`, the integer field `versionNumber` and the `Reference` field `parent`; a UUID object, a Timestamp object, `{"integer":"5"}`, `{"boolean":true}`, `{"absent":{}}` and the bare JSON string `"Ready"` in `title`; and a Text object holding the canonical UUID text in `token` each refuse `invalid_runtime_input`/`wrong-value-kind` at the nested path (the sequence element with its index) and read the text as no other kind. A Text object whose text is longer than `title`'s maximum, placed in `label`, refuses `wrong-value-kind`, never `invalid-value`, because type fit comes before the length check. `{"kind":"text","text":5}` in `token` refuses `wrong-value-kind` naming `text`, and `{"kind":"text","text":"x","extra":1}` in `versionNumber` refuses `unknown-member`, because both defects come before type fit. The same cases in a parameter or result use the invocation binding as the path root. | Test (TC-1038) |
| FR-106-AC-36 | Text length is measured on the retained sequence under the declared profile and refuses `invalid_runtime_input`/`invalid-value` naming `text`, the declared bounds, the profile and the measured length, nested at the field. Over `title` (`Text[1, 8; nfc]`): `""` (length 0), `"abcdefghi"` (9), nine pairs of U+0065 U+0301 (nine retained scalars) and a text of 65,536 `a` refuse, while eight pairs of U+0065 U+0301 admit. Over `subtitle` (`Text[0, 4; unicode-scalars]`): `"abcd"` and four U+00E9 admit; `"abcde"` and three pairs of U+0065 U+0301 (six scalars, nothing normalized) refuse. Over `code` (`Text[0, 4; binary-utf8]`, length in bytes): `"abcd"`, two U+00E9 and one U+1F600 admit; three U+00E9 (six bytes) and two U+1F600 refuse. The same refusals hold for the `name` argument and the `result` of `retitle`, and under `PostStateRange::Witness` a post snapshot holding an over-length Text still refuses `invalid-value` and one holding a String object in `title` still refuses `wrong-value-kind`. | Test (TC-1038) |
| FR-106-AC-37 | An invocation changing `title` between two admitted payloads admits its frame when the selected `modifies` grant covers the field and refuses `frame_violation`/`unauthorized-change` naming `child` and `title` without it; one whose post holds the same text with differently ordered members, written `"\u0052eady"`, or changed from U+00E9 to U+0065 U+0301 under `nfc`, is no change and admits without the grant. A change of `subtitle` from U+00E9 to U+0065 U+0301 under `unicode-scalars` is a change and refuses naming `child` and `subtitle` when `retitle`'s frame does not grant it. With `observation.values` set to the exact count of a document whose `aliases` holds one Text element as the last value in walk order, the document admits; one less returns `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`, the bound and the count the Text object inside `aliases` would have reached. | Test (TC-1038) |
| FR-106-AC-38 | A `title` text holding a lone high surrogate escape `\ud800`, a lone low surrogate escape `\udc00` or the byte `0xC0` refuses `stale_dependency`/`byte-digest-mismatch` at check 1.3 carrying the selected digest and the raw bytes' own digest, before any value is read: the same document with a wrong-kind `token` elsewhere gives that refusal and not `wrong-value-kind`. A `title` text that brings the document over the byte limit refuses `stage_limit_exceeded`/`input-bytes-exceeded` naming `observation.input_bytes` at check 1.2, not `invalid-value`. | Test (TC-1038) |
| FR-106-AC-39 | Over TC-1035's base snapshot **B** in a package that also declares population `Spare` (member type `Pump`), which **B** does not list, admission issues a closed manifest with the rows `Q` complete and `Spare` incomplete in ascending declaration-key order; with `Spare` listed `"complete": false` the rows are the same, and listed `"complete": true` both rows are complete. The admitted observation's `resolve` for each stored instance end is `present`. | Test (TC-1041) |
| FR-106-AC-40 | Over TC-1044's fixture, the frame check decides each declared type by its own equality: a `String` changed from U+00E9 to U+0065 U+0301 is a change; a `Text[0, 8; nfc]` changed so is no change and a `Text[0, 8; unicode-scalars]` changed so is a change; a `Text[0, 8; nfc]` changed from `a` to `b` is a change; a `UUID` changed to another canonical payload is a change; an `Option` changed from absent to present is a change and present to present with equal contents is none; a `Sequence` reordered is a change and the same elements in the same order with the document's whitespace changed is none; the members of an object written in another order are no change. A change not covered by the grant refuses `frame_violation`/`unauthorized-change` naming the object and the field, and an unchanged field admits without any grant. The Text rows depend on the validator applicability change of FR-056's Text paragraph. | Test (TC-1044) |
| FR-106-AC-41 | Over TC-1045's `Orders` fixture, the invocation `i1` of `place` (`self` `null`, `flags` `{placed: true}`) with `pre` context `{placed: false, cancelled: false}` and `post` context `{placed: true, cancelled: false}`, selected by `Context` side `pre` for the invariant `Consistent`, admits one observation holding the pre context, the identity labels of `i1`, no self object, no parameter and no result; selected side `post` it admits the post context. Both runs give `Completed(true)`. Reordering the `fields` members and the `contexts` entries gives the same typed observation (the digest is of the document and changes with entry order), and reordering `fields` members and whitespace gives the same digest. | Test (TC-1045) |
| FR-106-AC-42 | The same invocation with a `post` context `{placed: true, cancelled: true}` admits side `post` and `Consistent` evaluates `Completed(false)`: a false invariant is a logical violation after admission, never an input refusal. Side `pre` over the same invocation still gives `Completed(true)`, and gives it with the `post` snapshot absent from the provision or malformed, with `parameters` ill-formed and with `result`, `created` and `deleted` removed. Side `post` with the `post` snapshot absent returns `Incomplete` with `unavailable_observation`/`missing-required-artifact`. | Test (TC-1045) |
| FR-106-AC-43 | A snapshot holding no `contexts` member, or none for `OrderLifecycle`, refuses `invalid_model_binding`/`wrong-context` naming `OrderLifecycle` and that snapshot, for the invariant on side `pre` (the `pre` snapshot), on side `post` (the `pre` snapshot first, then the `post`), for the precondition `PlaceRequires` selected by `Invocation` or `PreCall`, and for the postcondition `FlagsPlaced`. No empty or default context is made, and the result is never `Incomplete`. Two entries naming `OrderLifecycle` in one snapshot refuse `invalid_runtime_input`/`duplicate-member` at the second; an entry for `OrderLifecycle` and one for its specialization `ExpressLifecycle` in one snapshot refuse the same way at check 9. A context of the unrelated machine `Billing` in a snapshot is validated and ignored. | Test (TC-1045) |
| FR-106-AC-44 | Each condition of checks 6.10 to 6.12 has one case that fails only it and returns exactly its code and cause, naming the snapshot and the entry: `contexts` on a snapshot that says `current` (`unknown-member` naming `contexts`); `contexts` as an object (`wrong-value-kind`); an entry without `machine`, without `fields`, with an extra member, with a numeric `machine`, with an array `fields`; a `machine` naming the object type `Pump`, the process `Fulfilment` and the unknown identity `ix://test/orders/Missing` (`wrong-role-mapping`); a field missing from `fields` (`missing-member` naming it); an undeclared field, a `state` member and a `transitions` member among them (`unknown-member` naming the first in document order); `placed` as `{"integer": "1"}` (`wrong-value-kind` at the context path `OrderLifecycle`, `placed`). Defects in two entries report the first entry's, and a defect in a context after a bad object reports the object's (objects, relationships, then contexts). | Test (TC-1045) |
| FR-106-AC-45 | The post-state range witness reaches a context: over `ExpressLifecycle` with a field `retries: Int[0, 5]`, a `post` context holding `-1` under `PostStateRange::Witness` admits side `post` and reports one `OutOfRange` naming the machine `ExpressLifecycle`, the field `retries`, the range `[0, 5]` and the observed `-1`; the same value in a `pre` context, and in a post context under `Refuse`, refuses `invalid_runtime_input`/`invalid-value` at `ExpressLifecycle`, `retries`. A context field of a record, String, UUID, Timestamp or Text type follows the value rules and payload order of the object fields. | Test (TC-1045) |
| FR-106-AC-46 | The frame check compares contexts: with `place` modifying `placed` and `cancelled`, a post context changing both admits; with `place` modifying only `placed`, a post context that also changes `cancelled` refuses `frame_violation`/`unauthorized-change` naming `OrderLifecycle` and `cancelled`, for the postcondition and for the invariant on side `post`, and side `pre` runs no frame check and admits. A `post` context equal to the `pre` context in every field, written in another member order, is no change. An invocation whose `self` is an object reference for a state-machine operation, and one whose `self` is `null` for an object operation, refuse `invalid_runtime_input`/`wrong-value-kind`. A machine invariant selected by `Current`, `PreCall` or `Invocation`, an object invariant selected by `Context`, and a `Current` selection naming a `self` for a machine precondition refuse `wrong_snapshot`/`wrong-observation`. | Test (TC-1045) |
| FR-106-AC-47 | A context field `owner: Reference<Pump>` of the machine `Owned` naming `p9`, absent from the complete population `Q` of the same snapshot, refuses `dangling_reference`/`absent-target-in-complete-population` naming `Owned`, `owner`, `p9` and `Q`; with `Q` marked `"complete": false` it returns `Incomplete` with `incomplete_population`/`incomplete-scope` naming `Q`, and the same field naming a listed key of `Q` admits. A second population `Spare` listed incomplete that no reference names admits. Each context entry and each of its field values counts toward `observation.values`: a document at the exact bound admits and one more value returns `stage_limit_exceeded`/`node-count-exceeded` naming `observation.values`. A `quire.state.snapshot/v1` environment document supplied to replay (FR-098) that holds `contexts` refuses `invalid_runtime_input`/`unknown-member` naming `contexts`, because an environment document is a current snapshot. | Test (TC-1045) |
| FR-106-AC-48 | `ValueInvariants` over TC-235 K2 with `Order` given `quoted`, `history`, `discount` and operation `requote(amount: Money)` admits observation `s1` holding `o1` and invocation `i1` of `requote` with `amount` `{amount_minor: -7, currency: "EUR"}`, and names `s1`, `i1`'s `pre` snapshot and `i1`'s parameter `amount`; the same selection with a `pre` document as `snapshot` refuses `wrong_snapshot`/`wrong-observation`, and with an `i1` whose `amount` is missing refuses `invalid_runtime_input`/`missing-member`. | Test (TC-1051) |
| FR-106-AC-49 | Over TC-1044's fixture with the record `Address { street: Title }` held by the field `address`, the frame check compares a record's field by that field's declared type: `street` changed from U+00E9 to U+0065 U+0301 under `nfc` is no change and admits without a grant, and `street` changed to another text is a change and refuses `frame_violation`/`unauthorized-change` naming `child` and `address`. | Test (TC-1044) |


## Dependencies

- FR-084 and FR-089 (population binding and identity), FR-103 (operation
  effect), FR-001 (the two labels; QSpec STD-150), FR-056 (the `sha256-jcs` rule and its
  digest-first order; the K2 String, UUID and Timestamp payloads and their runtime-input
  refusals), FR-038 (integer spelling).
- QSpec FR-208 (State-machine context anchors: the context, exactly one per
  machine in each observation, `wrong-context` and `duplicate-member`; AC-10:
  the invariant in the pre and post observation of every invocation), FR-207
  (the receiver binding path), FR-013 and FR-340 (frame grants). FR-107
  (receiver path of a context) and FR-098 (the environment document is a
  current snapshot).
- FR-364 (relationship extents: the instance model, its admission conditions
  and the read contract navigation uses).
- QSpec FR-141 and FR-149 (the `Text[min, max; profile]` retained sequence, its
  length and its equality; the Text object's profile semantics).
- QSpec FR-153 and `state-contract.md` (anchors, completeness and closure),
  `native-diagnostics.md` (every code and cause above).
- QSpec FR-046-AC-7 and FR-208-AC-13 make a precondition's input the
  pre-call observation; `PreCall` and a precondition selected by
  `Invocation` both admit exactly that observation.
