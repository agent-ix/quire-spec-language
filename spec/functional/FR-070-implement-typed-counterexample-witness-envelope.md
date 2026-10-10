---
id: FR-070
title: "Implement the typed counterexample/witness envelope"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: traces_to
---
# FR-070: Implement the typed counterexample/witness envelope

## Description

QSL SHALL implement a typed counterexample/witness envelope, defined in the
layer-6 `replay` module and part of its public API (ADR-011 FB-05; ADR-013
O-25), that stores QSL's `Witness`'s admitted transcript as its one stored
fact and derives every other witness fact (`harness_symbol`,
`check`/`check_text`, `concrete_values`, `decode`) from that transcript on
every call, never as a second, independently-stored copy (ADR-013 QC-13; the
AD-016 Replay-ownership row's five `Witness` fields collapse to this one
stored field plus four derived ones).

The envelope carries, for deterministic replay, every ADR-013 O-25 member:
the obligation identity and clause occurrence key; the selected function's
`QualifiedName`; the `package_id` and `RawSourceRef`
digests of the package the harness was generated from; the semantic profile
selections; the `run_limits` and declared domains; the `backend` member
(O-19); the trace position where the family has one; and the `ReplaySource`
(`Witness` or `Input`). A packet missing any of these members SHALL be
refused at reconstruction; none is optional or defaulted.

