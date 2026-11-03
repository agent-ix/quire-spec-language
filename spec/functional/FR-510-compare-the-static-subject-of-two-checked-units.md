---
id: FR-510
title: "Compare the static subject of two checked units"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
---
# FR-510: Compare the static subject of two checked units

## Description

FR-036-AC-5 and TC-115 require that changing a runtime population, window,
trace or backend leaves the static meaning of a package unchanged, that
changing a required semantic selection changes it, and that a change of
resource-only configuration stays visible without being a semantic change. The
composed lane carried this as `StaticSubject` (six components) and
`BuildProvenance` (resource configuration), with `differences()` naming the
component that changed. The spine has no such value: its closest identity is
the emitted package and its evidence. This requirement defines the spine
static subject.

## Inputs

- A `spine::CheckedUnit` (FR-362) or an `EmittedUnit`, with the
  `SpineLimits` and usage it was produced under.

## Outputs

- `StaticSubject`: six components, each an ordered list of values:
  1. **sources**: the unit's source reference (authority, identity) and the
     content identity of the unit and of each resolved library, in canonical
     order. A *resolved* library is one the check reached through the unit's
     imports, transitively (`CheckedUnit::libraries()`); a supplied library
     the imports never reach is not resolved and is not in the subject;
  2. **language**: the edition the headers agree on;
  3. **profiles**: each header profile alias with its resolved identity, in
     source order, and, for each declaration, the profile closure it is
     checked under;
  4. **models**: each selected domain package: alias, package identity,
     version, content identity;
  5. **dependencies**: each resolved library: identity and the declaration
     kinds the unit refers to, and each resolved reference to a predicate,
     operation contract or temporal obligation, with its referring
     declaration and its target;
  6. **roles**: each declaration-owned binding role: owner declaration,
     spelling, kind, anchor and type identity.
The legacy components are carried as follows: the legacy profile component
(each declaration's root definition and closure) is component 3, and the
legacy dependency component (resolved predicate, operation-contract and
temporal-obligation references) is component 5; the library list in
component 5 is added because the spine, unlike the composed lane, imports
libraries (FR-099). This is intended.
- `BuildProvenance`: the `SpineLimits` and the usage of the producing run.
- `differences(&other) -> Vec<ComponentKind>`: the components that differ.

## Behavior

- The spine SHALL compute the subject from the checked unit alone, and the
  subject SHALL exclude any stage limit, usage, cancellation, clock, backend,
  population, window, trace, observation or cache state.
- Two subjects SHALL be equal iff all six components are equal, and
  `differences` SHALL name exactly the components that are unequal, in the
  order above, empty when equal.
- Equal subjects assert only that the declared static components agree. They
  are not a checked package identity, an executable package, or a qualified
  canonical identity, and no consumer SHALL present them as one.
- A refused check produces no checked unit and therefore no subject.
- Roles with equal spelling in two declarations SHALL be two role entries with
  different owners (FR-036-AC-4).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-510-AC-1 | Checking one source, model and profile inventory twice, once with every `SpineLimits` field at its default and once with every limit raised, gives equal subjects, `differences` empty, and unequal `BuildProvenance`. | Test (TC-1305) |
| FR-510-AC-2 | Replaying the emitted unit against two different populations, windows, traces and backend selections (FR-098) leaves the subject computed before and after unchanged, `differences` empty. | Test (TC-1305) |
| FR-510-AC-3 | Changing only a header profile's identity (`quire.value.complete/v1` to `quire.state.core/v1` where the source still checks) gives `differences` = [profiles]; changing only the selected domain package's version gives [models]; changing only the unit's source text gives [sources]; adding an unrelated unused library that the unit does not reach gives [] and one the unit imports gives [sources, dependencies]. | Test (TC-1305) |
| FR-510-AC-4 | Two declarations that each bind a role spelled `x` give two role entries with different owners; renaming one declaration's role changes [sources, roles], because the exact unit source content and declaration-owned role spelling both change. | Test (TC-1305) |
| FR-510-AC-5 | A source that fails to check yields no subject: the check returns its refusal and no `StaticSubject` value can be built for it. | Test (TC-1305) |
| FR-510-AC-6 | A downstream crate that calls a method on `StaticSubject` to obtain a checked-package identity or the emitted bytes, or that converts it into either, does not compile (compile-fail test); the only comparison it offers is component-wise equality and `differences`. | Test (TC-1305) |

## Dependencies

- **Upstream:** FR-036-AC-5 (the legacy behavior; its tests ride on
  `tests/it/composed_admission_stages.rs` and are orphaned when that file is
  deleted), FR-049-AC-10 (the compile path retains the selected source, model,
  profile and configuration identities), FR-362, FR-255.
