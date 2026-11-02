---
id: FR-511
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
# FR-511: Name the successor or keep of each retained criterion that rides a retired path

## Description

FR-493-AC-5 requires `quire matrix` to list no criterion of a retained
requirement without a tagged test because its test went with a retired path.
The deletion table (QSL-858 comment, section C.5 S:E-7) lists the retained
criteria whose only tests ride on the native, composed or protocol-artifact
paths. This requirement gives each of the criteria below one disposition:
**Successor** (new criteria here or an existing spine criterion, with its test)
or **Keep in place** (the legacy code and test stay, with the condition that
ends the keep). No criterion is dropped.

## Behavior

| Legacy criterion | Disposition | Successor or condition |
| --- | --- | --- |
| FR-015-AC-1 | Successor | Bounded scalars: FR-103-AC-1 (`Int[0, 1000]` ConfigVersion); units and quantities: FR-497-AC-1; bounded Rational: FR-503; duplicate-preserving sequence items: FR-046-AC-3 (TC-127); the step operation: FR-103-AC-1. The runtime-object-ID-free rule: the package holds no object identities (FR-084). |
| FR-015-AC-2 | Successor | Rational and Text site refusals: FR-503, FR-056-AC-38; sequence maximum: FR-504; unsigned, saturating and rational declarations refusing: unrepresentable on the spine (a model scalar is an `Int[lower, upper]` or a constrained value type; the document has no unsigned or saturating vocabulary), recorded here as satisfied by construction. |
| FR-015-AC-3 | Successor | FR-511-AC-1 (below). |
| FR-015-AC-4 | Successor | False source loci: positions come from the document the reader read, with no separately supplied coordinate input (FR-056 retained position, FR-154), so a false coordinate cannot be offered, by construction; two documents of one package identity: FR-511-AC-2. |
| FR-015-AC-6 | Successor | FR-504. |
| MAX_SEQUENCE_ITEMS 10,000 | Successor | FR-504-AC-2, AC-3 (as a raisable setting; see FR-504 Dependencies). |
| FR-046-AC-1 | Successor | FR-511-AC-3. |
| FR-046-AC-2 | Successor | FR-511-AC-4. |
| FR-046-AC-4 | Successor | FR-511-AC-5. |
| FR-046-AC-7 | Successor | FR-511-AC-6. |
| FR-046-AC-8 | Successor | FR-511-AC-7. |
| FR-056-AC-8 | Successor | FR-511-AC-8. |
| FR-050-AC-2, AC-3 | Successor (emission) and Keep in place (reader vocabulary) | Emission: FR-511-AC-9. The `v2.*` refusal vocabulary in `src/protocol_artifact/v2/refusal.rs` stays until quire-protocol stops matching on it (QSL-1501); then it is deleted with the reader whose refusals the QSpec FR-322 reader owns. |
| FR-050-AC-5 | Successor | FR-511-AC-10. |
| FR-054-AC-1 to AC-4 | Successor (derived map) and Keep in place (`/3` reader) | Derived map: FR-511-AC-11. `protocol_artifact::v3` and TC-143 stay until QSL-1501; the deletion is gated on FR-511-AC-11 passing. |
| FR-048-AC-4 | Successor | FR-511-AC-12. |
| FR-048-AC-9 | Successor | FR-511-AC-13. |
| FR-049-AC-2, AC-5 | Successor (spine admission) and Keep in place (wire spellings) | Admission: FR-511-AC-14. The wire spellings `ArtifactKind::{Population, Window}` and `BindingKind::{Population, Window, Clock}` (`tests/it/assessment_artifact_kinds.rs`, TC-136) stay with `wire.rs` until QSL-1501. |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-511-AC-1 | Over the ConfigVersion package, changing a field's bound, an operation's frame entry, the package's source identity or revision, or an **unused** declaration's field bound each changes the checked package's bytes; reordering set-like records (operations, populations, types) leaves them equal (FR-103-AC-5); a package identity that is not the one a unit's `model` declaration names cannot be selected by it (`missing_import`). | Test (TC-1306) |
| FR-511-AC-2 | Two supplied documents with one package identity refuse `invalid_package`/`conflicting-definition` naming both, whether their contents are equal or differ, and an import selecting that package gets no partial binding. | Test (TC-1306) |
| FR-511-AC-3 | `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` called from a function, a state `invariant` and a protocol `check` yields one callee node in the checked package, referenced by three call terms that each keep their argument order, source span and receiver; a second predicate named `Positive` in an admitted domain package is a different node and a call that could mean either refuses `ambiguous_declaration`/`ambiguous-name`. | Test (TC-1306) |
| FR-511-AC-4 | A self-calling predicate and a two-predicate cycle with no measure each refuse at check with the FR-146 `missing-measure` obligation (FR-091-AC-30); the same cycles with a measure QSpec FR-146 accepts admit (the spine widens the legacy always-refuse rule, recorded); a call with the wrong argument count or type or unit refuses `ill_typed` at the call; a predicate whose result is not `Boolean` refuses at the assembler; an unguarded `10 / x` refuses `undefined_expression`/`unproved-nonzero` and its guarded counterpart admits; each refusal is at the original span and the dependent calls refuse too. | Test (TC-1306) |
| FR-511-AC-5 | Query evaluation over `Sequence<Int[0, 9]>`: an invalid request or value refuses before any result whatever its position; the empty sequence gives `size` 0, `contains` false, `forall` true, `exists` false, empty `filter`, empty `map`, `count` 0 and `sum` 0; `contains`, `forall` and `exists` stop at the decisive occurrence and ignore later unavailable data but report unavailable data met before it; `filter`, `map`, `count` and `sum` met-unavailable return the incomplete outcome with no partial output; the maximum-admitted and a nested input keep occurrence order and duplicates. | Test (TC-1306) |
| FR-511-AC-6 | For one predicate-expanding, nested-traversing query returning a retained sequence, the setting `accounting.work_units` at its measured usage completes, usage less one gives `Incomplete` at `accounting.work_units`, and 0 gives `Incomplete`; the same for `accounting.result_units` against the retained output; a retry with a fresh meter returns the first result, and the earlier immutable inputs are unchanged. | Test (TC-1306) |
| FR-511-AC-7 | The emitted call and query records of AC-3 and AC-5's package keep the original handles, binder scopes, type and unit selections and captures (FR-093); the run report names the stage that produced each result (`check` evidence versus `replay` evaluation evidence) and a `check` pass never stands for an unexecuted replay. | Test (TC-1306) |
| FR-511-AC-8 | The filament-core-data#173 architecture fixture, read by the spine's I2 intake, resolves its ports with owning part, direction, interface type and multiplicity, admits a connection between them under FR-152, and a `1-draft` unit whose `model` declaration selects the package binds references to its field, port and connection declarations (FR-036-AC-9); this is the FR-056-AC-8 test on the spine, with the fixture, not the composed linker. | Test (TC-148, TC-1306) |
| FR-511-AC-9 | A `temporal_bindings` selection with a surplus entry, a duplicate for one node, entries out of `NodeRef` order, an entry for a node of another package, an out-of-range node, a profile and definition mismatch, or a clock alternative the profile does not take each refuses at emission with the cause FR-050-AC-14 gives (`invalid_package`/`timed-node-reference` for node defects, `missing_declaration`/`missing-selection` for omissions) and no package; an empty or over-long `clock_name`, a zero, negative, unreduced or overflowing period, and a substituted epoch, unit or sequence authority each refuse `invalid_package`/`invalid-value` naming the field; none is defaulted. | Test (TC-1306) |
| FR-511-AC-10 | The emitted bytes and the read of a package with N temporal bindings are bounded: `identity.input_bytes` and `i2.input_bytes` at the measured size admit, one short refuse naming the setting, and a retry with sufficient limits reproduces the same bytes. | Test (TC-1306) |
| FR-511-AC-11 | A checked package's derived activation mapping holds one row per event-triggered control paired with its temporal declaration; round-tripped through emission and the reader it is unchanged. A mutated mapping (a row removed, duplicated, for a foreign control, with an out-of-range index, for a non-event-triggered control, reordered, or replaced by a row chosen from a matching profile or clock, a display name, a source position, a timestamp or an observation) is refused at read-back before a view is admitted. | Test (TC-1306) |
| FR-511-AC-12 | In a choice or repeat guard, a `visible` atom that the owner cannot causally see (a binder of another branch, a success-only effect binder, a value owned by another role, a same-spelled name of another owner) refuses at S3 with the FR-218 visible-entry typed cause, also when it appears only in an operand a short-circuit would skip. | Test (TC-1306) |
| FR-511-AC-13 | The emitted package of a protocol unit is read by Contract IR's reader independently of QSL and admitted; changing the source identity, the domain-package identity or the result identity, or re-serialising the bytes non-canonically, is refused by the reader. | Test (TC-1306) |
| FR-511-AC-14 | FR-106 admission refuses a duplicate, surplus, foreign or wrong-type member of a snapshot or population, keeps population, window and clock inputs distinct, and refuses a duplicate storage key; evaluation of a dereference to a target absent from a complete population is `undefined` with reason `absent-key`, and to a target of an unavailable membership or closure is the incomplete unavailable outcome (FR-049-AC-5). | Test (TC-1306) |
| FR-511-AC-15 | The Keep-in-place rows of the table hold: while `protocol_artifact::v2` refusal vocabulary, `protocol_artifact::v3` and the `wire.rs` kind spellings exist, each of their tests keeps its tag, and no deletion of them merges before the successor criterion of its row passes. | Inspection |

## Dependencies

- **Upstream:** FR-493-AC-5, the deletion table (QSL-858), planner rulings 5,
  8 and 9 on QSL-858.
- **QSpec contract questions:** FR-511-AC-11 and AC-13 assume the QSpec FR-322
  reader refuses the mutations they list; FR-511-AC-9's `invalid-value` is the
  FR-480-AC-4 cause reused. Where QSpec names another cause the criterion is
  reworded, not dropped.
