---
id: TC-447
title: "Duplicate backend identity registration matches quire-specification TC-282 under every order"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-290
    type: verifies
---
# TC-447: Duplicate backend identity registration matches quire-specification TC-282 under every order

## Description

Mirror quire-specification's TC-282 vectors DB-01 through DB-08 (FR-290-AC-9,
FR-290-AC-10) against `qsl-route`'s real `Registry`, run under every ordering
of each vector's registrations, checking that the registry snapshot, its
registration refusals, and every named candidate set agree with TC-282's
expected result regardless of order. Scope: FR-075-AC-4, FR-075-AC-7.

## Test Procedure

Build fixture `BackendDescriptor` values matching TC-282's own: `A1` and `A2`
for identity `a`, two manifests with distinct digests `d1` and `d2`, both
advertising (`value-validity`, `bounded`); `A1'`, identity `a` and digest
`d1`, advertising (`value-validity`, `unbounded`) instead; `B`, identity `b`,
advertising (`value-validity`, `bounded`); and `M`, a registration for
identity `a` with digest `d2` advertising the invalid mode `finite` (which
`BackendDescriptor::admit` refuses before it becomes a descriptor, so it
cannot itself enter a `Registry` permutation pool).

For each of DB-01 through DB-06 and DB-08, build a registry for every
ordering of the vector's registrations. For each snapshot, record its held
registrations, its registration refusals (via `Registry::refusals`) in
reported order, and the candidate set of a `value-validity` item that names
no backend, one that names `a`, and (where relevant) one that names `b`.
Compare every ordering's record with the first ordering's record and with
quire-specification TC-282's expected result. DB-07 is exercised with `M`
attempted before and after `A1`'s registration, since `M` cannot appear in a
permuted pool.

## Vectors

Same as quire-specification TC-282:

| Vector | Registrations | Expected result |
| --- | --- | --- |
| DB-01 | `A1`, `A1` | `a` held once with `d1`; no refusal; the unnamed item's candidates are exactly (`a`, `d1`) |
| DB-02 | `A1`, `A1`, `A1`, `B` | As DB-01, with `b` also held; the unnamed item's candidates are `a` then `b` |
| DB-03 | `A1`, `A2` | Two refusals, `invalid_capability`/`duplicate-backend` keyed (`a`, `d1`) then (`a`, `d2`); `a` is not held; the unnamed item's candidates are empty; the item naming `a` receives the unknown-backend mark |
| DB-04 | `A1`, `A2`, `B` | As DB-03 for `a`; `b` is held; the unnamed item's candidates are exactly `b` |
| DB-05 | `A1`, `A2`, `A1` | As DB-03; the second `A1` is refused with the first, and no third refusal key is reported |
| DB-06 | `A1`, `A1'` | One refusal keyed (`a`, `d1`); `a` is not held; the item naming `a` receives the unknown-backend mark |
| DB-07 | `A1`, `M` | `M` refuses at admission as `invalid_capability`/`unknown-mode`; it never reaches the registry, so `a` is held once with `d1` and no `duplicate-backend` is reported |
| DB-08 | Register `A1` and read the snapshot; register `A2` and read the snapshot again; repeat with `A2` first | The first snapshot holds `a` with the first registration's digest; the second equals DB-03's snapshot, with the first registration withdrawn and its refusal reported |

## Expected Results

Every ordering of a vector's registrations gives the same registry, the same
refusals in the same reported order, and the same candidate set for every
item. A repeat of an identical manifest is never refused (FR-075-AC-7). A
conflicting identity is never held, contributes no candidate, and never
refuses a registration of another identity (FR-075-AC-4). Each
`duplicate-backend` refusal retains the repeated backend identity and its
own manifest digest.
