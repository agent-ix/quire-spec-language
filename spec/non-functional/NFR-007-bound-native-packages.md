---
id: NFR-007
title: "Bound native package content and reconstruction work"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: constrains
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: constrains
---
# NFR-007: Bound native package content and reconstruction work

## Statement

If the next package operation exceeds its selected ceiling, then the native package boundary shall stop with resource_exhausted before performing that operation.

## Scope

One NativePackage construction or raw read/rebind request. Package limits are
inclusive unsigned counts with defaults equal to hard ceilings; callers may
lower each independently and elevated options clamp. Parse/link/check retain
their existing independent limits and meanings. No budget becomes a domain,
population or backend-fuel bound.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Offered package bytes | At most 16777216 bytes before hashing or decoding | 16777216 bytes | negative-abuse-testing |
| Emitted package bytes | At most 16777216 bytes before each append | 16777216 bytes | negative-abuse-testing |
| Inspected package string content | At most 16777216 decoded UTF-8 bytes per pass | 16777216 bytes | negative-abuse-testing |
| Package entries | At most 100000 aggregate object members and array elements per pass | 100000 entries | negative-abuse-testing |
| JSON nesting | At most 128 entered containers per pass | 128 containers | negative-abuse-testing |

## Counter definitions

PackageUsage distinguishes admitted input bytes, output bytes, decoded string
content, entries and maximum container depth. An over-limit offered byte slice
is rejected before hashing/parsing and records zero admitted input bytes.
Decoded member names and string values count their UTF-8 length before retention;
output escaping is separately charged by emitted bytes. An object member or
array element costs one entry before storage/traversal, including duplicates.
The root container has depth one; scalars enter no container. Serde supplies
JSON grammar and container events, including strings containing delimiter text.

The string limit is checked before package-owned storage or cloning. Serde's
temporary string/number recognition is bounded by the admitted input bytes;
this contract does not assert that its lexical scratch obeys a smaller decoded
string budget before the complete token is recognized. Containers are checked
before entry. A library recursion guard cannot turn selected depth exhaustion
into invalid_package or silently lower the promised depth ceiling.

Recognition, typed decoding, manifest derivation, canonical encoding, artifact
encoding and comparison are separate bounded passes with separately reported
usage under the contract's PackageUsage records; their counters are not silently
reset into a single apparently smaller total. Byte comparison/hashing is bounded
by the corresponding admitted byte length. Frontend rechecking reports its own
limits and native error cause, rather than charging proof expansion as package
entries. The finite existing source/model/AST ceilings remain in force.
Recognition and typed decoding each traverse the original bytes. There is no
unmetered header probe or unrestricted raw-value shortcut around nesting limits.
For non-decoding passes, depth records the visited/emitted container nesting
and remains bounded by the same selected ceiling. Byte limits cover canonical
content as well as final artifact output; each pass begins with fresh counters.

All request-local state is discarded after success/refusal/exhaustion; a retry
cannot inherit feature sets, claims, caches or successful derived state. No
thread, async task, timing guarantee or cancellation callback is introduced by
this package API. Runtime cancellation remains in the existing runtime API.
The bounds describe content/work, not caller allocations or allocator capacity.

## Verification

Use generated closed records, long escaped/Unicode strings, ordered inventories
and malformed nested payloads. Test zero, exact, one-below, elevated hard options
and one-over input for each independent dimension. Delimiters inside strings
must not count as containers. When an upstream or other content ceiling stops
first, record that coupling and use a lowered isolated limit; do not claim an
unexecuted maximum success. Vary all frontend limits independently on a valid
read/rebind fixture and preserve native failure stages. Tests run serially using
the existing Rust cache, without timing sleeps or concurrency stress.

## Dependencies

- [FR-019](../functional/FR-019-package-checked-native-clauses.md).
- [FR-020](../functional/FR-020-read-and-rebind-native-packages.md).
- [FR-021](../functional/FR-021-derive-native-package-identity.md).
- [NFR-005](NFR-005-rust-verification-paths.md) retains Rust qualification.

## Status

Reviewed draft. Qualified Task-016 tests measure derive/canonical/encode limits,
including isolated hard-ceiling controls (c195950 / SR-111).
All reader/reconstruction passes remain pending; no runtime or backend budget
result is inferred from these package counters.
