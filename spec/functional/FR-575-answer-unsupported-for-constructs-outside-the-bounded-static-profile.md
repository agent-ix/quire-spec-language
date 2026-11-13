---
id: FR-575
title: "Answer unsupported for constructs outside the bounded-static profile"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-004
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-573
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-174
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-175
    type: depends_on
---
# FR-575: Answer unsupported for constructs outside the bounded-static profile

## Description

QSL SHALL decide projection and realizability only in the single profile
`bounded-static`, and SHALL answer `unsupported`, naming a located construct
cause under the existing `unsupported_projection` code, for every construct
outside it, with no substitute graph and no guess. It implements ADR-036
PA-21 and PA-22 and QSpec FR-174-AC-3, and fixes the open item that ADR-036
left to the requirements: the profile word and the cause slugs.

## Inputs

A projection or realizability request (FR-570, FR-571) with a profile and a
checked protocol.

## Outputs

An `unsupported` result (FR-573) carrying `unsupported_projection` with one
cause slug and the checked node id of the construct; or, for a profile word
other than `bounded-static`, a refusal.

## Behavior

The profile is `bounded-static`: static roles, literal queue capacities,
sequence, send, receive, `choice`, and `parallel` with `join all`. The
causes, each a slug of `unsupported_projection` in the style of
`unsupported-requested-capability` and `unbounded-extent` (FR-057), are:

| Slug | Construct |
| --- | --- |
| `dynamic-role-population` | A role with a dynamic population. |
| `symbolic-queue-capacity` | A queue with a symbolic capacity. |
| `unsupported-join` | A `join` with policy `any`, `quorum` or `predicate`, or an `outstanding` policy with no rule; the policy is named. |
| `repeat` | A `repeat` construct. |
| `compensation` | A compensation. |
| `timed-await` | A timed await. |
| `unsupported-control` | A `check`, `attempt`, `effect`, `event`, `commit` or `fence` control, which the profile's list does not include. |
| `evaluator-unsupported` | Realizability only: a construct `qsl-eval` refuses to step; the `qsl-eval` cause is carried. |

When a protocol holds several such constructs the engine reports the first in
the order roles, queues, compensations, then the other constructs in layout
order; `unsupported-control` is reported only when no other construct is found.
A choice the protocol assigns to no role is not a construct a checked package
can hold (the checker refuses a choice whose owner is not a role), so the
profile needs no cause for it.

Names are chosen as the construct's own name, kebab case, one slug per
construct, so a corpus author reads the construct from the slug; `join any`,
`quorum`, `predicate` and `outstanding` share `unsupported-join` because the
remedy is the same (a `join all`) and the named policy distinguishes them.

- QSL shall refuse a profile word other than `bounded-static` at admission,
  before any analysis, `invalid_runtime_input`/`invalid-value` naming the
  member `profile`; it is a request value, not a construct, so it is not an
  `unsupported` result.
- The engine shall never answer a located unsupported construct with a
  projected graph, a strategy, a sampled result or a guessed one (PA-22).
- QSL shall make `repeat` and non-`all` joins unavailable to corpora under
  projection and realizability in this profile; a corpus that needs a retry
  loop or such a join receives the cause above.
- When a construct is both in a row above and refused by `qsl-eval`, the
  engine shall report that row's own slug; `evaluator-unsupported` is
  reported only for a `qsl-eval` cause that has no row above.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-575-AC-1 | For each construct row other than `evaluator-unsupported`, a protocol using it yields `unsupported`, `unsupported_projection` with exactly that slug and the construct's checked node id, for projection and for realizability, and returns no local graph, strategy or counterexample (implements FR-174-AC-3). | Test (TC-1415) |
| FR-575-AC-2 | A construct `qsl-eval` refuses (a protocol capture, `activation on each` with compensation, an unordered channel with a head or tail loss selector, an unenumerable step binder) yields `evaluator-unsupported` for realizability carrying the `qsl-eval` cause, except that a refused construct that has a row above (a symbolic capacity) reports that row's slug. | Test (TC-1415) |
| FR-575-AC-3 | A request with profile `bounded-static` over a protocol using only the profile's constructs is decided, and a request with any other profile word is refused `invalid_runtime_input`/`invalid-value` naming `profile` before any analysis. | Test (TC-1415) |
| FR-575-AC-4 | A `NegotiationCause` has exactly its three existing variants and a kind with no candidate still negotiates `unsupported_projection`/`unsupported-requested-capability`; the slugs above arise only from an engine result and add no code family. | Test (TC-1415) |

## Dependencies

- ADR-036 PA-4, PA-21, PA-22.
- [FR-057](FR-057-admit-shared-capability-kinds.md): `unsupported_projection` and its slugs.
- [FR-570](FR-570-project-a-global-protocol-onto-one-role.md), [FR-571](FR-571-decide-whether-local-strategies-realize-a-global-protocol.md), [FR-573](FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md).
- QSpec FR-174, FR-175.

## References

- ADR-036; QSL-1889, QSL-1893.
