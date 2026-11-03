---
id: FR-497
title: "Check and emit carried quantities and their conversion"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-140
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: depends_on
---
# FR-497: Check and emit carried quantities and their conversion

## Description

QSpec FR-142 "Carried quantity" defines `Quantity<u, C>`, a quantity whose
magnitude is a value of a bounded numeric carrier `C`, and the conversion that
produces one. QSL has no requirement that reads the form, lowers it, or
implements the conversion's inexactness refusal, so QSpec rows TC-187 U07, U08,
U16, U17, U18 and U29 (IR-669, QSL-1389) cannot be reached from source. This
requirement owns that QSL share. QSpec owns the semantics; this requirement
states only how `check`, lowering and the kernel realize them, and invents none.

## Inputs

- A `1-draft` or complete-V1 source that writes `Quantity<u, C>` as a type, or
  `convert<Quantity<u, C>>(e)` with `e` of type `Quantity<u'>` or
  `Quantity<u', C'>`.
- QSpec FR-142-AC-27 to AC-31, FR-322-AC-301, AC-302 and AC-382, with the
  vectors `quantity-carrier-vectors.json` `CQ-W01` to `CQ-W19` and `CQ-E01` to
  `CQ-E26`.

## Outputs

- A checked type, a checked package whose graph holds the anonymous
  `scalar_type`/`carried_quantity` node and one `quire.op.quantity.convert`
  application per conversion, and the kernel outcomes of the conversion.

## Behavior

- **Source form.** The spelling is exactly the QSpec type form:
  `Quantity<m, Int[-2, 2]>`, `Quantity<m, Int[-2, 2; nearest-even]>`,
  `Quantity<m, Decimal[0, 1; 0, 4; nearest-even]>` and
  `Quantity<m, Rational[-3, 7; 1, 12]>`. The unit `u` resolves as for
  `Quantity<u>` (FR-094). The carrier `C` is a bounded `Int[..]`,
  `Decimal[..]` or `Rational[..]` type, an `Int[..]` carrier taking an
  optional trailing `; mode`; a `Rational[..]` carrier names no mode and its
  mode is `exact`. `check` SHALL refuse a bare `Integer`, a bare `Decimal`, a
  bare `Rational`, a `Text[..]` or a record carrier `ill_typed`/`type-mismatch`
  at `C`, a carrier with `lo > hi` or `smin > smax` as FR-140 refuses its
  `Decimal[..]`, and a `Rational[..]` carrier with `n1 > n2` or outside
  `1 <= d1 <= d2` `ill_typed` at `C` as QSpec FR-149 "Bounded rationals"
  refuses a malformed `Rational[..]` (QSpec FR-142). The members of a
  unit-bearing FR-505 Rational value type have a `Rational[..]` carried type.
- **Expression grammar.** The QSL source grammar does not define exponentiation.
  The source parser SHALL reject an expression written with `**`, such as
  `a ** 2`, before semantic checking, regardless of operand type; this syntax
  rejection does not produce an `ill_typed` type-mismatch refusal.
- **Type identity.** `Quantity<u, C>` is a distinct checked type from
  `Quantity<u>` and from every carried type that differs in unit, carrier
  bound, scale range or mode. `check` SHALL NOT convert one to another
  implicitly. A carried quantity as an operand of quantity addition,
  subtraction, negation, multiplication, division, equality or ordering, or
  where `Quantity<u>` or another carried type is required, refuses
  `ill_typed`/`type-mismatch` at the expression before any charge. A carried
  quantity is not an element of `collection.sum.quantity`, which is defined over
  `Quantity<u>` only; it is converted back
  with `convert<Quantity<u''>>(e)` first (QSpec FR-142-AC-27, AC-30).
