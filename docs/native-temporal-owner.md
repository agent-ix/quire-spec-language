# Native temporal request/result owner

`protocol_artifact::native_temporal` is the FR-052 formula-wide owner boundary.
It wraps the existing FR-043/FR-044 evaluator without introducing another
parser, evaluator, truth input, callback, plug-in, network path, trust flag, or
Boolean coercion.

The immutable contracts are:

- `quire.native-temporal-request/v1`, schema SHA-256
  `2539140ff1f6fb5e481e5ae658b81c325a284bfcd85cc5c048ea1d49c4dfdebe`;
- `quire.native-temporal-result/v1`, schema SHA-256
  `e55e15cc852f0145244d233ca5c88381e0969daf25f644e45a4da3362b611e28`.

Each module exports `CONTRACT`, `SCHEMA_BYTES`, and `SCHEMA_SHA256`. Schema
validation alone grants no authority. A consumer must use the bounded Rust
readers with the exact constructor-private predecessor:

```text
request::produce(&ValidatedTemporalSubject, Input, Limits) -> Report<Document>
request::read(bytes, &ValidatedTemporalSubject, Limits) -> Report<ValidatedRequest>
result::evaluate(&ValidatedRequest, Relation, Limits) -> Report<Document>
result::read(bytes, &ValidatedRequest, Relation, Limits) -> Report<ValidatedResult>
```

The request producer derives its subject, profile, revision, clock name, clock
configuration, reachable formula and checked leaves from an FR-051 strict-read
temporal subject. Its `Input` supplies only external evidence references,
positions and explicit leaf valuations, activation/capture inputs, progress,
closure, completeness, execution, origin, and eviction facts. Each external
reference remains opaque owner evidence. QSL checks its closed shape, digest
spellings, scope allocation, uniqueness and population binding but does not
claim to authenticate Quire Observation bytes.

Positions are canonicalized by coordinate and one admitted order authority.
Their QSL identities bind the observation reference, coordinate, order and
complete checked-leaf valuation. Insertion order is not semantic. Missing,
duplicate, foreign or cross-wired members refuse rather than becoming false.
Version 1 supports event-position and exact fixed-sample clocks; timestamped
and dense clocks remain explicitly unsupported at this bridge boundary.

Only `temporal::evaluate_v2` supplies formula truth, settlement, activation and
decision support. The result producer has no truth, settlement or support
parameter. Refused, incomplete, inactive, unknown and unactivated outcomes are
typed non-values with no Boolean fallback. The strict result reader repeats the
evaluation under the exact request, relation and effective limits, then requires
canonical byte equality.

`Relation::Original` starts revision 1. `Superseding` and `Invalidating` accept a
complete `ValidatedResult`, require a changed request with the same subject,
instance and correspondence, increment the predecessor revision, and retain its
exact semantic identity and raw digest. The predecessor bytes remain immutable.

All wire counters use checked fixed-width conversions. Caller limits can lower
but cannot raise owner maxima and independently cover bytes, JSON depth, strings,
formula nodes/depth, positions, valuations, captures, support, history span,
evaluation work, lineage and total visited work. Resource exhaustion and
allocation failure return typed errors without a partial request, result or
Boolean. Run the traced owner suite with:

```sh
cargo test --locked --target-dir target -j 1 --no-default-features \
  --test it native_temporal_owner:: -- --test-threads=1
```
