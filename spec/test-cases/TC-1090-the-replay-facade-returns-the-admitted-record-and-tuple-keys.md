---
id: TC-1090
title: "The replay facade returns the admitted record, tuple and union keys"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-396
    type: verifies
---
# TC-1090: The replay facade returns the admitted record, tuple and union keys

## Description

Verify `qsl_replay::compile_admitted`'s context lists the admitted record, tuple
and union keys and a union's member ids, finds them by wire node id, and
refuses as typed. Scope: FR-396-AC-1 to FR-396-AC-9.

## Test Procedure

1. From an integration test that names only the `qsl_replay` root, the
   kernel and the QSV declaration API, compile a unit declaring a tuple and a
   record with a field of that tuple's type; look both up; build a
   `TypeEnvironment`, a tuple value and a record value from the keys.
2. Compile a unit with two same-shaped records, tuples and unions
   and a function over one record; compare the table with the spine's
   checked `TypeEnvironment` and `composite_type` semantic nodes and the
   function's declared parameter type; look each up twice; recompile.
3. Look up an id no package admitted, the unit's enum and function, and a
   record and a union of a library imported directly and through an
   intermediate library.
4. Rerun TC-1088's enum lookups and compile refusals over the extended
   context.
5. Replay a predicate over a record holding a tuple from a witness built
   from the admitted keys, then from one built from the node ids.
6. Compile a unit with two same-shaped unions, a union with a record payload
   and a union with a union payload; compare each union's members with the
   producer's minted `VariantId`s, the sealed `TypeEnvironment` bindings and
   literal payload types (FR-396-AC-6, AC-8).
7. From the root-only consumer, declare a union from its admitted key and
   payloads, seal it with the admitted bindings, and construct and admit a
   value of every member, including a member holding another union
   (FR-396-AC-7).
8. Replay a union witness naming the node id, then the kernel key
   (FR-396-AC-9).

## Expected Results

1. Both are found as `Tuple` and `Record`; the environment admits, and it
   admits the record value at the record's `ValueType::Composite`.
2. Equal tables; the parameter is `ValueType::Composite` of the admitted
   key, not the node id; distinct keys; the same declaration each time.
3. `Unknown`; `WrongKind` twice; `Foreign` naming the library's
   `package_id` for the record and the union, both times, with every own
   declaration still found, unions included.
4. TC-1088's expected results, unchanged.
5. Reproduced on both arms from the keys; refused on both from the node ids.
6. Members equal the producer's in declared order; the payload positions are
   the literal types, a union position `ValueType::Composite` of that union's
   key; the same-shaped unions have distinct keys and no shared member id; a
   recompile returns the same.
7. The sealed environment constructs and admits a value of each member and of
   the union holding a union; a member id of another union is refused.
8. The node-id witness reproduces on both arms and the kernel-key witness
   refuses.