- **Conversion.** `convert<Quantity<u, C>>(e)` is built as a conversion node
  whose source `e` has a `Quantity<u'>` or carried type with `u'` of the same
  dimension as `u` (FR-093's dimension-equivalence rule applies unchanged); a
  different dimension refuses `ill_typed` for incompatible dimensions (QSpec
  FR-142-AC-2). It SHALL be the only way a carried quantity is produced.
- **Lowering.** `Quantity<u, C>` lowers to the anonymous
  `scalar_type`/`carried_quantity` node of QSpec FR-322 "Derived type forms and
  forwarding", with body bindings `unit` (the unit node of `u`), `carrier` (the
  carrier's `integer_range`, `decimal_range` or `rational_range` node) and
  `rounding` (the carrier's mode, `exact` when an `Int` carrier omits it and
  always `exact` for a `Rational` carrier). Carried types equal
  in unit, carrier and rounding lower to one node; the `Int[-2, 2]` exact and
  `nearest-even` forms are two nodes. The conversion lowers to
  `quire.op.quantity.convert` whose `result_type` is that node and whose
  `mode` is `{kind: rounding, value: <the node's rounding>}`, never `null` and
  never another mode (FR-322-AC-302). The FR-093 `ConvertScalar` row for a
  quantity operand therefore takes the carried node's rounding as its mode when
  the target is a carried quantity, and `exact` when the target is `Quantity<u>`.
- **Kernel evaluation.** The conversion SHALL evaluate as QSpec FR-142-AC-28
  to AC-30 state, in this order of observation: exact conversion of the
  operand to the target unit; rounding under the carrier's mode against the
  carrier grid `10^-smax` (scale zero for an `Int` carrier) with a
  `DecimalLoss` record; under mode `exact` a value not on the grid refuses
  `inexact_conversion`/`inexact-conversion` at the conversion node, before
  `unit.target-domain`, with no value and `expected` the carrier type; then
  the `unit.target-domain` charge and the bounds check, an on-grid value
  outside the bounds refusing `decimal_out_of_domain` (a `Decimal` carrier) or
  `integer_out_of_domain` (an `Int` carrier) with `outside-domain`, where the
  charge is denied returning `incomplete` at `unit.target-domain`, and then
  `unit.result-retain`. An inexact value that is also out of bounds refuses
  `inexact_conversion` first, and the refusal stays a refusal, not
  `incomplete`, under a work limit that would deny later charges (U16).
  Rounding precedes the bounds check under a non-`exact` mode (`CQ-E17` to
  `CQ-E19`).
- **Rational carrier evaluation.** A `Rational[n1, n2; d1, d2]` carrier has
  no grid. A conversion into `Quantity<u, Rational[..]>` SHALL compose the
  exact value in the target unit and reduce it to lowest terms with a
  positive denominator; it never rounds and never refuses
  `inexact_conversion`. After the `unit.target-domain` charge a reduced value
  whose numerator is outside `[n1, n2]` or whose denominator is outside
  `[d1, d2]` refuses `rational_out_of_domain`/`outside-domain` at the
  conversion node, before `unit.result-retain`, retaining the carrier as
  `expected` and no value (the existing FR-100 `RationalOutOfDomain` record);
  a denied `unit.target-domain` charge returns `incomplete` there (QSpec
  FR-142-AC-31; `CQ-E21` to `CQ-E26`).
- **New refusal record.** The kernel refusal `InexactConversion` joins FR-096's
  key table and FR-100's kernel-record table as code `inexact_conversion`,
  cause `inexact-conversion`, field `expected`: the target carrier type
  (`Decimal[lo, hi; smin, smax]` or `Int[lo, hi]`), and exits 20 like the
  other value refusals.
- **No ambient conversion.** There is no implicit conversion between a carried
  quantity and its carrier, a bare `Quantity<u>`, or a numeric type; every such
  use refuses as "Type identity" states.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-497-AC-1 | With `dimension Length;` and `unit m : Length = rational(1, 1);`, `Quantity<m, Decimal[-100, 100; 2, 2; exact]>`, `Quantity<m, Int[-2, 2]>`, `Quantity<m, Int[-2, 2; nearest-even]>` and `Quantity<m, Rational[-3, 7; 1, 12]>` each check as a type; the four lower to four distinct `scalar_type`/`carried_quantity` nodes with `unit`, `carrier` and `rounding` bindings (`exact` for the omitted `Int` mode and for the `Rational` carrier, whose `carrier` is the `rational_range` node), and two uses of one form share one node (QSpec `CQ-W01` to `CQ-W04`, `CQ-W16`). `Quantity<m, Rational[-3, 7; 1, 12]>` is distinct from `Quantity<m>`, from `Quantity<m, Rational[-3, 7; 1, 13]>` and from `Quantity<cm, Rational[-3, 7; 1, 12]>`. A bare `Integer`, a bare `Decimal`, a bare `Rational`, `Text[0, 8; nfc]`, a record carrier, `Int[2, -2]`, `Rational[8, 7; 1, 12]` and `Rational[0, 7; 0, 12]` refuse as AC-2 states. | Test (TC-1287) |
| FR-497-AC-2 | A carrier that is not a bounded `Int[..]`, `Decimal[..]` or `Rational[..]` (a bare `Integer`, `Decimal` or `Rational`, a `Text[..]`, a record) refuses `ill_typed`/`type-mismatch` at the carrier; a carrier with `lo > hi` or `smin > smax` refuses as FR-140 refuses its `Decimal[..]`, and a `Rational[..]` carrier with `n1 > n2` or outside `1 <= d1 <= d2` refuses `ill_typed` at the carrier (QSpec FR-149); a `Quantity<m, Rational[-3, 7; 1, 12]>` operand of `=` or `<` refuses as every carried quantity does; a carried quantity where `Quantity<u>` or a different carried type is required, and as an operand of `+`, `-`, unary `-`, `*`, `/`, `=` and `<`, refuses `ill_typed`/`type-mismatch` at the expression with no charge; `a ** 2` is rejected by the source parser before semantic checking, not as a carried-quantity type mismatch; `convert<Quantity<m'>>` of a carried quantity of the same dimension checks, and of another dimension refuses `ill_typed` for incompatible dimensions (TC-187 U41; `CQ-W13` for the dimension refusal, `CQ-W15` for the carried-to-carried conversion). | Test |
| FR-497-AC-3 | `convert<Quantity<m, Decimal[-100, 100; 2, 2; exact]>>(1 in)` lowers to one `quire.op.quantity.convert` application whose `result_type` is the carried node and whose `mode` is `{kind: rounding, value: exact}`; the same conversion into a carrier whose mode is `nearest-even` carries `nearest-even`; a conversion into `Quantity<m, Rational[-3, 7; 1, 12]>` carries `{kind: rounding, value: exact}`; no lowering carries a `mode` other than the result node's `rounding` or `null` (`CQ-W09`, `CQ-W10`, `CQ-W17` to `CQ-W19`). | Test (TC-1287) |
| FR-497-AC-4 | Under `CheckMode::Kernel`, `1 in` into `Decimal[-100, 100; 2, 2]` (exact) refuses `inexact_conversion`/`inexact-conversion` at the convert node with `expected` that carrier and no value, before `unit.target-domain`, and `25 cm` converts to `(25, 2) m`; `1 in` into `Int[-2, 2]` refuses likewise and `2 m` converts; the same inexact refusal holds after one read, one edge and two rational events under work limit six and under work limit four, where it is a refusal and not `incomplete` (U07, U16, U29; `CQ-E01`, `CQ-E02`, `CQ-E07`, `CQ-E08`, `CQ-E12`, `CQ-E13`, and `CQ-E20` for a carrier with `smin` below `smax`). | Test (TC-1287) |
| FR-497-AC-5 | `-2 m` and `2 m` into `Decimal[-2, 2; 0, 0]` succeed and `-3 m` and `3 m` refuse `decimal_out_of_domain`/`outside-domain` after `unit.target-domain` and before `unit.result-retain`; `3 m` into `Decimal[-2, 2; 0, 0]` refuses by membership under work limit two and is `incomplete` at `unit.target-domain` under work limit one; `1 m` into `Decimal[0, 1; 0, 4294967295]` is `incomplete` at `unit.target-domain` for `decimal_digits` with the retained coefficient never materialized; `3 m` into `Int[-2, 2]` refuses `integer_out_of_domain`; an inexact value that is also out of bounds refuses `inexact_conversion` (U08, U17, U18, U29; QSpec FR-142-AC-29). | Test (TC-1287) |
| FR-497-AC-6 | A non-`exact` carrier mode rounds under FR-140 with a `DecimalLoss` record before the bounds check and a conversion never rounds under a mode its target type does not name; the `nearest-even` outcomes of U07, U16 and U29 are unchanged (U44; `CQ-E17` to `CQ-E19`). A record or collection holding a carried quantity compares it by exact value in its identical type, and `collection.sum.quantity` is defined over `Quantity<u>` only, so a carried operand is converted back first (QSpec FR-142-AC-30). | Test (TC-1287) |
| FR-497-AC-7 | The `InexactConversion` kernel refusal renders through FR-096's `RefusalRecord` and FR-100's kernel-record table as code `inexact_conversion`, cause `inexact-conversion`, field `expected` equal to the carrier type text, exit 20, and `Refusal::code()` and `Refusal::cause()` agree. | Test (TC-1287) |
| FR-497-AC-8 | Under `CheckMode::Kernel`, with `unit cm : Length = rational(1, 100);` and `unit in : Length = rational(127, 5000);`, `convert<Quantity<m, Rational[-3, 7; 1, 12]>>` of `25 cm` gives `1/4 m` and of `-3 m` gives `-3 m`; `1 in`, `8 m` and `-4 m` each refuse `rational_out_of_domain`/`outside-domain` at the convert node after `unit.target-domain` and before `unit.result-retain`, with `expected` the carrier and no value, and none refuses `inexact_conversion`; `8 m` under a work limit that denies the `unit.target-domain` charge is `incomplete` there; `convert<Quantity<cm>>` of the `1/4 m` value gives `25 cm` (QSpec FR-142-AC-31; `CQ-E21` to `CQ-E26`). | Test (TC-1287) |

## Dependencies

- **Upstream:** QSpec FR-142 (FR-142-AC-27 to AC-31), FR-140, FR-149
  ("Bounded rationals") and FR-322 (FR-322-AC-301, AC-302 and AC-382) define
  the type, the node and the conversion;
  FR-093 owns the `ConvertScalar` lowering row this amends; FR-094 owns the
  unit resolution of `Quantity<u>`; FR-096 and FR-100 own the refusal tables
  this extends.
- **Downstream:** the IR strict-reader leg for `carried_quantity` and the
  mode-mismatch refusals is separate (luna:IR:plan). IR-669 rows U07, U08, U16,
  U17, U18 and U29 depend on this requirement.
