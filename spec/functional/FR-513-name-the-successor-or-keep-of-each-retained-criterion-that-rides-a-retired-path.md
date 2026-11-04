---
id: FR-513
title: "Name the successor or keep of each retained criterion that rides a retired path"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-054
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
---
# FR-513: Name the successor or keep of each retained criterion that rides a retired path

## Description

FR-493-AC-5 requires `quire matrix` to list no criterion of a retained
requirement without a tagged test because its test went with a retired path.
The deletion table (QSL-858 comment, section C.5 S:E-7) lists the retained
criteria whose only tests ride on the native, composed or protocol-artifact
paths. This requirement gives each of the criteria below one disposition:
**Successor** (new criteria here or an existing spine criterion, with its test)
or **Keep until green** (the legacy code and test stay until the named
successor is green, as FR-499's rows state; there is no permanent keep). No criterion is dropped.

## Behavior

| Legacy criterion | Disposition | Successor or condition |
| --- | --- | --- |
| FR-015-AC-1 | Successor | Bounded scalars: FR-103-AC-1 (`Int[0, 1000]` ConfigVersion); units and quantities: FR-497-AC-1; bounded Rational: FR-505; duplicate-preserving sequence items: FR-046-AC-3 (TC-127); the step operation: FR-103-AC-1. The runtime-object-ID-free rule: the package holds no object identities (FR-084). |
| FR-015-AC-2 | Successor | Rational and Text site refusals: FR-505, FR-056-AC-38; sequence maximum: FR-506; unsigned, saturating and rational declarations refusing: unrepresentable on the spine (a model scalar is an `Int[lower, upper]` or a constrained value type; the document has no unsigned or saturating vocabulary), recorded here as satisfied by construction. |
| FR-015-AC-3 | Successor | FR-513-AC-1 (below). |
| FR-015-AC-4 | Successor | False source loci: positions come from the document the reader read, with no separately supplied coordinate input (FR-056 retained position, FR-154), so a false coordinate cannot be offered, by construction; two documents of one package identity, and conflicting native bindings for one formal source identity (two `model` declarations that bind one identity to different packages or versions): FR-513-AC-2. |
| FR-015-AC-6 | Successor | FR-506. |
| MAX_SEQUENCE_ITEMS 10,000 | Successor | FR-506-AC-2, AC-3 (as the raisable `model.sequence_maximum` setting, bounded by `u32::MAX`; see FR-506 Dependencies). |
| FR-046-AC-1 | Successor | FR-513-AC-3. |
| FR-046-AC-2 | Successor | FR-513-AC-4. |
| FR-046-AC-4 | Successor | FR-513-AC-5. |
| FR-046-AC-7 | Successor | FR-513-AC-6. |
| FR-046-AC-8 | Successor | FR-513-AC-7. |
| FR-056-AC-8 | Successor | FR-513-AC-8. |
| FR-050-AC-2, AC-3 | Successor (emission) and Keep until green (reader vocabulary) | Emission: FR-513-AC-9. The `v2.*` refusal vocabulary in `src/protocol_artifact/v2/refusal.rs` is kept until the successor QSL-1506 ports (FR-499's row) is green and quire-protocol has stopped matching on it (QSL-1501); then it is deleted with the reader whose refusals the QSpec FR-322 reader owns. |
| FR-050-AC-5 | Successor | FR-513-AC-10. |
| FR-054-AC-1 to AC-4 | Successor (derived relation) and Keep until green (`/3` reader) | Derived relation: FR-513-AC-11. `protocol_artifact::v3` and TC-143 remain until the derived-relation tests and QSL-1506 port (FR-499's row) are green and QSL-1501 has repointed quire-protocol; the checked-package reader continues to reject the incompatible compiled-protocol versions. |
| FR-048-AC-4 | Successor | FR-513-AC-12. |
| FR-048-AC-9 | Successor | FR-513-AC-13. |
| FR-049-AC-2, AC-5 | Successor (spine admission and versioned kind vocabulary) and Keep until green (legacy wire spellings) | Admission: FR-513-AC-14. The exact `ArtifactKind::{Population, Window}` and `BindingKind::{Population, Window, Clock}` spellings are exposed by `qsl_package::protocol_artifact_v1` (FR-513-AC-16). The legacy `wire.rs` spellings and TC-136 test remain until QSL-1501 has repointed quire-protocol. |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-513-AC-1 | Over the ConfigVersion package, changing a field's bound, an operation's frame entry, the package's source identity or revision, or an **unused** declaration's field bound each changes the checked package's bytes; reordering set-like records (operations, populations, types) leaves them equal (FR-103-AC-5); a package identity that is not the one a unit's `model` declaration names cannot be selected by it (`missing_import`). | Test (TC-1308) |
| FR-513-AC-2 | Two supplied documents with one package identity refuse `invalid_package`/`conflicting-definition` naming both, whether their contents are equal or differ, and an import selecting that package gets no partial binding; a unit whose two `model` declarations bind one package identity to two different versions or contents refuses `invalid_package`/`conflicting-definition` naming both declarations, and one that binds it twice to the same version and content under two aliases admits. | Test (TC-1308) |
| FR-513-AC-3 | `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` called from a function, a state `invariant`, a protocol `check` and a `temporal` clause yields one callee node in the checked package, referenced by four call terms that each keep their argument order, source span and receiver; a second predicate named `Positive` in an admitted domain package is a different node and a call that could mean either refuses `ambiguous_declaration`/`ambiguous-name`. | Test (TC-1308) |
| FR-513-AC-4 | A self-calling predicate and a two-predicate cycle with no measure each refuse at check with the FR-146 `missing-measure` obligation (FR-091-AC-30); the same cycles with a measure QSpec FR-146 accepts admit (a widening of the legacy always-refuse rule, which FR-046-AC-2 is amended to state; the planner ruled for it); a call with the wrong argument count or type or unit refuses `ill_typed` at the call; a predicate whose result is not `Boolean` refuses at the assembler; an unguarded `10 / x` refuses `undefined_expression`/`unproved-nonzero` and its guarded counterpart admits; each refusal is at the original span and the dependent calls refuse too. | Test (TC-1308) |
| FR-513-AC-5 | Query evaluation over `Sequence<Int[0, 9]>`: an invalid request or value refuses before any result whatever its position; the empty sequence gives `size` 0, `contains` false, `forall` true, `exists` false, empty `filter`, empty `map`, `count` 0 and `sum` 0; `contains`, `forall` and `exists` stop at the decisive occurrence and ignore later unavailable data but report unavailable data met before it; `filter`, `map`, `count` and `sum` met-unavailable return the incomplete outcome with no partial output; the maximum-admitted and a nested input keep occurrence order and duplicates. | Test (TC-1308) |
| FR-513-AC-6 | For one predicate-expanding, nested-traversing query returning a retained sequence, the setting `accounting.work_units` at its measured usage completes, usage less one gives `Incomplete` at `accounting.work_units`, and 0 gives `Incomplete`; the same for `accounting.result_units` against the retained output; a retry with a fresh meter returns the first result, and the earlier immutable inputs are unchanged. | Test (TC-1308) |
| FR-513-AC-7 | The emitted call and query records of AC-3 and AC-5's package keep the original handles, binder scopes, type and unit selections and captures (FR-093); the run report names the stage that produced each result (`check` evidence versus `replay` evaluation evidence) and a `check` pass never stands for an unexecuted replay. | Test (TC-1308) |
| FR-513-AC-8 | The filament-core-data#173 architecture fixture, read by the spine's I2 intake, resolves its ports with owning part, direction, interface type and multiplicity, admits a connection between them under FR-152, and a `1-draft` unit whose `model` declaration selects the package binds references to its field, port and connection declarations (FR-036-AC-9); this is the FR-056-AC-8 test on the spine, with the fixture, not the composed linker. | Test (TC-148, TC-1308) |
| FR-513-AC-9 | A `temporal_bindings` selection with a surplus entry, a duplicate for one node, entries out of `NodeRef` order, an entry for a node of another package, an out-of-range node, a profile and definition mismatch, or a clock alternative the profile does not take each refuses at emission with the cause FR-050-AC-14 gives (`invalid_package`/`timed-node-reference` for node defects, `missing_declaration`/`missing-selection` for omissions) and no package; an empty or over-long `clock_name`, a zero, negative, unreduced or overflowing period, and a substituted epoch, unit, sequence authority, definition identity, definition revision or definition artifact reference each refuse `invalid_package`/`invalid-value` naming the field; none is defaulted. | Test (TC-1308) |
| FR-513-AC-10 | For a package with N temporal bindings, S3's checked declaration preimages and I2's read of the emitted package are each bounded by their stage's `input_bytes`: the measured size admits, one byte short refuses naming `s3.input_bytes` or `i2.input_bytes`, respectively, and a retry with sufficient limits reproduces the same bytes. | Test (TC-1308) |
| FR-513-AC-11 | For each checked protocol clause, the spine derives its activation relation only from that clause's validated structural node identity, activation trigger and required temporal node identities; it returns one row per unique requirement, in structural clause and temporal-key order. The relation round-tripped through emission and the I2 reader is unchanged. The package carries no independent activation-mapping input, so rows cannot be supplied, reordered or replaced by profile, clock, display name, source position, timestamp or observation; the reader refuses a missing dependency or a foreign, absent or otherwise invalid trigger/temporal node before a view is admitted. The checked-package reader refuses the incompatible `quire.compiled-protocol/1`, `/2` and `/3` envelopes rather than upgrading them. The legacy `/3` reader and TC-143 remain until QSL-1501 repoints quire-protocol. | Test (TC-1194, TC-1308, TC-143) |
| FR-513-AC-12 | In a choice or repeat guard, a `visible` atom that the owner cannot causally see (a binder of another branch, a success-only effect binder, a value owned by another role, a same-spelled name of another owner) refuses at S3 with the FR-218 visible-entry typed cause, also when it appears only in an operand a short-circuit would skip. | Test (TC-1308) |
| FR-513-AC-13 | The emitted package of a protocol unit is read by Contract IR's reader independently of QSL and admitted; changing the source identity, the domain-package identity or the result identity, or re-serialising the bytes non-canonically, is refused by the reader. | Test (TC-1308) |
| FR-513-AC-14 | FR-106 admission refuses a duplicate, surplus, foreign or wrong-type member of a snapshot or population, keeps population, window and clock inputs distinct, refuses a duplicate storage key, and admits distinct observation occurrences of one observation under their complete observation, model, universe, object-type and object identity; evaluation of a dereference to a target absent from a complete population is `undefined` with reason `absent-key`, and to a target of an unavailable membership or closure is the incomplete unavailable outcome (FR-049-AC-5). | Test (TC-1308) |
| FR-513-AC-15 | While `protocol_artifact::v2` refusal vocabulary, `protocol_artifact::v3` or the `wire.rs` kind spellings exist, `quire matrix` lists FR-050-AC-2, AC-3, FR-054-AC-1 to AC-4 and FR-049-AC-2 and AC-5 each with a tagged test; a change that deletes one of those modules without a green successor test for its row leaves a criterion untagged, and the matrix test fails. | Test (TC-1308) |
| FR-513-AC-16 | The versioned `qsl_package::protocol_artifact_v1` API serializes and deserializes exactly `ArtifactKind::{Population, Window}` as `population` and `window`, and `BindingKind::{Population, Window, Clock}` as `population`, `window` and `clock`; each kind round-trips to the same variant, and every other string is refused. | Test (TC-136) |

## Dependencies

- **Upstream:** FR-493-AC-5, the deletion table (QSL-858), planner rulings 5,
  8 and 9 on QSL-858.
- **QSpec contract questions:** FR-513-AC-11 and AC-13 assume the QSpec FR-322
  reader refuses the mutations they list; FR-513-AC-9's `invalid-value` is the
  FR-480-AC-4 cause reused. Where QSpec names another cause the criterion is
  reworded, not dropped.
