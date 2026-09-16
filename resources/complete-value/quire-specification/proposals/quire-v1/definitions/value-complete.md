# Complete value definition

Definition identity: `quire.value.complete/v1`; revision: `1-draft.1`.
Language: `ix:native` / `1-draft`. This definition is the complete-V1 value,
type and expression root; implementation and qualification remain separately
reported.

## Always-selected definitions

Every package that selects this root also selects:

| Identity | Revision | Definition artifact |
| --- | --- | --- |
| `ix:native` / `1-draft` | `1-draft.2` | [Edition](edition.md) |
| `quire.value.accounting/v1` | `1-draft.1` | [Scalar accounting](value-accounting.md) |
| `quire.value.compound-unit/v1` | `1-draft.1` | [Compound-unit value identity](value-compound-unit.md) |
| `quire.value.compound-unit.schema/v1` | `1-draft.1` | [Compound-unit schema](value-compound-unit.schema.json) |
| `quire.value.compound-unit.vectors/v1` | `1-draft.1` | [Compound-unit vectors](value-compound-unit-vectors.json) |
| `quire.value.complete.rules/v1` | `1-draft.1` | [Exact rule closure](value-complete-rules.json) |

## Conditional and alternative definitions

Selection of the remaining profiles is decided per package by a closed trigger
vocabulary: `text_bearing`, `ieee_operation` and `integer_div_rem`.

- A package is `text_bearing` when it declares or evaluates a text type or
  value; it then selects the Unicode definition
  [`quire.value.text.unicode-17.0.0/v1`](value-text-unicode-17.md), and a
  package without that trigger does not select it.
- A package has `ieee_operation` when it declares or evaluates a `float32` or
  `float64` type, value or operation; it then selects the IEEE definition
  [`quire.value.ieee754-2019-default/v1`](value-ieee754-2019-default.md), and a
  package without that trigger does not select it.
- A package has `integer_div_rem` when it evaluates integer `div` or `rem`; it
  then selects exactly one of
  [`quire.value.integer-division.euclidean/v1`](value-integer-division-euclidean.md),
  [`quire.value.integer-division.floor/v1`](value-integer-division-floor.md) or
  [`quire.value.integer-division.truncating/v1`](value-integer-division-truncating.md),
  and a package without that trigger selects none of them.

Every selected artifact's exact bytes and `quire.definition.bytes/v1` digest are
retained in the checked package; an identity string or installed library alone
does not select meaning.

The exact-rule manifest assigns every selected architecture/requirement document
a stable definition identity, revision and raw-byte SHA-256. A checked package
retains the manifest and every listed artifact as separate DefinitionRefs under
`quire.definition.bytes/v1`; selecting only this root document or a moving
repository revision is insufficient. Any rule-byte change requires a successor
manifest revision and changes package identity.

## Qualification catalog and package selection

The canonical lock is [`complete-value-lock.json`](complete-value-lock.json).
It separates two facts. Its `qualification_catalog` closes every artifact a
package may select — always-selected, conditional, alternative and rule
artifacts — by role, identity, revision, digest domain and raw-byte digest;
tests consume those exact DefinitionRefs rather than inventing placeholder
digests. Catalog membership is not selection. Its `package_selection` assigns
each catalog role to exactly one slot: `always`, one `conditional` trigger slot,
or one `exactly_one` alternative slot.

A package selection is admitted only when it names known triggers and catalog
roles once each, includes every `always` role, and selects each conditional
role exactly when its trigger is present and exactly one alternative exactly
when the alternative trigger is present. A refused selection reports the first
failing check in this order, using the closed codes
`selection_unknown_trigger`, `selection_unknown_role`,
`selection_duplicate_role`, `selection_required_missing`,
`selection_alternative_conflict`, `selection_trigger_unsatisfied` and
`selection_untriggered_profile`. The canonical accepted and refused selections
are [`complete-value-selection-vectors.json`](complete-value-selection-vectors.json).

## Selected rules

The links below are readable projections only. Their normative identities and
digests are the entries in `value-complete-rules.json`.

| Rule |
| --- |
| [Complete source/package contract](../package-contract.md) |
| [Complete shared grammar](../shared-grammar.md) |
| [Complete value architecture](../../../spec/assurance/AD-005-complete-value-expression-system.md) |
| [Exact decimals](../../../spec/functional/type-model/FR-140-evaluate-exact-decimals.md) |
| [Text and enumerations](../../../spec/functional/type-model/FR-141-evaluate-text-and-enumerations.md) |
| [Dimensions and units](../../../spec/functional/type-model/FR-142-evaluate-quantities-and-units.md) |
| [Integer division](../../../spec/functional/expressions/FR-147-evaluate-integer-division-domains.md) |
| [IEEE profiles](../../../spec/functional/expressions/FR-148-evaluate-ieee-floating-profiles.md) |
| [Equality matrix](../../../spec/functional/type-model/FR-149-apply-complete-equality-matrix.md) |
