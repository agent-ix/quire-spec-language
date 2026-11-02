---
id: TC-1301
title: "Admit a profile only at the use sites of its kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-506
    type: verifies
---
# TC-1301: Admit a profile only at the use sites of its kind

## Description

Verify per-use profile-kind admission on the spine. Scope: FR-506-AC-1 to
FR-506-AC-6.

## Test Procedure

Header `H`: `profile v = "quire.value.complete/v1"; profile pp =
"quire.protocol.finite-global/v1"; profile t = "quire.temporal.timed/v1";
profile e = "quire.temporal.event-position.false-extension/v1"; profile i =
"quire.temporal.infinite-trace/v1";` plus one alias per other admitted
identity. Each row compiles `H` and the stated protocol, whose body is the
smallest valid body for the site.

| Row | Site and alias | Expected |
| --- | --- | --- |
| 1 | `protocol P using pp` | admitted |
| 2 | `protocol P using v` | `unknown_profile`/`wrong-selection-role` at `v`; alias `v`, identity `quire.value.complete/v1`, required kind protocol profile |
| 3 | `protocol P using t`; `protocol P using s` where `s` selects `quire.state.core/v1` | each refused as row 2 |
| 4 | `await ... using` and `compensate ... using` each of the four temporal identities | all admitted |
| 5 | `await ... using pp`; `await ... using v`; `compensate ... using i` | each `unknown_profile`/`wrong-selection-role` at the alias |
| 6 | `check C using` each of the six state-bearing layers | all admitted |
| 7 | `check C using pp`; `check C using e` | each `unknown_profile`/`wrong-selection-role` at the alias |
| 8 | `protocol P using f` where `f` is a function name | `MissingProfile` only |
| 9 | wrong-kind protocol alias and wrong-kind check alias in one protocol | two refusals in source order; no package |
| 10 | `protocol P using pp` beside `function f using v` | admitted; package equals the package of the same unit before FR-506 |

Tag the test `#[trace("FR-506-AC-1", "FR-506-AC-2", "FR-506-AC-3", "FR-506-AC-4", "FR-506-AC-5", "FR-506-AC-6", "TC-1301")]`.

## Expected Results

Each row's Expected column; a refusal produces no package or bytes.
