---
id: TC-1092
title: "The replay facade returns the admitted unit table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-398
    type: verifies
---
# TC-1092: The replay facade returns the admitted unit table

## Description

Verify `qsl_replay::compile_admitted`'s context lists the admitted dimensions
and declared units, finds them by wire node id, returns each unit's kernel id
and conversion data, and refuses as typed. Scope: FR-398-AC-1 to FR-398-AC-3.

## Test Procedure

1. From an integration test that names only the `qsl_replay` root and the
   kernel, compile a unit declaring `dimension Length;`, `unit m` and
   `unit km : Length = rational(1000, 1) * m`; look both units up; build a type environment over
   `unit_table`, a quantity of `m`, and convert it to `km` exactly.
2. Compile a unit with two dimensions, three units and a function over a
   `Quantity<u>` of each; compare the tables with the spine's checked
   semantic unit and dimension nodes and the function's declared parameter
   types; check `converts_to`; look each unit up twice; recompile.
3. Look up an id no package admitted, the unit's dimension, enum and record as
   units, a unit as a dimension and as a record, and a library's unit and
   dimension imported directly and through an intermediate library.
4. Declare a unit the package never uses and a dimension no unit names; look
   the unit up as a unit, a record, a dimension and an enum.
5. Declare a derived dimension, a scaled chain and an offset unit; list the
   dimension's parts; convert 2 km to mm and 0 C to K.
6. Replay a `Quantity<m>` parameter with a witness naming the admitted id of
   `m`, then of `km`.

## Expected Results

1. Both are found; the environment admits the `m` quantity at
   `Quantity<m>` and not at `Quantity<km>`; the conversion completes with the
   exact value 5/1000 in `km`.
2. Equal tables; each parameter is `ValueType::Quantity` of the admitted id,
   in the declared-unit domain over the node id's bytes; three distinct ids; `m` and `km` convert to each other,
   neither to `s`; the same entries each time.
3. `Unknown`; `WrongKind` for each; `Foreign` naming the library's
   `package_id` both times, with every own unit still found.
4. `unit_declaration` finds the unused unit; the record and dimension lookups
   refuse it `WrongKind`, the enum lookup `Unknown`; the dimension is not
   listed.
5. The derived dimension lists its parts and exponents; 2 km is 2,000,000 mm
   and 0 C is 5463/20 K.
6. The `m` witness reproduces on both arms; the `km` witness refuses.
