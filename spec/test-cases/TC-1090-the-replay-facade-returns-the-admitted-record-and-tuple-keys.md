---
id: TC-1090
title: "The replay facade returns the admitted record and tuple keys"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-396
    type: verifies
---
# TC-1090: The replay facade returns the admitted record and tuple keys

## Description

Verify `qsl_replay::compile_admitted`'s context lists the admitted record and
tuple keys, finds them by wire node id, and refuses as typed. Scope:
FR-396-AC-1 to FR-396-AC-5.

## Test Procedure

1. From an integration test that names only the `qsl_replay` root, the
   kernel and the QSV declaration API, compile a unit declaring a tuple and a
   record with a field of that tuple's type; look both up; build a
   `TypeEnvironment`, a tuple value and a record value from the keys.
2. Compile a unit with two same-shaped records and two same-shaped tuples
   and a function over one record; compare the table with the spine's
   checked `TypeEnvironment` and `composite_type` semantic nodes and the
   function's declared parameter type; look each up twice; recompile.
3. Look up an id no package admitted, the unit's enum, union and function,
   and a record of a library imported directly and through an intermediate
   library.
4. Rerun TC-1088's enum lookups and compile refusals over the extended
   context.
5. Replay a predicate over a record holding a tuple from a witness built
   from the admitted keys, then from one built from the node ids.

## Expected Results

1. Both are found as `Tuple` and `Record`; the environment admits, and it
   admits the record value at the record's `ValueType::Composite`.
2. Equal tables; the parameter is `ValueType::Composite` of the admitted
   key, not the node id; four distinct keys; the same declaration each time.
3. `Unknown`; `WrongKind` three times; `Foreign` naming the library's
   `package_id` both times, with every own declaration still found.
4. TC-1088's expected results, unchanged.
5. Reproduced on both arms from the keys; refused on both from the node ids.
