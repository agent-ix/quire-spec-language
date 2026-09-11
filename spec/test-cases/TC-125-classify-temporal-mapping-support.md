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

Controls execute in `tests/composed_temporal_mapping.rs`. No TL evaluator, bridge
definition or contract-IR predicate projection participates; the classification is
a total function of the admitted declaration.

## Test Procedure

1. Compile one admitted declaration per support-table row: event-position with
   bounded future operators under a closed decision scope; the same under an open
   prefix; fixed-sample with bounded future operators; timestamped-event finite
   window; a bounded past operator under each of the three profiles; and a
   finite-window future request against an index-based target. Require each to
   return exactly that row's disposition. Inspect the classifier's inputs and
   require that no backend capability report, installed version, syntax match or
   historical result is consulted.
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
5. Require a supported classification to name its outstanding bridge premises —
   total Boolean predicate projection, source and clause identity, model, type and
   predicate bindings, evaluation anchor and capture environment, clock and
   observation binding, interval, closure and history premises, and the
   unencoded result dimensions — and to assert no correspondence. Compile two
   declarations whose temporal formula bytes are equal but whose selected profiles
   differ, and require their classifications to differ.

## Expected Results

Every disposition is derived from the admitted declaration alone. Unsupported
results name every unmatched dimension and retain the native subject, its profile
and its activation record; nothing is substituted or reduced.

The emission half of the bridge — the TL formula, valuation request and
correspondence record under
`ix://agent-ix/quire-specification/FR-095` — is not exercised here and remains
blocked on `quire-contract-ir#63`, `quire-contract-ir#64` and actual TL
capability, recorded as remaining work on compiler
[#38](https://github.com/agent-ix/quire-spec-language/issues/38). A supported
classification is a statement about the table, not evidence that any mapping was
performed.