This requirement builds the common envelope only. It does not add any
family-specific payload (that is #186's). `qsl-replay` owns `Witness`,
`ReplaySource` and `WitnessEnvelope` (ADR-013 OQ-H) and implements
`Witness::parse`'s admission rule; this envelope's constructor requires an
already-admitted `Witness`. CG builds the envelope from its backend run, and
its backend adapter parses the backend output into the transcript `parse`
admits.

A `union` value text decodes, but a replay cannot convert one until S6a
admits union arguments (QSL-503).

## Inputs

- A `WitnessPacket` carrying `source: ReplaySource` and its O-25 members
  (obligation identity, occurrence key, package reference, profile
  selections, bounds, backend, trace position), built by CG.
- An already-admitted QSL `Witness` (admitted through `Witness::parse`) for
  the `ReplaySource::Witness` arm, or canonical assignments for the
  `ReplaySource::Input` arm.

## Outputs

- A typed counterexample/witness envelope carrying the O-25 members above,
  or a structured refusal when any required member is absent or a
  transcript fails admission.

## Behavior

- The envelope SHALL store the admitted transcript as its only witness-shape
  field; `harness_symbol()`, `check()`, `check_text()`, `concrete_values()`
  and `decode(&[WitnessBinding])` SHALL each recompute their answer from the
  stored transcript on every call, so the envelope cannot disagree with its
  own backend evidence.
- `decode` SHALL read each transcript entry as its binding's declared
  `WitnessValueType`: `Boolean` as `0` or `1`, `I64` as a decimal `i64`,
  and `Canonical` as a witness value text (below). It SHALL return one
  `WitnessValue` per binding, in binding order, or one `DecodeRefusal`
  with no partial result.
- Envelope construction SHALL refuse a transcript that is not the exact,
  single, trimmed assertion block `Witness::parse` selects (a cover
  transcript, an untrimmed transcript, or a transcript with zero or more
  than one assertion block), and SHALL NOT produce a partially-populated
  envelope in that case.
- A construct → serialize → read round trip SHALL preserve the stored
  transcript byte-for-byte and every O-25 member exactly: obligation
  identity, occurrence key, `package_id` and every `RawSourceRef` digest,
  semantic profile selections, `run_limits` and declared domains, `backend`
  member, trace position, and the `ReplaySource` variant (with its
  `Witness`/`Input` payload).
- Envelope reconstruction SHALL refuse when any one O-25 member is absent,
  and SHALL NOT substitute a default value for the missing member.
- The common envelope type SHALL expose an extension point through which a
  family (starting with #186's state `forall`) attaches its own typed
  payload as a distinct, family-owned type, without adding a variant,
  field, or special case to the common envelope's own type or constructors.
  The common envelope type SHALL define that extension point as a trait a
  family-owned payload type implements, or as a generic parameter the
  common envelope carries, and SHALL NOT define it as a `String`-keyed or
  otherwise untyped map that a family populates by convention.
- This envelope carries no `contract_version` of its own: version ownership
  is delegated to the FR-331 envelope (FR-069) within which a
  `counterexamples` entry travels, and to the closed FR-201 digest-domain
  set every `RawSourceRef` and `package_id` digest this envelope stores must
  belong to. Envelope construction SHALL refuse a digest whose domain falls
  outside that closed set.
- Envelope construction SHALL refuse when the envelope's encoded size
  exceeds the configured reader bound, and SHALL NOT return a truncated or
  partially-populated envelope in that case.

## Witness value text

A parameter whose declared type is `Boolean` or an integer type keeps its
scalar entry: `0`/`1`, or a decimal `i64` that decodes to
`WitnessValue::Integer(i64)`, as before. An integer leaf inside a witness
value text decodes to `WitnessValue::ExactInteger(Integer)` instead, so the
form of each integer follows from where it appears. Every other parameter's entry is a witness value text: a leaf
of any value family (enum, text, rational, decimal, float, quantity or
reference), or a composite (an `Option`, a record, a tuple, a union, or a
`Sequence`, `Set`, `Bag` or `OrderedSet` collection) whose leaves may be of
any value family, `Boolean` and integer included. A composite can nest to
any depth. The language has no map value type, so no map value has a form
here. A `Population<T>` parameter is not a value (a population binding is
a model input, FR-106), so it has no witness form.

CG writes this form from its composite leaf binding (CG FR-025, planned
under IR-635). It builds the value from the shadow leaves its harness drew,
then writes the value text below. QSL owns this form and its decode
(QSL-640).

**Transcript entry.** The entry is `<parameter node id>=<escaped value
text>`, joined by node id like every other entry. The value text is the RFC
8785 (JCS) encoding of the value in QSpec FR-181's typed canonical form,
limited to the shapes below.

**Transcript escaping.** JCS escapes only `"`, `\` and the control
characters U+0000 to U+001F. It writes `;`, `<` and `>` as themselves, so a
text value such as `a;b` or `x>>>y` would break the transcript grammar's
entry delimiter (`;`) or block end (`>>>`). The entry therefore carries the
value text with exactly four characters replaced: `%` by `%25`, `;` by
`%3B`, `<` by `%3C` and `>` by `%3E` (uppercase hex). Decode reverses the
replacement before it reads the JSON. Every other byte is written as
itself, so each value text has exactly one escaped form. `=` needs no
escape, because an entry splits at its first `=`, and the node id holds
none. `|` needs no escape either, because the values field is the block's
last field.

| Shape | Value text (FR-181) |
| --- | --- |
| Boolean | `{"type":"boolean","value":true}` or `false` |
| integer | `{"type":"integer","value":"<decimal>"}`: the exact value as FR-181's decimal string (`0`, or an optional `-` then `[1-9][0-9]*`), unbounded. It decodes to `WitnessValue::ExactInteger`, which holds a kernel `Integer`, whether or not the value fits `i64` |
| enum | `{"type":"enum","name":"<declaration>","member":"<member>"}` |
| text | `{"type":"text","value":"<string>"}`: the text's Unicode scalars as a JSON string |
| rational | `{"type":"rational","numerator":"<decimal>","denominator":"<decimal>"}`: reduced (the greatest common divisor is 1), with a positive denominator; zero is `0`/`1` |
| decimal | `{"type":"decimal","coefficient":"<decimal>","scale":"<decimal>"}`: the retained coefficient and scale, before normalization (QSpec FR-140) |
| float | `{"type":"float32","bits":"<8 hex>"}` or `{"type":"float64","bits":"<16 hex>"}`: the exact IEEE bits, big-endian lowercase hex, so NaN payloads and signed zeros stay distinct |
| quantity | its magnitude's rational or decimal form with an added `"unit":"<unit>"` member |
| reference | `{"type":"reference","universe":"<64 hex>","object_type":"<64 hex>","identity":"<hex>"}`: the universe and effective object type identities, and the object's identity bytes in lowercase hex |
| `Option<T>` | `{"type":"option","value":<value>}` when present, `{"type":"option","value":null}` for `none` |
| record | `{"type":"record","name":"<declaration>","fields":[{"name":"<field>","value":<slot>}, ...]}`, one entry per declared field in declaration order. `<slot>` is a value when present, `null` when an optional (`?`) field is absent, and `{"type":"null"}` for a `null` slot |
| tuple | `{"type":"tuple","name":"<declaration>","components":[<value>, ...]}`, in declaration order |
| union | `{"type":"union","name":"<declaration>","member":"<member>","components":[<value>, ...]}`, an empty array for a nullary member |
| `Sequence<T>` | `{"type":"sequence","elements":[<value>, ...]}`, in sequence order |
| `OrderedSet<T>` | `{"type":"ordered-set","elements":[<value>, ...]}`, in its order, with no two equal elements |
| `Set<T>`, `Bag<T>` | `{"type":"set"\|"bag","elements":[<value>, ...]}`, ascending by each element's JCS UTF-8 bytes; a set holds no two equal elements, a bag keeps every duplicate |

`<declaration>` is the enum, record, tuple or union declaration's checked
node id, in the 64-lowercase-hex `WireNodeId` spelling the entry's own
parameter name uses. FR-181 leaves `<qualified-name>`'s text form to the
implementation, and a node id is the identity FR-098 already resolves in
the recompiled package. `<field>` and `<member>` are the declared member
identifiers. `<unit>` is the 64-lowercase-hex digest of the declared
quantity type's unit identity; for a declared unit that is the unit's node
id.

**Decode.** The binding type of a parameter with a witness value text is
`WitnessValueType::Canonical`. `decode` SHALL unescape a `Canonical`
binding's entry and read the value text through `quire-canonical`'s shared
reader, on an explicit heap stack and at any nesting depth
([FR-261](FR-261-read-other-untrusted-json-at-any-depth.md)), into one
`WitnessValue` tree. It checks the value's form only. The value's declared
type (declaration and unit identity, members, arity, ranges, text bounds and
profile, scales and cardinality) is checked when FR-098 converts and admits
it. `decode` SHALL refuse with `DecodeRefusal::Malformed`, naming the
parameter, when the entry:

- has a `%` that does not begin one of the four escapes, or an escape in
  lowercase hex;
- is not one JSON value after unescaping, or has bytes after it;
- is not its own JCS encoding (for example, extra whitespace, reordered
  members, or a string escape JCS does not write), so each value has
  exactly one value text;
- uses a `type` tag outside the table, or a shape with a missing, extra or
  mistyped member;
- holds a decimal string not in FR-181's spelling, a rational that is not
  reduced or whose denominator is not positive, a `bits` string of the
  wrong length or not lowercase hex, or an `identity` that is not
  lowercase hex of whole bytes;
- uses `{"type":"null"}` anywhere but as a record field's slot;
- has `set` or `bag` elements out of ascending order, or two elements with
  equal JCS bytes in a `set` or an `ordered-set`.

Duplicates by the language's equality are FR-098's check, not decode's. The
kernel's equality relates values whose value texts differ: decimals compare
by numeric value, so `1.0` and `1.00` (coefficients `10` and `100`, scales
`1` and `2`) are equal. FR-098's conversion refuses a `set` or
`ordered-set` holding two elements that relation equates.

**Entry order.** Every transcript's entries SHALL be in strictly ascending
order of parameter node id, comparing the 64-hex spellings bytewise (the
order CG FR-025 already gives its arguments). Together with the one value
text per value, this means two equal witnesses always have identical
transcripts (ADR-014 TR-1). `decode` SHALL refuse
`DecodeRefusal::EntryOrder`, naming the first entry out of order, for a
transcript whose entries are in any other order. A repeated node id stays
`DecodeRefusal::Duplicate`.

No nesting depth refuses. Only the envelope's encoded size bounds a value
text: the reader bound of FR-070-AC-7 is `replay.input_bytes`
([FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md)). Every
walk over a `WitnessValue` tree (decode, clone, equality, the redacted
`Debug` of FR-073, and drop) SHALL use an explicit heap stack, never the
native stack in proportion to depth (ADR-030 D-1). An `Input` arm's
`CanonicalAssignment` SHALL carry a `WitnessValue` of the same shapes. The
envelope's measured encoded size SHALL count each such value at the byte
length of its escaped value text.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-070-CON-1 | Decoding a witness or its derived facts SHALL rely only on `Witness`'s own typed accessors, never on parsing or interpreting the transcript's rendered/display text. | Design | Test (TC-182) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-070-AC-1 | The envelope stores the transcript once; `harness_symbol`, `concrete_values` and `decode` each recompute their result from that one stored transcript rather than from a separately-stored value, so no code path can leave the envelope's derived facts disagreeing with its transcript. | Test (TC-180) |
| FR-070-AC-2 | A cover transcript, an untrimmed transcript, and a transcript with zero or two assertion blocks each refuse at envelope construction; none produces a partially-built envelope. | Test (TC-181) |
| FR-070-AC-3 | A positive envelope's construct → serialize → read round trip preserves the transcript byte-for-byte and every O-25 member (obligation identity, occurrence key, package reference and digests, profile selections, bounds and domains, backend, trace position, `ReplaySource` variant) exactly, with no member re-derived, reordered or dropped. | Test (TC-182) |
| FR-070-AC-4 | An O-25 packet missing any one required member (for example, no trace position on a family that carries one, or no `backend` member) refuses at reconstruction; no envelope is built with a defaulted or absent value in that member's place. | Test (TC-183) |
| FR-070-AC-5 | #186 can add a state-`forall`-specific witness payload as a typed consumer of the envelope's extension point, in #186's own change, with no edit to this envelope's type, constructors, or round-trip contract, and the extension point itself is typed (a trait or generic parameter), never a `String`-keyed untyped map. | Test (TC-184) |
| FR-070-AC-6 | This envelope defines no `contract_version` member of its own; a `RawSourceRef` or `package_id` digest it stores whose domain falls outside the closed FR-201 digest-domain set refuses at construction. | Test (TC-181) |
| FR-070-AC-7 | An envelope whose encoded size exceeds the configured reader bound refuses at construction, with no truncated or partially-populated envelope returned. | Test (TC-181) |
| FR-070-AC-8 | A transcript with one `Canonical` entry and one `I64` entry decodes to two `WitnessValue`s in binding order. The `Canonical` entry is a record whose fields hold a nested record with a present field, an absent optional field and a `null` slot; a present option and a `none` option; a three-element sequence; a set and a bag in ascending JCS order (the bag with a duplicate); an ordered set; a union with a one-component member; and a union with a nullary member. Each slot state, option state, element order, member identifier and declaration node id decodes exactly as written. The `I64` entry decodes as it did before this change. | Test (TC-905) |
| FR-070-AC-9 | Each of these entries refuses `DecodeRefusal::Malformed` naming the parameter, with no partial result: trailing bytes; added whitespace; record members out of JCS order; an unknown `type` tag; a record field entry with no `value` member; an integer `"007"`, and an integer as a JSON number; a rational `2`/`4`, and one with denominator `-1`; a `float64` with 15 hex digits, and one with uppercase hex; a reference `identity` of odd length; `{"type":"null"}` as a sequence element; a set with two equal elements; set elements out of order; an ordered set with two equal elements; a raw `%` not starting an escape; a lowercase escape `%3b`; and a record field name written `\u0061` where JCS writes `a`. | Test (TC-905) |
| FR-070-AC-10 | On a thread with a 512 KiB stack, a transcript entry holding a 100,000-long recursive list (`record List { head: Int[0, 9]; tail?: List; }`) decodes, with the envelope's reader bound raised to fit it through `replay.input_bytes`. The decoded value clones, compares equal to itself, renders its redacted `Debug` and drops with no stack overflow. With the bound one byte below the envelope's size, construction refuses `BoundExceeded` naming `replay.input_bytes`. | Test (TC-736) |
| FR-070-AC-11 | One `Canonical` entry per leaf family decodes exactly as written: an integer `-170141183460469231731687303715884105728` (outside `i64`); an enum member; the text `a;b=c<<<d>>>e%f` (its entry holds `a%3Bb=c%3C%3C%3Cd%3E%3E%3Ee%25f`, and its escaped bytes count toward `replay.input_bytes`), which parses as one block whose entry splits only at its own delimiters; the rational `-3`/`4`; the decimal with coefficient `1050` and scale `2`; a `float32` NaN with payload `7fc00001` and a `float64` negative zero `8000000000000000`; a quantity `{"type":"decimal","coefficient":"15","scale":"1","unit":"<unit>"}`; and a reference. A sequence of text values holding `;` and `>` decodes in order. | Test (TC-905) |
| FR-070-AC-12 | A transcript with entries for node ids `0a…` and `0b…` in that order decodes. The same entries with `0b…` first refuse `DecodeRefusal::EntryOrder` naming the `0a…` entry, with no partial result. | Test (TC-905) |
| FR-070-AC-13 | Encoding a witness value does work proportional to its output at any depth. The counted work is the number of bytes the encoder's JCS writer produces plus the number of bytes compared while ordering set and bag elements; the output is the value's JCS bytes before transcript escaping. Each value's bytes are produced once and set and bag elements are ordered by comparing finished bytes in place, so for nested sets (each level a set of the next level and a text), nested options and nested sequences (each level a sequence of the next level and an integer), doubling the depth from 5,000 to 10,000 at most triples the counted work (a subtree copied into every ancestor quadruples it); at depth 5,000 the counted work is at least the output's bytes, and at depth 10,000 it is at most three times the output's bytes. The value text's length is counted from the encoder's finished bytes: for a text value of 1,000,000 `;` characters, `value_text_len` returns 3,000,026, the length of the text `to_value_text` returns, and the encoder holds that value as one finished value whose buffers together hold at least its 1,000,026 JCS bytes, each buffer's capacity smaller than 3,000,026 bytes. | Test (TC-905) |

## Dependencies

- **Upstream**: [US-010](../usecase/US-010-carry-a-proof-witness-or-replay-outcome-without-a-shadow-type.md);
  ADR-013 O-25 (witness/counterexample carrier), QC-13 (transcript-only
  storage), QC-6/QC-8 (FR-331 `counterexamples` shape), OQ-H (`qsl-replay`
  owns `Witness`, its admission rule and its `decode` types); QSpec
  [FR-331](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-331-backend-provider-envelope.md)
  AC-9 (`counterexamples` entry shape) and
  [QSpec FR-351](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/protocol/FR-351-separating-witness-record.md)
  (separating-witness record; QSpec status **Draft** as of this writing —
  fully specified with acceptance criteria, cited here as the normative
  shape this envelope's `Input`-arm and #217/#186 consumers read).
- **Shared types**: [#213](https://github.com/agent-ix/quire-spec-language/issues/213)
  (ARCH-20) owns the O-04 `NodeKey`/occurrence-key shapes the obligation
  identity and occurrence key members reuse, the O-18 digest record the
  `RawSourceRef` and `package_id` digests reuse, and the O-11 `QualifiedName`
  the selected function's identity reuses; this requirement defines no
  parallel identity or digest type.
- **Witness value text**: QSpec FR-181 (typed canonical form);
  [FR-261](FR-261-read-other-untrusted-json-at-any-depth.md) (the shared
  reader at any depth); [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md)
  (`replay.input_bytes`); ADR-030 D-1; CG FR-025's composite leaf binding
  (planned, IR-635) writes the form, and
  [FR-358](FR-358-settle-a-composite-bounded-shadow-item.md) settles the item
  it comes from.
- **Downstream**: [FR-071](FR-071-implement-typed-replay-request.md) carries
  this envelope's members into the replay request;
  [FR-072](FR-072-implement-typed-replay-result.md)'s per-item result embeds
  the QSpec FR-351 record this envelope's `Witness` arm decodes.

