---
id: FR-508
title: "Admit a profile only at the use sites of its kind"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-508: Admit a profile only at the use sites of its kind

## Description

The composed linker refused a profile used at a site that needs another kind
of profile (`Cause::WrongProfileKind{selected, required}`): `protocol P using
<profile>` needed the protocol profile, `await` and `compensate` a temporal
profile, and `check` a state profile. The spine resolves each header profile
by identity (FR-110) and checks only that the `using` alias of a protocol, a
`check` or an `await` names some profile (`ProtocolAnchorCause::MissingProfile`),
so `protocol P using v` with `v` a value profile is admitted. This requirement
is the spine successor of that per-use admission.

## Inputs

- The unit's header profile selections as FR-110 resolves them (alias,
  identity), and the `using` alias at each `protocol`, `check`, `await` and
  `compensate` site of a protocol (FR-218).

## Outputs

- An admitted use, or the refusal `unknown_profile`/`wrong-selection-role` at
  the `using` alias, retaining the alias, the selected identity and the
  required role.

## Behavior

The kind each site requires is:

| Site | Required kind | Identities admitted |
| --- | --- | --- |
| `protocol <name> using <alias>` | protocol profile | `quire.protocol.finite-global/v1` |
| `await ... using <alias>` and `compensate ... using <alias>` | temporal profile | `quire.temporal.event-position.false-extension/v1`, `quire.temporal.fixed-sample.false-extension/v1`, `quire.temporal.timestamped-event.finite-window/v1`, `quire.temporal.timed/v1` |
| `check <name> using <alias> { ... }` | state-bearing layer | the header layers whose closure holds `quire.state.core/v1`: `quire.state.core/v1`, `quire.state.queries/v1`, `quire.state.graph/v1`, `quire.value.complete/v1`, `quire.model.complete/v1`, `quire.temporal.complete/v1` |

- The protocol clause checker SHALL, at each site, resolve the alias to its
  selected identity and refuse an identity outside the site's admitted set
  with `unknown_profile`/`wrong-selection-role` at the `using` alias,
  retaining the alias, the selected identity and the required kind. It SHALL
  report every refusing site in source order, as FR-110 reports every refusing
  header profile.
- An alias that names no profile SHALL keep its existing refusal
  (`MissingProfile`) and not the new one.
- The identity `quire.temporal.infinite-trace/v1` is not admitted at `await` or
  `compensate`, which settle over a finite window (the composed lane admitted
  only the finite profiles).
- A refusal produces no checked package and no bytes (FR-110).
- A unit that selects the protocol profile under one alias and a value layer
  under another SHALL admit `protocol P using pp` and `function f using v`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-508-AC-1 | With `profile v = "quire.value.complete/v1";` and `profile pp = "quire.protocol.finite-global/v1";`, `protocol P using pp` admits and `protocol P using v` refuses `unknown_profile`/`wrong-selection-role` at `v`, retaining alias `v`, identity `quire.value.complete/v1` and the required kind protocol profile. | Test (TC-1303) |
| FR-508-AC-2 | `protocol P using t` with `t` selecting `quire.temporal.timed/v1` refuses the same way; so does an alias selecting a state layer. | Test (TC-1303) |
| FR-508-AC-3 | An `await` and a `compensate` `using` an alias that selects each of the four admitted temporal identities admit; `using pp` (protocol profile), `using v` (value layer) and `using i` (`quire.temporal.infinite-trace/v1`) each refuse `unknown_profile`/`wrong-selection-role` at the alias. | Test (TC-1303) |
| FR-508-AC-4 | A `check` `using` each of the six admitted layers admits; `using pp` and `using t` (any temporal profile) refuse `unknown_profile`/`wrong-selection-role` at the alias. | Test (TC-1303) |
| FR-508-AC-5 | A protocol whose `using` alias is not a profile alias (a function name) keeps `MissingProfile` and not `wrong-selection-role`; a protocol with a wrong-kind `protocol` alias and a wrong-kind `check` alias reports both refusals in source order and produces no package. | Test (TC-1303) |
| FR-508-AC-6 | A unit selecting `pp` and `v` admits `protocol P using pp` beside `function f using v`, and its checked package is equal to the package of the same unit before this requirement. | Test (TC-1303) |

## Dependencies

- **Upstream:** FR-110 (refusal vocabulary), FR-218 (protocol clause forms).
- **Consequence for existing fixtures:** spine tests that write `protocol X
  using v` with a value profile `v` (for example the refinement fixtures)
  must select the protocol profile for that alias; that is a fixture change,
  not a behavior change.
