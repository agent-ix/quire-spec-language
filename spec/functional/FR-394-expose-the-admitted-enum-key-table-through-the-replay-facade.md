---
id: FR-394
title: "Expose the admitted enum key table through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-060
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-015
    type: depends_on
---
# FR-394: Expose the admitted enum key table through the replay facade

## Description

ADR-011 T12 and FR-087 forbid a consumer of QSL to build a `NodeKey`; the
check stage mints every key. A contract generator that lowers an enum
equality needs the key of an enum declaration and of each of its cases, and
`compile_package` (FR-060-AC-5) drops the checked context that holds them.
This requirement adds the replay-owned entry that returns the context
beside the emitted package, so a consumer looks a key up and never builds one,
and never names `qsl_replay::spine` (FR-060, T12-A).

## Inputs

- `compile_package`'s arguments and limits (FR-060-AC-5).
- For a lookup, the wire node id of an enum declaration, as the emitted
  package or a contract IR document carries it, and optionally a case
  identifier.

## Outputs

- `compile_admitted` returns the emitted package (as `compile_package`) and an
  `AdmittedContext`: the package's `package_id`; every enum declaration the
  check admitted, each with its key, qualified name, ordering and every case
  in canonical order with its member key, used or not.
- A lookup returns the admitted declaration or case, or a typed refusal.

## Behavior

- The context holds only the keys the check minted. A lookup finds a key or
  refuses; it creates none.
- A declaration id that no package of the dependency closure admitted is
  `Unknown`; one a dependency admitted is `Foreign`, naming that dependency's
  `package_id`; a case the declaration lacks is `Unknown`, naming the case.
- A unit, library or dependency input that does not compile is a `Package`
  refusal, a stage limit reached or an S1 limit above the reader limit is a
  `Resource` refusal, and a broken QSL invariant is a `Fault`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-394-AC-1 | `compile_admitted` over a unit that declares an enum returns the same package bytes and `package_id` as `compile_package`, and a context whose `package_id` is that id and whose enum has every declared case, including a case no function uses, in canonical order, with keys equal to those the spine's checked graph holds for the declaration and member nodes. | Test (TC-1088) |
| FR-394-AC-2 | `enum_declaration` and `enum_case` return the admitted declaration and case for the declaration's wire node id, and refuse `Unknown` for an id no package admitted and `Unknown` naming the case for a case the enum lacks. | Test (TC-1088) |
| FR-394-AC-3 | Over a unit importing a library that declares an enum, `enum_declaration` of the library enum's id refuses `Foreign` naming the library's `package_id`, while the unit's own enum is found. | Test (TC-1088) |
| FR-394-AC-4 | A malformed source and a library with the unit's source owner refuse `Package`; an S1 limit below the source size and an S1 limit above the reader limit refuse `Resource`. | Test (TC-1088) |
| FR-394-AC-5 | A consumer reaches the entry and its types through the `qsl_replay` root alone, without naming `qsl_replay::spine`. | Test (TC-1088) |

## Dependencies

- FR-060 (the T12 boundary and `compile_package`), FR-087 (the cross-package
  node key), ADR-015 (the dependency closure).
