---
id: ADR-034
title: "Timestamp as one nanosecond count, native collection spelling in QSL documents, and a retired std evaluator"
type: ADR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-402
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/FR-405
    type: relates_to
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-144
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-149
    type: relates_to
  - target: ix://agent-ix/quire-specification/FR-181
    type: relates_to
---
# ADR-034: Timestamp as one nanosecond count, native collection spelling in QSL documents, and a retired std evaluator

## Context

This record settles three questions about QSL and QSpec wire forms and QSL's
evaluators.

- **Timestamp wire form.** QSL FR-056 defines the K2 Timestamp payload as one
  exact signed count of nanoseconds since `1970-01-01T00:00:00Z` on the POSIX
  scale, within the signed 128-bit domain, written
  `{"kind":"timestamp","nanoseconds":"<decimal>"}`. QSpec FR-149 keys and
  compares a Timestamp by that count. QSpec FR-181's typed canonical form
  also needs a Timestamp form, for state keys and for supplied collection
  elements. The other candidate was a pair of decimal members, whole seconds
  and a nanosecond remainder, in UTC.
- **Collection spelling.** QSL FR-106's snapshot and invocation documents
  (`quire.state.snapshot/v1`, `quire.state.invocation/v1`) write every value
  in one closed grammar: either single-tag objects (`{"integer": "1"}`,
  `{"sequence": [...]}`) or payload objects that have a `kind` member. QSpec
  FR-144 defines the wire form of a supplied collection as the FR-181 typed
  canonical form, `{"type": "set", "elements": [...]}`, whose elements are
  FR-181 typed forms "with no second grammar". The two specifications can
  disagree about how a set, bag or ordered set is spelled in a QSL
  document.
- **The std reference evaluator.** AD-001 has two implementations of FR-262's
  evaluation semantics while `quire-eval` is built: the std reference
  evaluator in `qsl-eval` and the `no_std` evaluator in `quire-eval`. FR-402
  keeps them equal until FR-405's switch. After the switch, the std evaluator
  could be deleted (retired) or kept in a lesser role (demoted), such as a
  test oracle or a fallback.

## Decision

| ID | Decision |
| --- | --- |
| TS-1 | A Timestamp is written in every wire form as the single signed 128-bit nanosecond count of QSL FR-056, as a decimal string: `{"kind":"timestamp","nanoseconds":"<decimal>"}` in QSL documents and `{"type":"timestamp","nanoseconds":"<decimal>"}` in QSpec FR-181's typed canonical form. No form splits an instant into seconds and nanoseconds, or carries a zone, a calendar text or a precision selector. |
| TS-2 | Each instant has exactly one spelling: `0`, or an optional `-` followed by a nonzero digit and more digits. A reader refuses any other spelling and never normalizes it, so equal instants have equal bytes everywhere: in a state key, in a set element and in a document. |
| CS-1 | In QSL FR-106 documents, a sequence, set, bag or ordered set is written in FR-106's native single-tag spelling: `{"sequence": [...]}`, `{"set": [...]}`, `{"bag": [...]}` and `{"ordered-set": [...]}`. Each element is a value of FR-106's grammar. An FR-106 document never contains a QSpec FR-181 typed form, an object whose `type` member is an FR-181 tag such as `{"type": "set", "elements": [...]}`; such an object is not a value of FR-106's grammar. FR-106's record payload `{"record": {"type": "<declaration identity>", "fields": ...}}` is FR-106's own form, and its `type` names a declaration, not an FR-181 tag. |
| CS-2 | FR-106's collection spelling corresponds one to one with QSpec FR-181's `sequence`, `set`, `bag` and `ordered-set` forms: the kinds are the same, each element maps to its FR-181 typed form, and a bag is written as repeated elements in both. Admission applies QSpec FR-144's "Supplied collection values" rules unchanged: the bound is checked before any element is read; in a set or ordered set a duplicate is refused and never removed, while a bag groups equal elements into one member whose multiplicity is their count; array order is ignored for a set or bag and kept for an ordered set; and an element type with no FR-181 form is refused. |
| CS-3 | QSpec FR-181's typed canonical form is the wire for QSpec's state keys and for a QSpec runtime input (FR-144). FR-144's "no second grammar" applies inside that form: an FR-181 collection's elements are FR-181 typed forms. A QSL document is a separate QSL wire with its own complete grammar, and QSL admission maps each of its values onto the FR-181 value it denotes. |
| EV-1 | FR-405's switch retires the std reference evaluator: it is deleted. It does not remain as a Cargo feature, a `cfg` branch, a runtime or test fallback, a differential oracle, a dev-dependency or a test-support crate. After the switch, `quire-eval` is QSL's only evaluator of checked terms. |
| EV-2 | After the switch, the corpus expectations recorded at the switch base check `quire-eval` (FR-405's regression harness). FR-402's differential comparison runs only before the switch, while both implementations exist. |

## Consequences

- A Timestamp's state-key bytes, set-element bytes and K2 payload are all
  built from one count, so FR-149 equality, the FR-181 encoding and the
  FR-106 document equality agree without any conversion. A reader needs no carry rule
  for a remainder of 10^9 or more, and no sign rule for an instant before the
  epoch with a nonzero remainder.
- QSL documents keep one grammar for every value. A collection element is read
  by the same rules as a field, parameter or result value. Tools that write
  QSL documents never mix `{"type": ...}` and single-tag objects.
- The FR-106 and FR-181 spellings must stay in one-to-one correspondence. A
  new collection kind or element form is added to both, and FR-106 states
  which FR-181 form each native value denotes.
- QSL, RT and CG share one implementation of the evaluation semantics, which
  is AD-001's purpose. No second evaluator needs FR-402 equality after the
  switch, and a fault in `quire-eval` is found by the recorded corpus and the
  requirement tests, not by comparison against a second implementation.

## Alternatives Considered

- **Timestamp as seconds and nanoseconds** (`{"type":"timestamp",
  "seconds":"<decimal>","nanos":"<decimal>"}`, UTC). Rejected. It gives a
  second representation of the K2 payload that FR-056 and FR-149 already
  define. It needs range and normalization rules for `nanos` and for negative
  instants, and keeping a reader canonical needs one more refusal case per
  member.
- **QSpec FR-181 typed forms inside QSL documents** (`{"type": "set",
  "elements": [...]}` in a snapshot). Rejected. Every other FR-106 value is a
  single-tag or `kind` object, so a snapshot would hold two value grammars,
  and FR-106 would need a second reader for collection values and their
  elements.
- **Both spellings in QSL documents.** Rejected. Two spellings of one value
  give two documents for one typed observation, and each would need its own
  refusal rules.
- **Demote the std evaluator to a differential test oracle.** Rejected. It
  keeps two implementations of one semantics in the workspace indefinitely,
  and FR-402 equality would have to be maintained for every later change of
  FR-262.
- **Keep the std evaluator behind a feature or as a fallback.** Rejected for
  the same reason. It also adds an evaluator selector that FR-405 rules out.

## References

- QSL FR-056 (K2 Timestamp payload), FR-106 (snapshot and invocation value
  grammar), FR-402 (differential testing), FR-405 (the switch), AD-001
  (shared evaluation core).
- QSpec FR-144 (supplied collection values), FR-149 (equality and keys),
  FR-181 (typed canonical form and canonical element encoding).
