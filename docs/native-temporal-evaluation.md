# Native temporal evaluation and accounting

Draft L5 contract under [FR-043](../spec/functional/FR-043-evaluate-bounded-native-temporal.md),
[FR-044](../spec/functional/FR-044-activate-temporal-obligations.md),
[FR-045](../spec/functional/FR-045-classify-temporal-mapping-support.md) and
[NFR-008](../spec/non-functional/NFR-008-bound-temporal-evaluation.md). The
selected accounting label is `quire.native.temporal-work/1`. Profile meaning,
evaluation accounting and any future backend fuel are separate.

Native temporal *meaning* is owned by agent E through
`quire-specification` FR-090, FR-091, FR-092, FR-093 and FR-094. This contract
describes the compiler's evaluation of the artifact it emits. Where the two
diverge, the shared rule governs and this implementation is defective.

## API and ownership

`temporal::evaluate` takes a borrowed `protocol_artifact::AdmittedPackage`, a
declaration index, a caller-constructed `Trace` and `temporal::Limits`. There is
no public entry point accepting a freely constructed `wire::Package`: admission
through the reader is the constructor-private evidence that the declaration was
compiled, not asserted.

`temporal::evaluate_with_progress` takes the same inputs plus a caller-held
`temporal::Ledger`, which retains the watermark, completeness assertion and
decision-scope closure each `temporal::Binding` — one declaration, one clock
binding name and one asserted profile identity — has already asserted. A caller
evaluating one declaration repeatedly as observation arrives carries one ledger,
so a contradicting assertion is refused rather than absorbed; progress recorded
under any other binding settles nothing. `temporal::evaluate` consults no ledger
and stands on the trace's own assertions alone.

The evaluator returns a `Report` carrying either the per-instance `Assessment`
list or one bounded stop. It does not evaluate composed value expressions, does
not read observations from any transport, and emits no protocol result. Agent F
supplies the trace; agent B owns protocol activation and result serialization.

## Orthogonal result dimensions

Truth is `True`, `False` or `Pending`. Settlement basis is `ClosedScope`,
`DecisiveWitness`, `DecisiveCounterexample`, `Unsettled` or `Unavailable`.
Activation is `Inactive`, `Unknown { completeness, execution }` or
`Active { instance }`. Decision-scope closure, surrounding-execution closure,
assessment execution and input completeness are four independent premise fields.

Closing or completing one dimension never closes or completes another. No stop —
exhaustion, overflow, eviction, contradiction, missing input or unsupported
mapping — is rendered as a Boolean. `Unavailable` means a fact inside the
completed decision-support set is missing; it is neither `False` nor `Pending`.

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

The emitted temporal body carries a clock binding name and the selected
definition entry, but no declared sample period, epoch, timestamp unit or
sequence-authority value. Those therefore participate in result identity as trace
premises and are not independently checked against the artifact. Checking them
would require an FR-042 wire extension, recorded as remaining work on
[#38](https://github.com/agent-ix/quire-spec-language/issues/38).

## Counters

`Limits`, `Usage` and `Dimension` carry eight counters. A refused charge never
increases usage, and the first unaffordable operation is not performed. Ceilings
above the defaults are clamped; zero is preserved; the effective clamped ceilings
participate in result identity.

| Counter | Kind | Default ceiling | Unit |
| --- | --- | --- | --- |
| `positions` | cumulative | 1,000,000 | admitted trace positions inspected |
| `valuations` | cumulative | 1,000,000 | atomic valuation lookups |
| `instances` | peak | 10,000 | obligation instances active at one time |
| `captures` | cumulative | 100,000 | retained capture records |
| `retention` | peak | 1,000,000 | retained valuation records required at one time |
| `visits` | cumulative | 1,000,000 | temporal graph node visits |
| `depth` | peak | 64 | temporal formula nesting levels |
| `horizon` | peak | `i64::MAX` | greatest admitted interval bound |

A position inspected under two operators charges `positions` twice. A valuation
lookup that finds the leaf absent still charges `valuations`. A shared temporal
node visited under two parents charges `visits` twice. Grouping nodes are
traversed iteratively and do not grow `depth`. A capture record charges `captures`
once when retained, not once per later read.

Interval composition, horizon computation and history need use checked `i64`
arithmetic. Overflow refuses before evaluation begins; it never wraps, saturates
or narrows. `horizon` records the greatest bound admitted after that checked
composition, so a rejected composition contributes nothing.

## Closed-boundary rules

Under `quire.temporal.event-position.false-extension/v1` and
`quire.temporal.fixed-sample.false-extension/v1`, while the decision scope is
closed and its input is complete, an atomic valuation is false at integer offsets
outside the scope and a temporal constant keeps its authored value. `not`, `and`,
`or` and `implies` are evaluated pointwise at every offset the enclosing operator
ranges over, including those offsets — so `always[0,1] not holds(p)` is false and
`eventually[0,1] not holds(p)` is true on a one-position closed trace with `p`
true.

Under `quire.temporal.timestamped-event.finite-window/v1`, quantification ranges
only over admitted instants inside the inclusive window and no synthetic atom is
added after closure. An instant exactly at the upper endpoint participates before
settlement. The polarity a later admitted instant could still overturn —
empty-existential or observed-existential `false`, and universal `true` — holds
only where the trace asserts completeness through the inclusive upper endpoint
and progress has reached it; otherwise the window is `Pending` with `Unsettled`.

An incomplete input applies neither rule, even where the scope is labelled closed.

A past operator requires history through its computed lower boundary, or an
authoritative execution origin at the trace's lower boundary. A bare history
cutoff produces missing-history incomplete.

## Settlement and progress

A closed, complete decision scope settles with `ClosedScope`. An open scope
settles only with `DecisiveWitness` or `DecisiveCounterexample`, and only where
every admitted continuation preserves the Boolean over complete exact decision
support; every other open future is `Pending` with `Unsettled`.

A deadline at offset `D` settles from watermark `W` only while `W >= D` and
completeness covers every required valuation through `D`. Fixed-sample and
timestamped progress advance through `D` with no business event. An
event-position clock does not advance during silence.

A regressing watermark or a conflicting completeness revision under one binding
returns a typed contradiction refusal. Progress is not rolled back, closure is not
restamped and earlier results are not rewritten.

## Activation and captures

An event-triggered instance is keyed by the clause subject and the semantic
trigger identity; a whole-execution-origin instance by the clause subject and the
admitted execution-origin identity. Receipt identity, timestamp, display name and
equal payload never key an instance. A repeated delivery retains its receipt
provenance on the existing instance; conflicting payloads under one trigger
identity refuse.

An activation guard is evaluated once at the anchor. A false guard creates no
instance and is not a refusal. A guard that cannot be established reports
incomplete or refused activation naming the guard.

Captures are evaluated in authored source order, exactly once per instance, at the
activation anchor. They are not re-evaluated during incremental re-evaluation,
restoration or replay; an evicted retained capture makes its instance incomplete
rather than being recomputed at a later anchor. The public API exposes no mutable
path to a retained capture record.

## Mapping support classification

`temporal::mapping_support` is a total function of the selected profile, the
reachable operator kinds and the requested decision-scope closure. It consults no
backend capability report, installed TL version, syntax match or historical
result, and it emits no TL formula, valuation request or correspondence record.
Its table restates `quire-specification` FR-095's reviewed disposition. The
emission half of that requirement remains blocked on `quire-contract-ir#63`,
`quire-contract-ir#64` and actual TL capability.
