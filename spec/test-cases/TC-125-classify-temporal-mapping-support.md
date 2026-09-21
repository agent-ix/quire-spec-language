---
id: TC-125
title: "Classify native-to-TL temporal mapping support"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: verifies
---
# TC-125: Classify native-to-TL temporal mapping support

## Description

Public Rust API controls for
[FR-045](../functional/FR-045-classify-temporal-mapping-support.md). Each case
compiles a real native temporal declaration and asks the classifier for its
mapping disposition. Expected dispositions come from the reviewed correspondence
support table in `resources/native-v1/spec/functional/FR-095-preserve-native-tl-correspondence.md`,
never from an installed TL version, a backend report or the classifier's own
output. Each numbered group corresponds to the matching acceptance criterion.

Groups 1–5 execute in `tests/composed_temporal_mapping.rs` and group 6 in
`tests/composed_temporal_mapping_v2.rs`. No TL evaluator, bridge definition or
contract-IR predicate projection participates; the classification is a total
function of the admitted declaration and the requested surrounding-execution
closure, which the owner ruling on `quire-contract-ir#64` fixes as the TL row
selector.

## Test Procedure

1. Compile one admitted declaration per support-table row: timestamped-event
   finite window; a bounded past operator under each of the three profiles;
   event-position with bounded future operators under a complete
   surrounding execution and again under an open one; fixed-sample with bounded
   future operators under each closure; and a declaration reaching no bounded
   temporal operator under each false-extension profile. Require each to return
   exactly that row's disposition and, where supported, its TL target identity —
   `mltl.closed-trace/v1` or `mltl.online-prefix/v1`. Inspect the classifier's
   inputs and require that no backend capability report, installed version,
   syntax match, historical result or decision-scope closure is consulted.
2. Require the timestamped-event request to return unsupported naming the
   finite-window dimension, and the bounded-past request to return unsupported
   naming the past-operator dimension. Require neither result to carry a
   substitute formula, profile or clock, and require no index conversion to appear
   for the timestamped request.
3. Inspect each unsupported result and require it to retain the native declaration
   subject, the selected profile identity and revision, and the activation record.
   Re-read the source declaration after classification and require it unchanged.
4. Compile a declaration unmatched on two dimensions at once — a bounded past
   operator under the timestamped-event profile — and require both the
   past-operator and finite-window dimensions named, not one summary cause.
5. Require a supported classification to name its TL target identity, the source
   table's baseline revision `782c1ce39a197cd52b8b35b50adf2e5e3ecedd0f`, and its
   outstanding bridge premises —
   total Boolean predicate projection, source and clause identity, model, type and
   predicate bindings, evaluation anchor and capture environment, clock and
   observation binding, interval, closure and history premises, and the
   unencoded result dimensions — and to assert no correspondence. Compile two
   declarations whose temporal formula bytes are equal but whose selected profiles
   differ, and require their classifications to differ.
6. Compile one declaration per profile, each reaching a bounded future operator,
   and emit it through both the `/1` producer and the `/2` producer. Admit the
   `/2` bytes with a strict reader given a temporal selection re-derived from the
   authored sources and registered definition catalog, not projected from the
   producer's table. Under each surrounding-execution closure, require the `/2`
   classification to equal the `/1` classification's disposition and retained
   native subject, and to retain the admitted package digest, the declaration
   index, the registered definition identity and revision, and the digest of the
   independently selected definition artifact, with the package digest computed
   from the emitted bytes and the revision compared with its namespace. Require
   the `/1` classification to retain no authenticated selection, and require a
   `/2` request for a declaration just past the package, for the largest index a
   caller can name and for the non-temporal protocol declaration to return its
   exact located refusal. Inspect the retained selection type and require it to
   have no public constructor (a compile-fail doctest) and no clock parameter
   map, and inspect the classifier's refusal of a temporal declaration with no
   binding, of an index no binding can represent, and of a binding that selects
   another definition, all of which the strict reader and the declaration lookup
   make unreachable. Group 6 exercises the
   bounded-future rows under each profile; equivalence for the past-operator and
   no-operator rows follows by inspection, because `/1` and `/2` share one
   selection and classification path and differ only in the retained selection.

## Expected Results

Every disposition is derived from the admitted declaration alone. Unsupported
results name every unmatched dimension and retain the native subject, its profile
and its activation record; nothing is substituted or reduced. A `/2`
classification retains the definition selection it authenticated, and a `/1`
classification is never mistakable for one.

The emission half of the bridge — the TL formula, valuation request and
correspondence record under
`ix://agent-ix/quire-specification/FR-095` — is not exercised here and remains
blocked on `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL
capability, recorded as remaining work on compiler
[#38](https://github.com/agent-ix/quire-spec-language/issues/38). A supported
classification is a statement about the table, not evidence that any mapping was
performed.
