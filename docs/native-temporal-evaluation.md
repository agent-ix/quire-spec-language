# Native temporal inputs and mapping classification

The bounded v1 evaluator that earlier versions of this document described is
retired. Its entry points, progress ledger, activation and capture machinery,
accounting counters and protocol-ledger driver are deleted from `src/temporal`;
their behavior is specified by
FR-325 to FR-329, FR-501, FR-502 and FR-522 to FR-526, and implemented in `qsl-eval`. The spec
requirements [FR-043](../spec/functional/FR-043-evaluate-bounded-native-temporal.md),
[FR-044](../spec/functional/FR-044-activate-temporal-obligations.md),
[FR-045](../spec/functional/FR-045-classify-temporal-mapping-support.md) and
[NFR-008](../spec/non-functional/NFR-008-bound-temporal-evaluation.md) keep
their text; the successor of each criterion is named in
[FR-525](../spec/functional/FR-525-map-each-legacy-temporal-evaluator-criterion-to-its-spine-successor.md).

What remains in `src/temporal` is the public surface that
[FR-521](../spec/functional/FR-521-gate-the-deletions-behind-quire-protocol-on-the-checked-package-and-name-each-behavior-it-relies-on.md)
row 5 keeps until `quire-protocol` and Contract IR repoint: `evaluate_v2` and its
`Report` (the native owner report over the checked package), `Trace` with its
`Position`, `Evidence` and `OrderKey` inputs, the `Closure`, `Completeness` and
`Execution` dimensions, `Classification` and `mapping_support_v2`.

## Trace inputs

A `Trace` is caller-constructed and carries its asserted profile identity and
revision, its clock binding name, the declared clock parameters the profile
requires as opaque retained premises, its admitted positions, its four
closure/completeness dimensions, its authoritative-origin flag and its progress
watermark.

Each position carries a clock coordinate in the profile's domain, an optional
admitted order key with its authority, and a map from `holds` leaf handle to
Boolean. A required leaf absent from a required position's map is a missing
valuation and produces an incomplete result; it is never read as false.

The trace's completeness assertion, admitted order and authoritative-origin claim
are trusted as supplied and retained as premises. They are not verified here, so
a later contradiction from agent F's contract can identify the results that
depended on them.

## Mapping support classification

The disposition `temporal::mapping_support_v2` returns is a total function of the
selected profile, the reachable operator kinds and the requested
surrounding-execution closure. The
owner ruling on `quire-contract-ir#64` fixes surrounding-execution closure as the
TL row selector; decision-scope closure stays a separate result axis and never
selects a row. It consults no backend capability report, installed TL version,
syntax match or historical result, and it emits no TL formula, valuation request
or correspondence record.

`temporal::mapping_support_v2` classifies a declaration through a strict `/2`
package. It resolves the selected definition through the admitted temporal
binding, refusing a missing binding or one that selects another definition, then
returns the disposition and retained native subject together with
`Classification::authenticated()`: the package identity and version, declaration,
definition identity and namespaced revision. Only that `AuthenticatedSelection`
is sealed: it is not constructible outside the crate, is read through accessors
and carries no clock parameters, because classification takes no trace. The
`support` and `retained` fields stay public plain data, so a consumer that needs
the disposition bound to the selection re-classifies from the named package.
QSL's temporal-artifact emission remains blocked on `quire-contract-ir#63`,
`quire-contract-ir#64` and actual TL capability.
